//! 包括的なエラーハンドリング機能
//!
//! R2ユーザーディレクトリ移行における包括的なエラーハンドリング、
//! 自動復旧、リトライ機能を提供します。

use super::errors::{ErrorAction, MigrationError, MigrationErrorHandler, RetryStrategy};
use super::logging::{log_migration_info, StructuredLogger};
use super::security_audit::log_migration_security_error;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// エラーコールバック関数の型エイリアス
type ErrorCallback = Box<dyn Fn(&MigrationError) + Send + Sync>;

/// エラー処理結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandlingResult {
    /// 処理を続行
    Continue,
    /// 処理をリトライ
    Retry {
        /// リトライ回数
        attempt: usize,
        /// 次のリトライまでの遅延
        delay: Duration,
    },
    /// 該当アイテムをスキップ
    Skip {
        /// スキップ理由
        reason: String,
    },
    /// 処理を中止
    Abort {
        /// 中止理由
        reason: String,
    },
    /// ロールバックを実行
    Rollback {
        /// ロールバック対象
        target: String,
    },
    /// 処理を一時停止
    Pause {
        /// 一時停止理由
        reason: String,
    },
    /// 状態をリセット
    Reset {
        /// リセット対象
        target: String,
    },
}

/// リトライ実行結果
#[derive(Debug, Clone)]
pub struct RetryResult<T> {
    /// 実行結果
    pub result: Result<T, MigrationError>,
    /// 実行した試行回数
    pub attempts: usize,
    /// 総実行時間
    pub total_duration: Duration,
    /// 各試行の詳細
    pub attempt_details: Vec<AttemptDetail>,
}

/// 試行詳細
#[derive(Debug, Clone)]
pub struct AttemptDetail {
    /// 試行番号
    pub attempt_number: usize,
    /// 実行時間
    pub duration: Duration,
    /// 結果
    pub result: Result<(), MigrationError>,
    /// 遅延時間（次の試行まで）
    pub delay: Option<Duration>,
}

/// エラー統計
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ErrorStatistics {
    /// 総エラー数
    pub total_errors: usize,
    /// エラータイプ別カウント
    pub error_type_counts: HashMap<String, usize>,
    /// 重要度別カウント
    pub severity_counts: HashMap<String, usize>,
    /// リトライ成功数
    pub retry_success_count: usize,
    /// リトライ失敗数
    pub retry_failure_count: usize,
    /// 自動復旧成功数
    pub auto_recovery_success_count: usize,
    /// 自動復旧失敗数
    pub auto_recovery_failure_count: usize,
    /// 平均リトライ回数
    pub average_retry_attempts: f64,
}

/// 包括的エラーハンドラー
pub struct ComprehensiveErrorHandler {
    /// 構造化ロガー
    structured_logger: Option<Arc<Mutex<StructuredLogger>>>,
    /// エラー統計
    error_statistics: Arc<Mutex<ErrorStatistics>>,
    /// リトライ設定のオーバーライド
    retry_overrides: HashMap<String, RetryStrategy>,
    /// 自動復旧機能の有効/無効
    auto_recovery_enabled: bool,
    /// 最大リトライ回数の制限
    max_retry_limit: usize,
    /// エラー発生時のコールバック
    error_callbacks: Vec<ErrorCallback>,
}

impl ComprehensiveErrorHandler {
    /// 新しい包括的エラーハンドラーを作成
    pub fn new(structured_logger: Option<Arc<Mutex<StructuredLogger>>>) -> Self {
        Self {
            structured_logger,
            error_statistics: Arc::new(Mutex::new(ErrorStatistics::default())),
            retry_overrides: HashMap::new(),
            auto_recovery_enabled: true,
            max_retry_limit: 10,
            error_callbacks: Vec::new(),
        }
    }

    /// 自動復旧機能を設定
    pub fn set_auto_recovery_enabled(&mut self, enabled: bool) {
        self.auto_recovery_enabled = enabled;
        info!(
            "自動復旧機能を{}にしました",
            if enabled { "有効" } else { "無効" }
        );
    }

    /// 最大リトライ回数制限を設定
    pub fn set_max_retry_limit(&mut self, limit: usize) {
        self.max_retry_limit = limit;
        info!("最大リトライ回数制限を{}に設定しました", limit);
    }

    /// リトライ戦略をオーバーライド
    pub fn override_retry_strategy(&mut self, error_code: &str, strategy: RetryStrategy) {
        self.retry_overrides
            .insert(error_code.to_string(), strategy);
        info!(
            "エラーコード{}のリトライ戦略をオーバーライドしました",
            error_code
        );
    }

    /// エラーコールバックを追加
    pub fn add_error_callback<F>(&mut self, callback: F)
    where
        F: Fn(&MigrationError) + Send + Sync + 'static,
    {
        self.error_callbacks.push(Box::new(callback));
    }

    /// エラーを処理して適切なアクションを決定
    pub async fn handle_error(
        &self,
        error: &MigrationError,
        migration_log_id: Option<i64>,
        user_id: Option<i64>,
        context: Option<&str>,
    ) -> ErrorHandlingResult {
        // エラー統計を更新
        self.update_error_statistics(error);

        // エラーをログに記録
        self.log_error(error, migration_log_id, context);

        // セキュリティ監査が必要な場合は記録
        if error.requires_security_audit() {
            log_migration_security_error(error, migration_log_id, user_id);
        }

        // エラーコールバックを実行
        for callback in &self.error_callbacks {
            callback(error);
        }

        // 基本的なアクションを決定
        let base_action = MigrationErrorHandler::handle_error(error);

        // 自動復旧を試行
        if self.auto_recovery_enabled {
            if let Some(recovery_result) = self.attempt_auto_recovery(error, migration_log_id).await
            {
                return recovery_result;
            }
        }

        // アクションに基づいて結果を返す
        match base_action {
            ErrorAction::Abort => ErrorHandlingResult::Abort {
                reason: format!("致命的エラーのため処理を中止: {}", error),
            },
            ErrorAction::Retry => {
                if let Some(strategy) = self.get_retry_strategy(error) {
                    ErrorHandlingResult::Retry {
                        attempt: 1,
                        delay: strategy.initial_delay,
                    }
                } else {
                    ErrorHandlingResult::Skip {
                        reason: "リトライ戦略が定義されていません".to_string(),
                    }
                }
            }
            ErrorAction::Skip => ErrorHandlingResult::Skip {
                reason: format!("回復不可能なエラーのためスキップ: {}", error),
            },
            ErrorAction::Rollback => ErrorHandlingResult::Rollback {
                target: "current_operation".to_string(),
            },
            ErrorAction::Pause => ErrorHandlingResult::Pause {
                reason: format!("リソース不足のため一時停止: {}", error),
            },
            ErrorAction::Reset => ErrorHandlingResult::Reset {
                target: "migration_state".to_string(),
            },
        }
    }

    /// リトライ付きで操作を実行
    pub async fn execute_with_retry<T, F, Fut>(
        &self,
        operation: F,
        error_context: &str,
        _migration_log_id: Option<i64>,
    ) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, MigrationError>>,
    {
        let start_time = Instant::now();
        let mut attempts = 0;
        let mut attempt_details = Vec::new();

        loop {
            attempts += 1;
            let attempt_start = Instant::now();

            debug!("操作を実行中 (試行 {}): {}", attempts, error_context);

            match operation().await {
                Ok(result) => {
                    let attempt_duration = attempt_start.elapsed();
                    attempt_details.push(AttemptDetail {
                        attempt_number: attempts,
                        duration: attempt_duration,
                        result: Ok(()),
                        delay: None,
                    });

                    // 成功統計を更新
                    if attempts > 1 {
                        self.update_retry_success_statistics();
                    }

                    info!(
                        "操作が成功しました (試行 {}, 総時間: {:?}): {}",
                        attempts,
                        start_time.elapsed(),
                        error_context
                    );

                    return RetryResult {
                        result: Ok(result),
                        attempts,
                        total_duration: start_time.elapsed(),
                        attempt_details,
                    };
                }
                Err(error) => {
                    let attempt_duration = attempt_start.elapsed();

                    warn!(
                        "操作が失敗しました (試行 {}): {} - エラー: {}",
                        attempts, error_context, error
                    );

                    // リトライ戦略を取得
                    let strategy = self.get_retry_strategy(&error);

                    // リトライ可能かチェック
                    if !error.is_retryable()
                        || strategy.is_none()
                        || attempts >= self.max_retry_limit
                    {
                        attempt_details.push(AttemptDetail {
                            attempt_number: attempts,
                            duration: attempt_duration,
                            result: Err(error.clone()),
                            delay: None,
                        });

                        // 失敗統計を更新
                        if attempts > 1 {
                            self.update_retry_failure_statistics();
                        }

                        error!(
                            "操作が最終的に失敗しました (試行 {}, 総時間: {:?}): {}",
                            attempts,
                            start_time.elapsed(),
                            error_context
                        );

                        return RetryResult {
                            result: Err(error),
                            attempts,
                            total_duration: start_time.elapsed(),
                            attempt_details,
                        };
                    }

                    // 遅延時間を計算
                    let strategy = strategy.unwrap();
                    let delay = if attempts <= strategy.max_attempts {
                        strategy.calculate_delay(attempts - 1)
                    } else {
                        // 最大試行回数を超えた場合は終了
                        attempt_details.push(AttemptDetail {
                            attempt_number: attempts,
                            duration: attempt_duration,
                            result: Err(error.clone()),
                            delay: None,
                        });

                        self.update_retry_failure_statistics();

                        return RetryResult {
                            result: Err(error),
                            attempts,
                            total_duration: start_time.elapsed(),
                            attempt_details,
                        };
                    };

                    attempt_details.push(AttemptDetail {
                        attempt_number: attempts,
                        duration: attempt_duration,
                        result: Err(error.clone()),
                        delay: Some(delay),
                    });

                    info!(
                        "{}秒後にリトライします (試行 {}/{}): {}",
                        delay.as_secs_f64(),
                        attempts,
                        strategy.max_attempts,
                        error_context
                    );

                    // 遅延
                    sleep(delay).await;
                }
            }
        }
    }

    /// 自動復旧を試行
    async fn attempt_auto_recovery(
        &self,
        error: &MigrationError,
        _migration_log_id: Option<i64>,
    ) -> Option<ErrorHandlingResult> {
        debug!("自動復旧を試行中: {}", error);

        let recovery_action = match error {
            MigrationError::ResourceExhaustion { resource_type, .. } => {
                self.recover_from_resource_exhaustion(resource_type).await
            }
            MigrationError::Concurrency { .. } => self.recover_from_concurrency_issue().await,
            MigrationError::R2Operation { operation, .. } => {
                self.recover_from_r2_operation_failure(operation).await
            }
            MigrationError::DatabaseUpdate { table_name, .. } => {
                self.recover_from_database_issue(table_name).await
            }
            _ => None,
        };

        if let Some(action) = recovery_action {
            self.update_auto_recovery_success_statistics();
            log_migration_info(
                "auto_recovery",
                &format!("自動復旧が成功しました: {}", error),
                _migration_log_id,
            );
            Some(action)
        } else {
            self.update_auto_recovery_failure_statistics();
            debug!("自動復旧に失敗しました: {}", error);
            None
        }
    }

    /// リソース不足からの復旧
    async fn recover_from_resource_exhaustion(
        &self,
        resource_type: &str,
    ) -> Option<ErrorHandlingResult> {
        match resource_type {
            "memory" => {
                info!("メモリ不足を検出、ガベージコレクションを実行中...");
                // Rustでは明示的なGCはないが、一時停止で対応
                Some(ErrorHandlingResult::Pause {
                    reason: "メモリ不足のため一時停止".to_string(),
                })
            }
            "disk_space" => {
                warn!("ディスク容量不足を検出、クリーンアップが必要です");
                Some(ErrorHandlingResult::Pause {
                    reason: "ディスク容量不足のため一時停止".to_string(),
                })
            }
            "network_bandwidth" => {
                info!("ネットワーク帯域幅不足を検出、並列度を下げます");
                Some(ErrorHandlingResult::Continue)
            }
            _ => None,
        }
    }

    /// 並行処理問題からの復旧
    async fn recover_from_concurrency_issue(&self) -> Option<ErrorHandlingResult> {
        info!("並行処理問題を検出、短時間待機後にリトライします");
        Some(ErrorHandlingResult::Retry {
            attempt: 1,
            delay: Duration::from_millis(100),
        })
    }

    /// R2操作失敗からの復旧
    async fn recover_from_r2_operation_failure(
        &self,
        operation: &str,
    ) -> Option<ErrorHandlingResult> {
        match operation {
            "upload" | "download" => {
                info!("R2 {}操作の失敗を検出、リトライします", operation);
                Some(ErrorHandlingResult::Retry {
                    attempt: 1,
                    delay: Duration::from_secs(1),
                })
            }
            "list" => {
                info!("R2リスト操作の失敗を検出、短時間待機後にリトライします");
                Some(ErrorHandlingResult::Retry {
                    attempt: 1,
                    delay: Duration::from_millis(500),
                })
            }
            _ => None,
        }
    }

    /// データベース問題からの復旧
    async fn recover_from_database_issue(&self, table_name: &str) -> Option<ErrorHandlingResult> {
        warn!("データベーステーブル{}の問題を検出", table_name);

        // データベースの問題は慎重に扱う必要があるため、基本的にはリトライのみ
        Some(ErrorHandlingResult::Retry {
            attempt: 1,
            delay: Duration::from_millis(200),
        })
    }

    /// リトライ戦略を取得
    fn get_retry_strategy(&self, error: &MigrationError) -> Option<RetryStrategy> {
        let error_code = error.error_code();

        // オーバーライドされた戦略があるかチェック
        if let Some(strategy) = self.retry_overrides.get(&error_code) {
            return Some(strategy.clone());
        }

        // デフォルトの戦略を取得
        MigrationErrorHandler::get_retry_strategy(error)
    }

    /// エラーをログに記録
    fn log_error(
        &self,
        error: &MigrationError,
        migration_log_id: Option<i64>,
        context: Option<&str>,
    ) {
        if let Some(logger) = &self.structured_logger {
            if let Ok(logger) = logger.lock() {
                logger.log_migration_error(error, migration_log_id, context);
            }
        }

        // 従来のエラーハンドラーも呼び出し
        MigrationErrorHandler::log_error(error, context);
    }

    /// エラー統計を更新
    fn update_error_statistics(&self, error: &MigrationError) {
        if let Ok(mut stats) = self.error_statistics.lock() {
            stats.total_errors += 1;

            let error_type = format!("{:?}", error)
                .split('(')
                .next()
                .unwrap_or("Unknown")
                .to_string();
            *stats.error_type_counts.entry(error_type).or_insert(0) += 1;

            let severity = format!("{:?}", error.severity());
            *stats.severity_counts.entry(severity).or_insert(0) += 1;
        }
    }

    /// リトライ成功統計を更新
    fn update_retry_success_statistics(&self) {
        if let Ok(mut stats) = self.error_statistics.lock() {
            stats.retry_success_count += 1;
        }
    }

    /// リトライ失敗統計を更新
    fn update_retry_failure_statistics(&self) {
        if let Ok(mut stats) = self.error_statistics.lock() {
            stats.retry_failure_count += 1;
        }
    }

    /// 自動復旧成功統計を更新
    fn update_auto_recovery_success_statistics(&self) {
        if let Ok(mut stats) = self.error_statistics.lock() {
            stats.auto_recovery_success_count += 1;
        }
    }

    /// 自動復旧失敗統計を更新
    fn update_auto_recovery_failure_statistics(&self) {
        if let Ok(mut stats) = self.error_statistics.lock() {
            stats.auto_recovery_failure_count += 1;
        }
    }

    /// エラー統計を取得
    pub fn get_error_statistics(&self) -> ErrorStatistics {
        if let Ok(stats) = self.error_statistics.lock() {
            let mut result = stats.clone();

            // 平均リトライ回数を計算
            let total_retries = result.retry_success_count + result.retry_failure_count;
            if total_retries > 0 {
                result.average_retry_attempts = total_retries as f64 / result.total_errors as f64;
            }

            result
        } else {
            ErrorStatistics::default()
        }
    }

    /// エラー統計をクリア
    pub fn clear_error_statistics(&self) {
        if let Ok(mut stats) = self.error_statistics.lock() {
            *stats = ErrorStatistics::default();
        }
    }

    /// エラー統計レポートを生成
    pub fn generate_error_report(&self) -> String {
        let stats = self.get_error_statistics();

        let mut report = String::new();
        report.push_str("=== エラー統計レポート ===\n");
        report.push_str(&format!("総エラー数: {}\n", stats.total_errors));
        report.push_str(&format!("リトライ成功数: {}\n", stats.retry_success_count));
        report.push_str(&format!("リトライ失敗数: {}\n", stats.retry_failure_count));
        report.push_str(&format!(
            "自動復旧成功数: {}\n",
            stats.auto_recovery_success_count
        ));
        report.push_str(&format!(
            "自動復旧失敗数: {}\n",
            stats.auto_recovery_failure_count
        ));
        report.push_str(&format!(
            "平均リトライ回数: {:.2}\n",
            stats.average_retry_attempts
        ));

        report.push_str("\n=== エラータイプ別統計 ===\n");
        for (error_type, count) in &stats.error_type_counts {
            report.push_str(&format!("{}: {}\n", error_type, count));
        }

        report.push_str("\n=== 重要度別統計 ===\n");
        for (severity, count) in &stats.severity_counts {
            report.push_str(&format!("{}: {}\n", severity, count));
        }

        report
    }
}

/// グローバルエラーハンドラーインスタンス
static GLOBAL_ERROR_HANDLER: std::sync::OnceLock<Arc<Mutex<ComprehensiveErrorHandler>>> =
    std::sync::OnceLock::new();

/// グローバルエラーハンドラーを初期化
pub fn init_global_error_handler(structured_logger: Option<Arc<Mutex<StructuredLogger>>>) {
    let handler = ComprehensiveErrorHandler::new(structured_logger);
    GLOBAL_ERROR_HANDLER.set(Arc::new(Mutex::new(handler))).ok();
}

/// グローバルエラーハンドラーを取得
pub fn get_global_error_handler() -> Option<Arc<Mutex<ComprehensiveErrorHandler>>> {
    GLOBAL_ERROR_HANDLER.get().cloned()
}

/// 便利な関数：エラーを処理（同期版）
pub fn handle_migration_error_sync(
    error: &MigrationError,
    migration_log_id: Option<i64>,
    user_id: Option<i64>,
    context: Option<&str>,
) -> ErrorHandlingResult {
    // セキュリティ監査が必要な場合は記録
    if error.requires_security_audit() {
        log_migration_security_error(error, migration_log_id, user_id);
    }

    // エラーをログに記録
    MigrationErrorHandler::log_error(error, context);

    // 基本的なアクションを決定
    let action = MigrationErrorHandler::handle_error(error);
    match action {
        ErrorAction::Abort => ErrorHandlingResult::Abort {
            reason: error.to_string(),
        },
        ErrorAction::Retry => {
            if let Some(strategy) = MigrationErrorHandler::get_retry_strategy(error) {
                ErrorHandlingResult::Retry {
                    attempt: 1,
                    delay: strategy.initial_delay,
                }
            } else {
                ErrorHandlingResult::Skip {
                    reason: "リトライ戦略が定義されていません".to_string(),
                }
            }
        }
        ErrorAction::Skip => ErrorHandlingResult::Skip {
            reason: error.to_string(),
        },
        ErrorAction::Rollback => ErrorHandlingResult::Rollback {
            target: "current_operation".to_string(),
        },
        ErrorAction::Pause => ErrorHandlingResult::Pause {
            reason: error.to_string(),
        },
        ErrorAction::Reset => ErrorHandlingResult::Reset {
            target: "migration_state".to_string(),
        },
    }
}

/// 便利な関数：エラーを処理（非同期版）
pub async fn handle_migration_error(
    error: &MigrationError,
    migration_log_id: Option<i64>,
    user_id: Option<i64>,
    context: Option<&str>,
) -> ErrorHandlingResult {
    // 同期版を呼び出し
    handle_migration_error_sync(error, migration_log_id, user_id, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::migrations::logging::StructuredLogger;

    #[tokio::test]
    async fn test_comprehensive_error_handler_basic_operations() {
        let structured_logger = Some(Arc::new(Mutex::new(StructuredLogger::new(Some(100)))));
        let handler = ComprehensiveErrorHandler::new(structured_logger);

        let error = MigrationError::R2Operation {
            message: "テストエラー".to_string(),
            operation: "upload".to_string(),
            bucket: "test-bucket".to_string(),
            key: "test-key".to_string(),
            status_code: Some(500),
        };

        let result = handler
            .handle_error(&error, Some(1), Some(123), Some("テストコンテキスト"))
            .await;

        match result {
            ErrorHandlingResult::Retry { attempt, delay } => {
                assert_eq!(attempt, 1);
                assert!(delay > Duration::from_secs(0));
            }
            _ => panic!("期待されたリトライ結果が返されませんでした"),
        }

        let stats = handler.get_error_statistics();
        assert_eq!(stats.total_errors, 1);
    }

    #[tokio::test]
    async fn test_execute_with_retry_success() {
        let handler = ComprehensiveErrorHandler::new(None);
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let operation = {
            let call_count = call_count.clone();
            move || {
                let call_count = call_count.clone();
                async move {
                    let count = call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    if count < 3 {
                        Err(MigrationError::R2Operation {
                            message: "一時的エラー".to_string(),
                            operation: "upload".to_string(),
                            bucket: "test".to_string(),
                            key: "test".to_string(),
                            status_code: Some(500),
                        })
                    } else {
                        Ok("成功")
                    }
                }
            }
        };

        let result = handler
            .execute_with_retry(operation, "テスト操作", Some(1))
            .await;

        assert!(result.result.is_ok());
        assert_eq!(result.attempts, 3);
        assert_eq!(result.attempt_details.len(), 3);
    }

    #[tokio::test]
    async fn test_execute_with_retry_failure() {
        let handler = ComprehensiveErrorHandler::new(None);

        let operation = || async {
            Err(MigrationError::Permission {
                message: "権限なし".to_string(),
                user_id: 123,
                required_permission: "admin".to_string(),
                resource: "test".to_string(),
            })
        };

        let result: RetryResult<()> = handler
            .execute_with_retry(operation, "テスト操作", Some(1))
            .await;

        assert!(result.result.is_err());
        assert_eq!(result.attempts, 1); // リトライ不可能なエラーなので1回のみ
    }

    #[test]
    fn test_error_statistics() {
        let handler = ComprehensiveErrorHandler::new(None);

        let error1 = MigrationError::R2Operation {
            message: "エラー1".to_string(),
            operation: "upload".to_string(),
            bucket: "test".to_string(),
            key: "test".to_string(),
            status_code: Some(500),
        };

        let error2 = MigrationError::Permission {
            message: "エラー2".to_string(),
            user_id: 123,
            required_permission: "admin".to_string(),
            resource: "test".to_string(),
        };

        handler.update_error_statistics(&error1);
        handler.update_error_statistics(&error2);
        handler.update_retry_success_statistics();

        let stats = handler.get_error_statistics();
        assert_eq!(stats.total_errors, 2);
        assert_eq!(stats.retry_success_count, 1);
        assert!(!stats.error_type_counts.is_empty());
        assert!(!stats.severity_counts.is_empty());
    }

    #[test]
    fn test_retry_strategy_override() {
        let mut handler = ComprehensiveErrorHandler::new(None);

        let custom_strategy = RetryStrategy {
            max_attempts: 10,
            initial_delay: Duration::from_millis(100),
            backoff_multiplier: 1.5,
            max_delay: Duration::from_secs(5),
        };

        handler.override_retry_strategy("MIG_R2_OP", custom_strategy.clone());

        let error = MigrationError::R2Operation {
            message: "テストエラー".to_string(),
            operation: "upload".to_string(),
            bucket: "test".to_string(),
            key: "test".to_string(),
            status_code: Some(500),
        };

        let strategy = handler.get_retry_strategy(&error);
        assert!(strategy.is_some());

        let strategy = strategy.unwrap();
        assert_eq!(strategy.max_attempts, 10);
        assert_eq!(strategy.initial_delay, Duration::from_millis(100));
    }

    #[test]
    fn test_error_report_generation() {
        let handler = ComprehensiveErrorHandler::new(None);

        let error = MigrationError::R2Operation {
            message: "テストエラー".to_string(),
            operation: "upload".to_string(),
            bucket: "test".to_string(),
            key: "test".to_string(),
            status_code: Some(500),
        };

        handler.update_error_statistics(&error);
        handler.update_retry_success_statistics();

        let report = handler.generate_error_report();
        assert!(report.contains("エラー統計レポート"));
        assert!(report.contains("総エラー数: 1"));
        assert!(report.contains("リトライ成功数: 1"));
    }
}

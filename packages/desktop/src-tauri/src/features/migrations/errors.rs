//! 移行機能専用のエラー型とハンドリング
//!
//! R2ユーザーディレクトリ移行に特化したエラー型と、
//! 包括的なエラーハンドリング機能を提供します。

use crate::shared::errors::{AppError, ErrorSeverity};
use log::{error, warn};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 移行機能専用のエラー型
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum MigrationError {
    /// 移行前検証エラー
    #[error("移行前検証エラー: {message}")]
    PreValidation { message: String, details: String },

    /// 移行対象特定エラー
    #[error("移行対象特定エラー: {message}")]
    TargetIdentification { message: String, file_count: usize },

    /// ファイル移行エラー
    #[error("ファイル移行エラー: {message}")]
    FileMigration {
        message: String,
        old_path: String,
        new_path: String,
        user_id: i64,
        error_code: String,
    },

    /// データベース更新エラー
    #[error("データベース更新エラー: {message}")]
    DatabaseUpdate {
        message: String,
        table_name: String,
        operation: String,
        affected_rows: Option<usize>,
    },

    /// 整合性検証エラー
    #[error("整合性検証エラー: {message}")]
    IntegrityValidation {
        message: String,
        validation_type: String,
        expected: String,
        actual: String,
    },

    /// バッチ処理エラー
    #[error("バッチ処理エラー: {message}")]
    BatchProcessing {
        message: String,
        batch_index: usize,
        total_batches: usize,
        failed_items: Vec<String>,
    },

    /// 並列処理エラー
    #[error("並列処理エラー: {message}")]
    Concurrency {
        message: String,
        max_concurrency: usize,
        active_tasks: usize,
    },

    /// R2操作エラー
    #[error("R2操作エラー: {message}")]
    R2Operation {
        message: String,
        operation: String,
        bucket: String,
        key: String,
        status_code: Option<u16>,
    },

    /// 権限エラー
    #[error("権限エラー: {message}")]
    Permission {
        message: String,
        user_id: i64,
        required_permission: String,
        resource: String,
    },

    /// 設定エラー
    #[error("設定エラー: {message}")]
    Configuration {
        message: String,
        config_key: String,
        expected_type: String,
    },

    /// タイムアウトエラー
    #[error("タイムアウトエラー: {message}")]
    Timeout {
        message: String,
        operation: String,
        timeout_seconds: u64,
    },

    /// リソース不足エラー
    #[error("リソース不足エラー: {message}")]
    ResourceExhaustion {
        message: String,
        resource_type: String,
        current_usage: u64,
        limit: u64,
    },

    /// 移行状態エラー
    #[error("移行状態エラー: {message}")]
    MigrationState {
        message: String,
        current_state: String,
        expected_state: String,
        migration_id: i64,
    },

    /// 外部依存エラー
    #[error("外部依存エラー: {message}")]
    ExternalDependency {
        message: String,
        service_name: String,
        endpoint: String,
        retry_count: usize,
    },
}

impl MigrationError {
    /// エラーの重要度を取得
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            MigrationError::PreValidation { .. } => ErrorSeverity::High,
            MigrationError::TargetIdentification { .. } => ErrorSeverity::Medium,
            MigrationError::FileMigration { .. } => ErrorSeverity::High,
            MigrationError::DatabaseUpdate { .. } => ErrorSeverity::High,
            MigrationError::IntegrityValidation { .. } => ErrorSeverity::Critical,
            MigrationError::BatchProcessing { .. } => ErrorSeverity::High,
            MigrationError::Concurrency { .. } => ErrorSeverity::Medium,
            MigrationError::R2Operation { .. } => ErrorSeverity::Medium,
            MigrationError::Permission { .. } => ErrorSeverity::Critical,
            MigrationError::Configuration { .. } => ErrorSeverity::High,
            MigrationError::Timeout { .. } => ErrorSeverity::Medium,
            MigrationError::ResourceExhaustion { .. } => ErrorSeverity::High,
            MigrationError::MigrationState { .. } => ErrorSeverity::Medium,
            MigrationError::ExternalDependency { .. } => ErrorSeverity::Medium,
        }
    }

    /// エラーカテゴリを取得
    pub fn category(&self) -> &'static str {
        match self {
            MigrationError::PreValidation { .. } => "validation",
            MigrationError::TargetIdentification { .. } => "identification",
            MigrationError::FileMigration { .. } => "file_operation",
            MigrationError::DatabaseUpdate { .. } => "database",
            MigrationError::IntegrityValidation { .. } => "integrity",
            MigrationError::BatchProcessing { .. } => "batch_processing",
            MigrationError::Concurrency { .. } => "concurrency",
            MigrationError::R2Operation { .. } => "r2_operation",
            MigrationError::Permission { .. } => "permission",
            MigrationError::Configuration { .. } => "configuration",
            MigrationError::Timeout { .. } => "timeout",
            MigrationError::ResourceExhaustion { .. } => "resource",
            MigrationError::MigrationState { .. } => "state",
            MigrationError::ExternalDependency { .. } => "external",
        }
    }

    /// エラーコードを取得
    pub fn error_code(&self) -> String {
        match self {
            MigrationError::PreValidation { .. } => "MIG_PRE_VALIDATION".to_string(),
            MigrationError::TargetIdentification { .. } => "MIG_TARGET_ID".to_string(),
            MigrationError::FileMigration { error_code, .. } => error_code.clone(),
            MigrationError::DatabaseUpdate { .. } => "MIG_DB_UPDATE".to_string(),
            MigrationError::IntegrityValidation { .. } => "MIG_INTEGRITY".to_string(),
            MigrationError::BatchProcessing { .. } => "MIG_BATCH_PROC".to_string(),
            MigrationError::Concurrency { .. } => "MIG_CONCURRENCY".to_string(),
            MigrationError::R2Operation { .. } => "MIG_R2_OP".to_string(),
            MigrationError::Permission { .. } => "MIG_PERMISSION".to_string(),
            MigrationError::Configuration { .. } => "MIG_CONFIG".to_string(),
            MigrationError::Timeout { .. } => "MIG_TIMEOUT".to_string(),
            MigrationError::ResourceExhaustion { .. } => "MIG_RESOURCE".to_string(),
            MigrationError::MigrationState { .. } => "MIG_STATE".to_string(),
            MigrationError::ExternalDependency { .. } => "MIG_EXTERNAL".to_string(),
        }
    }

    /// ユーザー向けメッセージを取得
    pub fn user_message(&self) -> String {
        match self {
            MigrationError::PreValidation { .. } => {
                "移行前の検証でエラーが発生しました。システム管理者にお問い合わせください。"
                    .to_string()
            }
            MigrationError::TargetIdentification { .. } => {
                "移行対象ファイルの特定でエラーが発生しました。".to_string()
            }
            MigrationError::FileMigration { .. } => {
                "ファイルの移行処理でエラーが発生しました。".to_string()
            }
            MigrationError::DatabaseUpdate { .. } => {
                "データベースの更新でエラーが発生しました。".to_string()
            }
            MigrationError::IntegrityValidation { .. } => {
                "データの整合性チェックでエラーが発見されました。移行を中止します。".to_string()
            }
            MigrationError::BatchProcessing { .. } => {
                "バッチ処理でエラーが発生しました。".to_string()
            }
            MigrationError::Concurrency { .. } => "並列処理でエラーが発生しました。".to_string(),
            MigrationError::R2Operation { .. } => {
                "クラウドストレージ操作でエラーが発生しました。".to_string()
            }
            MigrationError::Permission { .. } => {
                "アクセス権限が不足しています。システム管理者にお問い合わせください。".to_string()
            }
            MigrationError::Configuration { .. } => {
                "システム設定にエラーがあります。システム管理者にお問い合わせください。".to_string()
            }
            MigrationError::Timeout { .. } => {
                "処理がタイムアウトしました。しばらく時間をおいて再試行してください。".to_string()
            }
            MigrationError::ResourceExhaustion { .. } => {
                "システムリソースが不足しています。しばらく時間をおいて再試行してください。"
                    .to_string()
            }
            MigrationError::MigrationState { .. } => "移行処理の状態が不正です。".to_string(),
            MigrationError::ExternalDependency { .. } => {
                "外部サービスとの通信でエラーが発生しました。".to_string()
            }
        }
    }

    /// 構造化ログ用のメタデータを取得
    pub fn log_metadata(&self) -> serde_json::Value {
        match self {
            MigrationError::PreValidation { message, details } => {
                serde_json::json!({
                    "error_type": "pre_validation",
                    "message": message,
                    "details": details
                })
            }
            MigrationError::TargetIdentification {
                message,
                file_count,
            } => {
                serde_json::json!({
                    "error_type": "target_identification",
                    "message": message,
                    "file_count": file_count
                })
            }
            MigrationError::FileMigration {
                message,
                old_path,
                new_path,
                user_id,
                error_code,
            } => {
                serde_json::json!({
                    "error_type": "file_migration",
                    "message": message,
                    "old_path": old_path,
                    "new_path": new_path,
                    "user_id": user_id,
                    "error_code": error_code
                })
            }
            MigrationError::DatabaseUpdate {
                message,
                table_name,
                operation,
                affected_rows,
            } => {
                serde_json::json!({
                    "error_type": "database_update",
                    "message": message,
                    "table_name": table_name,
                    "operation": operation,
                    "affected_rows": affected_rows
                })
            }
            MigrationError::IntegrityValidation {
                message,
                validation_type,
                expected,
                actual,
            } => {
                serde_json::json!({
                    "error_type": "integrity_validation",
                    "message": message,
                    "validation_type": validation_type,
                    "expected": expected,
                    "actual": actual
                })
            }
            MigrationError::BatchProcessing {
                message,
                batch_index,
                total_batches,
                failed_items,
            } => {
                serde_json::json!({
                    "error_type": "batch_processing",
                    "message": message,
                    "batch_index": batch_index,
                    "total_batches": total_batches,
                    "failed_items": failed_items
                })
            }
            MigrationError::Concurrency {
                message,
                max_concurrency,
                active_tasks,
            } => {
                serde_json::json!({
                    "error_type": "concurrency",
                    "message": message,
                    "max_concurrency": max_concurrency,
                    "active_tasks": active_tasks
                })
            }
            MigrationError::R2Operation {
                message,
                operation,
                bucket,
                key,
                status_code,
            } => {
                serde_json::json!({
                    "error_type": "r2_operation",
                    "message": message,
                    "operation": operation,
                    "bucket": bucket,
                    "key": key,
                    "status_code": status_code
                })
            }
            MigrationError::Permission {
                message,
                user_id,
                required_permission,
                resource,
            } => {
                serde_json::json!({
                    "error_type": "permission",
                    "message": message,
                    "user_id": user_id,
                    "required_permission": required_permission,
                    "resource": resource
                })
            }
            MigrationError::Configuration {
                message,
                config_key,
                expected_type,
            } => {
                serde_json::json!({
                    "error_type": "configuration",
                    "message": message,
                    "config_key": config_key,
                    "expected_type": expected_type
                })
            }
            MigrationError::Timeout {
                message,
                operation,
                timeout_seconds,
            } => {
                serde_json::json!({
                    "error_type": "timeout",
                    "message": message,
                    "operation": operation,
                    "timeout_seconds": timeout_seconds
                })
            }
            MigrationError::ResourceExhaustion {
                message,
                resource_type,
                current_usage,
                limit,
            } => {
                serde_json::json!({
                    "error_type": "resource_exhaustion",
                    "message": message,
                    "resource_type": resource_type,
                    "current_usage": current_usage,
                    "limit": limit
                })
            }
            MigrationError::MigrationState {
                message,
                current_state,
                expected_state,
                migration_id,
            } => {
                serde_json::json!({
                    "error_type": "migration_state",
                    "message": message,
                    "current_state": current_state,
                    "expected_state": expected_state,
                    "migration_id": migration_id
                })
            }
            MigrationError::ExternalDependency {
                message,
                service_name,
                endpoint,
                retry_count,
            } => {
                serde_json::json!({
                    "error_type": "external_dependency",
                    "message": message,
                    "service_name": service_name,
                    "endpoint": endpoint,
                    "retry_count": retry_count
                })
            }
        }
    }

    /// セキュリティ監査が必要かどうかを判定
    pub fn requires_security_audit(&self) -> bool {
        matches!(
            self,
            MigrationError::Permission { .. }
                | MigrationError::IntegrityValidation { .. }
                | MigrationError::Configuration { .. }
        )
    }

    /// 自動リトライが可能かどうかを判定
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            MigrationError::R2Operation { .. }
                | MigrationError::Timeout { .. }
                | MigrationError::ExternalDependency { .. }
                | MigrationError::Concurrency { .. }
        )
    }

    /// エラーの詳細情報を取得（デバッグ用）
    pub fn debug_info(&self) -> String {
        format!("{self:?}")
    }
}

/// MigrationErrorからAppErrorへの変換
impl From<MigrationError> for AppError {
    fn from(migration_error: MigrationError) -> Self {
        match migration_error.severity() {
            ErrorSeverity::Critical => AppError::Security(migration_error.to_string()),
            ErrorSeverity::High => AppError::Database(migration_error.to_string()),
            ErrorSeverity::Medium => AppError::ExternalService(migration_error.to_string()),
            ErrorSeverity::Low => AppError::Validation(migration_error.to_string()),
        }
    }
}

/// AppErrorからMigrationErrorへの変換（可能な場合）
impl TryFrom<AppError> for MigrationError {
    type Error = AppError;

    fn try_from(app_error: AppError) -> Result<Self, Self::Error> {
        match app_error {
            AppError::Database(msg) => Ok(MigrationError::DatabaseUpdate {
                message: msg,
                table_name: "unknown".to_string(),
                operation: "unknown".to_string(),
                affected_rows: None,
            }),
            AppError::Validation(msg) => Ok(MigrationError::PreValidation {
                message: msg,
                details: "Converted from AppError::Validation".to_string(),
            }),
            AppError::Security(msg) => Ok(MigrationError::Permission {
                message: msg,
                user_id: 0,
                required_permission: "unknown".to_string(),
                resource: "unknown".to_string(),
            }),
            AppError::ExternalService(msg) => Ok(MigrationError::ExternalDependency {
                message: msg,
                service_name: "unknown".to_string(),
                endpoint: "unknown".to_string(),
                retry_count: 0,
            }),
            AppError::Configuration(msg) => Ok(MigrationError::Configuration {
                message: msg,
                config_key: "unknown".to_string(),
                expected_type: "unknown".to_string(),
            }),
            AppError::Concurrency(msg) => Ok(MigrationError::Concurrency {
                message: msg,
                max_concurrency: 0,
                active_tasks: 0,
            }),
            AppError::R2(msg) => Ok(MigrationError::R2Operation {
                message: msg,
                operation: "unknown".to_string(),
                bucket: "unknown".to_string(),
                key: "unknown".to_string(),
                status_code: None,
            }),
            _ => Err(app_error),
        }
    }
}

/// エラーハンドリングヘルパー関数
pub struct MigrationErrorHandler;

impl MigrationErrorHandler {
    /// エラーをログに記録
    pub fn log_error(error: &MigrationError, context: Option<&str>) {
        let metadata = error.log_metadata();
        let context_str = context.unwrap_or("unknown");

        match error.severity() {
            ErrorSeverity::Critical => {
                error!(
                    "CRITICAL Migration Error [{}]: {} | Context: {} | Metadata: {}",
                    error.error_code(),
                    error,
                    context_str,
                    metadata
                );
            }
            ErrorSeverity::High => {
                error!(
                    "HIGH Migration Error [{}]: {} | Context: {} | Metadata: {}",
                    error.error_code(),
                    error,
                    context_str,
                    metadata
                );
            }
            ErrorSeverity::Medium => {
                warn!(
                    "MEDIUM Migration Error [{}]: {} | Context: {} | Metadata: {}",
                    error.error_code(),
                    error,
                    context_str,
                    metadata
                );
            }
            ErrorSeverity::Low => {
                warn!(
                    "LOW Migration Error [{}]: {} | Context: {} | Metadata: {}",
                    error.error_code(),
                    error,
                    context_str,
                    metadata
                );
            }
        }
    }

    /// エラーを処理して適切なアクションを決定
    pub fn handle_error(error: &MigrationError) -> ErrorAction {
        match error {
            MigrationError::PreValidation { .. } => ErrorAction::Abort,
            MigrationError::TargetIdentification { .. } => ErrorAction::Retry,
            MigrationError::FileMigration { .. } => ErrorAction::Skip,
            MigrationError::DatabaseUpdate { .. } => ErrorAction::Rollback,
            MigrationError::IntegrityValidation { .. } => ErrorAction::Abort,
            MigrationError::BatchProcessing { .. } => ErrorAction::Retry,
            MigrationError::Concurrency { .. } => ErrorAction::Retry,
            MigrationError::R2Operation { .. } => ErrorAction::Retry,
            MigrationError::Permission { .. } => ErrorAction::Abort,
            MigrationError::Configuration { .. } => ErrorAction::Abort,
            MigrationError::Timeout { .. } => ErrorAction::Retry,
            MigrationError::ResourceExhaustion { .. } => ErrorAction::Pause,
            MigrationError::MigrationState { .. } => ErrorAction::Reset,
            MigrationError::ExternalDependency { .. } => ErrorAction::Retry,
        }
    }

    /// リトライ戦略を取得
    pub fn get_retry_strategy(error: &MigrationError) -> Option<RetryStrategy> {
        if !error.is_retryable() {
            return None;
        }

        match error {
            MigrationError::R2Operation { .. } => Some(RetryStrategy {
                max_attempts: 3,
                initial_delay: std::time::Duration::from_secs(1),
                backoff_multiplier: 2.0,
                max_delay: std::time::Duration::from_secs(30),
            }),
            MigrationError::Timeout { .. } => Some(RetryStrategy {
                max_attempts: 2,
                initial_delay: std::time::Duration::from_secs(5),
                backoff_multiplier: 1.5,
                max_delay: std::time::Duration::from_secs(60),
            }),
            MigrationError::ExternalDependency { .. } => Some(RetryStrategy {
                max_attempts: 5,
                initial_delay: std::time::Duration::from_millis(500),
                backoff_multiplier: 2.0,
                max_delay: std::time::Duration::from_secs(15),
            }),
            MigrationError::Concurrency { .. } => Some(RetryStrategy {
                max_attempts: 3,
                initial_delay: std::time::Duration::from_millis(100),
                backoff_multiplier: 1.2,
                max_delay: std::time::Duration::from_secs(5),
            }),
            _ => None,
        }
    }
}

/// エラー発生時のアクション
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorAction {
    /// 処理を中止
    Abort,
    /// 処理をリトライ
    Retry,
    /// 該当アイテムをスキップして続行
    Skip,
    /// ロールバックを実行
    Rollback,
    /// 処理を一時停止
    Pause,
    /// 状態をリセット
    Reset,
}

/// リトライ戦略
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    /// 最大試行回数
    pub max_attempts: usize,
    /// 初期遅延時間
    pub initial_delay: std::time::Duration,
    /// バックオフ倍率
    pub backoff_multiplier: f64,
    /// 最大遅延時間
    pub max_delay: std::time::Duration,
}

impl RetryStrategy {
    /// 指定された試行回数に対する遅延時間を計算
    pub fn calculate_delay(&self, attempt: usize) -> std::time::Duration {
        if attempt == 0 {
            return self.initial_delay;
        }

        let delay_ms =
            self.initial_delay.as_millis() as f64 * self.backoff_multiplier.powi(attempt as i32);

        let delay = std::time::Duration::from_millis(delay_ms as u64);

        if delay > self.max_delay {
            self.max_delay
        } else {
            delay
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_error_severity() {
        let error = MigrationError::Permission {
            message: "テスト".to_string(),
            user_id: 123,
            required_permission: "admin".to_string(),
            resource: "migration".to_string(),
        };

        assert_eq!(error.severity(), ErrorSeverity::Critical);
        assert_eq!(error.category(), "permission");
        assert_eq!(error.error_code(), "MIG_PERMISSION");
        assert!(error.requires_security_audit());
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_migration_error_log_metadata() {
        let error = MigrationError::FileMigration {
            message: "テストエラー".to_string(),
            old_path: "old/path".to_string(),
            new_path: "new/path".to_string(),
            user_id: 456,
            error_code: "TEST_ERROR".to_string(),
        };

        let metadata = error.log_metadata();
        assert_eq!(metadata["error_type"], "file_migration");
        assert_eq!(metadata["message"], "テストエラー");
        assert_eq!(metadata["old_path"], "old/path");
        assert_eq!(metadata["new_path"], "new/path");
        assert_eq!(metadata["user_id"], 456);
        assert_eq!(metadata["error_code"], "TEST_ERROR");
    }

    #[test]
    fn test_error_action_handling() {
        let permission_error = MigrationError::Permission {
            message: "権限なし".to_string(),
            user_id: 123,
            required_permission: "admin".to_string(),
            resource: "migration".to_string(),
        };

        let action = MigrationErrorHandler::handle_error(&permission_error);
        assert_eq!(action, ErrorAction::Abort);

        let timeout_error = MigrationError::Timeout {
            message: "タイムアウト".to_string(),
            operation: "upload".to_string(),
            timeout_seconds: 30,
        };

        let action = MigrationErrorHandler::handle_error(&timeout_error);
        assert_eq!(action, ErrorAction::Retry);
    }

    #[test]
    fn test_retry_strategy() {
        let error = MigrationError::R2Operation {
            message: "R2エラー".to_string(),
            operation: "upload".to_string(),
            bucket: "test-bucket".to_string(),
            key: "test-key".to_string(),
            status_code: Some(500),
        };

        let strategy = MigrationErrorHandler::get_retry_strategy(&error);
        assert!(strategy.is_some());

        let strategy = strategy.unwrap();
        assert_eq!(strategy.max_attempts, 3);
        assert_eq!(strategy.initial_delay, std::time::Duration::from_secs(1));

        // 遅延時間の計算テスト
        let delay1 = strategy.calculate_delay(0);
        let delay2 = strategy.calculate_delay(1);
        let delay3 = strategy.calculate_delay(2);

        assert_eq!(delay1, std::time::Duration::from_secs(1));
        assert_eq!(delay2, std::time::Duration::from_secs(2));
        assert_eq!(delay3, std::time::Duration::from_secs(4));
    }

    #[test]
    fn test_migration_error_conversion() {
        let migration_error = MigrationError::DatabaseUpdate {
            message: "DB更新エラー".to_string(),
            table_name: "expenses".to_string(),
            operation: "UPDATE".to_string(),
            affected_rows: Some(0),
        };

        let app_error: AppError = migration_error.into();
        assert!(matches!(app_error, AppError::Database(_)));

        // 逆変換のテスト
        let app_error = AppError::Validation("バリデーションエラー".to_string());
        let migration_error: Result<MigrationError, _> = app_error.try_into();
        assert!(migration_error.is_ok());
        assert!(matches!(
            migration_error.unwrap(),
            MigrationError::PreValidation { .. }
        ));
    }

    #[test]
    fn test_user_message() {
        let error = MigrationError::IntegrityValidation {
            message: "ハッシュ不一致".to_string(),
            validation_type: "file_hash".to_string(),
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };

        let user_msg = error.user_message();
        assert!(user_msg.contains("整合性チェック"));
        assert!(user_msg.contains("中止"));
    }
}

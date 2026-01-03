//! 移行機能専用の構造化ログ機能
//!
//! R2ユーザーディレクトリ移行に特化した構造化ログ出力と
//! ログ分析機能を提供します。

use super::errors::{MigrationError, MigrationErrorHandler};
use crate::shared::errors::ErrorSeverity;
use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// ログレベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    /// デバッグ情報
    Debug,
    /// 一般情報
    Info,
    /// 警告
    Warn,
    /// エラー
    Error,
    /// 致命的エラー
    Critical,
}

impl From<ErrorSeverity> for LogLevel {
    fn from(severity: ErrorSeverity) -> Self {
        match severity {
            ErrorSeverity::Low => LogLevel::Info,
            ErrorSeverity::Medium => LogLevel::Warn,
            ErrorSeverity::High => LogLevel::Error,
            ErrorSeverity::Critical => LogLevel::Critical,
        }
    }
}

/// 構造化ログエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredLogEntry {
    /// タイムスタンプ（JST）
    pub timestamp: DateTime<Utc>,
    /// ログレベル
    pub level: LogLevel,
    /// ログカテゴリ
    pub category: String,
    /// メッセージ
    pub message: String,
    /// 移行ログID（関連する場合）
    pub migration_log_id: Option<i64>,
    /// ユーザーID（関連する場合）
    pub user_id: Option<i64>,
    /// コンテキスト情報
    pub context: HashMap<String, serde_json::Value>,
    /// エラーコード（エラーの場合）
    pub error_code: Option<String>,
    /// スタックトレース（エラーの場合）
    pub stack_trace: Option<String>,
    /// セッションID
    pub session_id: Option<String>,
    /// 処理ID（バッチ処理など）
    pub process_id: Option<String>,
}

impl StructuredLogEntry {
    /// 新しいログエントリを作成
    pub fn new(
        level: LogLevel,
        category: &str,
        message: &str,
        migration_log_id: Option<i64>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            category: category.to_string(),
            message: message.to_string(),
            migration_log_id,
            user_id: None,
            context: HashMap::new(),
            error_code: None,
            stack_trace: None,
            session_id: None,
            process_id: None,
        }
    }

    /// コンテキスト情報を追加
    pub fn with_context(mut self, key: &str, value: serde_json::Value) -> Self {
        self.context.insert(key.to_string(), value);
        self
    }

    /// ユーザーIDを設定
    pub fn with_user_id(mut self, user_id: i64) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// エラーコードを設定
    pub fn with_error_code(mut self, error_code: &str) -> Self {
        self.error_code = Some(error_code.to_string());
        self
    }

    /// スタックトレースを設定
    pub fn with_stack_trace(mut self, stack_trace: &str) -> Self {
        self.stack_trace = Some(stack_trace.to_string());
        self
    }

    /// セッションIDを設定
    pub fn with_session_id(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    /// 処理IDを設定
    pub fn with_process_id(mut self, process_id: &str) -> Self {
        self.process_id = Some(process_id.to_string());
        self
    }

    /// JSON形式でシリアライズ
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// 人間が読みやすい形式でフォーマット
    pub fn to_human_readable(&self) -> String {
        let level_str = match self.level {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Critical => "CRITICAL",
        };

        let mut parts = vec![
            format!(
                "[{}]",
                self.timestamp
                    .with_timezone(&chrono_tz::Asia::Tokyo)
                    .format("%Y-%m-%d %H:%M:%S JST")
            ),
            format!("[{}]", level_str),
            format!("[{}]", self.category),
        ];

        if let Some(migration_id) = self.migration_log_id {
            parts.push(format!("[MIG:{}]", migration_id));
        }

        if let Some(user_id) = self.user_id {
            parts.push(format!("[USER:{}]", user_id));
        }

        if let Some(error_code) = &self.error_code {
            parts.push(format!("[{}]", error_code));
        }

        parts.push(self.message.clone());

        if !self.context.is_empty() {
            parts.push(format!(
                "Context: {}",
                serde_json::to_string(&self.context).unwrap_or_default()
            ));
        }

        parts.join(" ")
    }
}

/// 構造化ログ出力機能
pub struct StructuredLogger {
    /// ログエントリのバッファ
    log_buffer: Arc<Mutex<Vec<StructuredLogEntry>>>,
    /// 最大バッファサイズ
    max_buffer_size: usize,
    /// 現在のセッションID
    session_id: Option<String>,
}

/// ファイル移行ログのパラメータ
#[derive(Debug)]
pub struct FileMigrationLogParams {
    pub migration_log_id: i64,
    pub old_path: String,
    pub new_path: String,
    pub user_id: i64,
    pub file_size: u64,
    pub success: bool,
    pub duration_ms: Option<u128>,
}

impl StructuredLogger {
    /// 新しい構造化ロガーを作成
    pub fn new(max_buffer_size: Option<usize>) -> Self {
        Self {
            log_buffer: Arc::new(Mutex::new(Vec::new())),
            max_buffer_size: max_buffer_size.unwrap_or(1000),
            session_id: None,
        }
    }

    /// セッションIDを設定
    pub fn set_session_id(&mut self, session_id: &str) {
        self.session_id = Some(session_id.to_string());
    }

    /// ログエントリを記録
    pub fn log(&self, mut entry: StructuredLogEntry) {
        // セッションIDを自動設定
        if entry.session_id.is_none() {
            entry.session_id = self.session_id.clone();
        }

        // 標準ログにも出力
        let human_readable = entry.to_human_readable();
        match entry.level {
            LogLevel::Debug => debug!("{}", human_readable),
            LogLevel::Info => info!("{}", human_readable),
            LogLevel::Warn => warn!("{}", human_readable),
            LogLevel::Error => error!("{}", human_readable),
            LogLevel::Critical => error!("CRITICAL: {}", human_readable),
        }

        // バッファに追加
        if let Ok(mut buffer) = self.log_buffer.lock() {
            buffer.push(entry);

            // バッファサイズ制限
            if buffer.len() > self.max_buffer_size {
                buffer.remove(0);
            }
        }
    }

    /// デバッグログを記録
    pub fn debug(&self, category: &str, message: &str, migration_log_id: Option<i64>) {
        let entry = StructuredLogEntry::new(LogLevel::Debug, category, message, migration_log_id);
        self.log(entry);
    }

    /// 情報ログを記録
    pub fn info(&self, category: &str, message: &str, migration_log_id: Option<i64>) {
        let entry = StructuredLogEntry::new(LogLevel::Info, category, message, migration_log_id);
        self.log(entry);
    }

    /// 警告ログを記録
    pub fn warn(&self, category: &str, message: &str, migration_log_id: Option<i64>) {
        let entry = StructuredLogEntry::new(LogLevel::Warn, category, message, migration_log_id);
        self.log(entry);
    }

    /// エラーログを記録
    pub fn error(&self, category: &str, message: &str, migration_log_id: Option<i64>) {
        let entry = StructuredLogEntry::new(LogLevel::Error, category, message, migration_log_id);
        self.log(entry);
    }

    /// 致命的エラーログを記録
    pub fn critical(&self, category: &str, message: &str, migration_log_id: Option<i64>) {
        let entry =
            StructuredLogEntry::new(LogLevel::Critical, category, message, migration_log_id);
        self.log(entry);
    }

    /// MigrationErrorからログを記録
    pub fn log_migration_error(
        &self,
        error: &MigrationError,
        migration_log_id: Option<i64>,
        context: Option<&str>,
    ) {
        let level = LogLevel::from(error.severity());
        let mut entry = StructuredLogEntry::new(
            level,
            error.category(),
            &error.to_string(),
            migration_log_id,
        )
        .with_error_code(&error.error_code());

        // エラーのメタデータをコンテキストに追加
        let metadata = error.log_metadata();
        if let serde_json::Value::Object(map) = metadata {
            for (key, value) in map {
                entry = entry.with_context(&key, value);
            }
        }

        // 追加のコンテキスト情報
        if let Some(ctx) = context {
            entry = entry.with_context(
                "additional_context",
                serde_json::Value::String(ctx.to_string()),
            );
        }

        // スタックトレース（デバッグ情報）
        entry = entry.with_stack_trace(&error.debug_info());

        self.log(entry);

        // セキュリティ監査が必要な場合は別途記録
        if error.requires_security_audit() {
            self.log_security_event(error, migration_log_id);
        }
    }

    /// セキュリティイベントを記録
    fn log_security_event(&self, error: &MigrationError, migration_log_id: Option<i64>) {
        let entry = StructuredLogEntry::new(
            LogLevel::Critical,
            "security_audit",
            &format!("セキュリティ監査要求: {}", error),
            migration_log_id,
        )
        .with_error_code(&format!("SECURITY_{}", error.error_code()))
        .with_context(
            "audit_reason",
            serde_json::Value::String("migration_security_violation".to_string()),
        )
        .with_context("error_metadata", error.log_metadata());

        self.log(entry);
    }

    /// 移行開始ログを記録
    pub fn log_migration_start(
        &self,
        migration_log_id: i64,
        total_items: usize,
        batch_size: usize,
        dry_run: bool,
    ) {
        let entry = StructuredLogEntry::new(
            LogLevel::Info,
            "migration_lifecycle",
            "R2ユーザーディレクトリ移行を開始しました",
            Some(migration_log_id),
        )
        .with_context(
            "total_items",
            serde_json::Value::Number(serde_json::Number::from(total_items)),
        )
        .with_context(
            "batch_size",
            serde_json::Value::Number(serde_json::Number::from(batch_size)),
        )
        .with_context("dry_run", serde_json::Value::Bool(dry_run))
        .with_context(
            "event_type",
            serde_json::Value::String("migration_start".to_string()),
        );

        self.log(entry);
    }

    /// 移行完了ログを記録
    pub fn log_migration_complete(
        &self,
        migration_log_id: i64,
        success_count: usize,
        error_count: usize,
        duration_ms: u128,
    ) {
        let level = if error_count > 0 {
            LogLevel::Warn
        } else {
            LogLevel::Info
        };

        let entry = StructuredLogEntry::new(
            level,
            "migration_lifecycle",
            "R2ユーザーディレクトリ移行が完了しました",
            Some(migration_log_id),
        )
        .with_context(
            "success_count",
            serde_json::Value::Number(serde_json::Number::from(success_count)),
        )
        .with_context(
            "error_count",
            serde_json::Value::Number(serde_json::Number::from(error_count)),
        )
        .with_context(
            "duration_ms",
            serde_json::Value::Number(serde_json::Number::from(duration_ms as u64)),
        )
        .with_context(
            "event_type",
            serde_json::Value::String("migration_complete".to_string()),
        );

        self.log(entry);
    }

    /// バッチ処理ログを記録
    pub fn log_batch_progress(
        &self,
        migration_log_id: i64,
        batch_index: usize,
        total_batches: usize,
        batch_success: usize,
        batch_errors: usize,
    ) {
        let entry = StructuredLogEntry::new(
            LogLevel::Info,
            "batch_processing",
            &format!("バッチ {}/{} 完了", batch_index + 1, total_batches),
            Some(migration_log_id),
        )
        .with_context(
            "batch_index",
            serde_json::Value::Number(serde_json::Number::from(batch_index)),
        )
        .with_context(
            "total_batches",
            serde_json::Value::Number(serde_json::Number::from(total_batches)),
        )
        .with_context(
            "batch_success",
            serde_json::Value::Number(serde_json::Number::from(batch_success)),
        )
        .with_context(
            "batch_errors",
            serde_json::Value::Number(serde_json::Number::from(batch_errors)),
        )
        .with_context(
            "progress_percent",
            serde_json::Value::Number(
                serde_json::Number::from_f64(
                    (batch_index + 1) as f64 / total_batches as f64 * 100.0,
                )
                .unwrap_or(serde_json::Number::from(0)),
            ),
        );

        self.log(entry);
    }

    /// ファイル移行ログを記録
    pub fn log_file_migration(&self, params: FileMigrationLogParams) {
        let level = if params.success {
            LogLevel::Debug
        } else {
            LogLevel::Error
        };
        let message = if params.success {
            "ファイル移行成功"
        } else {
            "ファイル移行失敗"
        };

        let mut entry = StructuredLogEntry::new(
            level,
            "file_migration",
            message,
            Some(params.migration_log_id),
        )
        .with_user_id(params.user_id)
        .with_context(
            "old_path",
            serde_json::Value::String(params.old_path.clone()),
        )
        .with_context(
            "new_path",
            serde_json::Value::String(params.new_path.clone()),
        )
        .with_context(
            "file_size",
            serde_json::Value::Number(serde_json::Number::from(params.file_size)),
        )
        .with_context("success", serde_json::Value::Bool(params.success));

        if let Some(duration) = params.duration_ms {
            entry = entry.with_context(
                "duration_ms",
                serde_json::Value::Number(serde_json::Number::from(duration as u64)),
            );
        }

        self.log(entry);
    }

    /// ログバッファを取得
    pub fn get_log_buffer(&self) -> Vec<StructuredLogEntry> {
        self.log_buffer
            .lock()
            .unwrap_or_else(|poisoned| {
                warn!("ログバッファのロック取得に失敗しました");
                poisoned.into_inner()
            })
            .clone()
    }

    /// ログバッファをクリア
    pub fn clear_log_buffer(&self) {
        if let Ok(mut buffer) = self.log_buffer.lock() {
            buffer.clear();
        }
    }

    /// 指定されたレベル以上のログを取得
    pub fn get_logs_by_level(&self, min_level: LogLevel) -> Vec<StructuredLogEntry> {
        let buffer = self.get_log_buffer();
        buffer
            .into_iter()
            .filter(|entry| self.level_priority(entry.level) >= self.level_priority(min_level))
            .collect()
    }

    /// 指定された移行ログIDのログを取得
    pub fn get_logs_by_migration_id(&self, migration_log_id: i64) -> Vec<StructuredLogEntry> {
        let buffer = self.get_log_buffer();
        buffer
            .into_iter()
            .filter(|entry| entry.migration_log_id == Some(migration_log_id))
            .collect()
    }

    /// ログレベルの優先度を取得
    fn level_priority(&self, level: LogLevel) -> u8 {
        match level {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
            LogLevel::Critical => 4,
        }
    }

    /// ログ統計を取得
    pub fn get_log_statistics(&self) -> LogStatistics {
        let buffer = self.get_log_buffer();
        let mut stats = LogStatistics::default();

        for entry in buffer {
            match entry.level {
                LogLevel::Debug => stats.debug_count += 1,
                LogLevel::Info => stats.info_count += 1,
                LogLevel::Warn => stats.warn_count += 1,
                LogLevel::Error => stats.error_count += 1,
                LogLevel::Critical => stats.critical_count += 1,
            }

            // カテゴリ別統計
            *stats.category_counts.entry(entry.category).or_insert(0) += 1;
        }

        stats.total_entries = stats.debug_count
            + stats.info_count
            + stats.warn_count
            + stats.error_count
            + stats.critical_count;
        stats
    }
}

/// ログ統計
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LogStatistics {
    /// 総エントリ数
    pub total_entries: usize,
    /// デバッグログ数
    pub debug_count: usize,
    /// 情報ログ数
    pub info_count: usize,
    /// 警告ログ数
    pub warn_count: usize,
    /// エラーログ数
    pub error_count: usize,
    /// 致命的エラーログ数
    pub critical_count: usize,
    /// カテゴリ別ログ数
    pub category_counts: HashMap<String, usize>,
}

/// グローバル構造化ロガーインスタンス
static GLOBAL_LOGGER: std::sync::OnceLock<Arc<Mutex<StructuredLogger>>> =
    std::sync::OnceLock::new();

/// グローバルロガーを初期化
pub fn init_global_logger(max_buffer_size: Option<usize>) {
    let logger = StructuredLogger::new(max_buffer_size);
    GLOBAL_LOGGER.set(Arc::new(Mutex::new(logger))).ok();
}

/// グローバルロガーを取得
pub fn get_global_logger() -> Option<Arc<Mutex<StructuredLogger>>> {
    GLOBAL_LOGGER.get().cloned()
}

/// 便利なマクロ関数
pub fn log_migration_info(category: &str, message: &str, migration_log_id: Option<i64>) {
    if let Some(logger) = get_global_logger() {
        if let Ok(logger) = logger.lock() {
            logger.info(category, message, migration_log_id);
        }
    }
}

pub fn log_migration_error_with_context(
    error: &MigrationError,
    migration_log_id: Option<i64>,
    context: Option<&str>,
) {
    if let Some(logger) = get_global_logger() {
        if let Ok(logger) = logger.lock() {
            logger.log_migration_error(error, migration_log_id, context);
        }
    }

    // 従来のエラーハンドラーも呼び出し
    MigrationErrorHandler::log_error(error, context);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structured_log_entry_creation() {
        let entry = StructuredLogEntry::new(
            LogLevel::Info,
            "test_category",
            "テストメッセージ",
            Some(123),
        )
        .with_user_id(456)
        .with_context(
            "test_key",
            serde_json::Value::String("test_value".to_string()),
        );

        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.category, "test_category");
        assert_eq!(entry.message, "テストメッセージ");
        assert_eq!(entry.migration_log_id, Some(123));
        assert_eq!(entry.user_id, Some(456));
        assert_eq!(
            entry.context.get("test_key").unwrap(),
            &serde_json::Value::String("test_value".to_string())
        );
    }

    #[test]
    fn test_structured_log_entry_json_serialization() {
        let entry =
            StructuredLogEntry::new(LogLevel::Error, "error_category", "エラーメッセージ", None);

        let json = entry.to_json();
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("error_category"));
        assert!(json_str.contains("エラーメッセージ"));
    }

    #[test]
    fn test_structured_log_entry_human_readable() {
        let entry = StructuredLogEntry::new(
            LogLevel::Warn,
            "warning_category",
            "警告メッセージ",
            Some(789),
        )
        .with_user_id(123)
        .with_error_code("WARN_001");

        let readable = entry.to_human_readable();
        assert!(readable.contains("WARN"));
        assert!(readable.contains("warning_category"));
        assert!(readable.contains("警告メッセージ"));
        assert!(readable.contains("MIG:789"));
        assert!(readable.contains("USER:123"));
        assert!(readable.contains("WARN_001"));
    }

    #[test]
    fn test_structured_logger_basic_operations() {
        let logger = StructuredLogger::new(Some(10));

        logger.info("test", "テスト情報", Some(1));
        logger.warn("test", "テスト警告", Some(1));
        logger.error("test", "テストエラー", Some(1));

        let buffer = logger.get_log_buffer();
        assert_eq!(buffer.len(), 3);

        let stats = logger.get_log_statistics();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.info_count, 1);
        assert_eq!(stats.warn_count, 1);
        assert_eq!(stats.error_count, 1);
    }

    #[test]
    fn test_log_level_filtering() {
        let logger = StructuredLogger::new(Some(10));

        logger.debug("test", "デバッグ", None);
        logger.info("test", "情報", None);
        logger.warn("test", "警告", None);
        logger.error("test", "エラー", None);

        let warn_and_above = logger.get_logs_by_level(LogLevel::Warn);
        assert_eq!(warn_and_above.len(), 2); // warn + error

        let error_and_above = logger.get_logs_by_level(LogLevel::Error);
        assert_eq!(error_and_above.len(), 1); // error only
    }

    #[test]
    fn test_migration_id_filtering() {
        let logger = StructuredLogger::new(Some(10));

        logger.info("test", "移行1", Some(1));
        logger.info("test", "移行1-2", Some(1));
        logger.info("test", "移行2", Some(2));
        logger.info("test", "一般", None);

        let migration_1_logs = logger.get_logs_by_migration_id(1);
        assert_eq!(migration_1_logs.len(), 2);

        let migration_2_logs = logger.get_logs_by_migration_id(2);
        assert_eq!(migration_2_logs.len(), 1);
    }

    #[test]
    fn test_buffer_size_limit() {
        let logger = StructuredLogger::new(Some(3));

        // バッファサイズを超えてログを追加
        for i in 0..5 {
            logger.info("test", &format!("メッセージ{}", i), None);
        }

        let buffer = logger.get_log_buffer();
        assert_eq!(buffer.len(), 3); // 最大サイズに制限される

        // 最新の3つのメッセージが保持されているはず
        assert!(buffer[0].message.contains("メッセージ2"));
        assert!(buffer[1].message.contains("メッセージ3"));
        assert!(buffer[2].message.contains("メッセージ4"));
    }
}

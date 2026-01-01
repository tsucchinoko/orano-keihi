//! 自動マイグレーションシステムのエラー型
//!
//! このモジュールは、自動マイグレーションシステムで発生する
//! 各種エラーの定義とエラーハンドリング機能を提供します。

use crate::shared::errors::AppError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// マイグレーションエラーの種類
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationErrorType {
    /// 初期化エラー（migrationsテーブル作成失敗など）
    Initialization,
    /// マイグレーション実行エラー
    Execution,
    /// 並行制御エラー（重複実行検出など）
    Concurrency,
    /// システムエラー（バックアップ作成失敗など）
    System,
    /// チェックサム不一致エラー
    ChecksumMismatch,
    /// 検証エラー
    Validation,
}

impl fmt::Display for MigrationErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationErrorType::Initialization => write!(f, "初期化エラー"),
            MigrationErrorType::Execution => write!(f, "実行エラー"),
            MigrationErrorType::Concurrency => write!(f, "並行制御エラー"),
            MigrationErrorType::System => write!(f, "システムエラー"),
            MigrationErrorType::ChecksumMismatch => write!(f, "チェックサム不一致エラー"),
            MigrationErrorType::Validation => write!(f, "検証エラー"),
        }
    }
}

/// マイグレーションエラー
///
/// 自動マイグレーションシステムで発生する各種エラーを表現します。
/// エラーの種類に応じて適切な処理を行うための情報を含みます。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationError {
    /// エラーの種類
    pub error_type: MigrationErrorType,
    /// エラーメッセージ
    pub message: String,
    /// 関連するマイグレーション名（該当する場合）
    pub migration_name: Option<String>,
    /// 詳細情報（デバッグ用）
    pub details: Option<String>,
    /// 復旧可能かどうか
    pub recoverable: bool,
    /// アプリケーション起動を停止すべきかどうか
    pub should_stop_startup: bool,
}

impl MigrationError {
    /// 初期化エラーを作成
    ///
    /// # 引数
    /// * `message` - エラーメッセージ
    /// * `details` - 詳細情報
    ///
    /// # 戻り値
    /// 初期化エラー
    pub fn initialization(message: String, details: Option<String>) -> Self {
        Self {
            error_type: MigrationErrorType::Initialization,
            message,
            migration_name: None,
            details,
            recoverable: false,
            should_stop_startup: true,
        }
    }

    /// マイグレーション実行エラーを作成
    ///
    /// # 引数
    /// * `migration_name` - マイグレーション名
    /// * `message` - エラーメッセージ
    /// * `details` - 詳細情報
    ///
    /// # 戻り値
    /// 実行エラー
    pub fn execution(migration_name: String, message: String, details: Option<String>) -> Self {
        Self {
            error_type: MigrationErrorType::Execution,
            message,
            migration_name: Some(migration_name),
            details,
            recoverable: false,
            should_stop_startup: true,
        }
    }

    /// 並行制御エラーを作成
    ///
    /// # 引数
    /// * `message` - エラーメッセージ
    /// * `details` - 詳細情報
    ///
    /// # 戻り値
    /// 並行制御エラー
    pub fn concurrency(message: String, details: Option<String>) -> Self {
        Self {
            error_type: MigrationErrorType::Concurrency,
            message,
            migration_name: None,
            details,
            recoverable: true,
            should_stop_startup: false,
        }
    }

    /// システムエラーを作成
    ///
    /// # 引数
    /// * `message` - エラーメッセージ
    /// * `details` - 詳細情報
    ///
    /// # 戻り値
    /// システムエラー
    pub fn system(message: String, details: Option<String>) -> Self {
        Self {
            error_type: MigrationErrorType::System,
            message,
            migration_name: None,
            details,
            recoverable: true,
            should_stop_startup: false,
        }
    }

    /// チェックサム不一致エラーを作成
    ///
    /// # 引数
    /// * `migration_name` - マイグレーション名
    /// * `expected` - 期待されるチェックサム
    /// * `actual` - 実際のチェックサム
    ///
    /// # 戻り値
    /// チェックサム不一致エラー
    pub fn checksum_mismatch(migration_name: String, expected: String, actual: String) -> Self {
        let message = format!("マイグレーション '{migration_name}' のチェックサムが一致しません");
        let details = Some(format!("期待値: {expected}, 実際: {actual}"));

        Self {
            error_type: MigrationErrorType::ChecksumMismatch,
            message,
            migration_name: Some(migration_name),
            details,
            recoverable: false,
            should_stop_startup: true,
        }
    }

    /// 検証エラーを作成
    ///
    /// # 引数
    /// * `message` - エラーメッセージ
    /// * `migration_name` - 関連するマイグレーション名
    /// * `details` - 詳細情報
    ///
    /// # 戻り値
    /// 検証エラー
    pub fn validation(
        message: String,
        migration_name: Option<String>,
        details: Option<String>,
    ) -> Self {
        Self {
            error_type: MigrationErrorType::Validation,
            message,
            migration_name,
            details,
            recoverable: false,
            should_stop_startup: true,
        }
    }

    /// エラーが復旧可能かチェック
    ///
    /// # 戻り値
    /// 復旧可能な場合はtrue
    pub fn is_recoverable(&self) -> bool {
        self.recoverable
    }

    /// アプリケーション起動を停止すべきかチェック
    ///
    /// # 戻り値
    /// 起動を停止すべき場合はtrue
    pub fn should_stop_startup(&self) -> bool {
        self.should_stop_startup
    }

    /// エラーの重要度を取得
    ///
    /// # 戻り値
    /// エラーの重要度（Critical, Warning, Info）
    pub fn severity(&self) -> ErrorSeverity {
        match self.error_type {
            MigrationErrorType::Initialization
            | MigrationErrorType::Execution
            | MigrationErrorType::ChecksumMismatch
            | MigrationErrorType::Validation => ErrorSeverity::Critical,
            MigrationErrorType::System => ErrorSeverity::Warning,
            MigrationErrorType::Concurrency => ErrorSeverity::Info,
        }
    }

    /// 詳細なエラー情報を取得
    ///
    /// # 戻り値
    /// 詳細なエラー情報
    pub fn detailed_message(&self) -> String {
        let mut message = format!("[{}] {}", self.error_type, self.message);

        if let Some(migration_name) = &self.migration_name {
            message.push_str(&format!(" (マイグレーション: {migration_name})"));
        }

        if let Some(details) = &self.details {
            message.push_str(&format!(" - 詳細: {details}"));
        }

        message
    }

    /// ログ出力用のメッセージを取得
    ///
    /// # 戻り値
    /// ログ出力用メッセージ
    pub fn log_message(&self) -> String {
        format!(
            "[{}] {} - 復旧可能: {}, 起動停止: {}",
            self.severity(),
            self.detailed_message(),
            self.recoverable,
            self.should_stop_startup
        )
    }
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.detailed_message())
    }
}

impl std::error::Error for MigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

/// エラーの重要度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// 致命的エラー（アプリケーション停止が必要）
    Critical,
    /// 警告（処理は継続可能だが注意が必要）
    Warning,
    /// 情報（軽微な問題）
    Info,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
            ErrorSeverity::Warning => write!(f, "WARNING"),
            ErrorSeverity::Info => write!(f, "INFO"),
        }
    }
}

/// MigrationErrorからAppErrorへの変換
impl From<MigrationError> for AppError {
    fn from(migration_error: MigrationError) -> Self {
        match migration_error.error_type {
            MigrationErrorType::Initialization => {
                AppError::Database(migration_error.detailed_message())
            }
            MigrationErrorType::Execution => AppError::Database(migration_error.detailed_message()),
            MigrationErrorType::Concurrency => {
                AppError::Concurrency(migration_error.detailed_message())
            }
            MigrationErrorType::System => {
                AppError::Configuration(migration_error.detailed_message())
            }
            MigrationErrorType::ChecksumMismatch => {
                AppError::Validation(migration_error.detailed_message())
            }
            MigrationErrorType::Validation => {
                AppError::Validation(migration_error.detailed_message())
            }
        }
    }
}

/// AppErrorからMigrationErrorへの変換（可能な場合）
impl From<AppError> for MigrationError {
    fn from(app_error: AppError) -> Self {
        match app_error {
            AppError::Database(msg) => MigrationError::initialization(msg, None),
            AppError::Validation(msg) => MigrationError::validation(msg, None, None),
            AppError::Configuration(msg) => MigrationError::system(msg, None),
            AppError::Concurrency(msg) => MigrationError::concurrency(msg, None),
            _ => MigrationError::system(format!("予期しないエラー: {app_error}"), None),
        }
    }
}

/// rusqlite::ErrorからMigrationErrorへの変換
impl From<rusqlite::Error> for MigrationError {
    fn from(sqlite_error: rusqlite::Error) -> Self {
        let message = format!("データベースエラー: {sqlite_error}");
        let details = Some(format!("SQLiteエラーコード: {:?}", sqlite_error));

        match sqlite_error {
            rusqlite::Error::SqliteFailure(_, _) => {
                MigrationError::execution("unknown".to_string(), message, details)
            }
            rusqlite::Error::InvalidColumnType(_, _, _) => {
                MigrationError::validation(message, None, details)
            }
            _ => MigrationError::system(message, details),
        }
    }
}

/// エラーハンドリングユーティリティ
pub struct ErrorHandler;

impl ErrorHandler {
    /// エラーをログに出力
    ///
    /// # 引数
    /// * `error` - マイグレーションエラー
    pub fn log_error(error: &MigrationError) {
        match error.severity() {
            ErrorSeverity::Critical => log::error!("{}", error.log_message()),
            ErrorSeverity::Warning => log::warn!("{}", error.log_message()),
            ErrorSeverity::Info => log::info!("{}", error.log_message()),
        }
    }

    /// エラーに基づいて適切なアクションを決定
    ///
    /// # 引数
    /// * `error` - マイグレーションエラー
    ///
    /// # 戻り値
    /// 推奨されるアクション
    pub fn determine_action(error: &MigrationError) -> ErrorAction {
        match error.error_type {
            MigrationErrorType::Initialization => ErrorAction::StopApplication,
            MigrationErrorType::Execution => ErrorAction::StopApplication,
            MigrationErrorType::ChecksumMismatch => ErrorAction::StopApplication,
            MigrationErrorType::Validation => ErrorAction::StopApplication,
            MigrationErrorType::Concurrency => ErrorAction::RetryLater,
            MigrationErrorType::System => {
                if error.recoverable {
                    ErrorAction::RetryLater
                } else {
                    ErrorAction::StopApplication
                }
            }
        }
    }

    /// 複数のエラーを統合
    ///
    /// # 引数
    /// * `errors` - エラー一覧
    ///
    /// # 戻り値
    /// 統合されたエラー
    pub fn aggregate_errors(errors: Vec<MigrationError>) -> MigrationError {
        if errors.is_empty() {
            return MigrationError::system("エラーが発生しましたが詳細不明".to_string(), None);
        }

        if errors.len() == 1 {
            return errors.into_iter().next().unwrap();
        }

        // 最も重要度の高いエラーを選択
        let most_critical = errors
            .iter()
            .max_by_key(|e| match e.severity() {
                ErrorSeverity::Critical => 3,
                ErrorSeverity::Warning => 2,
                ErrorSeverity::Info => 1,
            })
            .unwrap();

        let message = format!(
            "複数のエラーが発生しました ({}件): {}",
            errors.len(),
            most_critical.message
        );

        let details = Some(
            errors
                .iter()
                .map(|e| e.detailed_message())
                .collect::<Vec<_>>()
                .join("; "),
        );

        MigrationError {
            error_type: most_critical.error_type.clone(),
            message,
            migration_name: most_critical.migration_name.clone(),
            details,
            recoverable: errors.iter().all(|e| e.recoverable),
            should_stop_startup: errors.iter().any(|e| e.should_stop_startup),
        }
    }
}

/// エラーに対する推奨アクション
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAction {
    /// アプリケーションを停止
    StopApplication,
    /// 後で再試行
    RetryLater,
    /// 無視して継続
    Continue,
}

impl fmt::Display for ErrorAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorAction::StopApplication => write!(f, "アプリケーション停止"),
            ErrorAction::RetryLater => write!(f, "後で再試行"),
            ErrorAction::Continue => write!(f, "継続"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_error_creation() {
        let init_error = MigrationError::initialization(
            "テーブル作成失敗".to_string(),
            Some("権限不足".to_string()),
        );

        assert_eq!(init_error.error_type, MigrationErrorType::Initialization);
        assert!(!init_error.is_recoverable());
        assert!(init_error.should_stop_startup());
        assert_eq!(init_error.severity(), ErrorSeverity::Critical);

        let concurrency_error = MigrationError::concurrency(
            "重複実行検出".to_string(),
            Some("別のプロセスが実行中".to_string()),
        );

        assert_eq!(
            concurrency_error.error_type,
            MigrationErrorType::Concurrency
        );
        assert!(concurrency_error.is_recoverable());
        assert!(!concurrency_error.should_stop_startup());
        assert_eq!(concurrency_error.severity(), ErrorSeverity::Info);
    }

    #[test]
    fn test_checksum_mismatch_error() {
        let error = MigrationError::checksum_mismatch(
            "test_migration".to_string(),
            "abc123".to_string(),
            "def456".to_string(),
        );

        assert_eq!(error.error_type, MigrationErrorType::ChecksumMismatch);
        assert_eq!(error.migration_name, Some("test_migration".to_string()));
        assert!(error.details.is_some());
        assert!(!error.is_recoverable());
        assert!(error.should_stop_startup());
    }

    #[test]
    fn test_error_handler_determine_action() {
        let critical_error =
            MigrationError::execution("test".to_string(), "実行失敗".to_string(), None);
        assert_eq!(
            ErrorHandler::determine_action(&critical_error),
            ErrorAction::StopApplication
        );

        let concurrency_error = MigrationError::concurrency("重複実行".to_string(), None);
        assert_eq!(
            ErrorHandler::determine_action(&concurrency_error),
            ErrorAction::RetryLater
        );
    }

    #[test]
    fn test_error_aggregation() {
        let errors = vec![
            MigrationError::concurrency("重複実行".to_string(), None),
            MigrationError::execution("test".to_string(), "実行失敗".to_string(), None),
            MigrationError::system("バックアップ失敗".to_string(), None),
        ];

        let aggregated = ErrorHandler::aggregate_errors(errors);
        assert_eq!(aggregated.error_type, MigrationErrorType::Execution);
        assert!(aggregated.message.contains("複数のエラーが発生しました"));
        assert!(aggregated.should_stop_startup());
    }

    #[test]
    fn test_error_conversion() {
        let migration_error = MigrationError::validation(
            "検証失敗".to_string(),
            Some("test_migration".to_string()),
            None,
        );

        let app_error: AppError = migration_error.into();
        match app_error {
            AppError::Validation(msg) => assert!(msg.contains("検証失敗")),
            _ => panic!("期待されるAppError::Validationではありません"),
        }
    }
}

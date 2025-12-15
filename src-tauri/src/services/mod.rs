// R2サービス関連のモジュール

pub mod cache_manager;
pub mod config;
pub mod r2_client;
pub mod security;

// 統一されたエラーハンドリングシステム
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 統一されたアプリケーションエラー型
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum AppError {
    // R2関連エラー
    #[error("R2接続に失敗しました")]
    R2ConnectionFailed {
        details: String,
        user_message: String,
        retry_possible: bool,
    },

    #[error("アップロードに失敗しました")]
    UploadFailed {
        details: String,
        user_message: String,
        retry_possible: bool,
    },

    #[error("ダウンロードに失敗しました")]
    DownloadFailed {
        details: String,
        user_message: String,
        retry_possible: bool,
    },

    #[error("ファイルが見つかりません")]
    FileNotFound {
        details: String,
        user_message: String,
        retry_possible: bool,
    },

    #[error("認証情報が無効です")]
    InvalidCredentials {
        details: String,
        user_message: String,
        retry_possible: bool,
    },

    #[error("ネットワークエラーが発生しました")]
    NetworkError {
        details: String,
        user_message: String,
        retry_possible: bool,
    },

    // ファイル関連エラー
    #[error("ファイル操作エラー")]
    FileOperationError {
        details: String,
        user_message: String,
        retry_possible: bool,
    },

    #[error("ファイル形式エラー")]
    InvalidFileFormat {
        details: String,
        user_message: String,
        retry_possible: bool,
    },

    #[error("ファイルサイズエラー")]
    FileSizeError {
        details: String,
        user_message: String,
        retry_possible: bool,
    },

    // データベース関連エラー
    #[error("データベースエラー")]
    DatabaseError {
        details: String,
        user_message: String,
        retry_possible: bool,
    },

    // 設定関連エラー
    #[error("設定エラー")]
    ConfigError {
        details: String,
        user_message: String,
        retry_possible: bool,
    },

    // キャッシュ関連エラー
    #[error("キャッシュエラー")]
    CacheError {
        details: String,
        user_message: String,
        retry_possible: bool,
    },

    // 一般的なエラー
    #[error("内部エラーが発生しました")]
    InternalError {
        details: String,
        user_message: String,
        retry_possible: bool,
    },
}

impl AppError {
    /// ユーザーフレンドリーなエラーメッセージを取得
    pub fn user_message(&self) -> &str {
        match self {
            AppError::R2ConnectionFailed { user_message, .. } => user_message,
            AppError::UploadFailed { user_message, .. } => user_message,
            AppError::DownloadFailed { user_message, .. } => user_message,
            AppError::FileNotFound { user_message, .. } => user_message,
            AppError::InvalidCredentials { user_message, .. } => user_message,
            AppError::NetworkError { user_message, .. } => user_message,
            AppError::FileOperationError { user_message, .. } => user_message,
            AppError::InvalidFileFormat { user_message, .. } => user_message,
            AppError::FileSizeError { user_message, .. } => user_message,
            AppError::DatabaseError { user_message, .. } => user_message,
            AppError::ConfigError { user_message, .. } => user_message,
            AppError::CacheError { user_message, .. } => user_message,
            AppError::InternalError { user_message, .. } => user_message,
        }
    }

    /// 詳細なエラー情報を取得
    pub fn details(&self) -> &str {
        match self {
            AppError::R2ConnectionFailed { details, .. } => details,
            AppError::UploadFailed { details, .. } => details,
            AppError::DownloadFailed { details, .. } => details,
            AppError::FileNotFound { details, .. } => details,
            AppError::InvalidCredentials { details, .. } => details,
            AppError::NetworkError { details, .. } => details,
            AppError::FileOperationError { details, .. } => details,
            AppError::InvalidFileFormat { details, .. } => details,
            AppError::FileSizeError { details, .. } => details,
            AppError::DatabaseError { details, .. } => details,
            AppError::ConfigError { details, .. } => details,
            AppError::CacheError { details, .. } => details,
            AppError::InternalError { details, .. } => details,
        }
    }

    /// リトライ可能かどうかを取得
    pub fn is_retry_possible(&self) -> bool {
        match self {
            AppError::R2ConnectionFailed { retry_possible, .. } => *retry_possible,
            AppError::UploadFailed { retry_possible, .. } => *retry_possible,
            AppError::DownloadFailed { retry_possible, .. } => *retry_possible,
            AppError::FileNotFound { retry_possible, .. } => *retry_possible,
            AppError::InvalidCredentials { retry_possible, .. } => *retry_possible,
            AppError::NetworkError { retry_possible, .. } => *retry_possible,
            AppError::FileOperationError { retry_possible, .. } => *retry_possible,
            AppError::InvalidFileFormat { retry_possible, .. } => *retry_possible,
            AppError::FileSizeError { retry_possible, .. } => *retry_possible,
            AppError::DatabaseError { retry_possible, .. } => *retry_possible,
            AppError::ConfigError { retry_possible, .. } => *retry_possible,
            AppError::CacheError { retry_possible, .. } => *retry_possible,
            AppError::InternalError { retry_possible, .. } => *retry_possible,
        }
    }

    /// エラーの重要度を取得
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            AppError::InvalidCredentials { .. } => ErrorSeverity::Critical,
            AppError::ConfigError { .. } => ErrorSeverity::Critical,
            AppError::DatabaseError { .. } => ErrorSeverity::High,
            AppError::R2ConnectionFailed { .. } => ErrorSeverity::High,
            AppError::UploadFailed { .. } => ErrorSeverity::Medium,
            AppError::DownloadFailed { .. } => ErrorSeverity::Medium,
            AppError::FileNotFound { .. } => ErrorSeverity::Medium,
            AppError::NetworkError { .. } => ErrorSeverity::Medium,
            AppError::FileOperationError { .. } => ErrorSeverity::Low,
            AppError::InvalidFileFormat { .. } => ErrorSeverity::Low,
            AppError::FileSizeError { .. } => ErrorSeverity::Low,
            AppError::CacheError { .. } => ErrorSeverity::Low,
            AppError::InternalError { .. } => ErrorSeverity::High,
        }
    }
}

/// エラーの重要度
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// 従来のR2Errorとの互換性のためのエラー型
#[derive(Debug, Error)]
pub enum R2Error {
    #[error("R2接続に失敗しました: {0}")]
    ConnectionFailed(String),

    #[error("アップロードに失敗しました: {0}")]
    UploadFailed(String),

    #[error("ダウンロードに失敗しました: {0}")]
    DownloadFailed(String),

    #[error("ファイルが見つかりません: {0}")]
    FileNotFound(String),

    #[error("認証情報が無効です")]
    InvalidCredentials,

    #[error("ネットワークエラー: {0}")]
    NetworkError(String),
}

impl From<R2Error> for AppError {
    fn from(error: R2Error) -> Self {
        match error {
            R2Error::ConnectionFailed(details) => AppError::R2ConnectionFailed {
                details: details.clone(),
                user_message:
                    "クラウドストレージへの接続に失敗しました。ネットワーク接続を確認してください。"
                        .to_string(),
                retry_possible: true,
            },
            R2Error::UploadFailed(details) => AppError::UploadFailed {
                details: details.clone(),
                user_message:
                    "ファイルのアップロードに失敗しました。しばらく時間をおいて再試行してください。"
                        .to_string(),
                retry_possible: true,
            },
            R2Error::DownloadFailed(details) => AppError::DownloadFailed {
                details: details.clone(),
                user_message:
                    "ファイルのダウンロードに失敗しました。しばらく時間をおいて再試行してください。"
                        .to_string(),
                retry_possible: true,
            },
            R2Error::FileNotFound(details) => AppError::FileNotFound {
                details: details.clone(),
                user_message:
                    "指定されたファイルが見つかりません。ファイルが削除されている可能性があります。"
                        .to_string(),
                retry_possible: false,
            },
            R2Error::InvalidCredentials => AppError::InvalidCredentials {
                details: "R2認証情報が無効です".to_string(),
                user_message:
                    "クラウドストレージの認証に失敗しました。管理者にお問い合わせください。"
                        .to_string(),
                retry_possible: false,
            },
            R2Error::NetworkError(details) => AppError::NetworkError {
                details: details.clone(),
                user_message:
                    "ネットワークエラーが発生しました。インターネット接続を確認してください。"
                        .to_string(),
                retry_possible: true,
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("設定の読み込みに失敗しました: {0}")]
    LoadFailed(String),

    #[error("アカウントIDが設定されていません")]
    MissingAccountId,

    #[error("アクセスキーが設定されていません")]
    MissingAccessKey,

    #[error("シークレットキーが設定されていません")]
    MissingSecretKey,

    #[error("バケット名が設定されていません")]
    MissingBucketName,

    #[error("環境変数エラー: {0}")]
    EnvVarError(#[from] std::env::VarError),
}

impl From<ConfigError> for AppError {
    fn from(error: ConfigError) -> Self {
        let (details, user_message) = match error {
            ConfigError::LoadFailed(msg) => (
                format!("設定読み込みエラー: {msg}"),
                "設定の読み込みに失敗しました。設定ファイルを確認してください。".to_string(),
            ),
            ConfigError::MissingAccountId => (
                "R2_ACCOUNT_IDが設定されていません".to_string(),
                "クラウドストレージの設定が不完全です。管理者にお問い合わせください。".to_string(),
            ),
            ConfigError::MissingAccessKey => (
                "R2_ACCESS_KEYが設定されていません".to_string(),
                "クラウドストレージの設定が不完全です。管理者にお問い合わせください。".to_string(),
            ),
            ConfigError::MissingSecretKey => (
                "R2_SECRET_KEYが設定されていません".to_string(),
                "クラウドストレージの設定が不完全です。管理者にお問い合わせください。".to_string(),
            ),
            ConfigError::MissingBucketName => (
                "R2_BUCKET_NAMEが設定されていません".to_string(),
                "クラウドストレージの設定が不完全です。管理者にお問い合わせください。".to_string(),
            ),
            ConfigError::EnvVarError(e) => (
                format!("環境変数エラー: {e}"),
                "システム設定の読み込みに失敗しました。管理者にお問い合わせください。".to_string(),
            ),
        };

        AppError::ConfigError {
            details,
            user_message,
            retry_possible: false,
        }
    }
}

/// エラーハンドリングユーティリティ
pub struct ErrorHandler;

impl ErrorHandler {
    /// エラーをログに記録し、ユーザーフレンドリーなメッセージを返す
    pub fn handle_error(error: AppError) -> String {
        use log::{error, info, warn};

        // 重要度に応じてログレベルを変更
        match error.severity() {
            ErrorSeverity::Critical => {
                error!("重大なエラー: {} - 詳細: {}", error, error.details());
            }
            ErrorSeverity::High => {
                error!("高レベルエラー: {} - 詳細: {}", error, error.details());
            }
            ErrorSeverity::Medium => {
                warn!("中レベルエラー: {} - 詳細: {}", error, error.details());
            }
            ErrorSeverity::Low => {
                info!("低レベルエラー: {} - 詳細: {}", error, error.details());
            }
        }

        // セキュリティログ記録
        let security_manager = security::SecurityManager::new();
        security_manager.log_security_event(
            "error_handled",
            &format!(
                "severity={:?}, error={}, details={}",
                error.severity(),
                error,
                error.details()
            ),
        );

        // ユーザーフレンドリーなメッセージを返す
        error.user_message().to_string()
    }

    /// ファイル操作エラーを作成
    pub fn file_operation_error(operation: &str, path: &str, error: std::io::Error) -> AppError {
        AppError::FileOperationError {
            details: format!(
                "ファイル操作「{operation}」が失敗しました: パス={path}, エラー={error}"
            ),
            user_message: match error.kind() {
                std::io::ErrorKind::NotFound => "指定されたファイルが見つかりません。".to_string(),
                std::io::ErrorKind::PermissionDenied => {
                    "ファイルへのアクセス権限がありません。".to_string()
                }
                std::io::ErrorKind::AlreadyExists => "同名のファイルが既に存在します。".to_string(),
                _ => "ファイル操作中にエラーが発生しました。".to_string(),
            },
            retry_possible: matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted
            ),
        }
    }

    /// ファイル形式エラーを作成
    pub fn invalid_file_format_error(filename: &str, allowed_formats: &[&str]) -> AppError {
        AppError::InvalidFileFormat {
            details: format!("無効なファイル形式: {filename}"),
            user_message: format!(
                "サポートされていないファイル形式です。対応形式: {}",
                allowed_formats.join(", ")
            ),
            retry_possible: false,
        }
    }

    /// ファイルサイズエラーを作成
    pub fn file_size_error(size: u64, max_size: u64) -> AppError {
        AppError::FileSizeError {
            details: format!(
                "ファイルサイズ超過: {size}bytes (最大: {max_size}bytes)"
            ),
            user_message: format!(
                "ファイルサイズが制限を超えています。最大サイズ: {}MB",
                max_size / (1024 * 1024)
            ),
            retry_possible: false,
        }
    }

    /// データベースエラーを作成
    pub fn database_error(operation: &str, error: rusqlite::Error) -> AppError {
        AppError::DatabaseError {
            details: format!("データベース操作「{operation}」が失敗しました: {error}"),
            user_message:
                "データベース操作中にエラーが発生しました。しばらく時間をおいて再試行してください。"
                    .to_string(),
            retry_possible: true,
        }
    }
}

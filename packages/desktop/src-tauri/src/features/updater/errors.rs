use log::error;
use serde::{Deserialize, Serialize};

/// アップデートエラーの種類
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum UpdateError {
    /// ネットワークエラー
    #[error("ネットワークエラー: {message}")]
    Network { message: String },

    /// 署名検証エラー
    #[error("署名検証エラー: {message}")]
    SignatureVerification { message: String },

    /// ダウンロードエラー
    #[error("ダウンロードエラー: {message}")]
    Download { message: String },

    /// インストールエラー
    #[error("インストールエラー: {message}")]
    Installation { message: String },

    /// 設定エラー
    #[error("設定エラー: {message}")]
    Configuration { message: String },

    /// ファイルシステムエラー
    #[error("ファイルシステムエラー: {message}")]
    FileSystem { message: String },

    /// 権限エラー
    #[error("権限エラー: {message}")]
    Permission { message: String },

    /// タイムアウトエラー
    #[error("タイムアウトエラー: {message}")]
    Timeout { message: String },

    /// 不正なバージョンエラー
    #[error("不正なバージョン: {message}")]
    InvalidVersion { message: String },

    /// アップデーター初期化エラー
    #[error("アップデーター初期化エラー: {message}")]
    InitializationError { message: String },

    /// 一般的なエラー
    #[error("エラー: {message}")]
    General { message: String },
}

impl UpdateError {
    /// ネットワークエラーを作成
    pub fn network<S: Into<String>>(message: S) -> Self {
        let msg = message.into();
        error!("ネットワークエラーが発生: {msg}");
        Self::Network { message: msg }
    }

    /// 署名検証エラーを作成
    pub fn signature_verification<S: Into<String>>(message: S) -> Self {
        let msg = message.into();
        error!("署名検証エラーが発生: {msg}");
        Self::SignatureVerification { message: msg }
    }

    /// ダウンロードエラーを作成
    pub fn download<S: Into<String>>(message: S) -> Self {
        let msg = message.into();
        error!("ダウンロードエラーが発生: {msg}");
        Self::Download { message: msg }
    }

    /// インストールエラーを作成
    pub fn installation<S: Into<String>>(message: S) -> Self {
        let msg = message.into();
        error!("インストールエラーが発生: {msg}");
        Self::Installation { message: msg }
    }

    /// 設定エラーを作成
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        let msg = message.into();
        error!("設定エラーが発生: {msg}");
        Self::Configuration { message: msg }
    }

    /// ファイルシステムエラーを作成
    pub fn file_system<S: Into<String>>(message: S) -> Self {
        let msg = message.into();
        error!("ファイルシステムエラーが発生: {msg}");
        Self::FileSystem { message: msg }
    }

    /// 権限エラーを作成
    pub fn permission<S: Into<String>>(message: S) -> Self {
        let msg = message.into();
        error!("権限エラーが発生: {msg}");
        Self::Permission { message: msg }
    }

    /// タイムアウトエラーを作成
    pub fn timeout<S: Into<String>>(message: S) -> Self {
        let msg = message.into();
        error!("タイムアウトエラーが発生: {msg}");
        Self::Timeout { message: msg }
    }

    /// 不正なバージョンエラーを作成
    pub fn invalid_version<S: Into<String>>(message: S) -> Self {
        let msg = message.into();
        error!("不正なバージョンエラーが発生: {msg}");
        Self::InvalidVersion { message: msg }
    }

    /// アップデーター初期化エラーを作成
    pub fn initialization_error<S: Into<String>>(message: S) -> Self {
        let msg = message.into();
        error!("アップデーター初期化エラーが発生: {msg}");
        Self::InitializationError { message: msg }
    }

    /// 一般的なエラーを作成
    pub fn general<S: Into<String>>(message: S) -> Self {
        let msg = message.into();
        error!("一般的なエラーが発生: {msg}");
        Self::General { message: msg }
    }

    /// エラーメッセージを取得
    pub fn message(&self) -> &str {
        match self {
            Self::Network { message } => message,
            Self::SignatureVerification { message } => message,
            Self::Download { message } => message,
            Self::Installation { message } => message,
            Self::Configuration { message } => message,
            Self::FileSystem { message } => message,
            Self::Permission { message } => message,
            Self::Timeout { message } => message,
            Self::InvalidVersion { message } => message,
            Self::InitializationError { message } => message,
            Self::General { message } => message,
        }
    }

    /// エラーの種類を取得
    pub fn error_type(&self) -> &'static str {
        match self {
            Self::Network { .. } => "Network",
            Self::SignatureVerification { .. } => "SignatureVerification",
            Self::Download { .. } => "Download",
            Self::Installation { .. } => "Installation",
            Self::Configuration { .. } => "Configuration",
            Self::FileSystem { .. } => "FileSystem",
            Self::Permission { .. } => "Permission",
            Self::Timeout { .. } => "Timeout",
            Self::InvalidVersion { .. } => "InvalidVersion",
            Self::InitializationError { .. } => "InitializationError",
            Self::General { .. } => "General",
        }
    }

    /// エラーが再試行可能かどうかを判定
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network { .. } => true,
            Self::Download { .. } => true,
            Self::Timeout { .. } => true,
            Self::SignatureVerification { .. } => false,
            Self::Installation { .. } => false,
            Self::Configuration { .. } => false,
            Self::FileSystem { .. } => false,
            Self::Permission { .. } => false,
            Self::InvalidVersion { .. } => false,
            Self::InitializationError { .. } => false,
            Self::General { .. } => false,
        }
    }

    /// エラーがセキュリティ関連かどうかを判定
    pub fn is_security_related(&self) -> bool {
        matches!(self, Self::SignatureVerification { .. })
    }

    /// ユーザー向けのエラーメッセージを取得
    pub fn user_friendly_message(&self) -> String {
        match self {
            Self::Network { .. } => {
                "ネットワーク接続に問題があります。インターネット接続を確認してください。".to_string()
            }
            Self::SignatureVerification { .. } => {
                "アップデートファイルの署名検証に失敗しました。セキュリティ上の理由でアップデートを中止します。".to_string()
            }
            Self::Download { .. } => {
                "アップデートファイルのダウンロードに失敗しました。しばらく時間をおいて再試行してください。".to_string()
            }
            Self::Installation { .. } => {
                "アップデートのインストールに失敗しました。アプリケーションを再起動して再試行してください。".to_string()
            }
            Self::Configuration { .. } => {
                "アップデーター設定に問題があります。設定を確認してください。".to_string()
            }
            Self::FileSystem { .. } => {
                "ファイルシステムエラーが発生しました。ディスク容量を確認してください。".to_string()
            }
            Self::Permission { .. } => {
                "権限が不足しています。管理者権限でアプリケーションを実行してください。".to_string()
            }
            Self::Timeout { .. } => {
                "処理がタイムアウトしました。ネットワーク接続を確認して再試行してください。".to_string()
            }
            Self::InvalidVersion { .. } => {
                "無効なバージョン情報です。アプリケーションを再起動してください。".to_string()
            }
            Self::InitializationError { .. } => {
                "アップデーター機能の初期化に失敗しました。アプリケーションを再起動してください。".to_string()
            }
            Self::General { message } => {
                format!("エラーが発生しました: {message}")
            }
        }
    }

    /// デバッグ情報を取得
    pub fn get_debug_info(&self) -> std::collections::HashMap<String, String> {
        let mut info = std::collections::HashMap::new();
        info.insert("error_type".to_string(), self.error_type().to_string());
        info.insert("message".to_string(), self.message().to_string());
        info.insert("is_retryable".to_string(), self.is_retryable().to_string());
        info.insert(
            "is_security_related".to_string(),
            self.is_security_related().to_string(),
        );
        info.insert(
            "user_friendly_message".to_string(),
            self.user_friendly_message(),
        );
        info
    }
}

/// 標準エラーからUpdateErrorへの変換
impl From<std::io::Error> for UpdateError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => Self::file_system("ファイルが見つかりません"),
            std::io::ErrorKind::PermissionDenied => {
                Self::permission("ファイルアクセス権限がありません")
            }
            std::io::ErrorKind::TimedOut => Self::timeout("ファイル操作がタイムアウトしました"),
            _ => Self::file_system(format!("ファイルシステムエラー: {error}")),
        }
    }
}

/// serde_jsonエラーからUpdateErrorへの変換
impl From<serde_json::Error> for UpdateError {
    fn from(error: serde_json::Error) -> Self {
        Self::configuration(format!("JSON解析エラー: {error}"))
    }
}

/// reqwestエラーからUpdateErrorへの変換
impl From<reqwest::Error> for UpdateError {
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::timeout(format!("HTTPリクエストタイムアウト: {error}"))
        } else if error.is_connect() {
            Self::network(format!("接続エラー: {error}"))
        } else {
            Self::network(format!("HTTPエラー: {error}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let network_error = UpdateError::network("接続に失敗しました");
        assert_eq!(network_error.error_type(), "Network");
        assert_eq!(network_error.message(), "接続に失敗しました");
        assert!(network_error.is_retryable());
        assert!(!network_error.is_security_related());

        let signature_error = UpdateError::signature_verification("署名が無効です");
        assert_eq!(signature_error.error_type(), "SignatureVerification");
        assert!(!signature_error.is_retryable());
        assert!(signature_error.is_security_related());
    }

    #[test]
    fn test_user_friendly_message() {
        let network_error = UpdateError::network("Connection failed");
        let user_message = network_error.user_friendly_message();
        assert!(user_message.contains("ネットワーク接続"));

        let signature_error = UpdateError::signature_verification("Invalid signature");
        let user_message = signature_error.user_friendly_message();
        assert!(user_message.contains("署名検証"));
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let update_error: UpdateError = io_error.into();
        assert_eq!(update_error.error_type(), "FileSystem");

        let json_error = serde_json::from_str::<i32>("invalid json").unwrap_err();
        let update_error: UpdateError = json_error.into();
        assert_eq!(update_error.error_type(), "Configuration");
    }

    #[test]
    fn test_get_debug_info() {
        let error = UpdateError::download("ダウンロードに失敗");
        let debug_info = error.get_debug_info();

        assert_eq!(debug_info.get("error_type"), Some(&"Download".to_string()));
        assert_eq!(
            debug_info.get("message"),
            Some(&"ダウンロードに失敗".to_string())
        );
        assert_eq!(debug_info.get("is_retryable"), Some(&"true".to_string()));
        assert_eq!(
            debug_info.get("is_security_related"),
            Some(&"false".to_string())
        );
        assert!(debug_info.contains_key("user_friendly_message"));
    }

    #[test]
    fn test_error_display() {
        let error = UpdateError::network("接続エラー");
        let display_string = format!("{error}");
        assert!(display_string.contains("ネットワークエラー"));
        assert!(display_string.contains("接続エラー"));
    }
}

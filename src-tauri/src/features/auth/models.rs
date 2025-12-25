use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// ユーザー情報を表す構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// ユーザーID
    pub id: i64,
    /// GoogleユーザーID
    pub google_id: String,
    /// メールアドレス
    pub email: String,
    /// 表示名
    pub name: String,
    /// プロフィール画像URL
    pub picture_url: Option<String>,
    /// 作成日時
    pub created_at: DateTime<Utc>,
    /// 更新日時
    pub updated_at: DateTime<Utc>,
}

/// Googleから取得したユーザー情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleUser {
    /// GoogleユーザーID
    pub id: String,
    /// メールアドレス
    pub email: String,
    /// 表示名
    pub name: String,
    /// プロフィール画像URL
    pub picture: Option<String>,
    /// メール認証済みフラグ
    pub verified_email: bool,
}

/// セッション情報を表す構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// セッションID
    pub id: String,
    /// ユーザーID
    pub user_id: i64,
    /// 有効期限
    pub expires_at: DateTime<Utc>,
    /// 作成日時
    pub created_at: DateTime<Utc>,
}

/// 認証状態を表す構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthState {
    /// ユーザー情報
    pub user: Option<User>,
    /// 認証済みフラグ
    pub is_authenticated: bool,
    /// ローディング状態
    pub is_loading: bool,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            user: None,
            is_authenticated: false,
            is_loading: false,
        }
    }
}

/// 認証エラーの種類
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// OAuth設定エラー
    #[error("OAuth設定エラー: {0}")]
    ConfigError(String),

    /// OAuth認証エラー
    #[error("OAuth認証エラー: {0}")]
    OAuthError(String),

    /// ネットワークエラー
    #[error("ネットワークエラー: {0}")]
    NetworkError(String),

    /// データベースエラー
    #[error("データベースエラー: {0}")]
    DatabaseError(String),

    /// セッションエラー
    #[error("セッションエラー: {0}")]
    SessionError(String),

    /// 暗号化エラー
    #[error("暗号化エラー: {0}")]
    EncryptionError(String),

    /// 無効なトークンエラー
    #[error("無効なトークン")]
    InvalidToken,

    /// セッション期限切れエラー
    #[error("セッションが期限切れです")]
    SessionExpired,

    /// 認証が必要エラー
    #[error("認証が必要です")]
    AuthenticationRequired,
}

impl From<rusqlite::Error> for AuthError {
    fn from(error: rusqlite::Error) -> Self {
        AuthError::DatabaseError(error.to_string())
    }
}

impl From<reqwest::Error> for AuthError {
    fn from(error: reqwest::Error) -> Self {
        AuthError::NetworkError(error.to_string())
    }
}

impl
    From<
        oauth2::RequestTokenError<
            oauth2::reqwest::Error<reqwest::Error>,
            oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
        >,
    > for AuthError
{
    fn from(
        error: oauth2::RequestTokenError<
            oauth2::reqwest::Error<reqwest::Error>,
            oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
        >,
    ) -> Self {
        AuthError::OAuthError(error.to_string())
    }
}

/// セッションエラーの種類
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// 暗号化エラー
    #[error("暗号化エラー: {0}")]
    EncryptionError(String),

    /// 復号化エラー
    #[error("復号化エラー: {0}")]
    DecryptionError(String),

    /// セッション期限切れ
    #[error("セッションが期限切れです")]
    Expired,

    /// セッションが見つからない
    #[error("セッションが見つかりません")]
    NotFound,

    /// データベースエラー
    #[error("データベースエラー: {0}")]
    DatabaseError(String),
}

impl From<rusqlite::Error> for SessionError {
    fn from(error: rusqlite::Error) -> Self {
        SessionError::DatabaseError(error.to_string())
    }
}

impl From<crate::features::auth::models::SessionError> for AuthError {
    fn from(error: crate::features::auth::models::SessionError) -> Self {
        match error {
            crate::features::auth::models::SessionError::Expired => AuthError::SessionExpired,
            crate::features::auth::models::SessionError::NotFound => AuthError::InvalidToken,
            crate::features::auth::models::SessionError::EncryptionError(msg) => {
                AuthError::EncryptionError(msg)
            }
            crate::features::auth::models::SessionError::DecryptionError(msg) => {
                AuthError::EncryptionError(msg)
            }
            crate::features::auth::models::SessionError::DatabaseError(msg) => {
                AuthError::DatabaseError(msg)
            }
        }
    }
}

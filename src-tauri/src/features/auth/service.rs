use crate::features::auth::models::{AuthError, GoogleUser, Session, User};
use crate::features::auth::repository::UserRepository;
use crate::features::auth::session::SessionManager;
use crate::shared::config::environment::GoogleOAuthConfig;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// OAuth認証フローの開始情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthStartInfo {
    /// 認証URL
    pub auth_url: String,
    /// PKCE検証子
    pub code_verifier: String,
    /// 状態パラメータ
    pub state: String,
}

/// OAuth認証サービス
#[derive(Clone)]
pub struct AuthService {
    /// OAuth2クライアント
    oauth_client: BasicClient,
    /// セッション管理
    session_manager: SessionManager,
    /// ユーザーリポジトリ
    user_repository: UserRepository,
    /// HTTPクライアント
    http_client: reqwest::Client,
}

impl AuthService {
    /// 新しいAuthServiceを作成する
    ///
    /// # 引数
    /// * `config` - Google OAuth設定
    /// * `db_connection` - データベース接続
    ///
    /// # 戻り値
    /// AuthServiceインスタンス
    pub fn new(
        config: GoogleOAuthConfig,
        db_connection: Arc<Mutex<Connection>>,
    ) -> Result<Self, AuthError> {
        // OAuth2クライアントを設定
        let client_id = ClientId::new(config.client_id);
        let client_secret = ClientSecret::new(config.client_secret);
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| AuthError::ConfigError(format!("認証URL設定エラー: {e}")))?;
        let token_url = TokenUrl::new("https://www.googleapis.com/oauth2/v4/token".to_string())
            .map_err(|e| AuthError::ConfigError(format!("トークンURL設定エラー: {e}")))?;
        let redirect_url = RedirectUrl::new(config.redirect_uri)
            .map_err(|e| AuthError::ConfigError(format!("リダイレクトURL設定エラー: {e}")))?;

        let oauth_client =
            BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
                .set_redirect_uri(redirect_url);

        // セッション管理を初期化
        let session_manager =
            SessionManager::new(Arc::clone(&db_connection), config.session_encryption_key);

        // ユーザーリポジトリを初期化
        let user_repository = UserRepository::new(Arc::clone(&db_connection));

        // HTTPクライアントを作成
        let http_client = reqwest::Client::new();

        log::info!("AuthServiceを初期化しました");

        Ok(Self {
            oauth_client,
            session_manager,
            user_repository,
            http_client,
        })
    }

    /// OAuth認証フローを開始する
    ///
    /// # 戻り値
    /// 認証開始情報（認証URL、PKCE検証子、状態パラメータ）
    pub fn start_oauth_flow(&self) -> Result<OAuthStartInfo, AuthError> {
        // PKCE（Proof Key for Code Exchange）を生成
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // 認証URLを生成
        let (auth_url, csrf_token) = self
            .oauth_client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        let oauth_info = OAuthStartInfo {
            auth_url: auth_url.to_string(),
            code_verifier: pkce_verifier.secret().clone(),
            state: csrf_token.secret().clone(),
        };

        log::info!("OAuth認証フローを開始しました");
        log::debug!("認証URL: {}", oauth_info.auth_url);

        Ok(oauth_info)
    }

    /// 認証コールバックを処理する
    ///
    /// # 引数
    /// * `code` - 認証コード
    /// * `state` - 状態パラメータ
    /// * `code_verifier` - PKCE検証子
    ///
    /// # 戻り値
    /// 認証されたユーザー情報とセッション
    pub async fn handle_callback(
        &self,
        code: String,
        _state: String,
        code_verifier: String,
    ) -> Result<(User, Session), AuthError> {
        log::info!("認証コールバックを処理開始");

        // 認証コードをアクセストークンに交換
        let pkce_verifier = PkceCodeVerifier::new(code_verifier);
        let token_result = self
            .oauth_client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await?;

        let access_token = token_result.access_token();
        log::debug!("アクセストークンを取得しました");

        // Googleユーザー情報を取得
        let google_user = self.fetch_google_user_info(access_token.secret()).await?;
        log::info!(
            "Googleユーザー情報を取得しました: email={}",
            google_user.email
        );

        // ユーザーを作成または取得
        let user = self
            .user_repository
            .find_or_create_user(google_user)
            .await?;

        // セッションを作成
        let session = self.session_manager.create_session(user.id)?;

        log::info!("認証コールバック処理が完了しました: user_id={}", user.id);

        Ok((user, session))
    }

    /// セッションを検証する
    ///
    /// # 引数
    /// * `token` - セッショントークン
    ///
    /// # 戻り値
    /// 認証されたユーザー情報
    pub async fn validate_session(&self, token: String) -> Result<User, AuthError> {
        // セッションを検証
        let session = self
            .session_manager
            .validate_session(token)
            .map_err(|e| match e {
                crate::features::auth::models::SessionError::Expired => AuthError::SessionExpired,
                crate::features::auth::models::SessionError::NotFound => AuthError::InvalidToken,
                _ => AuthError::SessionError(e.to_string()),
            })?;

        // ユーザー情報を取得
        let user = self
            .user_repository
            .get_user_by_id(session.user_id)
            .await?
            .ok_or_else(|| AuthError::DatabaseError("ユーザーが見つかりません".to_string()))?;

        Ok(user)
    }

    /// ログアウト処理
    ///
    /// # 引数
    /// * `session_id` - セッションID
    ///
    /// # 戻り値
    /// 処理結果
    pub async fn logout(&self, session_id: String) -> Result<(), AuthError> {
        self.session_manager
            .invalidate_session(&session_id)
            .map_err(|e| AuthError::SessionError(e.to_string()))?;

        log::info!("ログアウト処理が完了しました: session_id={session_id}");
        Ok(())
    }

    /// Googleユーザー情報を取得する
    ///
    /// # 引数
    /// * `access_token` - アクセストークン
    ///
    /// # 戻り値
    /// Googleユーザー情報
    async fn fetch_google_user_info(&self, access_token: &str) -> Result<GoogleUser, AuthError> {
        let url = "https://www.googleapis.com/oauth2/v2/userinfo";

        let response = self
            .http_client
            .get(url)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuthError::NetworkError(format!(
                "Googleユーザー情報取得エラー: status={}",
                response.status()
            )));
        }

        let google_user: GoogleUser = response.json().await?;

        // メール認証済みかチェック
        if !google_user.verified_email {
            return Err(AuthError::OAuthError(
                "メールアドレスが認証されていません".to_string(),
            ));
        }

        Ok(google_user)
    }

    /// セッション暗号化トークンを生成する
    ///
    /// # 引数
    /// * `session_id` - セッションID
    ///
    /// # 戻り値
    /// 暗号化されたトークン
    pub fn create_session_token(&self, session_id: &str) -> Result<String, AuthError> {
        self.session_manager
            .encrypt_session_id(session_id)
            .map_err(|e| AuthError::EncryptionError(e.to_string()))
    }

    /// 期限切れセッションをクリーンアップする
    ///
    /// # 戻り値
    /// 削除されたセッション数
    pub async fn cleanup_expired_sessions(&self) -> Result<usize, AuthError> {
        self.session_manager
            .cleanup_expired_sessions()
            .map_err(|e| AuthError::SessionError(e.to_string()))
    }

    /// UserRepositoryへの参照を取得する（テスト用）
    #[cfg(test)]
    pub fn user_repository(&self) -> &UserRepository {
        &self.user_repository
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::environment::GoogleOAuthConfig;
    use crate::shared::database::connection::create_in_memory_connection;

    fn setup_test_auth_service() -> AuthService {
        let conn = create_in_memory_connection().unwrap();

        let config = GoogleOAuthConfig {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            redirect_uri: "http://localhost:3000/auth/callback".to_string(),
            session_encryption_key: "test_encryption_key_32_bytes_long".to_string(),
        };

        AuthService::new(config, Arc::new(Mutex::new(conn))).unwrap()
    }

    #[test]
    fn test_start_oauth_flow() {
        let auth_service = setup_test_auth_service();

        let oauth_info = auth_service.start_oauth_flow().unwrap();

        assert!(!oauth_info.auth_url.is_empty());
        assert!(!oauth_info.code_verifier.is_empty());
        assert!(!oauth_info.state.is_empty());
        assert!(oauth_info.auth_url.contains("accounts.google.com"));
    }

    #[test]
    fn test_create_session_token() {
        let auth_service = setup_test_auth_service();
        let session_id = "test-session-id";

        let token = auth_service.create_session_token(session_id).unwrap();

        assert!(!token.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_expired_sessions() {
        let auth_service = setup_test_auth_service();

        let result = auth_service.cleanup_expired_sessions().await;

        assert!(result.is_ok());
    }
}

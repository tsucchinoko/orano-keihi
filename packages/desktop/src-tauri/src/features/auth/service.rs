use crate::features::auth::loopback::{LoopbackServer, OAuthCallback};
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
use tokio::sync::oneshot;

/// OAuth認証フローの開始情報（ループバック方式）
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthStartInfo {
    /// 認証URL
    pub auth_url: String,
    /// ループバックサーバーのポート番号
    pub loopback_port: u16,
    /// コールバック受信用のReceiver（内部使用）
    #[serde(skip)]
    pub callback_receiver: Option<oneshot::Receiver<OAuthCallback>>,
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
    /// PKCE検証子の一時保存（実際のプロダクションではより適切な方法を使用）
    pkce_verifier: Arc<Mutex<Option<PkceCodeVerifier>>>,
    /// 現在のリダイレクトURI（トークン交換時に使用）
    current_redirect_uri: Arc<Mutex<Option<String>>>,
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
        // OAuth2クライアントを設定（ネイティブアプリ用：クライアントシークレットなし）
        let client_id = ClientId::new(config.client_id);
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| AuthError::ConfigError(format!("認証URL設定エラー: {e}")))?;
        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .map_err(|e| AuthError::ConfigError(format!("トークンURL設定エラー: {e}")))?;

        // ネイティブアプリではクライアントシークレットを使用しない（PKCE使用）
        // 一時的にクライアントシークレットを使用（テスト用）
        let client_secret = if config.client_secret.is_empty() {
            None
        } else {
            Some(ClientSecret::new(config.client_secret))
        };
        let oauth_client = BasicClient::new(client_id, client_secret, auth_url, Some(token_url));

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
            pkce_verifier: Arc::new(Mutex::new(None)),
            current_redirect_uri: Arc::new(Mutex::new(None)),
        })
    }

    /// OAuth認証フローを開始する（ループバック方式）
    ///
    /// # 戻り値
    /// 認証開始情報（認証URL、ループバックポート、コールバック受信用Receiver）
    pub async fn start_oauth_flow(&self) -> Result<OAuthStartInfo, AuthError> {
        // ループバックサーバーを作成
        let (mut loopback_server, port) = LoopbackServer::new()
            .map_err(|e| AuthError::NetworkError(format!("ループバックサーバー作成エラー: {e}")))?;

        // リダイレクトURIを動的に設定
        let redirect_uri = loopback_server.get_redirect_uri();
        let redirect_url = RedirectUrl::new(redirect_uri.clone())
            .map_err(|e| AuthError::ConfigError(format!("リダイレクトURL設定エラー: {e}")))?;

        // 現在のリダイレクトURIを保存
        {
            let mut current_redirect_uri = self.current_redirect_uri.lock().unwrap();
            *current_redirect_uri = Some(redirect_uri);
        }

        // OAuth2クライアントのリダイレクトURIを更新
        let oauth_client = self.oauth_client.clone().set_redirect_uri(redirect_url);

        // PKCE（Proof Key for Code Exchange）を生成
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // PKCE検証子を保存
        {
            let mut verifier_guard = self.pkce_verifier.lock().unwrap();
            *verifier_guard = Some(pkce_verifier);
        }

        // 認証URLを生成
        let (auth_url, _csrf_token) = oauth_client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        // ループバックサーバーを開始してコールバック待機
        let callback_receiver = loopback_server
            .start_and_wait()
            .await
            .map_err(|e| AuthError::NetworkError(format!("ループバックサーバー開始エラー: {e}")))?;

        let oauth_info = OAuthStartInfo {
            auth_url: auth_url.to_string(),
            loopback_port: port,
            callback_receiver: Some(callback_receiver),
        };

        log::info!("OAuth認証フロー（ループバック方式）を開始しました");
        log::debug!("認証URL: {}", oauth_info.auth_url);
        log::debug!("ループバックポート: {}", port);

        Ok(oauth_info)
    }

    /// 認証コールバックを処理する（ループバック方式）
    ///
    /// # 引数
    /// * `callback_receiver` - コールバック受信用のReceiver
    ///
    /// # 戻り値
    /// 認証されたユーザー情報とセッション
    pub async fn handle_loopback_callback(
        &self,
        callback_receiver: oneshot::Receiver<OAuthCallback>,
    ) -> Result<(User, Session), AuthError> {
        log::info!("ループバック認証コールバックを処理開始");

        // コールバックを待機（タイムアウト付き）
        let callback = tokio::time::timeout(
            std::time::Duration::from_secs(300), // 5分のタイムアウト
            callback_receiver,
        )
        .await
        .map_err(|_| AuthError::OAuthError("認証タイムアウト".to_string()))?
        .map_err(|_| AuthError::OAuthError("コールバック受信エラー".to_string()))?;

        log::debug!(
            "受信したコールバック: code={}, state={}, error={:?}",
            if callback.code.is_empty() {
                "なし"
            } else {
                "あり"
            },
            if callback.state.is_empty() {
                "なし"
            } else {
                "あり"
            },
            callback.error
        );

        // エラーチェック
        if let Some(error) = callback.error {
            return Err(AuthError::OAuthError(format!("OAuth認証エラー: {error}")));
        }

        if callback.code.is_empty() {
            return Err(AuthError::OAuthError("認証コードが空です".to_string()));
        }

        // 認証コードをアクセストークンに交換
        // 保存されたPKCE検証子を取得
        let pkce_verifier = {
            let mut verifier_guard = self.pkce_verifier.lock().unwrap();
            verifier_guard.take()
        };

        let pkce_verifier = pkce_verifier
            .ok_or_else(|| AuthError::OAuthError("PKCE検証子が見つかりません".to_string()))?;

        // 保存されたリダイレクトURIを取得
        let redirect_uri = {
            let redirect_uri_guard = self.current_redirect_uri.lock().unwrap();
            redirect_uri_guard.clone()
        };

        let redirect_uri = redirect_uri
            .ok_or_else(|| AuthError::OAuthError("リダイレクトURIが見つかりません".to_string()))?;

        // リダイレクトURIを設定したOAuth2クライアントを作成
        let redirect_url = RedirectUrl::new(redirect_uri)
            .map_err(|e| AuthError::ConfigError(format!("リダイレクトURL設定エラー: {e}")))?;
        let oauth_client_with_redirect = self.oauth_client.clone().set_redirect_uri(redirect_url);

        let token_result = oauth_client_with_redirect
            .exchange_code(AuthorizationCode::new(callback.code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| {
                log::error!("OAuth2トークン交換エラー: {e:?}");
                AuthError::OAuthError(format!("トークン交換に失敗しました: {e}"))
            })?;

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
        let session = self.session_manager.create_session(&user.id)?;

        log::info!(
            "ループバック認証コールバック処理が完了しました: user_id={}",
            user.id
        );

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
            .get_user_by_id(&session.user_id)
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
        let rt = tokio::runtime::Runtime::new().unwrap();
        let auth_service = setup_test_auth_service();

        let oauth_info = rt
            .block_on(async { auth_service.start_oauth_flow().await })
            .unwrap();

        assert!(!oauth_info.auth_url.is_empty());
        assert!(oauth_info.loopback_port > 0);
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

/// APIサーバー経由のGoogle OAuth認証サービス
///
/// このモジュールは、Google OAuth認証をAPIサーバー経由で行います。
/// デスクトップアプリ側にはGoogle認証情報を保存せず、
/// すべての認証処理をAPIサーバーに委譲します。
use crate::features::auth::loopback::{LoopbackServer, OAuthCallback};
use crate::features::auth::models::{AuthError, Session, User};
use crate::features::auth::repository::UserRepository;
use crate::features::auth::session::SessionManager;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

/// APIサーバーからの認証開始レスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthStartResponse {
    /// Google認証URL
    pub auth_url: String,
    /// CSRF対策用のstate
    pub state: String,
    /// PKCE検証子
    pub code_verifier: String,
}

/// APIサーバーへの認証コールバックリクエスト
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthCallbackRequest {
    /// 認証コード
    pub code: String,
    /// CSRF対策用のstate
    pub state: String,
    /// PKCE検証子
    pub code_verifier: String,
    /// リダイレクトURI
    pub redirect_uri: String,
}

/// APIサーバーからの認証コールバックレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthCallbackResponse {
    /// JWTアクセストークン
    pub access_token: String,
    /// トークンタイプ（通常は"Bearer"）
    pub token_type: String,
    /// トークンの有効期限（秒）
    pub expires_in: u64,
    /// ユーザー情報
    pub user: UserInfo,
}

/// ユーザー情報
#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    /// Google User ID
    pub id: String,
    /// メールアドレス
    pub email: String,
    /// 表示名
    pub name: String,
    /// プロフィール画像URL
    pub picture: Option<String>,
}

/// OAuth認証フローの開始情報（APIサーバー経由）
#[derive(Debug)]
pub struct OAuthStartInfo {
    /// 認証URL
    pub auth_url: String,
    /// ループバックサーバーのポート番号
    pub loopback_port: u16,
    /// CSRF対策用のstate
    pub state: String,
    /// PKCE検証子
    pub code_verifier: String,
    /// コールバック受信用のReceiver
    pub callback_receiver: Option<oneshot::Receiver<OAuthCallback>>,
}

/// APIサーバー経由のOAuth認証サービス
#[derive(Clone)]
pub struct AuthService {
    /// APIサーバーのベースURL
    api_base_url: String,
    /// HTTPクライアント
    http_client: reqwest::Client,
    /// セッション管理
    session_manager: SessionManager,
    /// ユーザーリポジトリ
    user_repository: UserRepository,
}

impl AuthService {
    /// 新しいApiAuthServiceを作成する
    ///
    /// # 引数
    /// * `api_base_url` - APIサーバーのベースURL
    /// * `db_connection` - データベース接続
    /// * `session_encryption_key` - セッション暗号化キー
    ///
    /// # 戻り値
    /// ApiAuthServiceインスタンス
    pub fn new(
        api_base_url: String,
        db_connection: Arc<Mutex<Connection>>,
        session_encryption_key: String,
    ) -> Result<Self, AuthError> {
        // HTTPクライアントを作成
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AuthError::NetworkError(format!("HTTPクライアント作成エラー: {e}")))?;

        // セッション管理を初期化
        let session_manager =
            SessionManager::new(Arc::clone(&db_connection), session_encryption_key);

        // ユーザーリポジトリを初期化
        let user_repository = UserRepository::new(Arc::clone(&db_connection));

        log::info!("ApiAuthServiceを初期化しました: api_base_url={api_base_url}");

        Ok(Self {
            api_base_url,
            http_client,
            session_manager,
            user_repository,
        })
    }

    /// OAuth認証フローを開始する（APIサーバー経由）
    ///
    /// # 戻り値
    /// 認証開始情報
    pub async fn start_oauth_flow(&self) -> Result<OAuthStartInfo, AuthError> {
        // ループバックサーバーを作成
        let (mut loopback_server, port) = LoopbackServer::new()
            .map_err(|e| AuthError::NetworkError(format!("ループバックサーバー作成エラー: {e}")))?;

        // リダイレクトURIを動的に設定
        let redirect_uri = loopback_server.get_redirect_uri();

        log::debug!("ループバックサーバーを起動しました: port={port}, redirect_uri={redirect_uri}");

        // APIサーバーに認証開始リクエストを送信
        let auth_start_url = format!("{}/api/v1/auth/google/start", self.api_base_url);
        let request_body = serde_json::json!({
            "redirect_uri": redirect_uri,
        });

        log::debug!("APIサーバーに認証開始リクエストを送信: url={auth_start_url}");

        let response = self
            .http_client
            .post(&auth_start_url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AuthError::NetworkError(format!("認証開始リクエストエラー: {e}")))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "不明なエラー".to_string());
            return Err(AuthError::OAuthError(format!(
                "認証開始リクエストが失敗しました: {error_text}"
            )));
        }

        let auth_start_response: AuthStartResponse = response
            .json()
            .await
            .map_err(|e| AuthError::OAuthError(format!("認証開始レスポンスのパースエラー: {e}")))?;

        log::debug!("APIサーバーから認証URLを取得しました");

        // ループバックサーバーを開始してコールバック待機
        let callback_receiver = loopback_server
            .start_and_wait()
            .await
            .map_err(|e| AuthError::NetworkError(format!("ループバックサーバー開始エラー: {e}")))?;

        let oauth_info = OAuthStartInfo {
            auth_url: auth_start_response.auth_url,
            loopback_port: port,
            state: auth_start_response.state,
            code_verifier: auth_start_response.code_verifier,
            callback_receiver: Some(callback_receiver),
        };

        log::info!("OAuth認証フロー（APIサーバー経由）を開始しました");

        Ok(oauth_info)
    }

    /// 認証コールバックを処理する（APIサーバー経由）
    ///
    /// # 引数
    /// * `callback_receiver` - コールバック受信用のReceiver
    /// * `state` - CSRF対策用のstate
    /// * `code_verifier` - PKCE検証子
    /// * `redirect_uri` - リダイレクトURI
    ///
    /// # 戻り値
    /// 認証されたユーザー情報とセッション
    pub async fn handle_loopback_callback(
        &self,
        callback_receiver: oneshot::Receiver<OAuthCallback>,
        state: String,
        code_verifier: String,
        redirect_uri: String,
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

        // stateの検証
        if callback.state != state {
            return Err(AuthError::OAuthError(
                "stateの検証に失敗しました（CSRF攻撃の可能性）".to_string(),
            ));
        }

        // APIサーバーに認証コールバックリクエストを送信
        let auth_callback_url = format!("{}/api/v1/auth/google/callback", self.api_base_url);
        let request_body = AuthCallbackRequest {
            code: callback.code,
            state: callback.state,
            code_verifier,
            redirect_uri,
        };

        log::debug!("APIサーバーに認証コールバックリクエストを送信: url={auth_callback_url}");

        let response = self
            .http_client
            .post(&auth_callback_url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                AuthError::NetworkError(format!("認証コールバックリクエストエラー: {e}"))
            })?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "不明なエラー".to_string());
            return Err(AuthError::OAuthError(format!(
                "認証コールバックリクエストが失敗しました: {error_text}"
            )));
        }

        let auth_callback_response: AuthCallbackResponse = response.json().await.map_err(|e| {
            AuthError::OAuthError(format!("認証コールバックレスポンスのパースエラー: {e}"))
        })?;

        log::info!(
            "APIサーバーからユーザー情報を取得しました: email={}",
            auth_callback_response.user.email
        );

        // ユーザー情報をローカルデータベースに保存
        let google_user = crate::features::auth::models::GoogleUser {
            id: auth_callback_response.user.id,
            email: auth_callback_response.user.email,
            name: auth_callback_response.user.name,
            picture: auth_callback_response.user.picture,
            verified_email: true, // APIサーバーで検証済み
        };

        let user = self
            .user_repository
            .find_or_create_user(google_user)
            .await?;

        // セッションを作成（JWTトークンを保存）
        let session = self.session_manager.create_session(&user.id)?;

        // JWTトークンをセッションに関連付けて保存（実装は後で追加）
        // TODO: JWTトークンをセキュアストレージに保存

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::database::connection::create_in_memory_connection;

    fn setup_test_api_auth_service() -> AuthService {
        let conn = create_in_memory_connection().unwrap();

        AuthService::new(
            "http://localhost:8787".to_string(),
            Arc::new(Mutex::new(conn)),
            "test_encryption_key_32_bytes_long".to_string(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_create_session_token() {
        let auth_service = setup_test_api_auth_service();
        let session_id = "test-session-id";

        let token = auth_service.create_session_token(session_id).unwrap();

        assert!(!token.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_expired_sessions() {
        let auth_service = setup_test_api_auth_service();

        let result = auth_service.cleanup_expired_sessions().await;

        assert!(result.is_ok());
    }
}

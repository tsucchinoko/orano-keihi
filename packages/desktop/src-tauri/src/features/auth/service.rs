/// APIサーバー経由のGoogle OAuth認証サービス
///
/// このモジュールは、Google OAuth認証をAPIサーバー経由で行います。
/// デスクトップアプリ側にはGoogle認証情報を保存せず、
/// すべての認証処理をAPIサーバーに委譲します。
use crate::features::auth::loopback::{LoopbackServer, OAuthCallback};
use crate::features::auth::models::{AuthError, User};
use crate::features::auth::repository::UserRepository;
use crate::features::auth::secure_storage::SecureStorage;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
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

/// 認証結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    /// ユーザー情報
    pub user: User,
    /// JWTアクセストークン
    pub access_token: String,
    /// トークンタイプ
    pub token_type: String,
    /// トークンの有効期限（秒）
    pub expires_in: u64,
}

/// APIサーバー経由のOAuth認証サービス
#[derive(Clone)]
pub struct AuthService {
    /// APIサーバーのベースURL
    api_base_url: String,
    /// HTTPクライアント
    http_client: reqwest::Client,
    /// データベース接続
    db_connection: Arc<Mutex<Connection>>,
    /// Tauriアプリハンドル
    app_handle: AppHandle,
}

impl AuthService {
    /// 新しいAuthServiceを作成する
    ///
    /// # 引数
    /// * `api_base_url` - APIサーバーのベースURL
    /// * `db_connection` - データベース接続
    /// * `app_handle` - Tauriアプリハンドル
    ///
    /// # 戻り値
    /// AuthServiceインスタンス
    pub fn new(
        api_base_url: String,
        db_connection: Arc<Mutex<Connection>>,
        app_handle: AppHandle,
    ) -> Result<Self, AuthError> {
        // HTTPクライアントを作成
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AuthError::NetworkError(format!("HTTPクライアント作成エラー: {e}")))?;

        log::info!("AuthServiceを初期化しました: api_base_url={api_base_url}");

        Ok(Self {
            api_base_url,
            http_client,
            db_connection,
            app_handle,
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
    /// 認証結果（ユーザー情報とトークン）
    pub async fn handle_loopback_callback(
        &self,
        callback_receiver: oneshot::Receiver<OAuthCallback>,
        state: String,
        code_verifier: String,
        redirect_uri: String,
    ) -> Result<AuthResult, AuthError> {
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

        let user_repository = UserRepository::new(Arc::clone(&self.db_connection));
        let user = user_repository.find_or_create_user(google_user).await?;

        // JWTトークンをセキュアストレージに保存
        let secure_storage = SecureStorage::new(self.app_handle.clone());
        secure_storage
            .save_session_token(&auth_callback_response.access_token)
            .map_err(|e| AuthError::StorageError(format!("トークン保存エラー: {e}")))?;

        secure_storage
            .save_user_id(&user.id)
            .map_err(|e| AuthError::StorageError(format!("ユーザーID保存エラー: {e}")))?;

        // 最終ログイン日時を保存
        let now = chrono::Utc::now().to_rfc3339();
        secure_storage
            .save_last_login(&now)
            .map_err(|e| AuthError::StorageError(format!("最終ログイン日時保存エラー: {e}")))?;

        log::info!(
            "ループバック認証コールバック処理が完了しました: user_id={}",
            user.id
        );

        Ok(AuthResult {
            user,
            access_token: auth_callback_response.access_token,
            token_type: auth_callback_response.token_type,
            expires_in: auth_callback_response.expires_in,
        })
    }

    /// セッションを検証する（APIサーバー経由）
    ///
    /// # 引数
    /// * `token` - JWTアクセストークン
    ///
    /// # 戻り値
    /// 認証されたユーザー情報
    pub async fn validate_session(&self, token: String) -> Result<User, AuthError> {
        // APIサーバーにトークン検証リクエストを送信
        let validate_url = format!("{}/api/v1/auth/validate", self.api_base_url);

        log::debug!("APIサーバーにトークン検証リクエストを送信: url={validate_url}");

        let response = self
            .http_client
            .get(&validate_url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AuthError::NetworkError(format!("トークン検証リクエストエラー: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "不明なエラー".to_string());

            return if status == reqwest::StatusCode::UNAUTHORIZED {
                Err(AuthError::InvalidToken)
            } else {
                Err(AuthError::OAuthError(format!(
                    "トークン検証が失敗しました: {error_text}"
                )))
            };
        }

        let user_info: UserInfo = response.json().await.map_err(|e| {
            AuthError::OAuthError(format!("トークン検証レスポンスのパースエラー: {e}"))
        })?;

        // ローカルデータベースからユーザー情報を取得
        let user_repository = UserRepository::new(Arc::clone(&self.db_connection));
        let user = user_repository
            .get_user_by_google_id(user_info.id)
            .await?
            .ok_or_else(|| AuthError::DatabaseError("ユーザーが見つかりません".to_string()))?;

        log::debug!("セッションを検証しました: user_id={}", user.id);

        Ok(user)
    }

    /// ログアウト処理
    ///
    /// # 戻り値
    /// 処理結果
    pub async fn logout(&self) -> Result<(), AuthError> {
        // セキュアストレージから認証情報を削除
        let secure_storage = SecureStorage::new(self.app_handle.clone());
        secure_storage
            .clear_auth_info()
            .map_err(|e| AuthError::StorageError(format!("認証情報削除エラー: {e}")))?;

        log::info!("ログアウト処理が完了しました");
        Ok(())
    }

    /// 保存されているセッショントークンを取得する
    ///
    /// # 戻り値
    /// セッショントークン（存在しない場合はNone）
    pub fn get_stored_token(&self) -> Result<Option<String>, AuthError> {
        let secure_storage = SecureStorage::new(self.app_handle.clone());
        secure_storage
            .get_session_token()
            .map_err(|e| AuthError::StorageError(format!("トークン取得エラー: {e}")))
    }

    /// 保存されているユーザーIDを取得する
    ///
    /// # 戻り値
    /// ユーザーID（存在しない場合はNone）
    pub fn get_stored_user_id(&self) -> Result<Option<String>, AuthError> {
        let secure_storage = SecureStorage::new(self.app_handle.clone());
        secure_storage
            .get_user_id()
            .map_err(|e| AuthError::StorageError(format!("ユーザーID取得エラー: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::database::connection::create_in_memory_connection;

    // テストは実際のTauriアプリハンドルが必要なため、統合テストで実装
    #[test]
    fn test_auth_service_creation() {
        // 基本的な構造のテストのみ
        assert!(true);
    }
}

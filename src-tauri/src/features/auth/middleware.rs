use crate::features::auth::models::{AuthError, User};
use crate::features::auth::service::AuthService;
use crate::features::security::service::SecurityService;
use std::sync::Arc;

/// API認証ミドルウェア
/// すべてのAPIリクエストに認証トークンを含めて、不正アクセスを検出・処理する
#[derive(Clone)]
pub struct AuthMiddleware {
    /// 認証サービス
    auth_service: Arc<AuthService>,
    /// セキュリティサービス
    security_service: Arc<SecurityService>,
}

impl AuthMiddleware {
    /// 新しいAuthMiddlewareを作成する
    ///
    /// # 引数
    /// * `auth_service` - 認証サービス
    /// * `security_service` - セキュリティサービス
    ///
    /// # 戻り値
    /// AuthMiddlewareインスタンス
    pub fn new(auth_service: Arc<AuthService>, security_service: Arc<SecurityService>) -> Self {
        Self {
            auth_service,
            security_service,
        }
    }

    /// APIリクエストの認証を検証する
    ///
    /// # 引数
    /// * `token` - 認証トークン
    /// * `request_path` - リクエストパス
    ///
    /// # 戻り値
    /// 認証されたユーザー情報
    pub async fn authenticate_request(
        &self,
        token: Option<&str>,
        request_path: &str,
    ) -> Result<User, AuthError> {
        log::debug!("APIリクエスト認証を開始: path={request_path}");

        // トークンが提供されているかチェック
        let token = token.ok_or_else(|| {
            log::warn!("認証トークンが提供されていません: path={request_path}");
            self.log_unauthorized_access(request_path, None);
            AuthError::InvalidToken
        })?;

        // トークンが空でないかチェック
        if token.is_empty() {
            log::warn!("空の認証トークンが提供されました: path={request_path}");
            self.log_unauthorized_access(request_path, Some(token));
            return Err(AuthError::InvalidToken);
        }

        // セキュリティサービスでトークンの形式を検証
        let is_valid_format = self
            .security_service
            .verify_api_request(token)
            .map_err(|e| {
                log::error!("トークン形式検証エラー: {e}");
                AuthError::SecurityError(e.to_string())
            })?;

        if !is_valid_format {
            log::warn!("無効なトークン形式: path={request_path}");
            self.log_unauthorized_access(request_path, Some(token));
            return Err(AuthError::InvalidToken);
        }

        // 認証サービスでセッションを検証
        match self.auth_service.validate_session(token.to_string()).await {
            Ok(user) => {
                log::debug!(
                    "APIリクエスト認証成功: user_id={}, path={request_path}",
                    user.id
                );
                Ok(user)
            }
            Err(e) => {
                log::warn!("セッション検証失敗: {e}, path={request_path}");
                self.log_unauthorized_access(request_path, Some(token));
                Err(e)
            }
        }
    }

    /// 認証が必要なAPIリクエストを処理する
    ///
    /// # 引数
    /// * `token` - 認証トークン
    /// * `request_path` - リクエストパス
    /// * `handler` - 認証成功時に実行するハンドラー
    ///
    /// # 戻り値
    /// ハンドラーの実行結果
    pub async fn require_auth<T, F, Fut>(
        &self,
        token: Option<&str>,
        request_path: &str,
        handler: F,
    ) -> Result<T, AuthError>
    where
        F: FnOnce(User) -> Fut,
        Fut: std::future::Future<Output = Result<T, AuthError>>,
    {
        let user = self.authenticate_request(token, request_path).await?;
        handler(user).await
    }

    /// オプション認証でAPIリクエストを処理する
    ///
    /// # 引数
    /// * `token` - 認証トークン（オプション）
    /// * `request_path` - リクエストパス
    /// * `handler` - 実行するハンドラー
    ///
    /// # 戻り値
    /// ハンドラーの実行結果
    pub async fn optional_auth<T, F, Fut>(
        &self,
        token: Option<&str>,
        request_path: &str,
        handler: F,
    ) -> Result<T, AuthError>
    where
        F: FnOnce(Option<User>) -> Fut,
        Fut: std::future::Future<Output = Result<T, AuthError>>,
    {
        let user = match token {
            Some(token) if !token.is_empty() => {
                match self.authenticate_request(Some(token), request_path).await {
                    Ok(user) => Some(user),
                    Err(e) => {
                        log::debug!("オプション認証失敗（続行）: {e}");
                        None
                    }
                }
            }
            _ => None,
        };

        handler(user).await
    }

    /// 管理者権限が必要なAPIリクエストを処理する
    ///
    /// # 引数
    /// * `token` - 認証トークン
    /// * `request_path` - リクエストパス
    /// * `handler` - 認証・認可成功時に実行するハンドラー
    ///
    /// # 戻り値
    /// ハンドラーの実行結果
    pub async fn require_admin<T, F, Fut>(
        &self,
        token: Option<&str>,
        request_path: &str,
        handler: F,
    ) -> Result<T, AuthError>
    where
        F: FnOnce(User) -> Fut,
        Fut: std::future::Future<Output = Result<T, AuthError>>,
    {
        let user = self.authenticate_request(token, request_path).await?;

        // 管理者権限チェック（現在は簡単な実装、将来的にはロールベースアクセス制御を実装）
        if !self.is_admin_user(&user) {
            log::warn!(
                "管理者権限が必要なリクエストに非管理者がアクセス: user_id={}, path={request_path}",
                user.id
            );
            self.log_unauthorized_access(request_path, token);
            return Err(AuthError::InsufficientPermissions);
        }

        handler(user).await
    }

    /// ユーザーが管理者かどうかを判定する
    ///
    /// # 引数
    /// * `user` - ユーザー情報
    ///
    /// # 戻り値
    /// 管理者かどうか
    fn is_admin_user(&self, user: &User) -> bool {
        // 現在は簡単な実装：特定のメールアドレスを管理者とする
        // 将来的にはデータベースのロール情報を参照する
        const ADMIN_EMAILS: &[&str] = &[
            // 必要に応じて管理者のメールアドレスを追加
        ];

        ADMIN_EMAILS.contains(&user.email.as_str())
    }

    /// 不正アクセスをログに記録する
    ///
    /// # 引数
    /// * `request_path` - リクエストパス
    /// * `token` - 認証トークン（オプション）
    fn log_unauthorized_access(&self, request_path: &str, token: Option<&str>) {
        let request_info = format!("不正アクセス試行: path={request_path}");

        if let Err(e) = self
            .security_service
            .detect_unauthorized_access(&request_info, token)
        {
            log::error!("不正アクセスログ記録エラー: {e}");
        }
    }

    /// レート制限チェック（将来の拡張用）
    ///
    /// # 引数
    /// * `user_id` - ユーザーID
    /// * `request_path` - リクエストパス
    ///
    /// # 戻り値
    /// レート制限に引っかかっていないかどうか
    pub fn check_rate_limit(&self, user_id: i64, request_path: &str) -> bool {
        // TODO: 実際のレート制限実装
        // 現在は常にtrueを返す（制限なし）
        log::debug!("レート制限チェック: user_id={user_id}, path={request_path}");
        true
    }

    /// APIキーベースの認証（将来の拡張用）
    ///
    /// # 引数
    /// * `api_key` - APIキー
    /// * `request_path` - リクエストパス
    ///
    /// # 戻り値
    /// 認証されたユーザー情報
    pub async fn authenticate_api_key(
        &self,
        api_key: &str,
        request_path: &str,
    ) -> Result<User, AuthError> {
        // TODO: APIキーベースの認証実装
        log::debug!("APIキー認証: path={request_path}");
        Err(AuthError::NotImplemented(
            "APIキー認証は未実装です".to_string(),
        ))
    }
}

/// 認証ヘルパー関数
pub mod auth_helpers {
    use super::*;

    /// リクエストヘッダーから認証トークンを抽出する
    ///
    /// # 引数
    /// * `authorization_header` - Authorizationヘッダーの値
    ///
    /// # 戻り値
    /// 抽出されたトークン
    pub fn extract_bearer_token(authorization_header: Option<&str>) -> Option<&str> {
        authorization_header
            .and_then(|header| header.strip_prefix("Bearer "))
            .map(|token| token.trim())
            .filter(|token| !token.is_empty())
    }

    /// セッショントークンを検証する
    ///
    /// # 引数
    /// * `token` - セッショントークン
    ///
    /// # 戻り値
    /// トークンが有効な形式かどうか
    pub fn is_valid_token_format(token: &str) -> bool {
        // 基本的な形式チェック
        !token.is_empty() && token.len() > 10 && token.is_ascii()
    }

    /// リクエストパスが認証不要かどうかを判定する
    ///
    /// # 引数
    /// * `path` - リクエストパス
    ///
    /// # 戻り値
    /// 認証不要かどうか
    pub fn is_public_endpoint(path: &str) -> bool {
        const PUBLIC_PATHS: &[&str] = &["/auth/start", "/auth/callback", "/health", "/version"];

        PUBLIC_PATHS
            .iter()
            .any(|&public_path| path.starts_with(public_path))
    }

    /// リクエストパスが管理者権限必要かどうかを判定する
    ///
    /// # 引数
    /// * `path` - リクエストパス
    ///
    /// # 戻り値
    /// 管理者権限が必要かどうか
    pub fn requires_admin_permission(path: &str) -> bool {
        const ADMIN_PATHS: &[&str] = &["/admin/", "/security/", "/system/"];

        ADMIN_PATHS
            .iter()
            .any(|&admin_path| path.starts_with(admin_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::auth::models::GoogleUser;
    use crate::features::security::models::SecurityConfig;
    use crate::shared::config::environment::GoogleOAuthConfig;
    use crate::shared::database::connection::create_in_memory_connection;
    use std::sync::{Arc, Mutex};

    async fn setup_test_middleware() -> AuthMiddleware {
        let conn = create_in_memory_connection().unwrap();

        let oauth_config = GoogleOAuthConfig {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            redirect_uri: "http://localhost:3000/auth/callback".to_string(),
            session_encryption_key: "test_encryption_key_32_bytes_long".to_string(),
        };

        let security_config = SecurityConfig {
            encryption_key: "test_encryption_key_32_bytes_long".to_string(),
            max_token_age_hours: 24,
            enable_audit_logging: true,
        };

        let auth_service =
            Arc::new(AuthService::new(oauth_config, Arc::new(Mutex::new(conn))).unwrap());
        let security_service = Arc::new(SecurityService::new(security_config).unwrap());

        AuthMiddleware::new(auth_service, security_service)
    }

    #[tokio::test]
    async fn test_authenticate_request_no_token() {
        let middleware = setup_test_middleware().await;

        let result = middleware.authenticate_request(None, "/test").await;
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[tokio::test]
    async fn test_authenticate_request_empty_token() {
        let middleware = setup_test_middleware().await;

        let result = middleware.authenticate_request(Some(""), "/test").await;
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn test_extract_bearer_token() {
        use auth_helpers::*;

        assert_eq!(
            extract_bearer_token(Some("Bearer token123")),
            Some("token123")
        );
        assert_eq!(extract_bearer_token(Some("Bearer ")), None);
        assert_eq!(extract_bearer_token(Some("token123")), None);
        assert_eq!(extract_bearer_token(None), None);
    }

    #[test]
    fn test_is_valid_token_format() {
        use auth_helpers::*;

        assert!(is_valid_token_format("valid_token_123"));
        assert!(!is_valid_token_format(""));
        assert!(!is_valid_token_format("short"));
    }

    #[test]
    fn test_is_public_endpoint() {
        use auth_helpers::*;

        assert!(is_public_endpoint("/auth/start"));
        assert!(is_public_endpoint("/auth/callback"));
        assert!(is_public_endpoint("/health"));
        assert!(!is_public_endpoint("/expenses"));
        assert!(!is_public_endpoint("/admin/users"));
    }

    #[test]
    fn test_requires_admin_permission() {
        use auth_helpers::*;

        assert!(requires_admin_permission("/admin/users"));
        assert!(requires_admin_permission("/security/config"));
        assert!(requires_admin_permission("/system/status"));
        assert!(!requires_admin_permission("/expenses"));
        assert!(!requires_admin_permission("/auth/start"));
    }
}

// ユーザー認証付きR2コマンド
// R2ユーザーディレクトリ移行機能のための認証付きコマンド

use std::sync::Arc;

use crate::features::auth::middleware::AuthMiddleware;
use crate::features::auth::service::AuthService;
use crate::features::security::service::SecurityManager;
use tauri::State;

/// 認証ミドルウェアを作成するヘルパー関数
#[allow(dead_code)]
async fn create_auth_middleware(
    auth_service: &State<'_, AuthService>,
    security_manager: &SecurityManager,
) -> Result<AuthMiddleware, String> {
    let auth_service = Arc::new(auth_service.inner().clone());

    // セキュリティサービスを取得（SecurityManagerはSecurityServiceのエイリアス）
    let security_service = Arc::new(security_manager.clone());

    Ok(AuthMiddleware::new(auth_service, security_service))
}

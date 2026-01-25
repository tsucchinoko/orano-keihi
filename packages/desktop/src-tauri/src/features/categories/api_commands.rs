/// API Server経由でのカテゴリー操作コマンド
///
/// API Serverを使用してカテゴリーデータを取得します
use crate::features::auth::middleware::AuthMiddleware;
use crate::features::categories::models::*;
use crate::shared::api_client::ApiClient;
use log::info;
use tauri::State;

/// カテゴリー一覧を取得する（API Server経由）
///
/// # 引数
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// カテゴリー一覧、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_categories(
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Vec<Category>, String> {
    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/categories/list")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // API Serverにカテゴリー一覧取得リクエストを送信
    let response: CategoriesResponse = api_client
        .get("/api/v1/categories", session_token.as_deref())
        .await
        .map_err(|e| format!("カテゴリー一覧取得APIエラー: {e}"))?;

    info!("カテゴリー一覧取得成功: count={}", response.count);
    Ok(response.categories)
}

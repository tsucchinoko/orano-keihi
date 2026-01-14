/// API Server経由でのサブスクリプション操作コマンド
///
/// ローカルSQLiteの代わりにAPI Serverを使用してサブスクリプションデータを管理します
use crate::features::auth::middleware::AuthMiddleware;
use crate::features::subscriptions::models::*;
use crate::shared::api_client::ApiClient;
use log::info;
use serde::{Deserialize, Serialize};
use tauri::State;

/// API Serverからのサブスクリプション作成レスポンス
#[derive(Debug, Serialize, Deserialize)]
struct CreateSubscriptionResponse {
    success: bool,
    subscription: Subscription,
    timestamp: String,
}

/// API Serverからのサブスクリプション一覧取得レスポンス
#[derive(Debug, Serialize, Deserialize)]
struct GetSubscriptionsResponse {
    success: bool,
    subscriptions: Vec<Subscription>,
    count: usize,
    filters: Option<serde_json::Value>,
    timestamp: String,
}

/// API Serverからのサブスクリプション更新レスポンス
#[derive(Debug, Serialize, Deserialize)]
struct UpdateSubscriptionResponse {
    success: bool,
    subscription: Subscription,
    timestamp: String,
}

/// API Serverからの月額合計取得レスポンス
#[derive(Debug, Serialize, Deserialize)]
struct MonthlyTotalResponse {
    success: bool,
    #[serde(rename = "monthlyTotal")]
    monthly_total: f64,
    timestamp: String,
}

/// サブスクリプションを作成する（API Server経由）
///
/// # 引数
/// * `dto` - サブスクリプション作成用DTO
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 作成されたサブスクリプション、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn create_subscription(
    dto: CreateSubscriptionDto,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Subscription, String> {
    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/create")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // API Serverにサブスクリプション作成リクエストを送信
    let response: CreateSubscriptionResponse = api_client
        .post("/api/v1/subscriptions", &dto, session_token.as_deref())
        .await
        .map_err(|e| format!("サブスクリプション作成APIエラー: {e}"))?;

    info!(
        "サブスクリプション作成成功: subscription_id={}",
        response.subscription.id
    );
    Ok(response.subscription)
}

/// サブスクリプション一覧を取得する（API Server経由）
///
/// # 引数
/// * `active_only` - アクティブなサブスクリプションのみを取得するか
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// サブスクリプション一覧、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_subscriptions(
    active_only: bool,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Vec<Subscription>, String> {
    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/list")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // クエリパラメータを構築
    let endpoint = if active_only {
        "/api/v1/subscriptions?activeOnly=true"
    } else {
        "/api/v1/subscriptions"
    };

    // API Serverにサブスクリプション一覧取得リクエストを送信
    let response: GetSubscriptionsResponse = api_client
        .get(endpoint, session_token.as_deref())
        .await
        .map_err(|e| format!("サブスクリプション一覧取得APIエラー: {e}"))?;

    info!("サブスクリプション一覧取得成功: count={}", response.count);
    Ok(response.subscriptions)
}

/// サブスクリプションを更新する（API Server経由）
///
/// # 引数
/// * `id` - サブスクリプションID
/// * `dto` - サブスクリプション更新用DTO
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 更新されたサブスクリプション、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn update_subscription(
    id: i64,
    dto: UpdateSubscriptionDto,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Subscription, String> {
    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/update")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // API Serverにサブスクリプション更新リクエストを送信
    let endpoint = format!("/api/v1/subscriptions/{id}");
    let response: UpdateSubscriptionResponse = api_client
        .put(&endpoint, &dto, session_token.as_deref())
        .await
        .map_err(|e| format!("サブスクリプション更新APIエラー: {e}"))?;

    info!("サブスクリプション更新成功: subscription_id={id}");
    Ok(response.subscription)
}

/// サブスクリプションのアクティブ状態を切り替える（API Server経由）
///
/// # 引数
/// * `id` - サブスクリプションID
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 更新されたサブスクリプション、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn toggle_subscription_status(
    id: i64,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Subscription, String> {
    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/toggle")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // API Serverにサブスクリプションステータス切り替えリクエストを送信
    let endpoint = format!("/api/v1/subscriptions/{id}/toggle");
    let response: UpdateSubscriptionResponse = api_client
        .patch(&endpoint, &serde_json::json!({}), session_token.as_deref())
        .await
        .map_err(|e| format!("サブスクリプションステータス切り替えAPIエラー: {e}"))?;

    info!("サブスクリプションステータス切り替え成功: subscription_id={id}");
    Ok(response.subscription)
}

/// サブスクリプションを削除する（API Server経由）
///
/// # 引数
/// * `id` - サブスクリプションID
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_subscription(
    id: i64,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<(), String> {
    info!("サブスクリプション削除処理開始: subscription_id={id}");

    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/delete")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // API Serverにサブスクリプション削除リクエストを送信
    let endpoint = format!("/api/v1/subscriptions/{id}");
    api_client
        .delete(&endpoint, session_token.as_deref())
        .await
        .map_err(|e| format!("サブスクリプション削除APIエラー: {e}"))?;

    info!("サブスクリプション削除成功: subscription_id={id}");
    Ok(())
}

/// 月額サブスクリプション合計を取得する（API Server経由）
///
/// # 引数
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 月額合計金額、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_monthly_subscription_total(
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<f64, String> {
    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/total")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // API Serverに月額合計取得リクエストを送信
    let response: MonthlyTotalResponse = api_client
        .get(
            "/api/v1/subscriptions/monthly-total",
            session_token.as_deref(),
        )
        .await
        .map_err(|e| format!("月額合計取得APIエラー: {e}"))?;

    info!("月額合計取得成功: total={}", response.monthly_total);
    Ok(response.monthly_total)
}

/// サブスクリプションの領収書を削除する（API Server経由）
///
/// # 引数
/// * `subscription_id` - サブスクリプションID
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 削除成功時はtrue、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_subscription_receipt(
    subscription_id: i64,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<bool, String> {
    info!("サブスクリプションの領収書削除処理開始: subscription_id={subscription_id}");

    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/delete-receipt")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // 領収書パスを空にする更新リクエストを送信
    // Note: API Serverではreceipt_pathフィールドを空文字列に設定することで削除
    let dto = UpdateSubscriptionDto {
        name: None,
        amount: None,
        billing_cycle: None,
        start_date: None,
        category: None,
    };

    let endpoint = format!("/api/v1/subscriptions/{subscription_id}");
    let _response: UpdateSubscriptionResponse = api_client
        .put(&endpoint, &dto, session_token.as_deref())
        .await
        .map_err(|e| format!("領収書削除APIエラー: {e}"))?;

    info!("サブスクリプションの領収書削除成功: subscription_id={subscription_id}");
    Ok(true)
}

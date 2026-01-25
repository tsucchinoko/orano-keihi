/// API Server経由での経費操作コマンド
///
/// ローカルSQLiteの代わりにAPI Serverを使用して経費データを管理します
use crate::features::auth::middleware::AuthMiddleware;
use crate::features::expenses::models::*;
use crate::shared::api_client::ApiClient;
use log::info;
use serde::{Deserialize, Serialize};
use tauri::State;

/// API Serverからの経費作成レスポンス
#[derive(Debug, Serialize, Deserialize)]
struct CreateExpenseResponse {
    success: bool,
    expense: Expense,
    timestamp: String,
}

/// API Serverからの経費一覧取得レスポンス
#[derive(Debug, Serialize, Deserialize)]
struct GetExpensesResponse {
    success: bool,
    expenses: Vec<Expense>,
    count: usize,
    filters: Option<serde_json::Value>,
    timestamp: String,
}

/// API Serverからの経費更新レスポンス
#[derive(Debug, Serialize, Deserialize)]
struct UpdateExpenseResponse {
    success: bool,
    expense: Expense,
    timestamp: String,
}

/// 経費を作成する（API Server経由）
///
/// # 引数
/// * `dto` - 経費作成用DTO
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 作成された経費、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn create_expense(
    dto: CreateExpenseDto,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Expense, String> {
    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/expenses/create")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // API Serverに経費作成リクエストを送信
    let response: CreateExpenseResponse = api_client
        .post("/api/v1/expenses", &dto, session_token.as_deref())
        .await
        .map_err(|e| format!("経費作成APIエラー: {e}"))?;

    info!("経費作成成功: expense_id={}", response.expense.id);
    Ok(response.expense)
}

/// 経費一覧を取得する（API Server経由）
///
/// # 引数
/// * `month` - 月フィルター（オプション、YYYY-MM形式）
/// * `category` - カテゴリフィルター（オプション）
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 経費一覧、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_expenses(
    month: Option<String>,
    category: Option<String>,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Vec<Expense>, String> {
    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/expenses/list")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // クエリパラメータを構築
    let mut endpoint = "/api/v1/expenses".to_string();
    let mut params = vec![];

    if let Some(m) = month {
        params.push(format!("month={m}"));
    }
    if let Some(c) = category {
        params.push(format!("category={c}"));
    }

    if !params.is_empty() {
        endpoint.push('?');
        endpoint.push_str(&params.join("&"));
    }

    // API Serverに経費一覧取得リクエストを送信
    let response: GetExpensesResponse = api_client
        .get(&endpoint, session_token.as_deref())
        .await
        .map_err(|e| format!("経費一覧取得APIエラー: {e}"))?;

    info!("経費一覧取得成功: count={}", response.count);
    Ok(response.expenses)
}

/// 経費を更新する（API Server経由）
///
/// # 引数
/// * `id` - 経費ID
/// * `dto` - 経費更新用DTO
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 更新された経費、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn update_expense(
    id: i64,
    dto: UpdateExpenseDto,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Expense, String> {
    info!("経費更新処理開始: expense_id={id}, dto={dto:?}");

    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/expenses/update")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // API Serverに経費更新リクエストを送信
    let endpoint = format!("/api/v1/expenses/{id}");
    let response: UpdateExpenseResponse = api_client
        .put(&endpoint, &dto, session_token.as_deref())
        .await
        .map_err(|e| format!("経費更新APIエラー: {e}"))?;

    info!("経費更新成功: expense_id={id}");
    Ok(response.expense)
}

/// 経費を削除する（API Server経由）
///
/// # 引数
/// * `id` - 経費ID
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_expense(
    id: i64,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<(), String> {
    info!("経費削除処理開始: expense_id={id}");

    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/expenses/delete")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // API Serverに経費削除リクエストを送信
    let endpoint = format!("/api/v1/expenses/{id}");
    api_client
        .delete(&endpoint, session_token.as_deref())
        .await
        .map_err(|e| format!("経費削除APIエラー: {e}"))?;

    info!("経費削除成功: expense_id={id}");
    Ok(())
}

/// 経費の領収書を削除する（API Server経由）
///
/// # 引数
/// * `expense_id` - 経費ID
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 削除成功時はtrue、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_expense_receipt(
    expense_id: i64,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<bool, String> {
    info!("経費の領収書削除処理開始: expense_id={expense_id}");

    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/expenses/delete-receipt")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| format!("APIクライアント作成エラー: {e}"))?;

    // 領収書URLを空文字列にする更新リクエストを送信
    // APIサーバー側で空文字列をNULLに変換する
    let dto = UpdateExpenseDto {
        date: None,
        amount: None,
        category: None,
        category_id: None,
        description: None,
        receipt_url: Some("".to_string()),
    };

    let endpoint = format!("/api/v1/expenses/{expense_id}");
    let _response: UpdateExpenseResponse = api_client
        .put(&endpoint, &dto, session_token.as_deref())
        .await
        .map_err(|e| format!("領収書削除APIエラー: {e}"))?;

    info!("経費の領収書削除成功: expense_id={expense_id}");
    Ok(true)
}

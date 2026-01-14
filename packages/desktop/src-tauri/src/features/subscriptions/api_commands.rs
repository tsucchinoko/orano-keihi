use super::models::{CreateSubscriptionDto, Subscription, UpdateSubscriptionDto};
use crate::features::auth::middleware::AuthMiddleware;
use crate::shared::api_client::ApiClient;
use base64::{engine::general_purpose, Engine as _};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::State;

/// サブスクリプション一覧のレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct SubscriptionListResponse {
    pub subscriptions: Vec<Subscription>,
    pub total: usize,
}

/// 月額合計のレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct MonthlyTotalResponse {
    #[serde(rename = "monthlyTotal")]
    pub monthly_total: f64,
    #[serde(rename = "activeSubscriptions")]
    pub active_subscriptions: i32,
}

/// APIサーバー経由でサブスクリプション一覧を取得する
///
/// # 引数
/// * `active_only` - アクティブなサブスクリプションのみを取得するか
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// サブスクリプション一覧のレスポンス、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn fetch_subscriptions_via_api(
    active_only: bool,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<SubscriptionListResponse, String> {
    info!("APIサーバー経由でサブスクリプション一覧を取得開始");

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/api/subscriptions/list")
        .await
        .map_err(|e| {
            error!("認証エラー: {e}");
            format!("認証エラー: {e}")
        })?;

    debug!("認証成功 - ユーザーID: {}", user.id);

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // クエリパラメータを構築
    let query_param = if active_only { "?activeOnly=true" } else { "" };

    // 開発環境では認証不要のエンドポイントを使用
    let endpoint = if cfg!(debug_assertions) {
        format!("/api/v1/subscriptions/dev{query_param}")
    } else {
        format!("/api/v1/subscriptions{query_param}")
    };

    debug!("APIエンドポイント: {endpoint}");

    // APIサーバーにリクエストを送信
    let response = api_client
        .get::<SubscriptionListResponse>(&endpoint, session_token.as_deref())
        .await
        .map_err(|e| {
            error!("APIリクエストエラー: {e}");
            format!("サブスクリプション一覧の取得に失敗しました: {e}")
        })?;

    info!(
        "サブスクリプション一覧取得成功 - 件数: {}",
        response.subscriptions.len()
    );

    Ok(response)
}

/// APIサーバー経由でサブスクリプションを作成する
///
/// # 引数
/// * `dto` - サブスクリプション作成用DTO
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 作成されたサブスクリプション、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn create_subscription_via_api(
    dto: CreateSubscriptionDto,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Subscription, String> {
    info!("APIサーバー経由でサブスクリプション作成開始");

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/api/subscriptions/create")
        .await
        .map_err(|e| {
            error!("認証エラー: {e}");
            format!("認証エラー: {e}")
        })?;

    debug!("認証成功 - ユーザーID: {}", user.id);

    // バリデーション
    validate_create_subscription_dto(&dto)?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // APIサーバーにリクエストを送信
    let subscription = api_client
        .post::<CreateSubscriptionDto, Subscription>(
            "/api/v1/subscriptions",
            &dto,
            session_token.as_deref(),
        )
        .await
        .map_err(|e| {
            error!("APIリクエストエラー: {e}");
            format!("サブスクリプションの作成に失敗しました: {e}")
        })?;

    info!("サブスクリプション作成成功 - ID: {}", subscription.id);

    Ok(subscription)
}

/// APIサーバー経由でサブスクリプションを更新する
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
pub async fn update_subscription_via_api(
    id: i64,
    dto: UpdateSubscriptionDto,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Subscription, String> {
    info!("APIサーバー経由でサブスクリプション更新開始 - ID: {id}");

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/api/subscriptions/update")
        .await
        .map_err(|e| {
            error!("認証エラー: {e}");
            format!("認証エラー: {e}")
        })?;

    debug!("認証成功 - ユーザーID: {}", user.id);

    // バリデーション
    validate_update_subscription_dto(&dto)?;

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // APIサーバーにリクエストを送信
    let subscription = api_client
        .put::<UpdateSubscriptionDto, Subscription>(
            &format!("/api/v1/subscriptions/{id}"),
            &dto,
            session_token.as_deref(),
        )
        .await
        .map_err(|e| {
            error!("APIリクエストエラー: {e}");
            format!("サブスクリプションの更新に失敗しました: {e}")
        })?;

    info!("サブスクリプション更新成功 - ID: {}", subscription.id);

    Ok(subscription)
}

/// APIサーバー経由でサブスクリプションのアクティブ状態を切り替える
///
/// # 引数
/// * `id` - サブスクリプションID
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 更新されたサブスクリプション、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn toggle_subscription_status_via_api(
    id: i64,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Subscription, String> {
    info!("APIサーバー経由でサブスクリプションステータス切り替え開始 - ID: {id}");

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/api/subscriptions/toggle")
        .await
        .map_err(|e| {
            error!("認証エラー: {e}");
            format!("認証エラー: {e}")
        })?;

    debug!("認証成功 - ユーザーID: {}", user.id);

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // APIサーバーにリクエストを送信
    let subscription = api_client
        .patch::<(), Subscription>(
            &format!("/api/v1/subscriptions/{id}/toggle"),
            &(),
            session_token.as_deref(),
        )
        .await
        .map_err(|e| {
            error!("APIリクエストエラー: {e}");
            format!("サブスクリプションステータスの切り替えに失敗しました: {e}")
        })?;

    info!(
        "サブスクリプションステータス切り替え成功 - ID: {}",
        subscription.id
    );

    Ok(subscription)
}

/// APIサーバー経由でサブスクリプションを削除する
///
/// # 引数
/// * `id` - サブスクリプションID
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 削除成功時はOk(())、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_subscription_via_api(
    id: i64,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<(), String> {
    info!("APIサーバー経由でサブスクリプション削除開始 - ID: {id}");

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/api/subscriptions/delete")
        .await
        .map_err(|e| {
            error!("認証エラー: {e}");
            format!("認証エラー: {e}")
        })?;

    debug!("認証成功 - ユーザーID: {}", user.id);

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // APIサーバーにリクエストを送信
    api_client
        .delete(
            &format!("/api/v1/subscriptions/{id}"),
            session_token.as_deref(),
        )
        .await
        .map_err(|e| {
            error!("APIリクエストエラー: {e}");
            format!("サブスクリプションの削除に失敗しました: {e}")
        })?;

    info!("サブスクリプション削除成功 - ID: {id}");

    Ok(())
}

/// APIサーバー経由で月額サブスクリプション合計を取得する
///
/// # 引数
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 月額合計のレスポンス、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn fetch_monthly_subscription_total_via_api(
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<MonthlyTotalResponse, String> {
    info!("APIサーバー経由で月額サブスクリプション合計取得開始");

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/api/subscriptions/total")
        .await
        .map_err(|e| {
            error!("認証エラー: {e}");
            format!("認証エラー: {e}")
        })?;

    debug!("認証成功 - ユーザーID: {}", user.id);

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // APIサーバーにリクエストを送信
    let response = api_client
        .get::<MonthlyTotalResponse>(
            "/api/v1/subscriptions/monthly-total",
            session_token.as_deref(),
        )
        .await
        .map_err(|e| {
            error!("APIリクエストエラー: {e}");
            format!("月額合計の取得に失敗しました: {e}")
        })?;

    info!(
        "月額サブスクリプション合計取得成功 - 合計: {}",
        response.monthly_total
    );

    Ok(response)
}

/// サブスクリプション作成DTOのバリデーション
///
/// # 引数
/// * `dto` - サブスクリプション作成用DTO
///
/// # 戻り値
/// バリデーション成功時はOk(())、失敗時はエラーメッセージ
fn validate_create_subscription_dto(dto: &CreateSubscriptionDto) -> Result<(), String> {
    // バリデーション: サービス名は必須
    if dto.name.trim().is_empty() {
        return Err("サービス名を入力してください".to_string());
    }

    // バリデーション: サービス名は100文字以内
    if dto.name.len() > 100 {
        return Err("サービス名は100文字以内で入力してください".to_string());
    }

    // バリデーション: 金額は正の数値
    if dto.amount <= 0.0 {
        return Err("金額は正の数値である必要があります".to_string());
    }

    // バリデーション: 金額は10桁以内
    if dto.amount > 9999999999.0 {
        return Err("金額は10桁以内で入力してください".to_string());
    }

    // バリデーション: billing_cycleは"monthly"または"annual"のみ
    if dto.billing_cycle != "monthly" && dto.billing_cycle != "annual" {
        return Err("支払いサイクルは'monthly'または'annual'である必要があります".to_string());
    }

    // バリデーション: 日付形式の確認
    validate_date_format(&dto.start_date)?;

    Ok(())
}

/// サブスクリプション更新DTOのバリデーション
///
/// # 引数
/// * `dto` - サブスクリプション更新用DTO
///
/// # 戻り値
/// バリデーション成功時はOk(())、失敗時はエラーメッセージ
fn validate_update_subscription_dto(dto: &UpdateSubscriptionDto) -> Result<(), String> {
    // バリデーション: サービス名が指定されている場合は必須かつ100文字以内
    if let Some(ref name) = dto.name {
        if name.trim().is_empty() {
            return Err("サービス名を入力してください".to_string());
        }
        if name.len() > 100 {
            return Err("サービス名は100文字以内で入力してください".to_string());
        }
    }

    // バリデーション: 金額が指定されている場合は正の数値
    if let Some(amount) = dto.amount {
        if amount <= 0.0 {
            return Err("金額は正の数値である必要があります".to_string());
        }
        if amount > 9999999999.0 {
            return Err("金額は10桁以内で入力してください".to_string());
        }
    }

    // バリデーション: billing_cycleが指定されている場合は"monthly"または"annual"のみ
    if let Some(ref billing_cycle) = dto.billing_cycle {
        if billing_cycle != "monthly" && billing_cycle != "annual" {
            return Err("支払いサイクルは'monthly'または'annual'である必要があります".to_string());
        }
    }

    // バリデーション: 日付が指定されている場合は形式を確認
    if let Some(ref start_date) = dto.start_date {
        validate_date_format(start_date)?;
    }

    Ok(())
}

/// 日付形式のバリデーション（YYYY-MM-DD形式）
///
/// # 引数
/// * `date` - 日付文字列
///
/// # 戻り値
/// バリデーション成功時はOk(())、失敗時はエラーメッセージ
fn validate_date_format(date: &str) -> Result<(), String> {
    // YYYY-MM-DD形式の基本チェック
    if date.len() != 10 {
        return Err("日付はYYYY-MM-DD形式で入力してください".to_string());
    }

    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err("日付はYYYY-MM-DD形式で入力してください".to_string());
    }

    // 年、月、日が数値かチェック
    let year: i32 = parts[0]
        .parse()
        .map_err(|_| "年は数値で入力してください".to_string())?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| "月は数値で入力してください".to_string())?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| "日は数値で入力してください".to_string())?;

    // 基本的な範囲チェック
    if !(1900..=2100).contains(&year) {
        return Err("年は1900年から2100年の間で入力してください".to_string());
    }
    if !(1..=12).contains(&month) {
        return Err("月は1から12の間で入力してください".to_string());
    }
    if !(1..=31).contains(&day) {
        return Err("日は1から31の間で入力してください".to_string());
    }

    Ok(())
}

/// APIサーバー経由でサブスクリプション領収書をR2にアップロードする
///
/// # 引数
/// * `subscription_id` - サブスクリプションID
/// * `file_path` - アップロードするファイルのパス
/// * `sessionToken` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// アップロードされたHTTPS URL、または失敗時はエラーメッセージ
#[tauri::command]
#[allow(non_snake_case)]
pub async fn upload_subscription_receipt_via_api(
    subscription_id: i64,
    file_path: String,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<String, String> {
    info!("APIサーバー経由でサブスクリプション領収書アップロード開始 - ID: {subscription_id}");

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(
            session_token.as_deref(),
            "/api/subscriptions/receipt/upload",
        )
        .await
        .map_err(|e| {
            error!("認証エラー: {e}");
            format!("認証エラー: {e}")
        })?;

    debug!("認証成功 - ユーザーID: {}", user.id);

    // ファイルの存在確認
    if !Path::new(&file_path).exists() {
        return Err("指定されたファイルが見つかりません".to_string());
    }

    // ファイルを読み込み
    let file_data = fs::read(&file_path).map_err(|e| {
        error!("ファイル読み込みエラー: {e}");
        format!("ファイルの読み込みに失敗しました: {e}")
    })?;

    // ファイル名を取得
    let file_name = Path::new(&file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("receipt")
        .to_string();

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // アップロード用のリクエストボディを作成
    let upload_request = serde_json::json!({
        "subscriptionId": subscription_id,
        "userId": user.id,
        "fileName": file_name,
        "fileData": general_purpose::STANDARD.encode(&file_data)
    });

    // APIサーバーにアップロードリクエストを送信
    let response = api_client
        .post::<serde_json::Value, serde_json::Value>(
            "/api/v1/subscriptions/receipt/upload",
            &upload_request,
            session_token.as_deref(),
        )
        .await
        .map_err(|e| {
            error!("APIリクエストエラー: {e}");
            format!("サブスクリプション領収書のアップロードに失敗しました: {e}")
        })?;

    // レスポンスからURLを取得
    let upload_url = response
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            error!("APIレスポンスにURLが含まれていません");
            "アップロードレスポンスが不正です".to_string()
        })?
        .to_string();

    info!("サブスクリプション領収書アップロード成功 - URL: {upload_url}");

    Ok(upload_url)
}

/// APIサーバー経由でサブスクリプション領収書をR2から削除する
///
/// # 引数
/// * `subscription_id` - サブスクリプションID
/// * `sessionToken` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 削除成功時はtrue、失敗時はエラーメッセージ
#[tauri::command]
#[allow(non_snake_case)]
pub async fn delete_subscription_receipt_via_api(
    subscription_id: i64,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<bool, String> {
    info!("APIサーバー経由でサブスクリプション領収書削除開始 - ID: {subscription_id}");

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(
            session_token.as_deref(),
            "/api/subscriptions/receipt/delete",
        )
        .await
        .map_err(|e| {
            error!("認証エラー: {e}");
            format!("認証エラー: {e}")
        })?;

    debug!("認証成功 - ユーザーID: {}", user.id);

    // APIクライアントを作成
    let api_client = ApiClient::new().map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // APIサーバーに削除リクエストを送信
    api_client
        .delete(
            &format!("/api/v1/subscriptions/{subscription_id}/receipt"),
            session_token.as_deref(),
        )
        .await
        .map_err(|e| {
            error!("APIリクエストエラー: {e}");
            format!("サブスクリプション領収書の削除に失敗しました: {e}")
        })?;

    info!("サブスクリプション領収書削除成功 - ID: {subscription_id}");

    Ok(true)
}

use super::models::{CreateSubscriptionDto, Subscription, UpdateSubscriptionDto};
use super::repository;
use crate::features::auth::middleware::AuthMiddleware;
use crate::AppState;
use tauri::State;

/// サブスクリプションを作成する
///
/// # 引数
/// * `dto` - サブスクリプション作成用DTO
/// * `session_token` - セッショントークン
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 作成されたサブスクリプション、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn create_subscription(
    dto: CreateSubscriptionDto,
    session_token: Option<String>,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Subscription, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/create")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // バリデーション
    validate_create_subscription_dto(&dto)?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 認証されたユーザーのサブスクリプションを作成
    repository::create(&db, dto, &user.id).map_err(|e| e.user_message().to_string())
}

/// サブスクリプション一覧を取得する
///
/// # 引数
/// * `active_only` - アクティブなサブスクリプションのみを取得するか
/// * `session_token` - セッショントークン
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// サブスクリプションのリスト、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_subscriptions(
    active_only: bool,
    session_token: Option<String>,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Vec<Subscription>, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/list")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 認証されたユーザーのサブスクリプション一覧を取得
    repository::find_all(&db, &user.id, active_only).map_err(|e| e.user_message().to_string())
}

/// サブスクリプションを更新する
///
/// # 引数
/// * `id` - サブスクリプションID
/// * `dto` - サブスクリプション更新用DTO
/// * `session_token` - セッショントークン
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 更新されたサブスクリプション、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn update_subscription(
    id: i64,
    dto: UpdateSubscriptionDto,
    session_token: Option<String>,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Subscription, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/update")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // バリデーション
    validate_update_subscription_dto(&dto)?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 認証されたユーザーのサブスクリプションを更新
    repository::update(&db, id, dto, &user.id).map_err(|e| e.user_message().to_string())
}

/// サブスクリプションのアクティブ状態を切り替える
///
/// # 引数
/// * `id` - サブスクリプションID
/// * `session_token` - セッショントークン
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 更新されたサブスクリプション、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn toggle_subscription_status(
    id: i64,
    session_token: Option<String>,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Subscription, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/toggle")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 認証されたユーザーのサブスクリプションのアクティブ状態を切り替え
    repository::toggle_status(&db, id, &user.id).map_err(|e| e.user_message().to_string())
}

/// アクティブなサブスクリプションの月額合計を取得する
///
/// # 引数
/// * `session_token` - セッショントークン
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 月額合計金額、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_monthly_subscription_total(
    session_token: Option<String>,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<f64, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/total")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 認証されたユーザーの月額合計を計算
    repository::calculate_monthly_total(&db, &user.id).map_err(|e| e.user_message().to_string())
}

/// サブスクリプション作成DTOのバリデーション
///
/// # 引数
/// * `dto` - サブスクリプション作成用DTO
///
/// # 戻り値
/// バリデーション成功時はOk(())、失敗時はエラーメッセージ
fn validate_create_subscription_dto(dto: &CreateSubscriptionDto) -> Result<(), String> {
    // デバッグログを追加
    log::info!("バリデーション開始 - 受信した日付: {}", dto.start_date);

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
    // 空文字チェック
    if date.is_empty() {
        return Err("日付が空です".to_string());
    }

    // YYYY-MM-DD形式の基本チェック
    if date.len() != 10 {
        return Err(format!(
            "日付はYYYY-MM-DD形式で入力してください（受信: '{}', 長さ: {}）",
            date,
            date.len()
        ));
    }

    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err(format!(
            "日付はYYYY-MM-DD形式で入力してください（受信: '{}', 分割数: {}）",
            date,
            parts.len()
        ));
    }

    // 年、月、日が数値かチェック
    let year: i32 = parts[0]
        .parse()
        .map_err(|_| format!("年は数値で入力してください（受信: '{}'）", parts[0]))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| format!("月は数値で入力してください（受信: '{}'）", parts[1]))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| format!("日は数値で入力してください（受信: '{}'）", parts[2]))?;

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

/// サブスクリプションを削除する
///
/// # 引数
/// * `id` - サブスクリプションID
/// * `session_token` - セッショントークン
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_subscription(
    id: i64,
    session_token: Option<String>,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<(), String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/delete")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 認証されたユーザーのサブスクリプションを削除
    repository::delete(&db, id, &user.id).map_err(|e| e.user_message().to_string())
}
/// サブスクリプションの領収書ファイルを保存する
///
/// # 引数
/// * `subscription_id` - サブスクリプションID
/// * `file_path` - 保存するファイルのパス
/// * `session_token` - セッショントークン
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 保存されたファイルパス、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn save_subscription_receipt(
    subscription_id: i64,
    file_path: String,
    session_token: Option<String>,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<String, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/receipt/save")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 領収書パスを設定
    repository::set_receipt_path(&db, subscription_id, file_path.clone(), &user.id)
        .map_err(|e| e.user_message().to_string())?;

    Ok(file_path)
}

/// サブスクリプションの領収書を削除する
///
/// # 引数
/// * `subscription_id` - サブスクリプションID
/// * `session_token` - セッショントークン
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 成功時はtrue、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_subscription_receipt(
    subscription_id: i64,
    session_token: Option<String>,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<bool, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/receipt/delete")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 領収書パスを削除（空文字列を設定）
    repository::set_receipt_path(&db, subscription_id, String::new(), &user.id)
        .map_err(|e| e.user_message().to_string())?;

    Ok(true)
}

/// サブスクリプションの領収書パスを取得する
///
/// # 引数
/// * `subscription_id` - サブスクリプションID
/// * `session_token` - セッショントークン
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 領収書パス（存在する場合）、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_subscription_receipt_path(
    subscription_id: i64,
    session_token: Option<String>,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Option<String>, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/subscriptions/receipt/get")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 領収書パスを取得
    repository::get_receipt_path(&db, subscription_id, &user.id)
        .map_err(|e| e.user_message().to_string())
}

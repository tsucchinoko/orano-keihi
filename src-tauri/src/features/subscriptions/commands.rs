use super::models::{CreateSubscriptionDto, Subscription, UpdateSubscriptionDto};
use super::repository;
use crate::AppState;
use tauri::State;

/// サブスクリプションを作成する
///
/// # 引数
/// * `dto` - サブスクリプション作成用DTO
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 作成されたサブスクリプション、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn create_subscription(
    dto: CreateSubscriptionDto,
    state: State<'_, AppState>,
) -> Result<Subscription, String> {
    // バリデーション
    validate_create_subscription_dto(&dto)?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // サブスクリプションを作成
    repository::create(&db, dto, 1i64).map_err(|e| e.user_message().to_string())
}

/// サブスクリプション一覧を取得する
///
/// # 引数
/// * `active_only` - アクティブなサブスクリプションのみを取得するか
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// サブスクリプションのリスト、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_subscriptions(
    active_only: bool,
    state: State<'_, AppState>,
) -> Result<Vec<Subscription>, String> {
    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // サブスクリプション一覧を取得
    repository::find_all(&db, 1i64, active_only).map_err(|e| e.user_message().to_string())
}

/// サブスクリプションを更新する
///
/// # 引数
/// * `id` - サブスクリプションID
/// * `dto` - サブスクリプション更新用DTO
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 更新されたサブスクリプション、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn update_subscription(
    id: i64,
    dto: UpdateSubscriptionDto,
    state: State<'_, AppState>,
) -> Result<Subscription, String> {
    // バリデーション
    validate_update_subscription_dto(&dto)?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // サブスクリプションを更新
    repository::update(&db, id, dto, 1i64).map_err(|e| e.user_message().to_string())
}

/// サブスクリプションのアクティブ状態を切り替える
///
/// # 引数
/// * `id` - サブスクリプションID
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 更新されたサブスクリプション、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn toggle_subscription_status(
    id: i64,
    state: State<'_, AppState>,
) -> Result<Subscription, String> {
    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // サブスクリプションのアクティブ状態を切り替え
    repository::toggle_status(&db, id, 1i64).map_err(|e| e.user_message().to_string())
}

/// アクティブなサブスクリプションの月額合計を取得する
///
/// # 引数
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 月額合計金額、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_monthly_subscription_total(state: State<'_, AppState>) -> Result<f64, String> {
    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 月額合計を計算
    repository::calculate_monthly_total(&db, 1i64).map_err(|e| e.user_message().to_string())
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

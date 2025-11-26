use crate::db::subscription_operations;
use crate::models::{CreateSubscriptionDto, Subscription, UpdateSubscriptionDto};
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
    // バリデーション: 金額は正の数値
    if dto.amount <= 0.0 {
        return Err("金額は正の数値である必要があります".to_string());
    }

    // バリデーション: billing_cycleは"monthly"または"annual"のみ
    if dto.billing_cycle != "monthly" && dto.billing_cycle != "annual" {
        return Err("支払いサイクルは'monthly'または'annual'である必要があります".to_string());
    }

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {}", e))?;

    // サブスクリプションを作成
    subscription_operations::create_subscription(&db, dto)
        .map_err(|e| format!("サブスクリプションの作成に失敗しました: {}", e))
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
        .map_err(|e| format!("データベースロックエラー: {}", e))?;

    // サブスクリプション一覧を取得
    subscription_operations::get_subscriptions(&db, active_only)
        .map_err(|e| format!("サブスクリプションの取得に失敗しました: {}", e))
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
    // バリデーション: 金額が指定されている場合は正の数値
    if let Some(amount) = dto.amount {
        if amount <= 0.0 {
            return Err("金額は正の数値である必要があります".to_string());
        }
    }

    // バリデーション: billing_cycleが指定されている場合は"monthly"または"annual"のみ
    if let Some(ref billing_cycle) = dto.billing_cycle {
        if billing_cycle != "monthly" && billing_cycle != "annual" {
            return Err(
                "支払いサイクルは'monthly'または'annual'である必要があります".to_string()
            );
        }
    }

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {}", e))?;

    // サブスクリプションを更新
    subscription_operations::update_subscription(&db, id, dto)
        .map_err(|e| format!("サブスクリプションの更新に失敗しました: {}", e))
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
        .map_err(|e| format!("データベースロックエラー: {}", e))?;

    // サブスクリプションのアクティブ状態を切り替え
    subscription_operations::toggle_subscription_status(&db, id)
        .map_err(|e| format!("サブスクリプションの状態切り替えに失敗しました: {}", e))
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
        .map_err(|e| format!("データベースロックエラー: {}", e))?;

    // 月額合計を計算
    subscription_operations::get_monthly_subscription_total(&db)
        .map_err(|e| format!("月額合計の計算に失敗しました: {}", e))
}

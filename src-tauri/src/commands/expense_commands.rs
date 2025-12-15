use crate::db::expense_operations;
use crate::models::{CreateExpenseDto, Expense, UpdateExpenseDto};
use crate::AppState;
use chrono::NaiveDate;
use chrono_tz::Asia::Tokyo;
use tauri::{Manager, State};

/// 経費を作成する
///
/// # 引数
/// * `dto` - 経費作成用DTO
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 作成された経費、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn create_expense(
    dto: CreateExpenseDto,
    state: State<'_, AppState>,
) -> Result<Expense, String> {
    // バリデーション: 金額は正の数値
    if dto.amount <= 0.0 {
        return Err("金額は正の数値である必要があります".to_string());
    }

    // バリデーション: 金額は10桁以内
    if dto.amount > 9999999999.0 {
        return Err("金額は10桁以内で入力してください".to_string());
    }

    // バリデーション: 日付が未来でないことを確認
    let expense_date = NaiveDate::parse_from_str(&dto.date, "%Y-%m-%d").map_err(|_| {
        "日付の形式が正しくありません（YYYY-MM-DD形式で入力してください）".to_string()
    })?;

    // JSTで今日の日付を取得
    let now_jst = chrono::Utc::now().with_timezone(&Tokyo);
    let today = now_jst.date_naive();
    if expense_date > today {
        return Err("未来の日付は指定できません".to_string());
    }

    // バリデーション: 説明は500文字以内
    if let Some(ref description) = dto.description {
        if description.len() > 500 {
            return Err("説明は500文字以内で入力してください".to_string());
        }
    }

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 経費を作成
    expense_operations::create_expense(&db, dto)
        .map_err(|e| format!("経費の作成に失敗しました: {e}"))
}

/// 経費一覧を取得する（月とカテゴリでフィルタリング可能）
///
/// # 引数
/// * `month` - 月フィルター（YYYY-MM形式、オプション）
/// * `category` - カテゴリフィルター（オプション）
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 経費のリスト、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_expenses(
    month: Option<String>,
    category: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Expense>, String> {
    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 経費一覧を取得
    expense_operations::get_expenses(&db, month, category)
        .map_err(|e| format!("経費の取得に失敗しました: {e}"))
}

/// 経費を更新する
///
/// # 引数
/// * `id` - 経費ID
/// * `dto` - 経費更新用DTO
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 更新された経費、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn update_expense(
    id: i64,
    dto: UpdateExpenseDto,
    state: State<'_, AppState>,
) -> Result<Expense, String> {
    // バリデーション: 金額が指定されている場合は正の数値
    if let Some(amount) = dto.amount {
        if amount <= 0.0 {
            return Err("金額は正の数値である必要があります".to_string());
        }
        if amount > 9999999999.0 {
            return Err("金額は10桁以内で入力してください".to_string());
        }
    }

    // バリデーション: 日付が指定されている場合は未来でないことを確認
    if let Some(ref date) = dto.date {
        let expense_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
            "日付の形式が正しくありません（YYYY-MM-DD形式で入力してください）".to_string()
        })?;

        // JSTで今日の日付を取得
        let now_jst = chrono::Utc::now().with_timezone(&Tokyo);
        let today = now_jst.date_naive();
        if expense_date > today {
            return Err("未来の日付は指定できません".to_string());
        }
    }

    // バリデーション: 説明が指定されている場合は500文字以内
    if let Some(ref description) = dto.description {
        if description.len() > 500 {
            return Err("説明は500文字以内で入力してください".to_string());
        }
    }

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 経費を更新
    expense_operations::update_expense(&db, id, dto)
        .map_err(|e| format!("経費の更新に失敗しました: {e}"))
}

/// 経費を削除する（R2対応）
///
/// # 引数
/// * `id` - 経費ID
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 成功時は空、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_expense(
    id: i64,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use chrono::Utc;
    use chrono_tz::Asia::Tokyo;

    // 現在のreceipt_urlを取得
    let current_receipt_url = {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {e}"))?;

        expense_operations::get_receipt_url(&db, id)
            .map_err(|e| format!("receipt_urlの取得に失敗しました: {e}"))?
    };

    // 領収書がR2に存在する場合は削除
    if let Some(receipt_url) = current_receipt_url {
        // R2からファイルを削除（トランザクション的な削除処理：R2→DB順）
        let deletion_result =
            crate::commands::receipt_commands::delete_from_r2_with_retry(&receipt_url).await;

        match deletion_result {
            Ok(_) => {
                // R2削除成功 - キャッシュからも削除
                if let Ok(app_data_dir) = app.path().app_data_dir() {
                    let cache_dir = app_data_dir.join("receipt_cache");
                    let cache_manager =
                        crate::services::cache_manager::CacheManager::new(cache_dir, 100);

                    let db = state
                        .db
                        .lock()
                        .map_err(|e| format!("データベースロックエラー: {e}"))?;

                    if let Err(e) = cache_manager.delete_cache_file(&receipt_url, &db) {
                        eprintln!("キャッシュ削除エラー: {}", e);
                    }
                }

                // 削除操作のログ記録
                let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();
                println!(
                    "経費削除時の領収書削除完了: expense_id={}, receipt_url={}, timestamp={}",
                    id, receipt_url, now
                );
            }
            Err(e) => {
                // R2削除失敗 - データベースの状態は変更しない
                return Err(format!(
                    "R2からのファイル削除に失敗しました。経費の削除を中止します: {}",
                    e
                ));
            }
        }
    }

    // データベースから経費を削除
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    expense_operations::delete_expense(&db, id)
        .map_err(|e| format!("経費の削除に失敗しました: {e}"))
}

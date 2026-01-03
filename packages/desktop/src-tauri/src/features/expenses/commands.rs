use crate::features::auth::middleware::AuthMiddleware;
use crate::features::expenses::{models::*, repository};
use crate::shared::errors::AppError;
use crate::AppState;
use chrono::NaiveDate;
use chrono_tz::Asia::Tokyo;
use tauri::{Manager, State};

/// 経費を作成する
///
/// # 引数
/// * `dto` - 経費作成用DTO
/// * `session_token` - セッショントークン
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 作成された経費、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn create_expense(
    dto: CreateExpenseDto,
    session_token: Option<String>,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Expense, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/expenses/create")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // バリデーション
    validate_expense_dto(&dto)?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::concurrency(format!("データベースロック取得失敗: {e}")))?;

    // 認証されたユーザーIDを使用して経費DTOを作成
    let mut user_dto = dto;
    user_dto.user_id = Some(user.id);

    // 経費を作成
    repository::create(&db, user_dto, user.id).map_err(|e| e.into())
}

/// 経費一覧を取得する（月とカテゴリでフィルタリング可能）
///
/// # 引数
/// * `month` - 月フィルター（YYYY-MM形式、オプション）
/// * `category` - カテゴリフィルター（オプション）
/// * `session_token` - セッショントークン
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 経費のリスト、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_expenses(
    month: Option<String>,
    category: Option<String>,
    session_token: Option<String>,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Vec<Expense>, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/expenses/list")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::concurrency(format!("データベースロック取得失敗: {e}")))?;

    // 認証されたユーザーの経費一覧を取得
    repository::find_all(&db, user.id, month.as_deref(), category.as_deref()).map_err(|e| e.into())
}

/// 経費を更新する
///
/// # 引数
/// * `id` - 経費ID
/// * `dto` - 経費更新用DTO
/// * `session_token` - セッショントークン
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 更新された経費、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn update_expense(
    id: i64,
    dto: UpdateExpenseDto,
    session_token: Option<String>,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<Expense, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/expenses/update")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // バリデーション
    validate_update_expense_dto(&dto)?;

    // データベース接続を取得
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::concurrency(format!("データベースロック取得失敗: {e}")))?;

    // 認証されたユーザーの経費を更新
    repository::update(&db, id, dto, user.id).map_err(|e| e.into())
}

/// 経費を削除する（R2対応）
///
/// # 引数
/// * `id` - 経費ID
/// * `session_token` - セッショントークン
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
/// * `auth_service` - 認証サービス
///
/// # 戻り値
/// 成功時は空、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_expense(
    id: i64,
    session_token: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    auth_middleware: State<'_, AuthMiddleware>,
    auth_service: State<'_, crate::features::auth::service::AuthService>,
) -> Result<(), String> {
    use chrono::Utc;
    use chrono_tz::Asia::Tokyo;

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/expenses/delete")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    // 現在のreceipt_urlを取得
    let current_receipt_url = {
        let db = state
            .db
            .lock()
            .map_err(|e| AppError::concurrency(format!("データベースロック取得失敗: {e}")))?;

        repository::get_receipt_url(&db, id, user.id).map_err(|e| e.user_message().to_string())?
    };

    // 領収書がR2に存在する場合は削除
    if let Some(receipt_url) = current_receipt_url {
        // 認証付きR2削除コマンドを使用（経費IDではなくreceipt_urlを渡す）
        let deletion_result = crate::features::receipts::auth_commands::delete_receipt_with_auth(
            session_token.unwrap_or_default(),
            receipt_url.clone(),
            auth_service,
            state.clone(),
        )
        .await;

        match deletion_result {
            Ok(_) => {
                // R2削除成功 - キャッシュからも削除
                if let Ok(app_data_dir) = app.path().app_data_dir() {
                    let cache_dir = app_data_dir.join("receipt_cache");
                    let cache_manager =
                        crate::features::receipts::CacheManager::new(cache_dir, 100);

                    let db = state.db.lock().map_err(|e| {
                        AppError::concurrency(format!("データベースロック取得失敗: {e}"))
                    })?;

                    if let Err(e) = cache_manager.delete_cache_file(&receipt_url, &db, 1i64) {
                        eprintln!("キャッシュ削除エラー: {e}");
                    }
                }

                // 削除操作のログ記録
                let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();
                println!(
                    "経費削除時の領収書削除完了: expense_id={id}, receipt_url={receipt_url}, timestamp={now}"
                );
            }
            Err(e) => {
                // R2削除失敗 - データベースの状態は変更しない
                return Err(format!(
                    "R2からのファイル削除に失敗しました。経費の削除を中止します: {e}"
                ));
            }
        }
    }

    // データベースから経費を削除
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::concurrency(format!("データベースロック取得失敗: {e}")))?;

    repository::delete(&db, id, user.id).map_err(|e| e.into())
}

/// 経費作成DTOのバリデーション
///
/// # 引数
/// * `dto` - 経費作成用DTO
///
/// # 戻り値
/// バリデーション成功時はOk(())、失敗時はエラー
fn validate_expense_dto(dto: &CreateExpenseDto) -> Result<(), String> {
    // バリデーション: 金額は正の数値
    if dto.amount <= 0.0 {
        return Err(AppError::validation("金額は正の数値である必要があります").into());
    }

    // バリデーション: 金額は10桁以内
    if dto.amount > 9999999999.0 {
        return Err(AppError::validation("金額は10桁以内で入力してください").into());
    }

    // バリデーション: 日付が未来でないことを確認
    let expense_date = NaiveDate::parse_from_str(&dto.date, "%Y-%m-%d").map_err(|_| {
        AppError::validation("日付の形式が正しくありません（YYYY-MM-DD形式で入力してください）")
    })?;

    // JSTで今日の日付を取得
    let now_jst = chrono::Utc::now().with_timezone(&Tokyo);
    let today = now_jst.date_naive();
    if expense_date > today {
        return Err(AppError::validation("未来の日付は指定できません").into());
    }

    // バリデーション: 説明は500文字以内
    if let Some(ref description) = dto.description {
        if description.len() > 500 {
            return Err(AppError::validation("説明は500文字以内で入力してください").into());
        }
    }

    Ok(())
}

/// 経費更新DTOのバリデーション
///
/// # 引数
/// * `dto` - 経費更新用DTO
///
/// # 戻り値
/// バリデーション成功時はOk(())、失敗時はエラー
fn validate_update_expense_dto(dto: &UpdateExpenseDto) -> Result<(), String> {
    // バリデーション: 金額が指定されている場合は正の数値
    if let Some(amount) = dto.amount {
        if amount <= 0.0 {
            return Err(AppError::validation("金額は正の数値である必要があります").into());
        }
        if amount > 9999999999.0 {
            return Err(AppError::validation("金額は10桁以内で入力してください").into());
        }
    }

    // バリデーション: 日付が指定されている場合は未来でないことを確認
    if let Some(ref date) = dto.date {
        let expense_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
            AppError::validation("日付の形式が正しくありません（YYYY-MM-DD形式で入力してください）")
        })?;

        // JSTで今日の日付を取得
        let now_jst = chrono::Utc::now().with_timezone(&Tokyo);
        let today = now_jst.date_naive();
        if expense_date > today {
            return Err(AppError::validation("未来の日付は指定できません").into());
        }
    }

    // バリデーション: 説明が指定されている場合は500文字以内
    if let Some(ref description) = dto.description {
        if description.len() > 500 {
            return Err(AppError::validation("説明は500文字以内で入力してください").into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_expense_dto_valid() {
        // 有効な経費作成DTOのテスト
        let dto = CreateExpenseDto {
            date: "2024-01-01".to_string(),
            amount: 1000.0,
            category: "食費".to_string(),
            description: Some("テスト経費".to_string()),
            user_id: None, // 認証後に設定される
        };

        assert!(validate_expense_dto(&dto).is_ok());
    }

    #[test]
    fn test_validate_expense_dto_invalid_amount() {
        // 無効な金額（負の値）のテスト
        let dto = CreateExpenseDto {
            date: "2024-01-01".to_string(),
            amount: -100.0,
            category: "食費".to_string(),
            description: None,
            user_id: None,
        };

        let result = validate_expense_dto(&dto);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("正の数値"));
    }

    #[test]
    fn test_validate_expense_dto_amount_too_large() {
        // 無効な金額（10桁超過）のテスト
        let dto = CreateExpenseDto {
            date: "2024-01-01".to_string(),
            amount: 99999999999.0, // 11桁
            category: "食費".to_string(),
            description: None,
            user_id: None,
        };

        let result = validate_expense_dto(&dto);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("10桁以内"));
    }

    #[test]
    fn test_validate_expense_dto_invalid_date_format() {
        // 無効な日付形式のテスト
        let dto = CreateExpenseDto {
            date: "2024/01/01".to_string(), // スラッシュ区切り
            amount: 1000.0,
            category: "食費".to_string(),
            description: None,
            user_id: None,
        };

        let result = validate_expense_dto(&dto);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("日付の形式"));
    }

    #[test]
    fn test_validate_expense_dto_description_too_long() {
        // 説明が長すぎる場合のテスト
        let long_description = "a".repeat(501); // 501文字
        let dto = CreateExpenseDto {
            date: "2024-01-01".to_string(),
            amount: 1000.0,
            category: "食費".to_string(),
            description: Some(long_description),
            user_id: None,
        };

        let result = validate_expense_dto(&dto);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("500文字以内"));
    }

    #[test]
    fn test_validate_update_expense_dto_valid() {
        // 有効な経費更新DTOのテスト
        let dto = UpdateExpenseDto {
            date: Some("2024-01-01".to_string()),
            amount: Some(1500.0),
            category: Some("交通費".to_string()),
            description: Some("更新されたテスト経費".to_string()),
        };

        assert!(validate_update_expense_dto(&dto).is_ok());
    }

    #[test]
    fn test_validate_update_expense_dto_partial() {
        // 部分更新DTOのテスト
        let dto = UpdateExpenseDto {
            date: None,
            amount: Some(2000.0),
            category: None,
            description: None,
        };

        assert!(validate_update_expense_dto(&dto).is_ok());
    }

    #[test]
    fn test_validate_update_expense_dto_invalid_amount() {
        // 無効な金額の更新DTOテスト
        let dto = UpdateExpenseDto {
            date: None,
            amount: Some(-500.0),
            category: None,
            description: None,
        };

        let result = validate_update_expense_dto(&dto);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("正の数値"));
    }
}

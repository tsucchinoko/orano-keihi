use crate::db::{
    connection::initialize_database,
    migrations::{
        is_receipt_url_migration_complete, migrate_receipt_path_to_url, restore_from_backup,
        MigrationResult,
    },
};
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use tauri::{AppHandle, Manager};

/// マイグレーション状態を確認する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// マイグレーション完了状態
#[tauri::command]
pub async fn check_migration_status(app_handle: AppHandle) -> Result<bool, String> {
    let conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {}", e))?;

    is_receipt_url_migration_complete(&conn)
        .map_err(|e| format!("マイグレーション状態確認エラー: {}", e))
}

/// receipt_pathからreceipt_urlへのマイグレーションを実行する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// マイグレーション結果
#[tauri::command]
pub async fn execute_receipt_url_migration(
    app_handle: AppHandle,
) -> Result<MigrationResult, String> {
    let conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {}", e))?;

    // バックアップファイルパスを生成（JST使用）
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("アプリデータディレクトリ取得エラー: {}", e))?;

    let now_jst = Utc::now().with_timezone(&Tokyo);
    let backup_path = app_data_dir.join(format!("database_backup_{}.db", now_jst.timestamp()));

    migrate_receipt_path_to_url(&conn, backup_path.to_str().unwrap())
        .map_err(|e| format!("マイグレーション実行エラー: {}", e))
}

/// バックアップからデータベースを復元する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
/// * `backup_path` - バックアップファイルのパス
///
/// # 戻り値
/// 復元結果
#[tauri::command]
pub async fn restore_database_from_backup(
    app_handle: AppHandle,
    backup_path: String,
) -> Result<String, String> {
    let mut conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {}", e))?;

    restore_from_backup(&mut conn, &backup_path)
        .map_err(|e| format!("データベース復元エラー: {}", e))?;

    Ok("データベースの復元が完了しました".to_string())
}

/// 利用可能なバックアップファイル一覧を取得する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// バックアップファイルのパス一覧
#[tauri::command]
pub async fn list_backup_files(app_handle: AppHandle) -> Result<Vec<String>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("アプリデータディレクトリ取得エラー: {}", e))?;

    let mut backup_files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&app_data_dir) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.starts_with("database_backup_") && file_name.ends_with(".db") {
                    if let Some(path_str) = entry.path().to_str() {
                        backup_files.push(path_str.to_string());
                    }
                }
            }
        }
    }

    // 作成日時順でソート（新しい順）
    backup_files.sort_by(|a, b| b.cmp(a));

    Ok(backup_files)
}

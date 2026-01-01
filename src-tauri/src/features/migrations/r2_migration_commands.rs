//! R2移行用Tauriコマンド
//!
//! このモジュールは、R2ユーザーディレクトリ移行機能の
//! Tauriコマンドを提供します。

use super::r2_user_directory_migration::{
    create_migration_log_entry, get_migration_progress, update_migration_log_status,
    MigrationProgress,
};
use crate::shared::database::connection::get_database_path;
use crate::shared::errors::{AppError, AppResult};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

/// R2移行開始パラメータ
#[derive(Debug, Serialize, Deserialize)]
pub struct StartR2MigrationParams {
    /// ドライランモード
    pub dry_run: bool,
    /// バッチサイズ
    pub batch_size: Option<usize>,
    /// 作成者
    pub created_by: Option<String>,
}

/// R2移行結果
#[derive(Debug, Serialize, Deserialize)]
pub struct R2MigrationResult {
    /// 成功フラグ
    pub success: bool,
    /// メッセージ
    pub message: String,
    /// 移行ログID
    pub migration_log_id: Option<i64>,
    /// 総アイテム数
    pub total_items: usize,
    /// 成功数
    pub success_count: usize,
    /// エラー数
    pub error_count: usize,
    /// 実行時間（ミリ秒）
    pub duration_ms: u64,
}

/// R2移行ステータス
#[derive(Debug, Serialize, Deserialize)]
pub struct R2MigrationStatus {
    /// 移行が実行中かどうか
    pub is_running: bool,
    /// 現在の移行ログID
    pub current_migration_id: Option<i64>,
    /// 進捗情報
    pub progress: Option<MigrationProgress>,
}

/// R2移行を開始する
///
/// # 引数
/// * `params` - 移行パラメータ
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// 移行結果
#[tauri::command]
pub async fn start_r2_migration(
    params: StartR2MigrationParams,
    app_handle: AppHandle,
) -> Result<R2MigrationResult, String> {
    log::info!(
        "R2移行コマンドを開始します: dry_run={}, batch_size={:?}",
        params.dry_run,
        params.batch_size
    );

    let start_time = std::time::Instant::now();

    // データベース接続を取得
    let db_path =
        get_database_path(&app_handle).map_err(|e| format!("データベースパス取得エラー: {e}"))?;

    let conn = Connection::open(&db_path).map_err(|e| format!("データベース接続エラー: {e}"))?;

    // 移行ログエントリを作成
    let metadata = serde_json::json!({
        "dry_run": params.dry_run,
        "batch_size": params.batch_size.unwrap_or(50),
        "started_by": "tauri_command"
    });

    let migration_log_id = create_migration_log_entry(
        &conn,
        "r2_user_directory",
        0, // 総アイテム数は後で更新
        &params.created_by.unwrap_or_else(|| "system".to_string()),
        Some(&metadata.to_string()),
    )
    .map_err(|e| format!("移行ログ作成エラー: {e}"))?;

    if params.dry_run {
        // ドライランモード: 実際の移行は行わず、移行対象を特定するだけ
        log::info!("ドライランモードで移行対象を特定します");

        // TODO: 実際の移行対象ファイル数を計算
        let estimated_items = estimate_migration_items(&conn)
            .await
            .map_err(|e| format!("移行対象推定エラー: {e}"))?;

        // ログを更新
        update_migration_log_status(
            &conn,
            migration_log_id,
            "completed",
            0,
            0,
            0,
            Some(
                &serde_json::json!({
                    "dry_run": true,
                    "estimated_items": estimated_items
                })
                .to_string(),
            ),
        )
        .map_err(|e| format!("移行ログ更新エラー: {e}"))?;

        let duration = start_time.elapsed();

        Ok(R2MigrationResult {
            success: true,
            message: format!("ドライラン完了: 推定移行対象ファイル数 {estimated_items}"),
            migration_log_id: Some(migration_log_id),
            total_items: estimated_items,
            success_count: 0,
            error_count: 0,
            duration_ms: duration.as_millis() as u64,
        })
    } else {
        // 実際の移行実行
        log::info!("実際のR2移行を開始します");

        // TODO: 実際の移行処理を実装
        // 現在はプレースホルダー実装
        let result = execute_r2_migration(&conn, migration_log_id, params.batch_size.unwrap_or(50))
            .await
            .map_err(|e| format!("R2移行実行エラー: {e}"))?;

        let duration = start_time.elapsed();

        Ok(R2MigrationResult {
            success: result.success,
            message: result.message,
            migration_log_id: Some(migration_log_id),
            total_items: result.total_items,
            success_count: result.success_count,
            error_count: result.error_count,
            duration_ms: duration.as_millis() as u64,
        })
    }
}

/// R2移行のステータスを取得する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// 移行ステータス
#[tauri::command]
pub async fn get_r2_migration_status(app_handle: AppHandle) -> Result<R2MigrationStatus, String> {
    log::debug!("R2移行ステータスを取得します");

    // データベース接続を取得
    let db_path =
        get_database_path(&app_handle).map_err(|e| format!("データベースパス取得エラー: {e}"))?;

    let conn = Connection::open(&db_path).map_err(|e| format!("データベース接続エラー: {e}"))?;

    // 最新の移行ログを取得
    let current_migration =
        get_current_migration_log(&conn).map_err(|e| format!("移行ログ取得エラー: {e}"))?;

    if let Some((migration_id, status)) = current_migration {
        let is_running = matches!(status.as_str(), "started" | "in_progress" | "paused");

        let progress = if is_running {
            Some(
                get_migration_progress(&conn, migration_id)
                    .map_err(|e| format!("移行進捗取得エラー: {e}"))?,
            )
        } else {
            None
        };

        Ok(R2MigrationStatus {
            is_running,
            current_migration_id: Some(migration_id),
            progress,
        })
    } else {
        Ok(R2MigrationStatus {
            is_running: false,
            current_migration_id: None,
            progress: None,
        })
    }
}

/// R2移行を一時停止する
///
/// # 引数
/// * `migration_id` - 移行ログID
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラーメッセージ
#[tauri::command]
pub async fn pause_r2_migration(migration_id: i64, app_handle: AppHandle) -> Result<(), String> {
    log::info!("R2移行を一時停止します: migration_id={migration_id}");

    let db_path =
        get_database_path(&app_handle).map_err(|e| format!("データベースパス取得エラー: {e}"))?;

    let conn = Connection::open(&db_path).map_err(|e| format!("データベース接続エラー: {e}"))?;

    // TODO: 実際の一時停止処理を実装
    update_migration_log_status(
        &conn,
        migration_id,
        "paused",
        0, // 現在の処理済み数を取得して設定
        0, // 現在の成功数を取得して設定
        0, // 現在のエラー数を取得して設定
        None,
    )
    .map_err(|e| format!("移行ログ更新エラー: {e}"))?;

    log::info!("R2移行を一時停止しました");
    Ok(())
}

/// R2移行を再開する
///
/// # 引数
/// * `migration_id` - 移行ログID
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラーメッセージ
#[tauri::command]
pub async fn resume_r2_migration(migration_id: i64, app_handle: AppHandle) -> Result<(), String> {
    log::info!("R2移行を再開します: migration_id={migration_id}");

    let db_path =
        get_database_path(&app_handle).map_err(|e| format!("データベースパス取得エラー: {e}"))?;

    let conn = Connection::open(&db_path).map_err(|e| format!("データベース接続エラー: {e}"))?;

    // TODO: 実際の再開処理を実装
    update_migration_log_status(
        &conn,
        migration_id,
        "in_progress",
        0, // 現在の処理済み数を取得して設定
        0, // 現在の成功数を取得して設定
        0, // 現在のエラー数を取得して設定
        None,
    )
    .map_err(|e| format!("移行ログ更新エラー: {e}"))?;

    log::info!("R2移行を再開しました");
    Ok(())
}

/// R2移行を停止する
///
/// # 引数
/// * `migration_id` - 移行ログID
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラーメッセージ
#[tauri::command]
pub async fn stop_r2_migration(migration_id: i64, app_handle: AppHandle) -> Result<(), String> {
    log::info!("R2移行を停止します: migration_id={migration_id}");

    let db_path =
        get_database_path(&app_handle).map_err(|e| format!("データベースパス取得エラー: {e}"))?;

    let conn = Connection::open(&db_path).map_err(|e| format!("データベース接続エラー: {e}"))?;

    // TODO: 実際の停止処理を実装
    update_migration_log_status(
        &conn,
        migration_id,
        "failed",
        0, // 現在の処理済み数を取得して設定
        0, // 現在の成功数を取得して設定
        0, // 現在のエラー数を取得して設定
        Some(
            &serde_json::json!({
                "reason": "user_stopped",
                "message": "ユーザーによって停止されました"
            })
            .to_string(),
        ),
    )
    .map_err(|e| format!("移行ログ更新エラー: {e}"))?;

    log::info!("R2移行を停止しました");
    Ok(())
}

/// R2移行の整合性を検証する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// 検証結果
#[tauri::command]
pub async fn validate_r2_migration_integrity(
    app_handle: AppHandle,
) -> Result<ValidationResult, String> {
    log::info!("R2移行の整合性検証を開始します");

    let db_path =
        get_database_path(&app_handle).map_err(|e| format!("データベースパス取得エラー: {e}"))?;

    let conn = Connection::open(&db_path).map_err(|e| format!("データベース接続エラー: {e}"))?;

    // TODO: 実際の整合性検証を実装
    let result =
        perform_integrity_validation(&conn).map_err(|e| format!("整合性検証エラー: {e}"))?;

    log::info!("R2移行の整合性検証が完了しました");
    Ok(result)
}

/// 検証結果
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    /// 検証成功フラグ
    pub success: bool,
    /// メッセージ
    pub message: String,
    /// データベースレシート数
    pub database_receipt_count: i64,
    /// R2ファイル数
    pub r2_file_count: i64,
    /// 孤立ファイル数
    pub orphaned_files: usize,
    /// 破損ファイル数
    pub corrupted_files: usize,
    /// 警告一覧
    pub warnings: Vec<String>,
    /// エラー一覧
    pub errors: Vec<String>,
}

// ヘルパー関数

/// 移行対象アイテム数を推定する
async fn estimate_migration_items(conn: &Connection) -> AppResult<usize> {
    // TODO: 実際のR2ファイル数を取得する実装
    // 現在はプレースホルダー
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM expenses WHERE receipt_url IS NOT NULL AND receipt_url LIKE '%/receipts/%'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(format!("移行対象推定エラー: {e}")))?;

    Ok(count as usize)
}

/// 現在の移行ログを取得する
fn get_current_migration_log(conn: &Connection) -> AppResult<Option<(i64, String)>> {
    let result = conn
        .query_row(
            "SELECT id, status FROM migration_log WHERE migration_type = 'r2_user_directory' ORDER BY started_at DESC LIMIT 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        );

    match result {
        Ok((id, status)) => Ok(Some((id, status))),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Database(format!("移行ログ取得エラー: {e}"))),
    }
}

/// R2移行を実行する（プレースホルダー実装）
async fn execute_r2_migration(
    conn: &Connection,
    migration_log_id: i64,
    batch_size: usize,
) -> AppResult<R2MigrationExecutionResult> {
    log::info!("R2移行を実行します: log_id={migration_log_id}, batch_size={batch_size}");

    // TODO: 実際の移行処理を実装
    // 現在はプレースホルダー実装

    // ステータスを進行中に更新
    update_migration_log_status(conn, migration_log_id, "in_progress", 0, 0, 0, None)?;

    // 移行処理をシミュレート
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 完了ステータスに更新
    update_migration_log_status(conn, migration_log_id, "completed", 0, 0, 0, None)?;

    Ok(R2MigrationExecutionResult {
        success: true,
        message: "移行が完了しました（プレースホルダー実装）".to_string(),
        total_items: 0,
        success_count: 0,
        error_count: 0,
    })
}

/// R2移行実行結果
#[derive(Debug)]
struct R2MigrationExecutionResult {
    success: bool,
    message: String,
    total_items: usize,
    success_count: usize,
    error_count: usize,
}

/// 整合性検証を実行する（プレースホルダー実装）
fn perform_integrity_validation(conn: &Connection) -> AppResult<ValidationResult> {
    log::info!("整合性検証を実行します");

    // TODO: 実際の整合性検証を実装
    // 現在はプレースホルダー実装

    let database_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM expenses WHERE receipt_url IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(format!("データベースカウントエラー: {e}")))?;

    Ok(ValidationResult {
        success: true,
        message: "整合性検証が完了しました（プレースホルダー実装）".to_string(),
        database_receipt_count: database_count,
        r2_file_count: database_count, // プレースホルダー
        orphaned_files: 0,
        corrupted_files: 0,
        warnings: vec![],
        errors: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::database::connection::create_in_memory_connection;

    #[tokio::test]
    async fn test_estimate_migration_items() {
        let conn = create_in_memory_connection().unwrap();

        // テストデータを挿入
        conn.execute(
            "INSERT INTO expenses (date, amount, category, description, receipt_url, created_at, updated_at) 
             VALUES ('2024-01-01', 1000.0, 'test', 'test', 'https://example.com/receipts/test.pdf', '2024-01-01T00:00:00+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        ).unwrap();

        let count = estimate_migration_items(&conn).await.unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_current_migration_log() {
        let conn = create_in_memory_connection().unwrap();

        // migration_logテーブルを作成
        conn.execute(
            "CREATE TABLE migration_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                migration_type TEXT NOT NULL,
                status TEXT NOT NULL,
                total_items INTEGER NOT NULL DEFAULT 0,
                processed_items INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                error_count INTEGER NOT NULL DEFAULT 0,
                error_details TEXT,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                created_by TEXT,
                metadata TEXT
            )",
            [],
        )
        .unwrap();

        // 移行ログが存在しない場合
        let result = get_current_migration_log(&conn).unwrap();
        assert!(result.is_none());

        // 移行ログを作成
        let log_id =
            create_migration_log_entry(&conn, "r2_user_directory", 100, "test", None).unwrap();

        // 移行ログが取得できることを確認
        let result = get_current_migration_log(&conn).unwrap();
        assert!(result.is_some());
        let (id, status) = result.unwrap();
        assert_eq!(id, log_id);
        assert_eq!(status, "started");
    }
}

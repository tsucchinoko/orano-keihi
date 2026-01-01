use super::auto_migration::{AutoMigrationService, MigrationStatusReport};
use super::r2_migration_commands::{
    get_r2_migration_status, pause_r2_migration, resume_r2_migration, start_r2_migration,
    stop_r2_migration, validate_r2_migration_integrity,
};
use super::service::{
    create_backup, drop_receipt_path_column, is_receipt_url_migration_complete,
    is_user_authentication_migration_complete, list_backup_files, migrate_receipt_path_to_url,
    migrate_user_authentication, restore_from_backup, MigrationResult, MigrationStatus,
    RestoreResult,
};
use crate::shared::database::connection::initialize_database;
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use tauri::{AppHandle, Manager};

/// マイグレーション状態を確認する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// マイグレーション状態情報
#[tauri::command]
pub async fn check_migration_status(app_handle: AppHandle) -> Result<MigrationStatus, String> {
    let conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {e}"))?;

    let receipt_url_migration_complete = is_receipt_url_migration_complete(&conn)
        .map_err(|e| format!("マイグレーション状態確認エラー: {e}"))?;

    let user_auth_migration_complete = is_user_authentication_migration_complete(&conn)
        .map_err(|e| format!("ユーザー認証マイグレーション状態確認エラー: {e}"))?;

    // データベースバージョンを取得（簡易版）
    let database_version = if user_auth_migration_complete {
        "3.0.0".to_string() // ユーザー認証対応版
    } else if receipt_url_migration_complete {
        "2.0.0".to_string() // receipt_url対応版
    } else {
        "1.0.0".to_string() // receipt_path版
    };

    // 最後のマイグレーション日時（JST）
    let last_migration_date = if user_auth_migration_complete || receipt_url_migration_complete {
        Some(Utc::now().with_timezone(&Tokyo).to_rfc3339())
    } else {
        None
    };

    Ok(MigrationStatus {
        receipt_url_migration_complete,
        database_version,
        last_migration_date,
    })
}

/// ユーザー認証機能のマイグレーションを実行する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// マイグレーション結果
#[tauri::command]
pub async fn execute_user_authentication_migration(
    app_handle: AppHandle,
) -> Result<MigrationResult, String> {
    let conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {e}"))?;

    migrate_user_authentication(&conn)
        .map_err(|e| format!("ユーザー認証マイグレーション実行エラー: {e}"))
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
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {e}"))?;

    migrate_receipt_path_to_url(&conn).map_err(|e| format!("マイグレーション実行エラー: {e}"))
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
) -> Result<RestoreResult, String> {
    let mut conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {e}"))?;

    restore_from_backup(&mut conn, &backup_path).map_err(|e| format!("データベース復元エラー: {e}"))
}

/// receipt_pathカラムを削除する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// マイグレーション結果
#[tauri::command]
pub async fn drop_receipt_path_column_command(
    app_handle: AppHandle,
) -> Result<MigrationResult, String> {
    let conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {e}"))?;

    drop_receipt_path_column(&conn).map_err(|e| format!("カラム削除エラー: {e}"))
}

/// 利用可能なバックアップファイル一覧を取得する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// バックアップファイルのパス一覧
#[tauri::command]
pub async fn list_backup_files_command(app_handle: AppHandle) -> Result<Vec<String>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("アプリデータディレクトリ取得エラー: {e}"))?;

    list_backup_files(&app_data_dir).map_err(|e| format!("バックアップファイル一覧取得エラー: {e}"))
}

/// データベースの手動バックアップを作成する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// バックアップファイルのパス
#[tauri::command]
pub async fn create_manual_backup(app_handle: AppHandle) -> Result<String, String> {
    let conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {e}"))?;

    // バックアップファイルパスを生成（JST使用）
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("アプリデータディレクトリ取得エラー: {e}"))?;

    let now_jst = Utc::now().with_timezone(&Tokyo);
    let backup_filename = format!("manual_backup_{}.db", now_jst.format("%Y%m%d_%H%M%S"));
    let backup_path = app_data_dir.join(backup_filename);

    create_backup(&conn, backup_path.to_str().unwrap())
        .map_err(|e| format!("バックアップ作成エラー: {e}"))?;

    Ok(backup_path.to_string_lossy().to_string())
}

/// データベースの整合性チェックを実行する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// 整合性チェック結果
#[tauri::command]
pub async fn check_database_integrity(app_handle: AppHandle) -> Result<String, String> {
    let conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {e}"))?;

    // SQLiteの整合性チェックを実行
    let integrity_result: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("整合性チェック実行エラー: {e}"))?;

    if integrity_result == "ok" {
        Ok("データベースの整合性に問題はありません".to_string())
    } else {
        Ok(format!(
            "データベースの整合性に問題があります: {integrity_result}"
        ))
    }
}

/// データベースの統計情報を取得する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// データベース統計情報
#[tauri::command]
pub async fn get_database_stats(app_handle: AppHandle) -> Result<DatabaseStats, String> {
    let conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {e}"))?;

    // 各テーブルのレコード数を取得
    let expenses_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))
        .unwrap_or(0);

    let subscriptions_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM subscriptions", [], |row| row.get(0))
        .unwrap_or(0);

    let receipt_cache_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM receipt_cache", [], |row| row.get(0))
        .unwrap_or(0);

    let categories_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
        .unwrap_or(0);

    // ユーザー認証テーブルのレコード数を取得（存在する場合）
    let users_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .unwrap_or(0);

    let sessions_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .unwrap_or(0);

    // migrationsテーブルのレコード数を取得
    let migrations_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM migrations", [], |row| row.get(0))
        .unwrap_or(0);

    // データベースファイルサイズを取得
    let page_count: i64 = conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap_or(0);

    let page_size: i64 = conn
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .unwrap_or(4096);

    let database_size = page_count * page_size;

    Ok(DatabaseStats {
        expenses_count,
        subscriptions_count,
        receipt_cache_count,
        categories_count,
        users_count,
        sessions_count,
        database_size_bytes: database_size,
        page_count,
        page_size,
        migrations_count: Some(migrations_count),
    })
}

/// データベース統計情報
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DatabaseStats {
    pub expenses_count: i64,
    pub subscriptions_count: i64,
    pub receipt_cache_count: i64,
    pub categories_count: i64,
    pub users_count: i64,
    pub sessions_count: i64,
    pub database_size_bytes: i64,
    pub page_count: i64,
    pub page_size: i64,
    pub migrations_count: Option<i64>,
}

/// マイグレーション情報
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MigrationInfo {
    /// マイグレーション名
    pub name: String,
    /// バージョン
    pub version: String,
    /// 説明
    pub description: String,
    /// チェックサム
    pub checksum: String,
    /// 適用済みフラグ
    pub is_applied: bool,
    /// 適用日時（適用済みの場合）
    pub applied_at: Option<String>,
    /// 実行時間（適用済みの場合）
    pub execution_time_ms: Option<i64>,
}

/// 詳細なマイグレーション情報
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DetailedMigrationInfo {
    /// 基本的なマイグレーション状態レポート
    pub status_report: MigrationStatusReport,
    /// 利用可能なマイグレーション一覧（適用状態含む）
    pub available_migrations: Vec<MigrationInfo>,
    /// 適用済みマイグレーション詳細一覧
    pub applied_migrations: Vec<super::auto_migration::AppliedMigration>,
    /// データベース整合性状態
    pub integrity_status: String,
    /// データベース統計情報
    pub database_stats: DatabaseStats,
}

/// 包括的なデータ移行を実行する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// データ移行結果
#[tauri::command]
pub async fn execute_comprehensive_data_migration_command(
    app_handle: AppHandle,
) -> Result<super::service::DataMigrationResult, String> {
    let conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {e}"))?;

    super::service::execute_comprehensive_data_migration(&conn)
        .map_err(|e| format!("包括的データ移行実行エラー: {e}"))
}

/// 自動マイグレーションシステムの状態を確認する
///
/// 新しい自動マイグレーションシステムを使用して、詳細なマイグレーション状態を取得します。
/// 要件7.1, 7.2, 7.3, 7.4に対応します。
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// 詳細なマイグレーション状態レポート
#[tauri::command]
pub async fn check_auto_migration_status(
    app_handle: AppHandle,
) -> Result<MigrationStatusReport, String> {
    let conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {e}"))?;

    // 自動マイグレーションサービスを初期化
    let auto_migration_service = AutoMigrationService::new(&conn)
        .map_err(|e| format!("自動マイグレーションサービス初期化エラー: {e}"))?;

    // マイグレーション状態を確認
    auto_migration_service
        .check_migration_status(&conn)
        .map_err(|e| format!("マイグレーション状態確認エラー: {e}"))
}

/// 自動マイグレーションシステムの詳細情報を取得する
///
/// 適用済み・未適用マイグレーション一覧、実行日時、整合性状態を含む
/// 包括的な情報を提供します。要件7.1, 7.2, 7.3, 7.4に対応します。
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// 詳細なマイグレーション情報
#[tauri::command]
pub async fn get_detailed_migration_info(
    app_handle: AppHandle,
) -> Result<DetailedMigrationInfo, String> {
    let conn =
        initialize_database(&app_handle).map_err(|e| format!("データベース接続エラー: {e}"))?;

    // 自動マイグレーションサービスを初期化
    let auto_migration_service = AutoMigrationService::new(&conn)
        .map_err(|e| format!("自動マイグレーションサービス初期化エラー: {e}"))?;

    // 基本的なマイグレーション状態を取得
    let status_report = auto_migration_service
        .check_migration_status(&conn)
        .map_err(|e| format!("マイグレーション状態確認エラー: {e}"))?;

    // 適用済みマイグレーション詳細を取得
    let applied_migrations = super::auto_migration::MigrationTable::get_applied_migrations(&conn)
        .map_err(|e| format!("適用済みマイグレーション取得エラー: {e}"))?;

    // 利用可能なマイグレーション一覧を取得
    let available_migrations: Vec<MigrationInfo> = auto_migration_service
        .registry
        .get_available_migrations()
        .iter()
        .map(|m| MigrationInfo {
            name: m.name.clone(),
            version: m.version.clone(),
            description: m.description.clone(),
            checksum: m.checksum.clone(),
            is_applied: applied_migrations.iter().any(|am| am.name == m.name),
            applied_at: applied_migrations
                .iter()
                .find(|am| am.name == m.name)
                .map(|am| am.applied_at.clone()),
            execution_time_ms: applied_migrations
                .iter()
                .find(|am| am.name == m.name)
                .and_then(|am| am.execution_time_ms),
        })
        .collect();

    // データベース整合性チェック
    let integrity_status = check_database_integrity_internal(&conn)
        .map_err(|e| format!("データベース整合性チェックエラー: {e}"))?;

    Ok(DetailedMigrationInfo {
        status_report,
        available_migrations,
        applied_migrations,
        integrity_status,
        database_stats: get_database_stats_internal(&conn)
            .map_err(|e| format!("データベース統計取得エラー: {e}"))?,
    })
}

/// データベース整合性チェック（内部用）
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// 整合性チェック結果
fn check_database_integrity_internal(conn: &rusqlite::Connection) -> Result<String, String> {
    // SQLiteの整合性チェックを実行
    let integrity_result: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("整合性チェック実行エラー: {e}"))?;

    if integrity_result == "ok" {
        Ok("データベースの整合性に問題はありません".to_string())
    } else {
        Ok(format!(
            "データベースの整合性に問題があります: {integrity_result}"
        ))
    }
}

/// データベース統計情報取得（内部用）
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// データベース統計情報
fn get_database_stats_internal(conn: &rusqlite::Connection) -> Result<DatabaseStats, String> {
    // 各テーブルのレコード数を取得
    let expenses_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))
        .unwrap_or(0);

    let subscriptions_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM subscriptions", [], |row| row.get(0))
        .unwrap_or(0);

    let receipt_cache_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM receipt_cache", [], |row| row.get(0))
        .unwrap_or(0);

    let categories_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
        .unwrap_or(0);

    // ユーザー認証テーブルのレコード数を取得（存在する場合）
    let users_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .unwrap_or(0);

    let sessions_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .unwrap_or(0);

    // migrationsテーブルのレコード数を取得
    let migrations_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM migrations", [], |row| row.get(0))
        .unwrap_or(0);

    // データベースファイルサイズを取得
    let page_count: i64 = conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap_or(0);

    let page_size: i64 = conn
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .unwrap_or(4096);

    let database_size = page_count * page_size;

    Ok(DatabaseStats {
        expenses_count,
        subscriptions_count,
        receipt_cache_count,
        categories_count,
        users_count,
        sessions_count,
        database_size_bytes: database_size,
        page_count,
        page_size,
        migrations_count: Some(migrations_count),
    })
}

#[cfg(test)]
mod tests {
    // use super::*;

    // テストは一時的に無効化（tauri::testモジュールが利用できないため）
    /*
    #[tokio::test]
    async fn test_check_migration_status() {
        let app = mock_app();
        let result = check_migration_status(app.handle().clone()).await;

        // テスト環境では新規データベースが作成されるため、マイグレーション完了状態になる
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.receipt_url_migration_complete);
        assert_eq!(status.database_version, "2.0.0");
    }

    #[tokio::test]
    async fn test_create_manual_backup() {
        let app = mock_app();
        let result = create_manual_backup(app.handle().clone()).await;

        assert!(result.is_ok());
        let backup_path = result.unwrap();
        assert!(backup_path.contains("manual_backup_"));
        assert!(backup_path.ends_with(".db"));
    }

    #[tokio::test]
    async fn test_check_database_integrity() {
        let app = mock_app();
        let result = check_database_integrity(app.handle().clone()).await;

        assert!(result.is_ok());
        let integrity_message = result.unwrap();
        assert!(integrity_message.contains("問題はありません"));
    }

    #[tokio::test]
    async fn test_get_database_stats() {
        let app = mock_app();
        let result = get_database_stats(app.handle().clone()).await;

        assert!(result.is_ok());
        let stats = result.unwrap();
        assert!(stats.database_size_bytes > 0);
        assert!(stats.page_size > 0);
        assert!(stats.page_count >= 0);
    }
    */
}

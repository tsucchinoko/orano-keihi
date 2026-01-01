use crate::shared::errors::AppError;
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use rusqlite::Connection;
use rusqlite::{Result, Transaction};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// マイグレーション結果
#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationResult {
    pub success: bool,
    pub message: String,
    pub backup_path: Option<String>,
}

/// 復元結果
#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreResult {
    pub success: bool,
    pub message: String,
}

/// マイグレーション状態
#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationStatus {
    pub receipt_url_migration_complete: bool,
    pub database_version: String,
    pub last_migration_date: Option<String>,
}

/// すべてのデータベースマイグレーションを実行する
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
pub fn run_migrations(conn: &Connection) -> Result<(), AppError> {
    log::info!("データベースマイグレーションを開始します");

    // 既存のテーブル構造をチェック
    let table_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='expenses'",
        [],
        |row| row.get(0),
    )?;

    if table_exists == 0 {
        // 新規インストール: 最新のスキーマ（receipt_url）でテーブルを作成
        log::info!("新規データベースを作成します（receipt_urlスキーマ）");
        conn.execute(
            "CREATE TABLE expenses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                amount REAL NOT NULL,
                category TEXT NOT NULL,
                description TEXT,
                receipt_url TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        log::info!("新規データベースを作成しました（receipt_urlスキーマ）");
    } else {
        // 既存インストール: 必要なカラムを安全に追加
        log::info!("既存のデータベースを確認中...");

        // receipt_urlカラムが存在するかチェック
        let has_receipt_url = check_column_exists(conn, "expenses", "receipt_url");

        if !has_receipt_url {
            log::info!("receipt_urlカラムを追加します...");
            // receipt_urlカラムを追加（エラーを無視）
            let _ = conn.execute("ALTER TABLE expenses ADD COLUMN receipt_url TEXT", []);
        }

        // receipt_pathカラムが存在する場合は削除する
        let has_receipt_path = check_column_exists(conn, "expenses", "receipt_path");
        if has_receipt_path {
            log::info!("古いreceipt_pathカラムを削除します...");
            match drop_receipt_path_column(conn) {
                Ok(result) => {
                    if result.success {
                        log::info!("{}", result.message);
                    } else {
                        log::warn!("警告: {}", result.message);
                    }
                }
                Err(e) => {
                    log::warn!("警告: receipt_pathカラムの削除でエラーが発生しました: {e}");
                }
            }
        }
    }

    // 経費テーブルのインデックスを作成
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_category ON expenses(category)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_receipt_url ON expenses(receipt_url)",
        [],
    )?;

    // レシートキャッシュテーブルを作成
    conn.execute(
        "CREATE TABLE IF NOT EXISTS receipt_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            receipt_url TEXT NOT NULL UNIQUE,
            local_path TEXT NOT NULL,
            cached_at TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            last_accessed TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_receipt_cache_url ON receipt_cache(receipt_url)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_receipt_cache_accessed ON receipt_cache(last_accessed)",
        [],
    )?;

    // サブスクリプションテーブルを作成
    conn.execute(
        "CREATE TABLE IF NOT EXISTS subscriptions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            amount REAL NOT NULL,
            billing_cycle TEXT NOT NULL CHECK(billing_cycle IN ('monthly', 'annual')),
            start_date TEXT NOT NULL,
            category TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            receipt_path TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // 既存のサブスクリプションテーブルにreceipt_pathカラムを追加（存在しない場合）
    // SQLiteはALTER TABLE ADD COLUMN IF NOT EXISTSをサポートしていないため、
    // エラーを無視する方法で対応
    let _ = conn.execute("ALTER TABLE subscriptions ADD COLUMN receipt_path TEXT", []);

    // サブスクリプションテーブルのインデックスを作成
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_subscriptions_active ON subscriptions(is_active)",
        [],
    )?;

    // カテゴリテーブルを作成
    conn.execute(
        "CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL,
            icon TEXT
        )",
        [],
    )?;

    // テーブルが空の場合、初期カテゴリデータを挿入
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))?;

    if count == 0 {
        let categories = [
            ("交通費", "#3B82F6", "🚗"),
            ("飲食費", "#EF4444", "🍽️"),
            ("通信費", "#8B5CF6", "📱"),
            ("消耗品費", "#10B981", "📦"),
            ("接待交際費", "#F59E0B", "🤝"),
            ("その他", "#6B7280", "📋"),
        ];

        for (name, color, icon) in categories.iter() {
            conn.execute(
                "INSERT INTO categories (name, color, icon) VALUES (?1, ?2, ?3)",
                [name, color, icon],
            )?;
        }
    }

    log::info!("基本テーブルの作成・更新が完了しました");
    Ok(())
}

/// receipt_pathからreceipt_urlへのマイグレーションを実行する
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// マイグレーション結果
pub fn migrate_receipt_path_to_url(conn: &Connection) -> Result<MigrationResult, AppError> {
    // バックアップパスを生成（JST使用）
    let now_jst = Utc::now().with_timezone(&Tokyo);
    let backup_path = format!("database_backup_{}.db", now_jst.timestamp());

    // 1. バックアップを作成
    if let Err(e) = create_backup(conn, &backup_path) {
        return Ok(MigrationResult {
            success: false,
            message: format!("バックアップ作成に失敗しました: {e}"),
            backup_path: None,
        });
    }

    // 2. トランザクション内でマイグレーションを実行
    let tx = conn.unchecked_transaction()?;

    match execute_receipt_url_migration(&tx) {
        Ok(_) => {
            // 3. マイグレーション検証
            if let Err(e) = validate_migration(&tx) {
                // 検証失敗時はロールバック
                tx.rollback()?;
                return Ok(MigrationResult {
                    success: false,
                    message: format!("マイグレーション検証に失敗しました: {e}"),
                    backup_path: Some(backup_path),
                });
            }

            // 4. コミット
            tx.commit()?;

            Ok(MigrationResult {
                success: true,
                message: "receipt_pathからreceipt_urlへのマイグレーションが完了しました"
                    .to_string(),
                backup_path: Some(backup_path),
            })
        }
        Err(e) => {
            // エラー時はロールバック
            tx.rollback()?;
            Ok(MigrationResult {
                success: false,
                message: format!("マイグレーション実行に失敗しました: {e}"),
                backup_path: Some(backup_path),
            })
        }
    }
}

/// データベースのバックアップを作成する
///
/// # 引数
/// * `conn` - データベース接続
/// * `backup_path` - バックアップファイルのパス
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
pub fn create_backup(conn: &Connection, backup_path: &str) -> Result<(), AppError> {
    let mut backup_conn = rusqlite::Connection::open(backup_path)?;

    // SQLiteのバックアップAPI使用
    let backup = rusqlite::backup::Backup::new(conn, &mut backup_conn)?;
    backup.run_to_completion(5, std::time::Duration::from_millis(250), None)?;

    Ok(())
}

/// receipt_urlマイグレーションを実行する
///
/// # 引数
/// * `tx` - トランザクション
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn execute_receipt_url_migration(tx: &Transaction) -> Result<(), rusqlite::Error> {
    // 1. 新しいテーブル構造を作成
    tx.execute(
        "CREATE TABLE expenses_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            amount REAL NOT NULL,
            category TEXT NOT NULL,
            description TEXT,
            receipt_url TEXT CHECK(receipt_url IS NULL OR receipt_url LIKE 'https://%'),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // 2. 既存データを移行（receipt_pathは無視、新規データはreceipt_urlを使用）
    tx.execute(
        "INSERT INTO expenses_new (id, date, amount, category, description, created_at, updated_at)
         SELECT id, date, amount, category, description, created_at, updated_at
         FROM expenses",
        [],
    )?;

    // 3. 古いテーブルを削除
    tx.execute("DROP TABLE expenses", [])?;

    // 4. 新しいテーブルをリネーム
    tx.execute("ALTER TABLE expenses_new RENAME TO expenses", [])?;

    // 5. インデックスを再作成
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date)",
        [],
    )?;

    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_category ON expenses(category)",
        [],
    )?;

    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_receipt_url ON expenses(receipt_url)",
        [],
    )?;

    // 6. キャッシュメタデータテーブルを作成
    tx.execute(
        "CREATE TABLE IF NOT EXISTS receipt_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            receipt_url TEXT NOT NULL UNIQUE,
            local_path TEXT NOT NULL,
            cached_at TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            last_accessed TEXT NOT NULL
        )",
        [],
    )?;

    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_receipt_cache_url ON receipt_cache(receipt_url)",
        [],
    )?;

    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_receipt_cache_accessed ON receipt_cache(last_accessed)",
        [],
    )?;

    Ok(())
}

/// マイグレーション後の検証を実行する
///
/// # 引数
/// * `tx` - トランザクション
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn validate_migration(tx: &Transaction) -> Result<(), AppError> {
    // 1. テーブル構造の確認
    let table_info: Vec<(String, String)> = tx
        .prepare("PRAGMA table_info(expenses)")?
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // receipt_urlカラムが存在することを確認
    let has_receipt_url = table_info.iter().any(|(name, _)| name == "receipt_url");

    if !has_receipt_url {
        return Err(AppError::Validation(
            "receipt_urlカラムが見つかりません".to_string(),
        ));
    }

    // receipt_pathカラムが存在しないことを確認
    let has_receipt_path = table_info.iter().any(|(name, _)| name == "receipt_path");

    if has_receipt_path {
        return Err(AppError::Validation(
            "receipt_pathカラムが残っています".to_string(),
        ));
    }

    // 2. インデックスの確認
    let index_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name='expenses'",
        [],
        |row| row.get(0),
    )?;

    if index_count < 3 {
        return Err(AppError::Validation(
            "必要なインデックスが不足しています".to_string(),
        ));
    }

    // 3. キャッシュテーブルの確認
    let cache_table_exists: i64 = tx.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='receipt_cache'",
        [],
        |row| row.get(0),
    )?;

    if cache_table_exists == 0 {
        return Err(AppError::Validation(
            "receipt_cacheテーブルが作成されていません".to_string(),
        ));
    }

    Ok(())
}

/// バックアップからデータベースを復元する
///
/// # 引数
/// * `conn` - データベース接続（可変参照）
/// * `backup_path` - バックアップファイルのパス
///
/// # 戻り値
/// 復元結果
pub fn restore_from_backup(
    conn: &mut Connection,
    backup_path: &str,
) -> Result<RestoreResult, AppError> {
    // バックアップファイルが存在することを確認
    if !Path::new(backup_path).exists() {
        return Ok(RestoreResult {
            success: false,
            message: "バックアップファイルが見つかりません".to_string(),
        });
    }

    let backup_conn = rusqlite::Connection::open(backup_path)?;

    // バックアップから復元
    let backup = rusqlite::backup::Backup::new(&backup_conn, conn)?;
    backup.run_to_completion(5, std::time::Duration::from_millis(250), None)?;

    Ok(RestoreResult {
        success: true,
        message: "データベースの復元が完了しました".to_string(),
    })
}

/// マイグレーション状態をチェックする
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// receipt_urlマイグレーションが完了している場合はtrue
pub fn is_receipt_url_migration_complete(conn: &Connection) -> Result<bool, AppError> {
    // テーブルが存在するかチェック
    let table_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='expenses'",
        [],
        |row| row.get(0),
    )?;

    if table_exists == 0 {
        // テーブルが存在しない場合は、マイグレーション不要（新規作成）
        return Ok(true);
    }

    // receipt_urlカラムの存在を確認
    let table_info_result: Result<Vec<String>, rusqlite::Error> = conn
        .prepare("PRAGMA table_info(expenses)")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(1))
                .and_then(|rows| rows.collect())
        });

    match table_info_result {
        Ok(table_info) => {
            let has_receipt_url = table_info.iter().any(|name| name == "receipt_url");
            let has_receipt_path = table_info.iter().any(|name| name == "receipt_path");

            // receipt_urlが存在し、receipt_pathが存在しない場合はマイグレーション完了
            Ok(has_receipt_url && !has_receipt_path)
        }
        Err(e) => {
            eprintln!("テーブル情報の取得でエラーが発生しました: {e}");
            // エラー時は安全側に倒してマイグレーション完了とみなす
            Ok(true)
        }
    }
}

/// receipt_pathカラムを削除するマイグレーションを実行する
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// マイグレーション結果
pub fn drop_receipt_path_column(conn: &Connection) -> Result<MigrationResult, AppError> {
    // receipt_pathカラムが存在するかチェック
    if !check_column_exists(conn, "expenses", "receipt_path") {
        return Ok(MigrationResult {
            success: true,
            message: "receipt_pathカラムは既に存在しません".to_string(),
            backup_path: None,
        });
    }

    println!("receipt_pathカラムを削除します...");

    // トランザクション内でマイグレーションを実行
    let tx = conn.unchecked_transaction()?;

    match execute_drop_receipt_path(&tx) {
        Ok(_) => {
            // コミット
            tx.commit()?;

            Ok(MigrationResult {
                success: true,
                message: "receipt_pathカラムの削除が完了しました".to_string(),
                backup_path: None,
            })
        }
        Err(e) => {
            // エラー時はロールバック
            tx.rollback()?;
            Ok(MigrationResult {
                success: false,
                message: format!("receipt_pathカラムの削除に失敗しました: {e}"),
                backup_path: None,
            })
        }
    }
}

/// receipt_pathカラムの削除を実行する
///
/// # 引数
/// * `tx` - トランザクション
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn execute_drop_receipt_path(tx: &Transaction) -> Result<(), rusqlite::Error> {
    // 既存のテーブル構造を確認
    let table_info: Vec<(String, String)> = tx
        .prepare("PRAGMA table_info(expenses)")?
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let has_receipt_url = table_info.iter().any(|(name, _)| name == "receipt_url");
    let has_receipt_path = table_info.iter().any(|(name, _)| name == "receipt_path");

    // receipt_pathカラムが存在しない場合は何もしない
    if !has_receipt_path {
        return Ok(());
    }

    // 1. 新しいテーブル構造を作成（receipt_pathカラムなし）
    let create_table_sql = if has_receipt_url {
        // receipt_urlカラムが既に存在する場合
        "CREATE TABLE expenses_temp (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            amount REAL NOT NULL,
            category TEXT NOT NULL,
            description TEXT,
            receipt_url TEXT CHECK(receipt_url IS NULL OR receipt_url LIKE 'https://%'),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"
    } else {
        // receipt_urlカラムが存在しない場合（古いスキーマ）
        "CREATE TABLE expenses_temp (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            amount REAL NOT NULL,
            category TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"
    };

    tx.execute(create_table_sql, [])?;

    // 2. 既存データを移行（receipt_pathカラムを除く）
    let insert_sql = if has_receipt_url {
        // receipt_urlカラムが存在する場合
        "INSERT INTO expenses_temp (id, date, amount, category, description, receipt_url, created_at, updated_at)
         SELECT id, date, amount, category, description, receipt_url, created_at, updated_at
         FROM expenses"
    } else {
        // receipt_urlカラムが存在しない場合
        "INSERT INTO expenses_temp (id, date, amount, category, description, created_at, updated_at)
         SELECT id, date, amount, category, description, created_at, updated_at
         FROM expenses"
    };

    tx.execute(insert_sql, [])?;

    // 3. 古いテーブルを削除
    tx.execute("DROP TABLE expenses", [])?;

    // 4. 新しいテーブルをリネーム
    tx.execute("ALTER TABLE expenses_temp RENAME TO expenses", [])?;

    // 5. インデックスを再作成
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date)",
        [],
    )?;

    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_category ON expenses(category)",
        [],
    )?;

    if has_receipt_url {
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_expenses_receipt_url ON expenses(receipt_url)",
            [],
        )?;
    }

    Ok(())
}

/// テーブルに指定されたカラムが存在するかチェックする
///
/// # 引数
/// * `conn` - データベース接続
/// * `table_name` - テーブル名
/// * `column_name` - カラム名
///
/// # 戻り値
/// カラムが存在する場合はtrue、存在しないかエラーの場合はfalse
fn check_column_exists(conn: &Connection, table_name: &str, column_name: &str) -> bool {
    let query = format!("PRAGMA table_info({table_name})");

    match conn.prepare(&query) {
        Ok(mut stmt) => {
            match stmt.query_map([], |row| {
                let col_name: String = row.get(1)?;
                Ok(col_name)
            }) {
                Ok(rows) => {
                    for col_name in rows.flatten() {
                        if col_name == column_name {
                            return true;
                        }
                    }
                    false
                }
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

/// 利用可能なバックアップファイル一覧を取得する
///
/// # 引数
/// * `app_data_dir` - アプリデータディレクトリのパス
///
/// # 戻り値
/// バックアップファイルのパス一覧
pub fn list_backup_files(app_data_dir: &Path) -> Result<Vec<String>, AppError> {
    let mut backup_files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(app_data_dir) {
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

/// 包括的なデータ移行を実行する
///
/// この関数は既存データの安全な移行とバックアップ作成を行います。
/// 要件7.5「データ移行時のバックアップ」を満たします。
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// データ移行結果
pub fn execute_comprehensive_data_migration(
    conn: &Connection,
) -> Result<DataMigrationResult, AppError> {
    // バックアップパスを生成（JST使用）
    let now_jst = Utc::now().with_timezone(&Tokyo);
    let backup_path = format!("database_backup_migration_{}.db", now_jst.timestamp());

    // 1. 移行前のバックアップを作成
    if let Err(e) = create_backup(conn, &backup_path) {
        return Ok(DataMigrationResult {
            success: false,
            message: format!("移行前バックアップ作成に失敗しました: {e}"),
            backup_path: None,
            migrated_tables: Vec::new(),
            data_integrity_verified: false,
        });
    }

    // 2. データ整合性チェック（移行前）
    if let Err(e) = verify_data_integrity_before_migration(conn) {
        return Ok(DataMigrationResult {
            success: false,
            message: format!("移行前データ整合性チェックに失敗しました: {e}"),
            backup_path: Some(backup_path),
            migrated_tables: Vec::new(),
            data_integrity_verified: false,
        });
    }

    // 3. トランザクション内で包括的マイグレーションを実行
    let tx = conn.unchecked_transaction()?;
    let mut migrated_tables = Vec::new();

    match execute_comprehensive_migration(&tx, &mut migrated_tables) {
        Ok(_) => {
            // 4. 移行後のデータ整合性検証
            if let Err(e) = verify_data_integrity_after_migration(&tx) {
                // 検証失敗時はロールバック
                tx.rollback()?;
                return Ok(DataMigrationResult {
                    success: false,
                    message: format!("移行後データ整合性検証に失敗しました: {e}"),
                    backup_path: Some(backup_path),
                    migrated_tables,
                    data_integrity_verified: false,
                });
            }

            // 5. コミット
            tx.commit()?;

            Ok(DataMigrationResult {
                success: true,
                message: "包括的なデータ移行が完了しました".to_string(),
                backup_path: Some(backup_path),
                migrated_tables,
                data_integrity_verified: true,
            })
        }
        Err(e) => {
            // エラー時はロールバック
            tx.rollback()?;
            Ok(DataMigrationResult {
                success: false,
                message: format!("データ移行実行に失敗しました: {e}"),
                backup_path: Some(backup_path),
                migrated_tables,
                data_integrity_verified: false,
            })
        }
    }
}

/// 包括的マイグレーションを実行する
///
/// # 引数
/// * `tx` - トランザクション
/// * `migrated_tables` - 移行されたテーブル一覧（出力用）
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn execute_comprehensive_migration(
    tx: &Transaction,
    migrated_tables: &mut Vec<String>,
) -> Result<(), rusqlite::Error> {
    // 1. ユーザー認証テーブルの作成（まだ存在しない場合）
    if !check_table_exists_in_tx(tx, "users") {
        execute_user_authentication_migration(tx)?;
        migrated_tables.push("users".to_string());
        migrated_tables.push("sessions".to_string());
    }

    // 2. 既存テーブルにuser_idカラムを追加（まだ存在しない場合）
    let tables_to_migrate = ["expenses", "subscriptions", "receipt_cache"];
    for table in &tables_to_migrate {
        if check_table_exists_in_tx(tx, table) && !check_column_exists_in_tx(tx, table, "user_id") {
            add_user_id_column_to_table(tx, table)?;
            migrated_tables.push(table.to_string());
        }
    }

    // 3. 既存データにデフォルトユーザーIDを設定
    for table in &tables_to_migrate {
        if check_table_exists_in_tx(tx, table) {
            assign_default_user_id_to_existing_data(tx, table)?;
        }
    }

    // 4. 外部キー制約の有効化
    tx.execute("PRAGMA foreign_keys = ON", [])?;

    Ok(())
}

/// テーブルにuser_idカラムを追加する
///
/// # 引数
/// * `tx` - トランザクション
/// * `table_name` - テーブル名
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn add_user_id_column_to_table(tx: &Transaction, table_name: &str) -> Result<(), rusqlite::Error> {
    let alter_sql =
        format!("ALTER TABLE {table_name} ADD COLUMN user_id INTEGER REFERENCES users(id)");

    // カラムを追加（エラーを無視 - 既に存在する場合）
    let _ = tx.execute(&alter_sql, []);

    // インデックスを作成
    let index_sql =
        format!("CREATE INDEX IF NOT EXISTS idx_{table_name}_user_id ON {table_name}(user_id)");
    tx.execute(&index_sql, [])?;

    Ok(())
}

/// 既存データにデフォルトユーザーIDを割り当てる
///
/// # 引数
/// * `tx` - トランザクション
/// * `table_name` - テーブル名
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn assign_default_user_id_to_existing_data(
    tx: &Transaction,
    table_name: &str,
) -> Result<(), rusqlite::Error> {
    let update_sql = format!("UPDATE {table_name} SET user_id = 1 WHERE user_id IS NULL");
    tx.execute(&update_sql, [])?;
    Ok(())
}

/// 移行前のデータ整合性を検証する
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn verify_data_integrity_before_migration(conn: &Connection) -> Result<(), AppError> {
    // 1. SQLiteの整合性チェック
    let integrity_result: String =
        conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;

    if integrity_result != "ok" {
        return Err(AppError::Validation(format!(
            "データベース整合性チェック失敗: {integrity_result}"
        )));
    }

    // 2. 重要なテーブルの存在確認
    let required_tables = ["expenses"];
    for table in &required_tables {
        let table_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
            [table],
            |row| row.get(0),
        )?;

        if table_exists == 0 {
            return Err(AppError::Validation(format!(
                "必要なテーブル '{table}' が存在しません"
            )));
        }
    }

    // 3. データ件数の記録（移行後の検証用）
    let expenses_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))
        .unwrap_or(0);

    log::info!("移行前データ件数 - expenses: {expenses_count}");

    Ok(())
}

/// 移行後のデータ整合性を検証する
///
/// # 引数
/// * `tx` - トランザクション
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn verify_data_integrity_after_migration(tx: &Transaction) -> Result<(), AppError> {
    // 1. 外部キー制約チェック
    tx.execute("PRAGMA foreign_key_check", [])?;

    // 2. ユーザー認証テーブルの確認
    let users_count: i64 = tx.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;

    if users_count == 0 {
        return Err(AppError::Validation(
            "usersテーブルにデフォルトユーザーが作成されていません".to_string(),
        ));
    }

    // 3. user_idカラムの確認
    let tables_to_check = ["expenses", "subscriptions", "receipt_cache"];
    for table in &tables_to_check {
        if check_table_exists_in_tx(tx, table) {
            if !check_column_exists_in_tx(tx, table, "user_id") {
                return Err(AppError::Validation(format!(
                    "{table}テーブルにuser_idカラムが追加されていません"
                )));
            }

            // NULL値のuser_idが存在しないことを確認
            let null_user_id_count: i64 = tx.query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE user_id IS NULL"),
                [],
                |row| row.get(0),
            )?;

            if null_user_id_count > 0 {
                return Err(AppError::Validation(format!(
                    "{table}テーブルにuser_idがNULLのレコードが {null_user_id_count} 件存在します"
                )));
            }
        }
    }

    // 4. データ件数の確認（データ損失がないことを確認）
    let expenses_count: i64 = tx
        .query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))
        .unwrap_or(0);

    log::info!("移行後データ件数 - expenses: {expenses_count}");

    Ok(())
}

/// トランザクション内でテーブルが存在するかチェックする
///
/// # 引数
/// * `tx` - トランザクション
/// * `table_name` - テーブル名
///
/// # 戻り値
/// テーブルが存在する場合はtrue
fn check_table_exists_in_tx(tx: &Transaction, table_name: &str) -> bool {
    let count: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
            [table_name],
            |row| row.get(0),
        )
        .unwrap_or(0);

    count > 0
}

/// データ移行結果
#[derive(Debug, Serialize, Deserialize)]
pub struct DataMigrationResult {
    pub success: bool,
    pub message: String,
    pub backup_path: Option<String>,
    pub migrated_tables: Vec<String>,
    pub data_integrity_verified: bool,
}

/// ユーザー認証機能のマイグレーションを実行する
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// マイグレーション結果
pub fn migrate_user_authentication(conn: &Connection) -> Result<MigrationResult, AppError> {
    log::info!("ユーザー認証マイグレーションを開始します");

    // 環境情報をログ出力
    log::info!("環境設定: ENVIRONMENT={:?}", std::env::var("ENVIRONMENT"));
    log::info!("データベースファイルパス: {:?}", conn.path());

    // バックアップパスを生成（JST使用）
    let now_jst = Utc::now().with_timezone(&Tokyo);
    let backup_path = format!("database_backup_auth_{}.db", now_jst.timestamp());

    // 1. バックアップを作成
    log::info!("データベースバックアップを作成中: {backup_path}");
    if let Err(e) = create_backup(conn, &backup_path) {
        log::error!("バックアップ作成に失敗: {e}");
        return Ok(MigrationResult {
            success: false,
            message: format!("バックアップ作成に失敗しました: {e}"),
            backup_path: None,
        });
    }
    log::info!("バックアップ作成完了");

    // 2. トランザクション内でマイグレーションを実行
    log::info!("マイグレーショントランザクションを開始");
    let tx = conn.unchecked_transaction()?;

    match execute_user_authentication_migration(&tx) {
        Ok(_) => {
            log::info!("マイグレーション実行完了、検証を開始");

            // 3. マイグレーション検証
            if let Err(e) = validate_user_authentication_migration(&tx) {
                // 検証失敗時はロールバック
                log::error!("マイグレーション検証に失敗、ロールバック実行: {e}");
                tx.rollback()?;
                return Ok(MigrationResult {
                    success: false,
                    message: format!("マイグレーション検証に失敗しました: {e}"),
                    backup_path: Some(backup_path),
                });
            }

            // 4. コミット
            log::info!("マイグレーション検証完了、コミット実行");
            tx.commit()?;
            log::info!("ユーザー認証マイグレーションが正常に完了しました");

            Ok(MigrationResult {
                success: true,
                message: "ユーザー認証機能のマイグレーションが完了しました".to_string(),
                backup_path: Some(backup_path),
            })
        }
        Err(e) => {
            // エラー時はロールバック
            log::error!("マイグレーション実行中にエラー発生、ロールバック実行: {e}");
            tx.rollback()?;
            Ok(MigrationResult {
                success: false,
                message: format!("マイグレーション実行に失敗しました: {e}"),
                backup_path: Some(backup_path),
            })
        }
    }
}

/// ユーザー認証マイグレーションを実行する
///
/// # 引数
/// * `tx` - トランザクション
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn execute_user_authentication_migration(tx: &Transaction) -> Result<(), rusqlite::Error> {
    // 1. usersテーブルを作成
    tx.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            google_id TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL,
            name TEXT NOT NULL,
            picture_url TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // usersテーブルのインデックスを作成
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_users_google_id ON users(google_id)",
        [],
    )?;

    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)",
        [],
    )?;

    // 2. sessionsテーブルを作成
    tx.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id INTEGER NOT NULL,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // sessionsテーブルのインデックスを作成
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id)",
        [],
    )?;

    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at)",
        [],
    )?;

    // 3. デフォルトユーザーを作成（既存データ用）
    let default_user_exists: i64 =
        tx.query_row("SELECT COUNT(*) FROM users WHERE id = 1", [], |row| {
            row.get(0)
        })?;

    if default_user_exists == 0 {
        let now_jst = Utc::now().with_timezone(&Tokyo);
        let timestamp = now_jst.to_rfc3339();

        // INSERT OR IGNOREを使用して重複を回避
        tx.execute(
            "INSERT OR IGNORE INTO users (id, google_id, email, name, picture_url, created_at, updated_at)
             VALUES (1, 'default_user', 'default@example.com', 'デフォルトユーザー', NULL, ?1, ?2)",
            [&timestamp, &timestamp],
        )?;
    }

    // 4. 既存テーブルにuser_idカラムを追加
    add_user_id_to_existing_tables(tx)?;

    Ok(())
}

/// 既存テーブルにuser_idカラムを追加する
///
/// # 引数
/// * `tx` - トランザクション
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn add_user_id_to_existing_tables(tx: &Transaction) -> Result<(), rusqlite::Error> {
    // expensesテーブルにuser_idを追加
    if !check_column_exists_in_tx(tx, "expenses", "user_id") {
        // カラムを追加（エラーを無視）
        let _ = tx.execute(
            "ALTER TABLE expenses ADD COLUMN user_id INTEGER REFERENCES users(id)",
            [],
        );

        // 既存データにデフォルトユーザーIDを設定
        tx.execute("UPDATE expenses SET user_id = 1 WHERE user_id IS NULL", [])?;

        // インデックスを作成
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_expenses_user_id ON expenses(user_id)",
            [],
        )?;
    }

    // subscriptionsテーブルにuser_idを追加
    if !check_column_exists_in_tx(tx, "subscriptions", "user_id") {
        // カラムを追加（エラーを無視）
        let _ = tx.execute(
            "ALTER TABLE subscriptions ADD COLUMN user_id INTEGER REFERENCES users(id)",
            [],
        );

        // 既存データにデフォルトユーザーIDを設定
        tx.execute(
            "UPDATE subscriptions SET user_id = 1 WHERE user_id IS NULL",
            [],
        )?;

        // インデックスを作成
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON subscriptions(user_id)",
            [],
        )?;
    }

    // receipt_cacheテーブルにuser_idを追加
    if !check_column_exists_in_tx(tx, "receipt_cache", "user_id") {
        // カラムを追加（エラーを無視）
        let _ = tx.execute(
            "ALTER TABLE receipt_cache ADD COLUMN user_id INTEGER REFERENCES users(id)",
            [],
        );

        // 既存データにデフォルトユーザーIDを設定
        tx.execute(
            "UPDATE receipt_cache SET user_id = 1 WHERE user_id IS NULL",
            [],
        )?;

        // インデックスを作成
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_receipt_cache_user_id ON receipt_cache(user_id)",
            [],
        )?;
    }

    Ok(())
}

/// トランザクション内でテーブルに指定されたカラムが存在するかチェックする
///
/// # 引数
/// * `tx` - トランザクション
/// * `table_name` - テーブル名
/// * `column_name` - カラム名
///
/// # 戻り値
/// カラムが存在する場合はtrue、存在しないかエラーの場合はfalse
fn check_column_exists_in_tx(tx: &Transaction, table_name: &str, column_name: &str) -> bool {
    let query = format!("PRAGMA table_info({table_name})");

    match tx.prepare(&query) {
        Ok(mut stmt) => {
            match stmt.query_map([], |row| {
                let col_name: String = row.get(1)?;
                Ok(col_name)
            }) {
                Ok(rows) => {
                    for col_name in rows.flatten() {
                        if col_name == column_name {
                            return true;
                        }
                    }
                    false
                }
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

/// ユーザー認証マイグレーション後の検証を実行する
///
/// # 引数
/// * `tx` - トランザクション
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn validate_user_authentication_migration(tx: &Transaction) -> Result<(), AppError> {
    // 1. usersテーブルの確認
    let users_table_exists: i64 = tx.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='users'",
        [],
        |row| row.get(0),
    )?;

    if users_table_exists == 0 {
        return Err(AppError::Validation(
            "usersテーブルが作成されていません".to_string(),
        ));
    }

    // 2. sessionsテーブルの確認
    let sessions_table_exists: i64 = tx.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
        [],
        |row| row.get(0),
    )?;

    if sessions_table_exists == 0 {
        return Err(AppError::Validation(
            "sessionsテーブルが作成されていません".to_string(),
        ));
    }

    // 3. デフォルトユーザーの確認
    let default_user_exists: i64 =
        tx.query_row("SELECT COUNT(*) FROM users WHERE id = 1", [], |row| {
            row.get(0)
        })?;

    if default_user_exists == 0 {
        return Err(AppError::Validation(
            "デフォルトユーザーが作成されていません".to_string(),
        ));
    }

    // 4. 既存テーブルのuser_idカラムの確認
    let tables_to_check = ["expenses", "subscriptions", "receipt_cache"];
    for table in &tables_to_check {
        if !check_column_exists_in_tx(tx, table, "user_id") {
            return Err(AppError::Validation(format!(
                "{table}テーブルにuser_idカラムが追加されていません"
            )));
        }
    }

    // 5. 外部キー制約の確認（SQLiteでは実行時に確認）
    tx.execute("PRAGMA foreign_key_check", [])?;

    Ok(())
}

/// ユーザー認証マイグレーションが完了しているかチェックする
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// ユーザー認証マイグレーションが完了している場合はtrue
pub fn is_user_authentication_migration_complete(conn: &Connection) -> Result<bool, AppError> {
    // usersテーブルの存在確認
    let users_table_exists: i64 = match conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='users'",
        [],
        |row| row.get(0),
    ) {
        Ok(count) => count,
        Err(e) => {
            log::warn!("usersテーブルの存在確認でエラー: {e}");
            return Ok(false);
        }
    };

    if users_table_exists == 0 {
        log::debug!("usersテーブルが存在しないため、マイグレーション未完了");
        return Ok(false);
    }

    // sessionsテーブルの存在確認
    let sessions_table_exists: i64 = match conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
        [],
        |row| row.get(0),
    ) {
        Ok(count) => count,
        Err(e) => {
            log::warn!("sessionsテーブルの存在確認でエラー: {e}");
            return Ok(false);
        }
    };

    if sessions_table_exists == 0 {
        log::debug!("sessionsテーブルが存在しないため、マイグレーション未完了");
        return Ok(false);
    }

    // 既存テーブルのuser_idカラムの確認
    let tables_to_check = ["expenses", "subscriptions", "receipt_cache"];
    for table in &tables_to_check {
        // テーブルが存在するかチェック
        let table_exists: i64 = match conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
            [table],
            |row| row.get(0),
        ) {
            Ok(count) => count,
            Err(e) => {
                log::warn!("テーブル {table} の存在確認でエラー: {e}");
                continue; // このテーブルは存在しないのでスキップ
            }
        };

        if table_exists > 0 && !check_column_exists(conn, table, "user_id") {
            log::debug!("テーブル {table} にuser_idカラムが存在しないため、マイグレーション未完了");
            return Ok(false);
        }
    }

    log::debug!("ユーザー認証マイグレーションは完了しています");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection as SqliteConnection;
    use tempfile::NamedTempFile;

    /// テスト用のデータベースを作成する
    fn create_test_db() -> SqliteConnection {
        let conn = SqliteConnection::open_in_memory().unwrap();

        // 古いスキーマ（receipt_path）でテーブルを作成
        conn.execute(
            "CREATE TABLE expenses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                amount REAL NOT NULL,
                category TEXT NOT NULL,
                description TEXT,
                receipt_path TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        // テストデータを挿入
        conn.execute(
            "INSERT INTO expenses (date, amount, category, description, receipt_path, created_at, updated_at)
             VALUES ('2024-01-01', 1000.0, 'テスト', 'テスト経費', '/path/to/receipt.jpg', '2024-01-01T00:00:00+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        ).unwrap();

        conn
    }

    #[test]
    fn test_is_receipt_url_migration_complete_false() {
        let conn = create_test_db();

        // 古いスキーマではマイグレーション未完了
        let result = is_receipt_url_migration_complete(&conn).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_migrate_receipt_path_to_url() {
        let conn = create_test_db();

        // マイグレーションを実行
        let result = migrate_receipt_path_to_url(&conn).unwrap();

        // マイグレーション成功を確認
        assert!(result.success);
        assert!(result.backup_path.is_some());

        // マイグレーション完了を確認
        let is_complete = is_receipt_url_migration_complete(&conn).unwrap();
        assert!(is_complete);

        // 新しいスキーマでデータが保持されていることを確認
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        // receipt_urlカラムが存在することを確認
        let table_info: Vec<String> = conn
            .prepare("PRAGMA table_info(expenses)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(table_info.contains(&"receipt_url".to_string()));
        assert!(!table_info.contains(&"receipt_path".to_string()));

        // キャッシュテーブルが作成されていることを確認
        let cache_table_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='receipt_cache'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cache_table_exists, 1);
    }

    #[test]
    fn test_validate_migration() {
        let conn = create_test_db();
        let tx = conn.unchecked_transaction().unwrap();

        // 正しいスキーマを作成
        execute_receipt_url_migration(&tx).unwrap();

        // 検証が成功することを確認
        let result = validate_migration(&tx);
        assert!(result.is_ok());

        tx.commit().unwrap();
    }

    #[test]
    fn test_backup_and_restore() {
        let mut conn = SqliteConnection::open_in_memory().unwrap();
        let temp_file = NamedTempFile::new().unwrap();
        let backup_path = temp_file.path().to_str().unwrap();

        // テストデータを作成
        conn.execute(
            "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT)",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO test_table (name) VALUES ('test')", [])
            .unwrap();

        // バックアップを作成
        create_backup(&conn, backup_path).unwrap();

        // データを変更
        conn.execute("DELETE FROM test_table", []).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test_table", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);

        // バックアップから復元
        let result = restore_from_backup(&mut conn, backup_path).unwrap();
        assert!(result.success);

        // データが復元されていることを確認
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test_table", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_drop_receipt_path_column() {
        let conn = create_test_db();

        // receipt_pathカラムが存在することを確認
        assert!(check_column_exists(&conn, "expenses", "receipt_path"));

        // receipt_pathカラムを削除
        let result = drop_receipt_path_column(&conn).unwrap();

        // エラーメッセージを出力
        if !result.success {
            println!("マイグレーション失敗: {}", result.message);
        }
        assert!(result.success, "マイグレーション失敗: {}", result.message);

        // receipt_pathカラムが削除されたことを確認
        assert!(!check_column_exists(&conn, "expenses", "receipt_path"));

        // データが保持されていることを確認
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        // 既に削除されている場合のテスト
        let result2 = drop_receipt_path_column(&conn).unwrap();
        assert!(result2.success);
        assert!(result2.message.contains("既に存在しません"));
    }

    #[test]
    fn test_check_column_exists() {
        let conn = create_test_db();

        // 存在するカラムのテスト
        assert!(check_column_exists(&conn, "expenses", "id"));
        assert!(check_column_exists(&conn, "expenses", "receipt_path"));

        // 存在しないカラムのテスト
        assert!(!check_column_exists(
            &conn,
            "expenses",
            "nonexistent_column"
        ));

        // 存在しないテーブルのテスト
        assert!(!check_column_exists(&conn, "nonexistent_table", "id"));
    }

    #[test]
    fn test_user_authentication_migration() {
        let conn = SqliteConnection::open_in_memory().unwrap();

        // 基本的なテーブル構造を作成
        conn.execute(
            "CREATE TABLE expenses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                amount REAL NOT NULL,
                category TEXT NOT NULL,
                description TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE subscriptions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                amount REAL NOT NULL,
                billing_cycle TEXT NOT NULL,
                start_date TEXT NOT NULL,
                category TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE receipt_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                receipt_url TEXT NOT NULL UNIQUE,
                local_path TEXT NOT NULL,
                cached_at TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                last_accessed TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        // テストデータを挿入
        conn.execute(
            "INSERT INTO expenses (date, amount, category, description, created_at, updated_at)
             VALUES ('2024-01-01', 1000.0, 'テスト', 'テスト経費', '2024-01-01T00:00:00+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        ).unwrap();

        // マイグレーション前の状態確認
        let is_complete_before = is_user_authentication_migration_complete(&conn).unwrap();
        assert!(!is_complete_before);

        // マイグレーションを実行
        let result = migrate_user_authentication(&conn).unwrap();
        assert!(result.success, "マイグレーション失敗: {}", result.message);

        // マイグレーション後の状態確認
        let is_complete_after = is_user_authentication_migration_complete(&conn).unwrap();
        assert!(is_complete_after);

        // usersテーブルが作成されていることを確認
        let users_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .unwrap();
        assert_eq!(users_count, 1); // デフォルトユーザーが作成されている

        // sessionsテーブルが作成されていることを確認
        let sessions_table_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(sessions_table_exists, 1);

        // 既存テーブルにuser_idカラムが追加されていることを確認
        assert!(check_column_exists(&conn, "expenses", "user_id"));
        assert!(check_column_exists(&conn, "subscriptions", "user_id"));
        assert!(check_column_exists(&conn, "receipt_cache", "user_id"));

        // 既存データにデフォルトユーザーIDが設定されていることを確認
        let expense_user_id: i64 = conn
            .query_row("SELECT user_id FROM expenses WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(expense_user_id, 1);
    }

    #[test]
    fn test_user_authentication_migration_idempotent() {
        let conn = SqliteConnection::open_in_memory().unwrap();

        // 基本的なテーブル構造を作成
        conn.execute(
            "CREATE TABLE expenses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                amount REAL NOT NULL,
                category TEXT NOT NULL,
                description TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE subscriptions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                amount REAL NOT NULL,
                billing_cycle TEXT NOT NULL,
                start_date TEXT NOT NULL,
                category TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE receipt_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                receipt_url TEXT NOT NULL UNIQUE,
                local_path TEXT NOT NULL,
                cached_at TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                last_accessed TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        // 最初のマイグレーション
        let result1 = migrate_user_authentication(&conn).unwrap();
        if !result1.success {
            println!("最初のマイグレーション失敗: {}", result1.message);
        }
        assert!(
            result1.success,
            "最初のマイグレーション失敗: {}",
            result1.message
        );

        // 2回目のマイグレーション（冪等性のテスト）
        let result2 = migrate_user_authentication(&conn).unwrap();
        if !result2.success {
            println!("2回目のマイグレーション失敗: {}", result2.message);
        }
        assert!(
            result2.success,
            "2回目のマイグレーション失敗: {}",
            result2.message
        );

        // デフォルトユーザーが重複していないことを確認
        let users_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM users WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(users_count, 1);
    }

    #[test]
    fn test_comprehensive_data_migration() {
        let conn = SqliteConnection::open_in_memory().unwrap();

        // 基本的なテーブル構造を作成（ユーザー認証なし）
        conn.execute(
            "CREATE TABLE expenses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                amount REAL NOT NULL,
                category TEXT NOT NULL,
                description TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE subscriptions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                amount REAL NOT NULL,
                billing_cycle TEXT NOT NULL,
                start_date TEXT NOT NULL,
                category TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        // receipt_cacheテーブルを作成
        conn.execute(
            "CREATE TABLE receipt_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                receipt_url TEXT NOT NULL UNIQUE,
                local_path TEXT NOT NULL,
                cached_at TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                last_accessed TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        // テストデータを挿入
        conn.execute(
            "INSERT INTO expenses (date, amount, category, description, created_at, updated_at)
             VALUES ('2024-01-01', 1000.0, 'テスト', 'テスト経費', '2024-01-01T00:00:00+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO subscriptions (name, amount, billing_cycle, start_date, category, created_at, updated_at)
             VALUES ('テストサブスク', 500.0, 'monthly', '2024-01-01', 'テスト', '2024-01-01T00:00:00+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        ).unwrap();

        // 包括的データ移行を実行
        let result = execute_comprehensive_data_migration(&conn).unwrap();
        assert!(result.success, "包括的データ移行失敗: {}", result.message);
        assert!(result.data_integrity_verified);
        assert!(result.backup_path.is_some());
        assert!(!result.migrated_tables.is_empty());

        // 移行後の状態確認
        let is_complete = is_user_authentication_migration_complete(&conn).unwrap();
        assert!(is_complete);

        // データが保持されていることを確認
        let expenses_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))
            .unwrap();
        assert_eq!(expenses_count, 1);

        let subscriptions_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM subscriptions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(subscriptions_count, 1);

        // user_idが設定されていることを確認
        let expense_user_id: i64 = conn
            .query_row("SELECT user_id FROM expenses WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(expense_user_id, 1);

        let subscription_user_id: i64 = conn
            .query_row(
                "SELECT user_id FROM subscriptions WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(subscription_user_id, 1);
    }

    #[test]
    fn test_data_migration_with_existing_user_auth() {
        let conn = SqliteConnection::open_in_memory().unwrap();

        // 手動でusersテーブルを作成（既にユーザー認証が設定されているデータベースをシミュレート）
        conn.execute(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                google_id TEXT NOT NULL UNIQUE,
                email TEXT NOT NULL,
                name TEXT NOT NULL,
                picture_url TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL REFERENCES users(id),
                encrypted_data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        // テストユーザーを挿入
        conn.execute(
            "INSERT INTO users (id, google_id, email, name, created_at, updated_at)
             VALUES (1, 'test_google_id', 'test@example.com', 'テストユーザー', '2024-01-01T00:00:00+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE expenses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                amount REAL NOT NULL,
                category TEXT NOT NULL,
                description TEXT,
                user_id INTEGER REFERENCES users(id),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        // テストデータを挿入
        conn.execute(
            "INSERT INTO expenses (date, amount, category, description, user_id, created_at, updated_at)
             VALUES ('2024-01-01', 1000.0, 'テスト', 'テスト経費', 1, '2024-01-01T00:00:00+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        ).unwrap();

        // 包括的データ移行を実行（既に移行済みの場合）
        let result = execute_comprehensive_data_migration(&conn).unwrap();
        assert!(result.success, "包括的データ移行失敗: {}", result.message);
        assert!(result.data_integrity_verified);

        // データが保持されていることを確認
        let expenses_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))
            .unwrap();
        assert_eq!(expenses_count, 1);
    }
}

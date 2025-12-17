use rusqlite::{Connection, Result, Transaction};

/// マイグレーションエラー
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("データベースエラー: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("バックアップ作成エラー: {0}")]
    BackupFailed(String),
    #[error("ロールバックエラー: {0}")]
    RollbackFailed(String),
    #[error("マイグレーション検証エラー: {0}")]
    ValidationFailed(String),
}

/// マイグレーション結果
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MigrationResult {
    pub success: bool,
    pub message: String,
    pub backup_path: Option<String>,
}

/// すべてのデータベースマイグレーションを実行する
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // 既存のテーブル構造をチェック
    let table_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='expenses'",
        [],
        |row| row.get(0),
    )?;

    if table_exists == 0 {
        // 新規インストール: 最新のスキーマ（receipt_url）でテーブルを作成
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

        println!("新規データベースを作成しました（receipt_urlスキーマ）");
    } else {
        // 既存インストール: 必要なカラムを安全に追加
        println!("既存のデータベースを確認中...");

        // receipt_urlカラムが存在するかチェック
        let has_receipt_url = check_column_exists(conn, "expenses", "receipt_url");

        if !has_receipt_url {
            println!("receipt_urlカラムを追加します...");
            // receipt_urlカラムを追加（エラーを無視）
            let _ = conn.execute("ALTER TABLE expenses ADD COLUMN receipt_url TEXT", []);
        }

        // receipt_pathカラムが存在する場合は警告を出力（削除はしない）
        let has_receipt_path = check_column_exists(conn, "expenses", "receipt_path");
        if has_receipt_path {
            println!("注意: 古いreceipt_pathカラムが検出されました。新しいreceipt_urlカラムを使用してください。");
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

    Ok(())
}

/// receipt_pathからreceipt_urlへのマイグレーションを実行する
///
/// # 引数
/// * `conn` - データベース接続
/// * `backup_path` - バックアップファイルのパス
///
/// # 戻り値
/// マイグレーション結果
pub fn migrate_receipt_path_to_url(
    conn: &Connection,
    backup_path: &str,
) -> Result<MigrationResult, MigrationError> {
    // 1. バックアップを作成
    if let Err(e) = create_backup(conn, backup_path) {
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
                    backup_path: Some(backup_path.to_string()),
                });
            }

            // 4. コミット
            tx.commit()?;

            Ok(MigrationResult {
                success: true,
                message: "receipt_pathからreceipt_urlへのマイグレーションが完了しました"
                    .to_string(),
                backup_path: Some(backup_path.to_string()),
            })
        }
        Err(e) => {
            // エラー時はロールバック
            tx.rollback()?;
            Ok(MigrationResult {
                success: false,
                message: format!("マイグレーション実行に失敗しました: {e}"),
                backup_path: Some(backup_path.to_string()),
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
fn create_backup(conn: &Connection, backup_path: &str) -> Result<(), MigrationError> {
    let mut backup_conn = Connection::open(backup_path)?;

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
fn validate_migration(tx: &Transaction) -> Result<(), MigrationError> {
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
        return Err(MigrationError::ValidationFailed(
            "receipt_urlカラムが見つかりません".to_string(),
        ));
    }

    // receipt_pathカラムが存在しないことを確認
    let has_receipt_path = table_info.iter().any(|(name, _)| name == "receipt_path");

    if has_receipt_path {
        return Err(MigrationError::ValidationFailed(
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
        return Err(MigrationError::ValidationFailed(
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
        return Err(MigrationError::ValidationFailed(
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
/// 成功時はOk(())、失敗時はエラー
pub fn restore_from_backup(conn: &mut Connection, backup_path: &str) -> Result<(), MigrationError> {
    // バックアップファイルが存在することを確認
    if !std::path::Path::new(backup_path).exists() {
        return Err(MigrationError::RollbackFailed(
            "バックアップファイルが見つかりません".to_string(),
        ));
    }

    let backup_conn = Connection::open(backup_path)?;

    // バックアップから復元
    let backup = rusqlite::backup::Backup::new(&backup_conn, conn)?;
    backup.run_to_completion(5, std::time::Duration::from_millis(250), None)?;

    Ok(())
}

/// マイグレーション状態をチェックする
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// receipt_urlマイグレーションが完了している場合はtrue
pub fn is_receipt_url_migration_complete(conn: &Connection) -> Result<bool> {
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
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    /// テスト用のデータベースを作成する
    fn create_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

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
        let temp_file = NamedTempFile::new().unwrap();
        let backup_path = temp_file.path().to_str().unwrap();

        // マイグレーションを実行
        let result = migrate_receipt_path_to_url(&conn, backup_path).unwrap();

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
        let mut conn = Connection::open_in_memory().unwrap();
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
        restore_from_backup(&mut conn, backup_path).unwrap();

        // データが復元されていることを確認
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test_table", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_https_url_constraint() {
        let conn = create_test_db();
        let temp_file = NamedTempFile::new().unwrap();
        let backup_path = temp_file.path().to_str().unwrap();

        // マイグレーションを実行して新しいスキーマを作成
        let _result = migrate_receipt_path_to_url(&conn, backup_path).unwrap();

        // HTTPS URLは挿入できることを確認
        let result = conn.execute(
            "INSERT INTO expenses (date, amount, category, description, receipt_url, created_at, updated_at)
             VALUES ('2024-01-01', 1000.0, 'テスト', 'テスト経費', 'https://example.com/receipt.jpg', '2024-01-01T00:00:00+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        );
        assert!(result.is_ok());

        // HTTP URLは制約違反でエラーになることを確認
        let result = conn.execute(
            "INSERT INTO expenses (date, amount, category, description, receipt_url, created_at, updated_at)
             VALUES ('2024-01-02', 2000.0, 'テスト', 'テスト経費2', 'http://example.com/receipt.jpg', '2024-01-02T00:00:00+09:00', '2024-01-02T00:00:00+09:00')",
            [],
        );
        assert!(result.is_err());

        // NULLは許可されることを確認
        let result = conn.execute(
            "INSERT INTO expenses (date, amount, category, description, receipt_url, created_at, updated_at)
             VALUES ('2024-01-03', 3000.0, 'テスト', 'テスト経費3', NULL, '2024-01-03T00:00:00+09:00', '2024-01-03T00:00:00+09:00')",
            [],
        );
        assert!(result.is_ok());
    }
}

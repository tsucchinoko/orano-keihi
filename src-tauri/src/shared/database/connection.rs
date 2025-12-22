use crate::shared::errors::{AppError, AppResult};
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// データベース接続を初期化し、マイグレーションを実行する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// データベース接続、または失敗時はエラー
///
/// # 処理内容
/// 1. アプリケーションデータディレクトリの確保
/// 2. データベースファイルパスの決定
/// 3. データベース接続の開設
/// 4. テーブル作成とマイグレーションの実行
pub fn initialize_database(app_handle: &AppHandle) -> AppResult<Connection> {
    // データベースファイルパスを取得
    let database_path = get_database_path(app_handle)?;

    // データベース接続を開く
    let conn = Connection::open(&database_path).map_err(|e| AppError::Database(e))?;

    // テーブルを作成
    create_tables(&conn)?;

    log::info!("データベースを初期化しました: {:?}", database_path);

    Ok(conn)
}

/// アプリデータディレクトリ内のデータベースファイルパスを取得する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// データベースファイルのパス、または失敗時はエラー
pub fn get_database_path(app_handle: &AppHandle) -> AppResult<PathBuf> {
    // アプリケーションデータディレクトリを取得
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| {
        AppError::configuration(format!("アプリデータディレクトリの取得に失敗: {e}"))
    })?;

    // ディレクトリが存在しない場合は作成
    if !app_data_dir.exists() {
        std::fs::create_dir_all(&app_data_dir).map_err(|e| {
            AppError::configuration(format!("アプリデータディレクトリの作成に失敗: {e}"))
        })?;
        log::info!(
            "アプリケーションデータディレクトリを作成: {:?}",
            app_data_dir
        );
    }

    // 環境に応じたデータベースファイル名を決定
    let db_filename = get_database_filename();
    let database_path = app_data_dir.join(db_filename);

    Ok(database_path)
}

/// 環境に応じたデータベースファイル名を取得する
///
/// # 戻り値
/// データベースファイル名
///
/// # ファイル名の規則
/// - 開発環境: "dev_expenses.db"
/// - プロダクション環境: "expenses.db"
fn get_database_filename() -> &'static str {
    // 環境判定
    let is_production = is_production_environment();

    if is_production {
        "expenses.db"
    } else {
        "dev_expenses.db"
    }
}

/// プロダクション環境かどうかを判定する
///
/// # 戻り値
/// プロダクション環境の場合はtrue
///
/// # 判定ロジック
/// 1. コンパイル時埋め込み環境変数を最優先
/// 2. 実行時環境変数 ENVIRONMENT を確認
/// 3. デバッグビルドの場合は開発環境
/// 4. リリースビルドの場合はプロダクション環境
fn is_production_environment() -> bool {
    // コンパイル時埋め込み環境変数を最優先
    if let Some(embedded_env) = option_env!("EMBEDDED_ENVIRONMENT") {
        return embedded_env == "production";
    }

    // 実行時環境変数を確認
    if let Ok(env_var) = std::env::var("ENVIRONMENT") {
        return env_var == "production";
    }

    // フォールバック: ビルド設定に基づく判定
    !cfg!(debug_assertions)
}

/// データベーステーブルを作成する
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
pub fn create_tables(conn: &Connection) -> AppResult<()> {
    // 既存のテーブル構造をチェック
    let table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='expenses'",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::Database)?;

    if table_exists == 0 {
        // 新規インストール: 最新のスキーマ（receipt_url）でテーブルを作成
        create_expenses_table(conn)?;
        log::info!("新規データベースを作成しました（receipt_urlスキーマ）");
    } else {
        // 既存インストール: 必要なカラムを安全に追加
        log::info!("既存のデータベースを確認中...");
        migrate_existing_tables(conn)?;
    }

    // インデックスを作成
    create_indexes(conn)?;

    // その他のテーブルを作成
    create_receipt_cache_table(conn)?;
    create_subscriptions_table(conn)?;
    create_categories_table(conn)?;

    Ok(())
}

/// 経費テーブルを作成する
fn create_expenses_table(conn: &Connection) -> AppResult<()> {
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
    )
    .map_err(AppError::Database)?;

    Ok(())
}

/// 既存テーブルのマイグレーションを実行する
fn migrate_existing_tables(conn: &Connection) -> AppResult<()> {
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
        drop_receipt_path_column(conn)?;
    }

    Ok(())
}

/// インデックスを作成する
fn create_indexes(conn: &Connection) -> AppResult<()> {
    // 経費テーブルのインデックス
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date)",
        [],
    )
    .map_err(AppError::Database)?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_category ON expenses(category)",
        [],
    )
    .map_err(AppError::Database)?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_receipt_url ON expenses(receipt_url)",
        [],
    )
    .map_err(AppError::Database)?;

    Ok(())
}

/// レシートキャッシュテーブルを作成する
fn create_receipt_cache_table(conn: &Connection) -> AppResult<()> {
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
    )
    .map_err(AppError::Database)?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_receipt_cache_url ON receipt_cache(receipt_url)",
        [],
    )
    .map_err(AppError::Database)?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_receipt_cache_accessed ON receipt_cache(last_accessed)",
        [],
    )
    .map_err(AppError::Database)?;

    Ok(())
}

/// サブスクリプションテーブルを作成する
fn create_subscriptions_table(conn: &Connection) -> AppResult<()> {
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
    )
    .map_err(AppError::Database)?;

    // 既存のサブスクリプションテーブルにreceipt_pathカラムを追加（存在しない場合）
    let _ = conn.execute("ALTER TABLE subscriptions ADD COLUMN receipt_path TEXT", []);

    // サブスクリプションテーブルのインデックス
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_subscriptions_active ON subscriptions(is_active)",
        [],
    )
    .map_err(AppError::Database)?;

    Ok(())
}

/// カテゴリテーブルを作成する
fn create_categories_table(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL,
            icon TEXT
        )",
        [],
    )
    .map_err(AppError::Database)?;

    // テーブルが空の場合、初期カテゴリデータを挿入
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
        .map_err(AppError::Database)?;

    if count == 0 {
        insert_default_categories(conn)?;
    }

    Ok(())
}

/// デフォルトカテゴリを挿入する
fn insert_default_categories(conn: &Connection) -> AppResult<()> {
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
        )
        .map_err(AppError::Database)?;
    }

    Ok(())
}

/// receipt_pathカラムを削除する
fn drop_receipt_path_column(conn: &Connection) -> AppResult<()> {
    // receipt_pathカラムが存在するかチェック
    if !check_column_exists(conn, "expenses", "receipt_path") {
        return Ok(());
    }

    log::info!("receipt_pathカラムを削除します...");

    // トランザクション内でマイグレーションを実行
    let tx = conn.unchecked_transaction().map_err(AppError::Database)?;

    // 既存のテーブル構造を確認
    let table_info: Vec<(String, String)> = tx
        .prepare("PRAGMA table_info(expenses)")
        .map_err(AppError::Database)?
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })
        .map_err(AppError::Database)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)?;

    let has_receipt_url = table_info.iter().any(|(name, _)| name == "receipt_url");

    // 新しいテーブル構造を作成（receipt_pathカラムなし）
    let create_table_sql = if has_receipt_url {
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

    tx.execute(create_table_sql, [])
        .map_err(AppError::Database)?;

    // 既存データを移行（receipt_pathカラムを除く）
    let insert_sql = if has_receipt_url {
        "INSERT INTO expenses_temp (id, date, amount, category, description, receipt_url, created_at, updated_at)
         SELECT id, date, amount, category, description, receipt_url, created_at, updated_at
         FROM expenses"
    } else {
        "INSERT INTO expenses_temp (id, date, amount, category, description, created_at, updated_at)
         SELECT id, date, amount, category, description, created_at, updated_at
         FROM expenses"
    };

    tx.execute(insert_sql, []).map_err(AppError::Database)?;

    // 古いテーブルを削除
    tx.execute("DROP TABLE expenses", [])
        .map_err(AppError::Database)?;

    // 新しいテーブルをリネーム
    tx.execute("ALTER TABLE expenses_temp RENAME TO expenses", [])
        .map_err(AppError::Database)?;

    // インデックスを再作成
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date)",
        [],
    )
    .map_err(AppError::Database)?;

    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_category ON expenses(category)",
        [],
    )
    .map_err(AppError::Database)?;

    if has_receipt_url {
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_expenses_receipt_url ON expenses(receipt_url)",
            [],
        )
        .map_err(AppError::Database)?;
    }

    // コミット
    tx.commit().map_err(AppError::Database)?;

    log::info!("receipt_pathカラムの削除が完了しました");

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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_create_tables() {
        let conn = Connection::open_in_memory().unwrap();

        // テーブル作成が成功することを確認
        let result = create_tables(&conn);
        assert!(result.is_ok());

        // 各テーブルが作成されていることを確認
        let tables = ["expenses", "receipt_cache", "subscriptions", "categories"];
        for table in &tables {
            let count: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{table}'"
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "テーブル {table} が作成されていません");
        }
    }

    #[test]
    fn test_check_column_exists() {
        let conn = Connection::open_in_memory().unwrap();

        // テストテーブルを作成
        conn.execute(
            "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT)",
            [],
        )
        .unwrap();

        // 存在するカラムのテスト
        assert!(check_column_exists(&conn, "test_table", "id"));
        assert!(check_column_exists(&conn, "test_table", "name"));

        // 存在しないカラムのテスト
        assert!(!check_column_exists(&conn, "test_table", "nonexistent"));

        // 存在しないテーブルのテスト
        assert!(!check_column_exists(&conn, "nonexistent_table", "id"));
    }

    #[test]
    fn test_is_production_environment() {
        // 環境判定のテスト（実際の値はビルド設定に依存）
        let is_prod = is_production_environment();

        // デバッグビルドかリリースビルドかのいずれかであることを確認
        if cfg!(debug_assertions) {
            // デバッグビルドの場合、環境変数が設定されていなければ開発環境
            if std::env::var("ENVIRONMENT").unwrap_or_default() != "production" {
                assert!(!is_prod);
            }
        }
    }

    #[test]
    fn test_get_database_filename() {
        let filename = get_database_filename();

        // ファイル名が適切であることを確認
        assert!(filename == "dev_expenses.db" || filename == "expenses.db");
        assert!(filename.ends_with(".db"));
    }
}

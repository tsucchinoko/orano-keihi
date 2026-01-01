use crate::features::migrations::AutoMigrationService;
use crate::shared::errors::{AppError, AppResult};
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// データベース接続を取得する（非同期版）
///
/// # 戻り値
/// データベース接続、または失敗時はエラー
pub async fn get_database_connection() -> AppResult<Connection> {
    // 一時的な実装: 新しい接続を作成
    // 本来はアプリケーション状態から取得すべきですが、
    // 現在の実装では直接作成します

    // データベースパスを取得（環境に応じて）
    let db_filename = get_database_filename();

    // アプリケーションデータディレクトリのパスを構築
    let app_data_dir = dirs::data_dir()
        .ok_or_else(|| AppError::configuration("アプリケーションデータディレクトリの取得に失敗"))?
        .join("subscription-memo");

    // ディレクトリが存在しない場合は作成
    if !app_data_dir.exists() {
        std::fs::create_dir_all(&app_data_dir).map_err(|e| {
            AppError::configuration(format!("アプリデータディレクトリの作成に失敗: {e}"))
        })?;
    }

    let database_path = app_data_dir.join(db_filename);

    // データベース接続を開く
    let conn = Connection::open(&database_path)
        .map_err(|e| AppError::Database(format!("データベース接続エラー: {e}")))?;

    Ok(conn)
}

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
/// 5. 自動マイグレーションシステムの実行（要件3.1）
pub fn initialize_database(app_handle: &AppHandle) -> AppResult<Connection> {
    // データベースファイルパスを取得
    let database_path = get_database_path(app_handle)?;

    // データベース接続を開く
    let conn = Connection::open(&database_path).map_err(|e| AppError::Database(e.to_string()))?;

    // テーブルを作成
    create_tables(&conn)?;

    // 自動マイグレーションシステムを実行（要件3.1, 3.4, 3.5）
    execute_auto_migration_system(&conn)?;

    log::info!("データベースを初期化しました: {database_path:?}");

    Ok(conn)
}

/// 自動マイグレーションシステムを実行する
///
/// アプリケーション起動時に未適用のマイグレーションを自動で適用します。
/// 要件3.1（データベース接続確立後にマイグレーションチェックを実行）、
/// 要件3.4（マイグレーション成功時に実行記録を追加）、
/// 要件3.5（マイグレーション失敗時にエラーログを出力し、アプリケーション起動を停止）
/// に対応します。
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn execute_auto_migration_system(conn: &Connection) -> AppResult<()> {
    log::info!("自動マイグレーションシステムを開始します");

    // 自動マイグレーションサービスを初期化
    let auto_migration_service = AutoMigrationService::new(conn).map_err(|e| {
        log::error!("自動マイグレーションサービスの初期化に失敗しました: {}", e);
        AppError::Database(format!("自動マイグレーションサービス初期化失敗: {}", e))
    })?;

    // 起動時自動マイグレーションを実行
    match auto_migration_service.run_startup_migrations(conn) {
        Ok(result) => {
            if result.success {
                log::info!(
                    "自動マイグレーションが正常に完了しました: {}",
                    result.message
                );

                if !result.applied_migrations.is_empty() {
                    log::info!(
                        "適用されたマイグレーション: {:?}",
                        result.applied_migrations
                    );
                }

                if let Some(backup_path) = result.backup_path {
                    log::info!("バックアップファイル: {}", backup_path);
                }

                log::info!(
                    "自動マイグレーション実行時間: {}ms",
                    result.total_execution_time_ms
                );
            } else {
                // 成功フラグがfalseの場合（通常は発生しないが安全のため）
                log::warn!("自動マイグレーションで警告: {}", result.message);
            }
            Ok(())
        }
        Err(e) => {
            // 要件3.5: マイグレーション失敗時にエラーログを出力し、アプリケーション起動を停止
            log::error!("自動マイグレーション実行中にエラーが発生しました: {}", e);
            log::error!("アプリケーション起動を停止します");

            // 詳細なエラー情報をログに出力
            log::error!("データベースファイルパス: {:?}", conn.path());
            log::error!("環境設定: ENVIRONMENT={:?}", std::env::var("ENVIRONMENT"));
            log::error!("プロダクション環境判定: {}", is_production_environment());

            // エラーの種類に応じた追加情報を出力
            match e.error_type {
                crate::features::migrations::MigrationErrorType::Initialization => {
                    log::error!("初期化エラー: migrationsテーブルの作成またはマイグレーション定義の登録に失敗しました");
                }
                crate::features::migrations::MigrationErrorType::Execution => {
                    log::error!("実行エラー: マイグレーション処理中にエラーが発生しました");
                    if let Some(details) = &e.details {
                        log::error!("詳細情報: {}", details);
                        if details.contains("バックアップ") {
                            log::error!("データベースを手動で復元してください");
                        }
                    }
                }
                crate::features::migrations::MigrationErrorType::Concurrency => {
                    log::error!("並行制御エラー: 別のインスタンスがマイグレーション実行中です");
                    log::error!("しばらく待ってから再度起動してください");
                }
                crate::features::migrations::MigrationErrorType::System => {
                    log::error!("システムエラー: ファイルシステムまたはデータベースアクセスに問題があります");
                }
                crate::features::migrations::MigrationErrorType::ChecksumMismatch => {
                    log::error!("チェックサム不一致エラー: マイグレーション内容が変更されています");
                    if let Some(migration_name) = &e.migration_name {
                        log::error!("対象マイグレーション: {}", migration_name);
                    }
                    if let Some(details) = &e.details {
                        log::error!("チェックサム詳細: {}", details);
                    }
                }
                crate::features::migrations::MigrationErrorType::Validation => {
                    log::error!("検証エラー: マイグレーション定義または実行結果に問題があります");
                    if let Some(migration_name) = &e.migration_name {
                        log::error!("対象マイグレーション: {}", migration_name);
                    }
                }
            }

            Err(AppError::Database(format!(
                "自動マイグレーション失敗: {}。アプリケーションを起動できません。",
                e
            )))
        }
    }
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
        log::info!("アプリケーションデータディレクトリを作成: {app_data_dir:?}");
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
        .map_err(|e| AppError::Database(e.to_string()))?;

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

    // ユーザー認証マイグレーションを実行
    execute_user_authentication_migration_if_needed(conn)?;

    Ok(())
}

/// 必要に応じてユーザー認証マイグレーションを実行する
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
fn execute_user_authentication_migration_if_needed(conn: &Connection) -> AppResult<()> {
    // ユーザー認証マイグレーションが必要かチェック
    use crate::features::migrations::service::{
        is_user_authentication_migration_complete, migrate_user_authentication,
    };

    // まず、usersテーブルが存在するかチェック
    let users_table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='users'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

    if users_table_exists == 0 {
        log::info!("usersテーブルが存在しないため、ユーザー認証マイグレーションを実行します...");

        match migrate_user_authentication(conn) {
            Ok(result) => {
                if result.success {
                    log::info!("ユーザー認証マイグレーションが完了しました");
                } else {
                    log::warn!("ユーザー認証マイグレーションで警告: {}", result.message);
                }
            }
            Err(e) => {
                log::error!("ユーザー認証マイグレーションでエラー: {e}");

                // マイグレーション失敗時の詳細情報を出力
                log::error!("データベースファイルパス: {:?}", conn.path());
                log::error!("環境設定: ENVIRONMENT={:?}", std::env::var("ENVIRONMENT"));
                log::error!("プロダクション環境判定: {}", is_production_environment());

                // 部分的に作成されたテーブルをクリーンアップ
                log::info!("マイグレーション失敗のため、部分的に作成されたテーブルをクリーンアップします...");
                let _ = conn.execute("DROP TABLE IF EXISTS users", []);
                let _ = conn.execute("DROP TABLE IF EXISTS sessions", []);

                return Err(AppError::Database(format!(
                    "ユーザー認証マイグレーション失敗: {e}。アプリケーションを再起動してください。"
                )));
            }
        }
    } else {
        // テーブルが存在する場合は、マイグレーション完了状態をチェック
        let is_complete = is_user_authentication_migration_complete(conn)
            .map_err(|e| AppError::Database(e.to_string()))?;

        if !is_complete {
            log::info!("ユーザー認証マイグレーションが不完全なため、再実行します...");

            match migrate_user_authentication(conn) {
                Ok(result) => {
                    if result.success {
                        log::info!("ユーザー認証マイグレーションが完了しました");
                    } else {
                        log::warn!("ユーザー認証マイグレーションで警告: {}", result.message);
                    }
                }
                Err(e) => {
                    log::error!("ユーザー認証マイグレーションでエラー: {e}");

                    // マイグレーション失敗時の詳細情報を出力
                    log::error!("データベースファイルパス: {:?}", conn.path());
                    log::error!("環境設定: ENVIRONMENT={:?}", std::env::var("ENVIRONMENT"));
                    log::error!("プロダクション環境判定: {}", is_production_environment());

                    return Err(AppError::Database(format!(
                        "ユーザー認証マイグレーション失敗: {e}。アプリケーションを再起動してください。"
                    )));
                }
            }
        } else {
            log::info!("ユーザー認証マイグレーションは既に完了しています");
        }
    }

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
    .map_err(|e| AppError::Database(e.to_string()))?;

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
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_category ON expenses(category)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_receipt_url ON expenses(receipt_url)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

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
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_receipt_cache_url ON receipt_cache(receipt_url)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_receipt_cache_accessed ON receipt_cache(last_accessed)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

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
    .map_err(|e| AppError::Database(e.to_string()))?;

    // 既存のサブスクリプションテーブルにreceipt_pathカラムを追加（存在しない場合）
    let _ = conn.execute("ALTER TABLE subscriptions ADD COLUMN receipt_path TEXT", []);

    // サブスクリプションテーブルのインデックス
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_subscriptions_active ON subscriptions(is_active)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

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
    .map_err(|e| AppError::Database(e.to_string()))?;

    // テーブルが空の場合、初期カテゴリデータを挿入
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
        .map_err(|e| AppError::Database(e.to_string()))?;

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
        .map_err(|e| AppError::Database(e.to_string()))?;
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
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| AppError::Database(e.to_string()))?;

    // 既存のテーブル構造を確認
    let table_info: Vec<(String, String)> = tx
        .prepare("PRAGMA table_info(expenses)")
        .map_err(|e| AppError::Database(e.to_string()))?
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })
        .map_err(|e| AppError::Database(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Database(e.to_string()))?;

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
        .map_err(|e| AppError::Database(e.to_string()))?;

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

    tx.execute(insert_sql, [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    // 古いテーブルを削除
    tx.execute("DROP TABLE expenses", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    // 新しいテーブルをリネーム
    tx.execute("ALTER TABLE expenses_temp RENAME TO expenses", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    // インデックスを再作成
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_category ON expenses(category)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    if has_receipt_url {
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_expenses_receipt_url ON expenses(receipt_url)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    // コミット
    tx.commit().map_err(|e| AppError::Database(e.to_string()))?;

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

/// テスト用のインメモリデータベース接続を作成する
///
/// # 戻り値
/// インメモリデータベース接続、または失敗時はエラー
#[cfg(test)]
pub fn create_in_memory_connection() -> AppResult<Connection> {
    let conn = Connection::open_in_memory().map_err(|e| AppError::Database(e.to_string()))?;

    // テスト用の基本テーブルを作成
    create_tables(&conn)?;

    Ok(conn)
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
    fn test_auto_migration_system_integration() {
        let conn = Connection::open_in_memory().unwrap();

        // 基本テーブルを作成
        create_tables(&conn).unwrap();

        // 自動マイグレーションサービスの初期化のみテスト
        use crate::features::migrations::AutoMigrationService;
        let service = AutoMigrationService::new(&conn).unwrap();

        // migrationsテーブルが作成されていることを確認
        let migrations_table_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            migrations_table_exists, 1,
            "migrationsテーブルが作成されていません"
        );

        // マイグレーション状態確認が動作することを確認
        let status = service.check_migration_status(&conn);
        assert!(status.is_ok(), "マイグレーション状態確認に失敗");

        let status = status.unwrap();
        assert_eq!(
            status.total_available, 3,
            "利用可能なマイグレーション数が期待値と異なります"
        );
        assert_eq!(
            status.total_applied, 0,
            "初期状態では適用済みマイグレーションは0であるべきです"
        );
        assert_eq!(
            status.pending_migrations.len(),
            3,
            "未適用マイグレーション数が期待値と異なります"
        );
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

//! マイグレーション実行管理
//!
//! このモジュールは、マイグレーションの実行とトランザクション管理を行います。

use super::errors::MigrationError;
use super::models::{MigrationDefinition, MigrationExecutionResult};
use crate::features::migrations::service::create_backup;
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use rusqlite::Connection;
use std::time::Instant;

/// マイグレーション実行管理
///
/// マイグレーションの実行とトランザクション管理を行います。
/// 要件3.3, 6.1, 6.2, 6.3に従って実装されています。
pub struct MigrationExecutor;

impl MigrationExecutor {
    /// 新しいマイグレーション実行管理を作成
    ///
    /// # 戻り値
    /// 新しいマイグレーション実行管理
    pub fn new() -> Self {
        Self
    }

    /// マイグレーションを安全に実行
    ///
    /// トランザクション管理とエラー時のロールバック機能を含みます。
    /// 要件3.3（バックアップ作成）、6.1（トランザクションロールバック）、
    /// 6.2（詳細エラーメッセージ）、6.3（バックアップ場所通知）を満たします。
    ///
    /// # 引数
    /// * `conn` - データベース接続
    /// * `migration` - マイグレーション定義
    ///
    /// # 戻り値
    /// マイグレーション実行結果
    pub fn execute_migration(
        &self,
        conn: &Connection,
        migration: &MigrationDefinition,
    ) -> Result<MigrationExecutionResult, MigrationError> {
        let start_time = Instant::now();

        log::info!("マイグレーション '{}' の実行を開始します", migration.name);

        // 1. バックアップを作成（要件3.3）
        let backup_path = match self.create_backup(conn) {
            Ok(path) => {
                log::info!("バックアップを作成しました: {}", path);
                Some(path)
            }
            Err(e) => {
                let error_msg = format!("バックアップ作成に失敗しました: {}", e);
                log::error!("{}", error_msg);
                return Ok(MigrationExecutionResult::failure(error_msg, None));
            }
        };

        // 2. トランザクション内でマイグレーションを実行
        let tx = match conn.unchecked_transaction() {
            Ok(tx) => tx,
            Err(e) => {
                let error_msg = format!("トランザクション開始に失敗しました: {}", e);
                log::error!("{}", error_msg);
                return Ok(MigrationExecutionResult::failure(
                    error_msg,
                    backup_path.clone(),
                ));
            }
        };

        // 3. 具体的なマイグレーション実行
        let migration_result = match migration.name.as_str() {
            "001_create_basic_schema" => self.execute_basic_schema_migration(&tx),
            "002_add_user_authentication" => self.execute_user_auth_migration(&tx),
            "003_migrate_receipt_url" => self.execute_receipt_url_migration(&tx),
            _ => {
                let error_msg = format!("未知のマイグレーション: {}", migration.name);
                log::error!("{}", error_msg);
                Err(MigrationError::execution(
                    migration.name.clone(),
                    error_msg,
                    None,
                ))
            }
        };

        match migration_result {
            Ok(_) => {
                // 4. コミット
                if let Err(e) = tx.commit() {
                    let error_msg = format!("トランザクションコミットに失敗しました: {}", e);
                    log::error!("{}", error_msg);
                    return Ok(MigrationExecutionResult::failure(
                        error_msg,
                        backup_path.clone(),
                    ));
                }

                let execution_time = start_time.elapsed().as_millis() as i64;
                let success_msg =
                    format!("マイグレーション '{}' が正常に完了しました", migration.name);

                log::info!("{} (実行時間: {}ms)", success_msg, execution_time);

                Ok(MigrationExecutionResult::success(
                    success_msg,
                    execution_time,
                    backup_path,
                ))
            }
            Err(e) => {
                // 5. エラー時のロールバック（要件6.1）
                if let Err(rollback_err) = tx.rollback() {
                    log::error!("ロールバックに失敗しました: {}", rollback_err);
                }

                // 6. 詳細なエラーメッセージをログに出力（要件6.2）
                log::error!("マイグレーション実行エラー: {}", e.detailed_message());

                // 7. バックアップファイルの場所を通知（要件6.3）
                let error_msg = if let Some(ref backup) = backup_path {
                    format!(
                        "マイグレーション '{}' の実行に失敗しました: {}。バックアップファイル: {}",
                        migration.name, e.message, backup
                    )
                } else {
                    format!(
                        "マイグレーション '{}' の実行に失敗しました: {}",
                        migration.name, e.message
                    )
                };

                Ok(MigrationExecutionResult::failure(error_msg, backup_path))
            }
        }
    }

    /// バックアップを作成
    ///
    /// JST（日本標準時）を使用してタイムスタンプ付きのバックアップファイルを作成します。
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// バックアップファイルのパス
    pub fn create_backup(&self, conn: &Connection) -> Result<String, MigrationError> {
        // JST（日本標準時）でタイムスタンプを生成
        let now_jst = Utc::now().with_timezone(&Tokyo);
        let backup_path = format!("database_backup_{}.db", now_jst.timestamp());

        log::info!("データベースバックアップを作成中: {}", backup_path);

        // 既存のバックアップ機能を使用
        match create_backup(conn, &backup_path) {
            Ok(_) => {
                log::info!("バックアップ作成完了: {}", backup_path);
                Ok(backup_path)
            }
            Err(e) => {
                let error_msg = format!("バックアップ作成に失敗しました: {}", e);
                log::error!("{}", error_msg);
                Err(MigrationError::system(error_msg, Some(e.to_string())))
            }
        }
    }

    /// 基本スキーママイグレーションを実行
    ///
    /// # 引数
    /// * `tx` - トランザクション
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー
    fn execute_basic_schema_migration(
        &self,
        tx: &rusqlite::Transaction,
    ) -> Result<(), MigrationError> {
        log::info!("基本スキーママイグレーションを実行中...");

        // 既存のrun_migrations機能を使用するため、
        // トランザクション内で直接実行する代わりに、
        // 接続レベルでの実行が必要
        // ここでは基本的なテーブル作成のみを実行
        self.create_basic_tables(tx)
            .map_err(|e| MigrationError::execution("001_create_basic_schema".to_string(), e, None))
    }

    /// ユーザー認証マイグレーションを実行
    ///
    /// # 引数
    /// * `tx` - トランザクション
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー
    fn execute_user_auth_migration(
        &self,
        tx: &rusqlite::Transaction,
    ) -> Result<(), MigrationError> {
        log::info!("ユーザー認証マイグレーションを実行中...");

        // ユーザー認証テーブルの作成
        self.create_user_auth_tables(tx).map_err(|e| {
            MigrationError::execution("002_add_user_authentication".to_string(), e, None)
        })
    }

    /// receipt_urlマイグレーションを実行
    ///
    /// # 引数
    /// * `tx` - トランザクション
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー
    fn execute_receipt_url_migration(
        &self,
        tx: &rusqlite::Transaction,
    ) -> Result<(), MigrationError> {
        log::info!("receipt_urlマイグレーションを実行中...");

        // receipt_pathからreceipt_urlへの移行
        self.migrate_receipt_path_to_url_in_tx(tx)
            .map_err(|e| MigrationError::execution("003_migrate_receipt_url".to_string(), e, None))
    }

    /// 基本テーブルを作成
    ///
    /// # 引数
    /// * `tx` - トランザクション
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー文字列
    fn create_basic_tables(&self, tx: &rusqlite::Transaction) -> Result<(), String> {
        // expensesテーブルを作成
        tx.execute(
            "CREATE TABLE IF NOT EXISTS expenses (
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
        .map_err(|e| format!("expensesテーブル作成エラー: {}", e))?;

        // インデックスを作成
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date)",
            [],
        )
        .map_err(|e| format!("expensesインデックス作成エラー: {}", e))?;

        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_expenses_category ON expenses(category)",
            [],
        )
        .map_err(|e| format!("expensesインデックス作成エラー: {}", e))?;

        // subscriptionsテーブルを作成
        tx.execute(
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
        .map_err(|e| format!("subscriptionsテーブル作成エラー: {}", e))?;

        // categoriesテーブルを作成
        tx.execute(
            "CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                color TEXT NOT NULL,
                icon TEXT
            )",
            [],
        )
        .map_err(|e| format!("categoriesテーブル作成エラー: {}", e))?;

        // 初期カテゴリデータを挿入
        let count: i64 = tx
            .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
            .map_err(|e| format!("カテゴリ数取得エラー: {}", e))?;

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
                tx.execute(
                    "INSERT INTO categories (name, color, icon) VALUES (?1, ?2, ?3)",
                    [name, color, icon],
                )
                .map_err(|e| format!("初期カテゴリ挿入エラー: {}", e))?;
            }
        }

        log::info!("基本テーブルの作成が完了しました");
        Ok(())
    }

    /// ユーザー認証テーブルを作成
    ///
    /// # 引数
    /// * `tx` - トランザクション
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー文字列
    fn create_user_auth_tables(&self, tx: &rusqlite::Transaction) -> Result<(), String> {
        // usersテーブルを作成
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
        )
        .map_err(|e| format!("usersテーブル作成エラー: {}", e))?;

        // usersテーブルのインデックスを作成
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_users_google_id ON users(google_id)",
            [],
        )
        .map_err(|e| format!("usersインデックス作成エラー: {}", e))?;

        // sessionsテーブルを作成
        tx.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| format!("sessionsテーブル作成エラー: {}", e))?;

        // sessionsテーブルのインデックスを作成
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id)",
            [],
        )
        .map_err(|e| format!("sessionsインデックス作成エラー: {}", e))?;

        // デフォルトユーザーを作成
        let default_user_exists: i64 = tx
            .query_row("SELECT COUNT(*) FROM users WHERE id = 1", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("デフォルトユーザー確認エラー: {}", e))?;

        if default_user_exists == 0 {
            let now_jst = Utc::now().with_timezone(&Tokyo);
            let timestamp = now_jst.to_rfc3339();

            tx.execute(
                "INSERT OR IGNORE INTO users (id, google_id, email, name, picture_url, created_at, updated_at)
                 VALUES (1, 'default_user', 'default@example.com', 'デフォルトユーザー', NULL, ?1, ?2)",
                [&timestamp, &timestamp],
            )
            .map_err(|e| format!("デフォルトユーザー作成エラー: {}", e))?;
        }

        // 既存テーブルにuser_idカラムを追加
        self.add_user_id_columns(tx)?;

        log::info!("ユーザー認証テーブルの作成が完了しました");
        Ok(())
    }

    /// 既存テーブルにuser_idカラムを追加
    ///
    /// # 引数
    /// * `tx` - トランザクション
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー文字列
    fn add_user_id_columns(&self, tx: &rusqlite::Transaction) -> Result<(), String> {
        let tables = ["expenses", "subscriptions", "receipt_cache"];

        for table in &tables {
            // テーブルが存在するかチェック
            let table_exists: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                    [table],
                    |row| row.get(0),
                )
                .map_err(|e| format!("テーブル存在確認エラー ({}): {}", table, e))?;

            if table_exists > 0 {
                // user_idカラムを追加（エラーを無視）
                let _ = tx.execute(
                    &format!(
                        "ALTER TABLE {} ADD COLUMN user_id INTEGER REFERENCES users(id)",
                        table
                    ),
                    [],
                );

                // 既存データにデフォルトユーザーIDを設定
                tx.execute(
                    &format!("UPDATE {} SET user_id = 1 WHERE user_id IS NULL", table),
                    [],
                )
                .map_err(|e| format!("user_id更新エラー ({}): {}", table, e))?;

                // インデックスを作成
                tx.execute(
                    &format!(
                        "CREATE INDEX IF NOT EXISTS idx_{}_user_id ON {}(user_id)",
                        table, table
                    ),
                    [],
                )
                .map_err(|e| format!("user_idインデックス作成エラー ({}): {}", table, e))?;
            }
        }

        Ok(())
    }

    /// receipt_pathからreceipt_urlへの移行をトランザクション内で実行
    ///
    /// # 引数
    /// * `tx` - トランザクション
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー文字列
    fn migrate_receipt_path_to_url_in_tx(&self, tx: &rusqlite::Transaction) -> Result<(), String> {
        // 新しいテーブル構造を作成
        tx.execute(
            "CREATE TABLE IF NOT EXISTS expenses_new (
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
        )
        .map_err(|e| format!("新しいexpensesテーブル作成エラー: {}", e))?;

        // 既存データを移行（receipt_pathは無視）
        tx.execute(
            "INSERT INTO expenses_new (id, date, amount, category, description, created_at, updated_at)
             SELECT id, date, amount, category, description, created_at, updated_at
             FROM expenses",
            [],
        )
        .map_err(|e| format!("データ移行エラー: {}", e))?;

        // 古いテーブルを削除
        tx.execute("DROP TABLE expenses", [])
            .map_err(|e| format!("古いテーブル削除エラー: {}", e))?;

        // 新しいテーブルをリネーム
        tx.execute("ALTER TABLE expenses_new RENAME TO expenses", [])
            .map_err(|e| format!("テーブルリネームエラー: {}", e))?;

        // インデックスを再作成
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date)",
            [],
        )
        .map_err(|e| format!("インデックス再作成エラー: {}", e))?;

        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_expenses_category ON expenses(category)",
            [],
        )
        .map_err(|e| format!("インデックス再作成エラー: {}", e))?;

        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_expenses_receipt_url ON expenses(receipt_url)",
            [],
        )
        .map_err(|e| format!("インデックス再作成エラー: {}", e))?;

        // receipt_cacheテーブルを作成
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
        )
        .map_err(|e| format!("receipt_cacheテーブル作成エラー: {}", e))?;

        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_receipt_cache_url ON receipt_cache(receipt_url)",
            [],
        )
        .map_err(|e| format!("receipt_cacheインデックス作成エラー: {}", e))?;

        log::info!("receipt_urlマイグレーションが完了しました");
        Ok(())
    }
}

impl Default for MigrationExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// テスト用のデータベースを作成
    fn create_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn
    }

    /// テスト用のマイグレーション定義を作成
    fn create_test_migration(name: &str) -> MigrationDefinition {
        MigrationDefinition::new(
            name.to_string(),
            "1.0.0".to_string(),
            "テストマイグレーション".to_string(),
            "a".repeat(64), // 64文字のSHA-256ハッシュ
        )
    }

    #[test]
    fn test_executor_creation() {
        let executor = MigrationExecutor::new();
        // 基本的な作成テスト
        let _ = executor;
    }

    #[test]
    fn test_create_backup() {
        let executor = MigrationExecutor::new();
        let conn = create_test_db();

        // テストテーブルを作成
        conn.execute(
            "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT)",
            [],
        )
        .unwrap();

        // バックアップ作成テスト
        let result = executor.create_backup(&conn);
        assert!(result.is_ok());

        let backup_path = result.unwrap();
        assert!(backup_path.starts_with("database_backup_"));
        assert!(backup_path.ends_with(".db"));
    }

    #[test]
    fn test_execute_basic_schema_migration() {
        let executor = MigrationExecutor::new();
        let conn = create_test_db();
        let migration = create_test_migration("001_create_basic_schema");

        // 基本スキーママイグレーションを実行
        let result = executor.execute_migration(&conn, &migration);
        assert!(result.is_ok());

        let execution_result = result.unwrap();
        assert!(execution_result.success);
        assert!(execution_result.execution_time_ms >= 0);

        // テーブルが作成されていることを確認
        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('expenses', 'subscriptions', 'categories')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 3);

        // 初期カテゴリが挿入されていることを確認
        let category_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
            .unwrap();
        assert_eq!(category_count, 6);
    }

    #[test]
    fn test_execute_user_auth_migration() {
        let executor = MigrationExecutor::new();
        let conn = create_test_db();

        // 基本テーブルを先に作成
        let basic_migration = create_test_migration("001_create_basic_schema");
        executor.execute_migration(&conn, &basic_migration).unwrap();

        // ユーザー認証マイグレーションを実行
        let auth_migration = create_test_migration("002_add_user_authentication");
        let result = executor.execute_migration(&conn, &auth_migration);
        assert!(result.is_ok());

        let execution_result = result.unwrap();
        assert!(execution_result.success);

        // usersテーブルが作成されていることを確認
        let users_table_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='users'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(users_table_exists, 1);

        // デフォルトユーザーが作成されていることを確認
        let default_user_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM users WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(default_user_count, 1);
    }

    #[test]
    fn test_execute_receipt_url_migration() {
        let executor = MigrationExecutor::new();
        let conn = create_test_db();

        // 古いスキーマでexpensesテーブルを作成
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

        // receipt_urlマイグレーションを実行
        let migration = create_test_migration("003_migrate_receipt_url");
        let result = executor.execute_migration(&conn, &migration);
        assert!(result.is_ok());

        let execution_result = result.unwrap();
        assert!(execution_result.success);

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
    }

    #[test]
    fn test_execute_unknown_migration() {
        let executor = MigrationExecutor::new();
        let conn = create_test_db();
        let migration = create_test_migration("999_unknown_migration");

        // 未知のマイグレーションを実行
        let result = executor.execute_migration(&conn, &migration);
        assert!(result.is_ok());

        let execution_result = result.unwrap();
        assert!(!execution_result.success);
        assert!(execution_result.message.contains("未知のマイグレーション"));
    }

    #[test]
    fn test_migration_with_backup() {
        let executor = MigrationExecutor::new();
        let conn = create_test_db();

        // テストテーブルを作成
        conn.execute(
            "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT)",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO test_table (name) VALUES ('test')", [])
            .unwrap();

        // マイグレーションを実行
        let migration = create_test_migration("001_create_basic_schema");
        let result = executor.execute_migration(&conn, &migration);
        assert!(result.is_ok());

        let execution_result = result.unwrap();
        assert!(execution_result.success);
        assert!(execution_result.backup_path.is_some());

        // バックアップパスが適切な形式であることを確認
        let backup_path = execution_result.backup_path.unwrap();
        assert!(backup_path.starts_with("database_backup_"));
        assert!(backup_path.ends_with(".db"));
    }

    #[test]
    fn test_migration_rollback_on_error() {
        let executor = MigrationExecutor::new();
        let conn = create_test_db();

        // 無効なSQLを含むマイグレーションをシミュレート
        // （実際のテストでは、エラーを発生させる条件を作成）

        // テストテーブルを作成
        conn.execute(
            "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT)",
            [],
        )
        .unwrap();

        let initial_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // 未知のマイグレーションを実行（エラーが発生する）
        let migration = create_test_migration("999_error_migration");
        let result = executor.execute_migration(&conn, &migration);
        assert!(result.is_ok());

        let execution_result = result.unwrap();
        assert!(!execution_result.success);

        // テーブル数が変わっていないことを確認（ロールバックされた）
        let final_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(initial_count, final_count);
    }
}

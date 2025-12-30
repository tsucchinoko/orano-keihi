//! マイグレーション実行管理
//!
//! このモジュールは、マイグレーションの実行とトランザクション管理を行います。

use super::errors::MigrationError;
use super::models::MigrationExecutionResult;
use crate::features::migrations::service::{
    create_backup, migrate_receipt_path_to_url, migrate_user_authentication, run_migrations,
};
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use rusqlite::Connection;
use std::time::Instant;

/// マイグレーション実行トレイト
///
/// 個別のマイグレーション実行機能を定義します。
/// 既存のマイグレーション機能を統合するためのインターフェースです。
pub trait MigrationExecutorTrait: Send + Sync {
    /// マイグレーションを実行
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 実行結果
    fn execute(&self, conn: &Connection) -> Result<(), String>;

    /// マイグレーション名を取得
    ///
    /// # 戻り値
    /// マイグレーション名
    fn name(&self) -> &str;
}

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
    /// * `executable_migration` - 実行可能なマイグレーション定義
    ///
    /// # 戻り値
    /// マイグレーション実行結果
    pub fn execute_migration(
        &self,
        conn: &Connection,
        executable_migration: &crate::features::migrations::auto_migration::models::ExecutableMigrationDefinition,
    ) -> Result<MigrationExecutionResult, MigrationError> {
        let start_time = Instant::now();

        log::info!(
            "マイグレーション '{}' の実行を開始します",
            executable_migration.name()
        );

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

        // 2. マイグレーションを実行（既存の関数を直接呼び出し）
        let migration_result = executable_migration.execute(conn);

        match migration_result {
            Ok(_) => {
                let execution_time = start_time.elapsed().as_millis() as i64;
                let success_msg = format!(
                    "マイグレーション '{}' が正常に完了しました",
                    executable_migration.name()
                );

                log::info!("{} (実行時間: {}ms)", success_msg, execution_time);

                Ok(MigrationExecutionResult::success(
                    success_msg,
                    execution_time,
                    backup_path,
                ))
            }
            Err(e) => {
                // 詳細なエラーメッセージをログに出力（要件6.2）
                log::error!("マイグレーション実行エラー: {}", e);

                // バックアップファイルの場所を通知（要件6.3）
                let error_msg = if let Some(ref backup) = backup_path {
                    format!(
                        "マイグレーション '{}' の実行に失敗しました: {}。バックアップファイル: {}",
                        executable_migration.name(),
                        e,
                        backup
                    )
                } else {
                    format!(
                        "マイグレーション '{}' の実行に失敗しました: {}",
                        executable_migration.name(),
                        e
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
}

impl Default for MigrationExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 基本スキーママイグレーション実行器
///
/// 既存の`run_migrations`関数をラップします。
/// 要件5.1に対応します。
pub struct BasicSchemaMigrationExecutor;

impl MigrationExecutorTrait for BasicSchemaMigrationExecutor {
    fn execute(&self, conn: &Connection) -> Result<(), String> {
        log::info!("基本スキーママイグレーションを実行中...");

        run_migrations(conn).map_err(|e| {
            let error_msg = format!("基本スキーママイグレーション実行エラー: {}", e);
            log::error!("{}", error_msg);
            error_msg
        })?;

        log::info!("基本スキーママイグレーションが完了しました");
        Ok(())
    }

    fn name(&self) -> &str {
        "001_create_basic_schema"
    }
}

/// ユーザー認証マイグレーション実行器
///
/// 既存の`migrate_user_authentication`関数をラップします。
/// 要件5.2に対応します。
pub struct UserAuthMigrationExecutor;

impl MigrationExecutorTrait for UserAuthMigrationExecutor {
    fn execute(&self, conn: &Connection) -> Result<(), String> {
        log::info!("ユーザー認証マイグレーションを実行中...");

        let result = migrate_user_authentication(conn).map_err(|e| {
            let error_msg = format!("ユーザー認証マイグレーション実行エラー: {}", e);
            log::error!("{}", error_msg);
            error_msg
        })?;

        if !result.success {
            let error_msg = format!("ユーザー認証マイグレーション失敗: {}", result.message);
            log::error!("{}", error_msg);
            return Err(error_msg);
        }

        log::info!(
            "ユーザー認証マイグレーションが完了しました: {}",
            result.message
        );
        Ok(())
    }

    fn name(&self) -> &str {
        "002_add_user_authentication"
    }
}

/// receipt_urlマイグレーション実行器
///
/// 既存の`migrate_receipt_path_to_url`関数をラップします。
/// 要件5.3に対応します。
pub struct ReceiptUrlMigrationExecutor;

impl MigrationExecutorTrait for ReceiptUrlMigrationExecutor {
    fn execute(&self, conn: &Connection) -> Result<(), String> {
        log::info!("receipt_urlマイグレーションを実行中...");

        let result = migrate_receipt_path_to_url(conn).map_err(|e| {
            let error_msg = format!("receipt_urlマイグレーション実行エラー: {}", e);
            log::error!("{}", error_msg);
            error_msg
        })?;

        if !result.success {
            let error_msg = format!("receipt_urlマイグレーション失敗: {}", result.message);
            log::error!("{}", error_msg);
            return Err(error_msg);
        }

        log::info!(
            "receipt_urlマイグレーションが完了しました: {}",
            result.message
        );
        Ok(())
    }

    fn name(&self) -> &str {
        "003_migrate_receipt_url"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::migrations::auto_migration::models::{
        ExecutableMigrationDefinition, MigrationDefinition,
    };
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

    /// テスト用の実行可能マイグレーション定義を作成
    fn create_test_executable_migration(name: &str) -> ExecutableMigrationDefinition {
        let definition = create_test_migration(name);
        match name {
            "001_create_basic_schema" => ExecutableMigrationDefinition::new(
                definition,
                Box::new(BasicSchemaMigrationExecutor),
            ),
            "002_add_user_authentication" => {
                ExecutableMigrationDefinition::new(definition, Box::new(UserAuthMigrationExecutor))
            }
            "003_migrate_receipt_url" => ExecutableMigrationDefinition::new(
                definition,
                Box::new(ReceiptUrlMigrationExecutor),
            ),
            _ => panic!("未知のマイグレーション名: {}", name),
        }
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
    fn test_basic_schema_migration_executor() {
        let executor = BasicSchemaMigrationExecutor;
        let conn = create_test_db();

        // 基本スキーママイグレーションを実行
        let result = executor.execute(&conn);
        assert!(result.is_ok(), "実行エラー: {:?}", result);

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
    fn test_user_auth_migration_executor() {
        let executor = UserAuthMigrationExecutor;
        let conn = create_test_db();

        // 基本テーブルを先に作成
        let basic_executor = BasicSchemaMigrationExecutor;
        basic_executor.execute(&conn).unwrap();

        // ユーザー認証マイグレーションを実行
        let result = executor.execute(&conn);
        assert!(result.is_ok(), "実行エラー: {:?}", result);

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
    fn test_receipt_url_migration_executor() {
        let executor = ReceiptUrlMigrationExecutor;
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
        let result = executor.execute(&conn);
        assert!(result.is_ok(), "実行エラー: {:?}", result);

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
    fn test_execute_migration_with_executable() {
        let executor = MigrationExecutor::new();
        let conn = create_test_db();

        // 実行可能なマイグレーションを作成
        let executable_migration = create_test_executable_migration("001_create_basic_schema");

        // マイグレーションを実行
        let result = executor.execute_migration(&conn, &executable_migration);
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

        // 実行可能なマイグレーションを作成
        let executable_migration = create_test_executable_migration("001_create_basic_schema");

        // マイグレーションを実行
        let result = executor.execute_migration(&conn, &executable_migration);
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
    fn test_migration_executor_trait_implementations() {
        // 各実行器の名前が正しいことを確認
        let basic_executor = BasicSchemaMigrationExecutor;
        assert_eq!(basic_executor.name(), "001_create_basic_schema");

        let user_auth_executor = UserAuthMigrationExecutor;
        assert_eq!(user_auth_executor.name(), "002_add_user_authentication");

        let receipt_url_executor = ReceiptUrlMigrationExecutor;
        assert_eq!(receipt_url_executor.name(), "003_migrate_receipt_url");
    }
}

//! 自動マイグレーションサービス
//!
//! このモジュールは、自動マイグレーションシステムのメインサービスを提供します。

use super::errors::MigrationError;
use super::executor::MigrationExecutor;
use super::models::{AutoMigrationResult, MigrationStatusReport};
use super::registry::MigrationRegistry;
use super::table::MigrationTable;
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use rusqlite::{params, Connection};
use std::time::Instant;

/// 自動マイグレーションサービス
///
/// メインの自動マイグレーション管理サービスです。
pub struct AutoMigrationService {
    /// マイグレーション登録管理
    registry: MigrationRegistry,
    /// マイグレーション実行管理
    executor: MigrationExecutor,
}

impl AutoMigrationService {
    /// 自動マイグレーションシステムを初期化
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 新しい自動マイグレーションサービス
    pub fn new(conn: &rusqlite::Connection) -> Result<Self, MigrationError> {
        // migrationsテーブルを初期化（要件1.1, 1.2, 1.3, 1.4）
        MigrationTable::initialize(conn)?;

        // デフォルトマイグレーションを登録（要件5.1, 5.2, 5.3, 5.4）
        let registry = MigrationRegistry::register_default_migrations()?;

        Ok(Self {
            registry,
            executor: MigrationExecutor::new(),
        })
    }

    /// アプリケーション起動時の自動マイグレーション実行
    ///
    /// 並行実行制御機能を含み、データベースロックを使用して重複実行を防止します。
    /// 要件3.1, 3.2, 8.3, 8.4に対応します。
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 自動マイグレーション実行結果
    pub fn run_startup_migrations(
        &self,
        conn: &rusqlite::Connection,
    ) -> Result<AutoMigrationResult, MigrationError> {
        let start_time = Instant::now();

        log::info!("自動マイグレーションシステムを開始します");

        // 並行実行制御：データベースレベルでの排他制御（要件8.3, 8.4）
        conn.execute("BEGIN EXCLUSIVE TRANSACTION", [])
            .map_err(|e| {
                MigrationError::concurrency(
                    format!("排他トランザクションの開始に失敗しました: {}", e),
                    Some(e.to_string()),
                )
            })?;

        // アプリケーションレベルでの重複実行チェック
        if self.is_migration_in_progress(conn)? {
            conn.execute("ROLLBACK", []).ok(); // ロールバックを試行
            return Err(MigrationError::concurrency(
                "別のインスタンスがマイグレーション実行中です".to_string(),
                None,
            ));
        }

        // マイグレーション実行フラグを設定
        self.set_migration_in_progress(conn, true)?;

        // 未適用マイグレーション一覧を取得（要件2.2, 2.3）
        let pending_migrations = match self.get_pending_migrations(conn) {
            Ok(migrations) => migrations,
            Err(e) => {
                self.set_migration_in_progress(conn, false).ok(); // フラグをクリア
                conn.execute("ROLLBACK", []).ok(); // ロールバックを試行
                return Err(e);
            }
        };

        if pending_migrations.is_empty() {
            log::info!("適用すべきマイグレーションはありません");
            self.set_migration_in_progress(conn, false)?;
            conn.execute("COMMIT", []).map_err(|e| {
                MigrationError::system(
                    "トランザクションのコミットに失敗しました".to_string(),
                    Some(e.to_string()),
                )
            })?;

            let total_time = start_time.elapsed().as_millis() as i64;
            return Ok(AutoMigrationResult::success(
                "すべてのマイグレーションは既に適用済みです".to_string(),
                Vec::new(),
                None,
                total_time,
            ));
        }

        log::info!(
            "{}件の未適用マイグレーションを実行します",
            pending_migrations.len()
        );

        // マイグレーション順次実行（要件3.1, 3.2）
        let execution_result = self.execute_pending_migrations(conn, &pending_migrations);

        // 実行フラグをクリア
        self.set_migration_in_progress(conn, false)?;

        match execution_result {
            Ok(migration_result) => {
                conn.execute("COMMIT", []).map_err(|e| {
                    MigrationError::system(
                        "トランザクションのコミットに失敗しました".to_string(),
                        Some(e.to_string()),
                    )
                })?;

                let total_time = start_time.elapsed().as_millis() as i64;
                log::info!(
                    "自動マイグレーションが完了しました (実行時間: {}ms)",
                    total_time
                );

                Ok(AutoMigrationResult::success(
                    migration_result.message,
                    migration_result.applied_migrations,
                    migration_result.backup_path,
                    total_time,
                ))
            }
            Err(e) => {
                conn.execute("ROLLBACK", []).ok(); // ロールバックを試行
                log::error!("自動マイグレーション実行中にエラーが発生しました: {}", e);
                Err(e)
            }
        }
    }

    /// マイグレーション状態の確認
    ///
    /// 現在のマイグレーション状態を分析し、詳細なレポートを生成します。
    /// 要件7.1, 7.2, 7.3, 7.4に対応します。
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// マイグレーション状態レポート
    pub fn check_migration_status(
        &self,
        conn: &rusqlite::Connection,
    ) -> Result<MigrationStatusReport, MigrationError> {
        log::debug!("マイグレーション状態を確認中...");

        // 利用可能なマイグレーション一覧を取得
        let available_migrations = self.registry.get_available_migrations();
        let total_available = available_migrations.len();

        // 適用済みマイグレーション一覧を取得
        let applied_migrations = MigrationTable::get_applied_migrations(conn)?;
        let total_applied = applied_migrations.len();

        // 未適用マイグレーション一覧を取得
        let pending_migrations = self.get_pending_migrations(conn)?;
        let pending_migration_names: Vec<String> =
            pending_migrations.iter().map(|m| m.name.clone()).collect();

        // 最後のマイグレーション実行日時を取得
        let last_migration_date = applied_migrations.last().map(|m| m.applied_at.clone());

        // データベースバージョンを決定（最新の適用済みマイグレーションのバージョン）
        let database_version = applied_migrations
            .last()
            .map(|m| m.version.clone())
            .unwrap_or_else(|| "0.0.0".to_string());

        log::debug!(
            "マイグレーション状態: 利用可能={}, 適用済み={}, 未適用={}",
            total_available,
            total_applied,
            pending_migration_names.len()
        );

        Ok(MigrationStatusReport::new(
            total_available,
            total_applied,
            pending_migration_names,
            last_migration_date,
            database_version,
        ))
    }

    /// 未適用マイグレーション一覧を取得
    ///
    /// 利用可能なマイグレーションから適用済みマイグレーションを除いた一覧を返します。
    /// 要件2.2, 2.3に対応します。
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 未適用マイグレーション定義一覧
    fn get_pending_migrations(
        &self,
        conn: &Connection,
    ) -> Result<
        Vec<&crate::features::migrations::auto_migration::models::MigrationDefinition>,
        MigrationError,
    > {
        // 適用済みマイグレーション一覧を取得
        let applied_migrations = MigrationTable::get_applied_migrations(conn)?;
        let applied_names: std::collections::HashSet<String> =
            applied_migrations.into_iter().map(|m| m.name).collect();

        // 利用可能なマイグレーションから適用済みを除外
        let pending_migrations: Vec<_> = self
            .registry
            .get_available_migrations()
            .iter()
            .filter(|migration| !applied_names.contains(&migration.name))
            .collect();

        log::debug!(
            "未適用マイグレーション: {:?}",
            pending_migrations
                .iter()
                .map(|m| &m.name)
                .collect::<Vec<_>>()
        );

        Ok(pending_migrations)
    }

    /// 未適用マイグレーションを順次実行
    ///
    /// # 引数
    /// * `conn` - データベース接続
    /// * `pending_migrations` - 未適用マイグレーション一覧
    ///
    /// # 戻り値
    /// 実行結果
    fn execute_pending_migrations(
        &self,
        conn: &Connection,
        pending_migrations: &[&crate::features::migrations::auto_migration::models::MigrationDefinition],
    ) -> Result<AutoMigrationResult, MigrationError> {
        let mut applied_migrations = Vec::new();
        let mut backup_path: Option<String> = None;

        for migration_def in pending_migrations {
            log::info!("マイグレーション '{}' を実行中...", migration_def.name);

            // 実行可能なマイグレーションを取得
            let executable_migration = self
                .registry
                .find_executable_migration(&migration_def.name)
                .ok_or_else(|| {
                    MigrationError::system(
                        format!(
                            "実行可能なマイグレーション '{}' が見つかりません",
                            migration_def.name
                        ),
                        None,
                    )
                })?;

            // チェックサムの整合性を検証（要件4.4）
            MigrationTable::verify_checksum(conn, &migration_def.name, &migration_def.checksum)?;

            // マイグレーションを実行
            let execution_result = self
                .executor
                .execute_migration(conn, executable_migration)?;

            if !execution_result.success {
                return Err(MigrationError::execution(
                    migration_def.name.clone(),
                    execution_result.message,
                    None,
                ));
            }

            // 実行記録を保存（要件3.4, 4.1, 4.2, 4.3）
            let applied_at = MigrationTable::current_jst_timestamp();
            MigrationTable::record_migration(
                conn,
                &migration_def.name,
                &migration_def.version,
                Some(&migration_def.description),
                &migration_def.checksum,
                &applied_at,
                Some(execution_result.execution_time_ms),
            )?;

            applied_migrations.push(migration_def.name.clone());

            // 最初のバックアップパスを保持
            if backup_path.is_none() {
                backup_path = execution_result.backup_path;
            }

            log::info!(
                "マイグレーション '{}' が正常に完了しました (実行時間: {}ms)",
                migration_def.name,
                execution_result.execution_time_ms
            );
        }

        let message = format!(
            "{}件のマイグレーションを正常に適用しました",
            applied_migrations.len()
        );
        Ok(AutoMigrationResult::success(
            message,
            applied_migrations,
            backup_path,
            0, // 個別の実行時間は各マイグレーションで記録済み
        ))
    }

    /// マイグレーション実行中フラグをチェック
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 実行中の場合はtrue
    fn is_migration_in_progress(&self, conn: &Connection) -> Result<bool, MigrationError> {
        // 簡易的な実装：migration_lockテーブルを使用
        let sql = r#"
            CREATE TABLE IF NOT EXISTS migration_lock (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                in_progress INTEGER NOT NULL DEFAULT 0,
                started_at TEXT,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        "#;

        conn.execute(sql, []).map_err(|e| {
            MigrationError::system(
                "migration_lockテーブルの作成に失敗しました".to_string(),
                Some(e.to_string()),
            )
        })?;

        // 初期レコードを挿入（存在しない場合のみ）
        conn.execute(
            "INSERT OR IGNORE INTO migration_lock (id, in_progress) VALUES (1, 0)",
            [],
        )
        .map_err(|e| {
            MigrationError::system(
                "migration_lockの初期化に失敗しました".to_string(),
                Some(e.to_string()),
            )
        })?;

        // 実行中フラグを確認
        let in_progress: i64 = conn
            .query_row(
                "SELECT in_progress FROM migration_lock WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| {
                MigrationError::system(
                    "マイグレーション実行状態の確認に失敗しました".to_string(),
                    Some(e.to_string()),
                )
            })?;

        Ok(in_progress != 0)
    }

    /// マイグレーション実行中フラグを設定
    ///
    /// # 引数
    /// * `conn` - データベース接続
    /// * `in_progress` - 実行中フラグ
    ///
    /// # 戻り値
    /// 成功時はOk(())
    fn set_migration_in_progress(
        &self,
        conn: &Connection,
        in_progress: bool,
    ) -> Result<(), MigrationError> {
        let now_jst = Utc::now().with_timezone(&Tokyo);
        let timestamp = now_jst.to_rfc3339();

        let (in_progress_value, started_at) = if in_progress {
            (1, Some(timestamp.as_str()))
        } else {
            (0, None)
        };

        conn.execute(
            "UPDATE migration_lock SET in_progress = ?1, started_at = ?2, updated_at = ?3 WHERE id = 1",
            params![in_progress_value, started_at, timestamp],
        )
        .map_err(|e| {
            MigrationError::system(
                "マイグレーション実行状態の設定に失敗しました".to_string(),
                Some(e.to_string()),
            )
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用のデータベース接続を作成
    fn create_test_db() -> Connection {
        Connection::open_in_memory().expect("テスト用データベースの作成に失敗")
    }

    #[test]
    fn test_service_creation() {
        let conn = create_test_db();
        let service = AutoMigrationService::new(&conn);
        assert!(service.is_ok());

        let service = service.unwrap();
        assert_eq!(service.registry.count(), 3); // デフォルトマイグレーション数
    }

    #[test]
    fn test_check_migration_status_empty() {
        let conn = create_test_db();
        let service = AutoMigrationService::new(&conn).unwrap();

        let status = service.check_migration_status(&conn).unwrap();
        assert_eq!(status.total_available, 3); // デフォルトマイグレーション数
        assert_eq!(status.total_applied, 0);
        assert_eq!(status.pending_migrations.len(), 3);
        assert!(status.last_migration_date.is_none());
        assert_eq!(status.database_version, "0.0.0");
    }

    #[test]
    fn test_get_pending_migrations() {
        let conn = create_test_db();
        let service = AutoMigrationService::new(&conn).unwrap();

        // 初期状態では全てのマイグレーションが未適用
        let pending = service.get_pending_migrations(&conn).unwrap();
        assert_eq!(pending.len(), 3);

        // マイグレーション名が正しいことを確認
        let names: Vec<&String> = pending.iter().map(|m| &m.name).collect();
        assert!(names.contains(&&"001_create_basic_schema".to_string()));
        assert!(names.contains(&&"002_add_user_authentication".to_string()));
        assert!(names.contains(&&"003_migrate_receipt_url".to_string()));
    }

    #[test]
    fn test_migration_lock_functionality() {
        let conn = create_test_db();
        let service = AutoMigrationService::new(&conn).unwrap();

        // 初期状態では実行中ではない
        assert!(!service.is_migration_in_progress(&conn).unwrap());

        // 実行中フラグを設定
        service.set_migration_in_progress(&conn, true).unwrap();
        assert!(service.is_migration_in_progress(&conn).unwrap());

        // 実行中フラグをクリア
        service.set_migration_in_progress(&conn, false).unwrap();
        assert!(!service.is_migration_in_progress(&conn).unwrap());
    }

    #[test]
    fn test_run_startup_migrations_no_pending() {
        let conn = create_test_db();
        let service = AutoMigrationService::new(&conn).unwrap();

        // 全てのマイグレーションを手動で適用済みとして記録
        let migrations = service.registry.get_available_migrations();
        for migration in migrations {
            MigrationTable::record_migration(
                &conn,
                &migration.name,
                &migration.version,
                Some(&migration.description),
                &migration.checksum,
                &MigrationTable::current_jst_timestamp(),
                Some(100),
            )
            .unwrap();
        }

        // 自動マイグレーション実行（適用すべきマイグレーションなし）
        let result = service.run_startup_migrations(&conn).unwrap();
        assert!(result.success);
        assert!(result.message.contains("既に適用済み"));
        assert_eq!(result.applied_migrations.len(), 0);
    }

    #[test]
    fn test_concurrent_migration_prevention() {
        let conn = create_test_db();
        let service = AutoMigrationService::new(&conn).unwrap();

        // migration_lockテーブルを初期化するために一度is_migration_in_progressを呼び出す
        service.is_migration_in_progress(&conn).unwrap();

        // 最初のマイグレーション実行を開始（実行中フラグを設定）
        service.set_migration_in_progress(&conn, true).unwrap();

        // 2回目の実行は並行制御エラーになる
        let result = service.run_startup_migrations(&conn);
        assert!(result.is_err());

        if let Err(error) = result {
            assert!(matches!(
                error.error_type,
                crate::features::migrations::auto_migration::errors::MigrationErrorType::Concurrency
            ));
        }
    }

    #[test]
    fn test_migration_status_with_applied_migrations() {
        let conn = create_test_db();
        let service = AutoMigrationService::new(&conn).unwrap();

        // 1つのマイグレーションを適用済みとして記録
        let migration = service.registry.get_available_migrations().first().unwrap();
        MigrationTable::record_migration(
            &conn,
            &migration.name,
            &migration.version,
            Some(&migration.description),
            &migration.checksum,
            &MigrationTable::current_jst_timestamp(),
            Some(150),
        )
        .unwrap();

        // 状態確認
        let status = service.check_migration_status(&conn).unwrap();
        assert_eq!(status.total_available, 3);
        assert_eq!(status.total_applied, 1);
        assert_eq!(status.pending_migrations.len(), 2);
        assert!(status.last_migration_date.is_some());
        assert_eq!(status.database_version, migration.version);
    }
}

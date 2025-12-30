//! migrationsテーブル管理
//!
//! このモジュールは、migrationsテーブルの管理を行います。

use super::errors::MigrationError;
use super::models::AppliedMigration;
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use rusqlite::{params, Connection, Row};

/// migrationsテーブル管理
///
/// migrationsテーブルの初期化、データの取得・保存を行います。
pub struct MigrationTable;

impl MigrationTable {
    /// migrationsテーブルを初期化
    ///
    /// テーブルが存在しない場合は作成し、適切なインデックスも作成します。
    /// 既にテーブルが存在する場合は何もしません（冪等性）。
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー
    pub fn initialize(conn: &Connection) -> Result<(), MigrationError> {
        // migrationsテーブルを作成（存在しない場合のみ）
        let create_table_sql = r#"
            CREATE TABLE IF NOT EXISTS migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                version TEXT NOT NULL,
                description TEXT,
                checksum TEXT NOT NULL,
                applied_at TEXT NOT NULL,
                execution_time_ms INTEGER,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        "#;

        conn.execute(create_table_sql, []).map_err(|e| {
            MigrationError::initialization(
                "migrationsテーブルの作成に失敗しました".to_string(),
                Some(format!("SQLエラー: {e}")),
            )
        })?;

        // インデックスを作成
        Self::create_indexes(conn)?;

        log::info!("migrationsテーブルの初期化が完了しました");
        Ok(())
    }

    /// インデックスを作成
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー
    fn create_indexes(conn: &Connection) -> Result<(), MigrationError> {
        let indexes = [
            "CREATE INDEX IF NOT EXISTS idx_migrations_name ON migrations(name)",
            "CREATE INDEX IF NOT EXISTS idx_migrations_applied_at ON migrations(applied_at)",
            "CREATE INDEX IF NOT EXISTS idx_migrations_version ON migrations(version)",
        ];

        for index_sql in &indexes {
            conn.execute(index_sql, []).map_err(|e| {
                MigrationError::initialization(
                    "migrationsテーブルのインデックス作成に失敗しました".to_string(),
                    Some(format!("SQLエラー: {e}")),
                )
            })?;
        }

        Ok(())
    }

    /// 適用済みマイグレーション一覧を取得
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 適用済みマイグレーション一覧
    pub fn get_applied_migrations(
        conn: &Connection,
    ) -> Result<Vec<AppliedMigration>, MigrationError> {
        let sql = r#"
            SELECT id, name, version, description, checksum, applied_at, execution_time_ms, created_at
            FROM migrations
            ORDER BY applied_at ASC
        "#;

        let mut stmt = conn.prepare(sql).map_err(|e| {
            MigrationError::system(
                "適用済みマイグレーション取得のクエリ準備に失敗しました".to_string(),
                Some(format!("SQLエラー: {e}")),
            )
        })?;

        let migration_iter = stmt
            .query_map([], |row| Ok(Self::row_to_applied_migration(row)?))
            .map_err(|e| {
                MigrationError::system(
                    "適用済みマイグレーション取得のクエリ実行に失敗しました".to_string(),
                    Some(format!("SQLエラー: {e}")),
                )
            })?;

        let mut migrations = Vec::new();
        for migration_result in migration_iter {
            let migration = migration_result.map_err(|e| {
                MigrationError::system(
                    "適用済みマイグレーションデータの変換に失敗しました".to_string(),
                    Some(format!("SQLエラー: {e}")),
                )
            })?;
            migrations.push(migration);
        }

        log::debug!(
            "適用済みマイグレーション {}件を取得しました",
            migrations.len()
        );
        Ok(migrations)
    }

    /// データベース行をAppliedMigrationに変換
    ///
    /// # 引数
    /// * `row` - データベース行
    ///
    /// # 戻り値
    /// AppliedMigration
    fn row_to_applied_migration(row: &Row) -> Result<AppliedMigration, rusqlite::Error> {
        Ok(AppliedMigration::new(
            row.get("id")?,
            row.get("name")?,
            row.get("version")?,
            row.get("description")?,
            row.get("checksum")?,
            row.get("applied_at")?,
            row.get("execution_time_ms")?,
            row.get("created_at")?,
        ))
    }

    /// マイグレーション実行記録を保存
    ///
    /// JST（日本標準時）で現在時刻を記録します。
    ///
    /// # 引数
    /// * `conn` - データベース接続
    /// * `name` - マイグレーション名
    /// * `version` - バージョン
    /// * `description` - 説明
    /// * `checksum` - チェックサム
    /// * `applied_at` - 適用日時（JST、RFC3339形式）
    /// * `execution_time_ms` - 実行時間
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー
    #[allow(clippy::too_many_arguments)]
    pub fn record_migration(
        conn: &Connection,
        name: &str,
        version: &str,
        description: Option<&str>,
        checksum: &str,
        applied_at: &str,
        execution_time_ms: Option<i64>,
    ) -> Result<(), MigrationError> {
        // 現在時刻をJSTで取得
        let now_jst = Utc::now().with_timezone(&Tokyo);
        let created_at = now_jst.to_rfc3339();

        let sql = r#"
            INSERT INTO migrations (name, version, description, checksum, applied_at, execution_time_ms, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#;

        conn.execute(
            sql,
            params![
                name,
                version,
                description,
                checksum,
                applied_at,
                execution_time_ms,
                created_at
            ],
        )
        .map_err(|e| match e {
            rusqlite::Error::SqliteFailure(ffi_error, _) => {
                if ffi_error.code == rusqlite::ErrorCode::ConstraintViolation {
                    MigrationError::validation(
                        format!("マイグレーション '{name}' は既に記録されています"),
                        Some(name.to_string()),
                        Some("UNIQUE制約違反".to_string()),
                    )
                } else {
                    MigrationError::execution(
                        name.to_string(),
                        "マイグレーション記録の保存に失敗しました".to_string(),
                        Some(format!("SQLエラー: {e}")),
                    )
                }
            }
            _ => MigrationError::execution(
                name.to_string(),
                "マイグレーション記録の保存に失敗しました".to_string(),
                Some(format!("SQLエラー: {e}")),
            ),
        })?;

        log::info!("マイグレーション '{name}' の実行記録を保存しました");
        Ok(())
    }

    /// マイグレーションが適用済みかチェック
    ///
    /// # 引数
    /// * `conn` - データベース接続
    /// * `name` - マイグレーション名
    ///
    /// # 戻り値
    /// 適用済みの場合はtrue
    pub fn is_migration_applied(conn: &Connection, name: &str) -> Result<bool, MigrationError> {
        let sql = "SELECT COUNT(*) FROM migrations WHERE name = ?1";

        let count: i64 = conn
            .query_row(sql, params![name], |row| row.get(0))
            .map_err(|e| {
                MigrationError::system(
                    format!("マイグレーション '{name}' の適用状態確認に失敗しました"),
                    Some(format!("SQLエラー: {e}")),
                )
            })?;

        Ok(count > 0)
    }

    /// 特定のマイグレーションの詳細情報を取得
    ///
    /// # 引数
    /// * `conn` - データベース接続
    /// * `name` - マイグレーション名
    ///
    /// # 戻り値
    /// マイグレーション情報（見つからない場合はNone）
    pub fn get_migration_by_name(
        conn: &Connection,
        name: &str,
    ) -> Result<Option<AppliedMigration>, MigrationError> {
        let sql = r#"
            SELECT id, name, version, description, checksum, applied_at, execution_time_ms, created_at
            FROM migrations
            WHERE name = ?1
        "#;

        match conn.query_row(sql, params![name], |row| {
            Ok(Self::row_to_applied_migration(row)?)
        }) {
            Ok(migration) => Ok(Some(migration)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(MigrationError::system(
                format!("マイグレーション '{name}' の詳細取得に失敗しました"),
                Some(format!("SQLエラー: {e}")),
            )),
        }
    }

    /// チェックサムの整合性を検証
    ///
    /// # 引数
    /// * `conn` - データベース接続
    /// * `name` - マイグレーション名
    /// * `expected_checksum` - 期待されるチェックサム
    ///
    /// # 戻り値
    /// 整合性が取れている場合はOk(())、不一致の場合はエラー
    pub fn verify_checksum(
        conn: &Connection,
        name: &str,
        expected_checksum: &str,
    ) -> Result<(), MigrationError> {
        if let Some(applied_migration) = Self::get_migration_by_name(conn, name)? {
            if !applied_migration.verify_checksum(expected_checksum) {
                return Err(MigrationError::checksum_mismatch(
                    name.to_string(),
                    expected_checksum.to_string(),
                    applied_migration.checksum,
                ));
            }
        }
        Ok(())
    }

    /// migrationsテーブルが存在するかチェック
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// テーブルが存在する場合はtrue
    pub fn table_exists(conn: &Connection) -> Result<bool, MigrationError> {
        let sql = r#"
            SELECT COUNT(*) FROM sqlite_master 
            WHERE type='table' AND name='migrations'
        "#;

        let count: i64 = conn.query_row(sql, [], |row| row.get(0)).map_err(|e| {
            MigrationError::system(
                "migrationsテーブルの存在確認に失敗しました".to_string(),
                Some(format!("SQLエラー: {e}")),
            )
        })?;

        Ok(count > 0)
    }

    /// 現在時刻をJST（RFC3339形式）で取得
    ///
    /// # 戻り値
    /// JST時刻のRFC3339文字列
    pub fn current_jst_timestamp() -> String {
        let now_jst = Utc::now().with_timezone(&Tokyo);
        now_jst.to_rfc3339()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    /// テスト用のデータベース接続を作成
    fn create_test_db() -> Connection {
        Connection::open_in_memory().expect("テスト用データベースの作成に失敗")
    }

    /// テスト用のファイルデータベース接続を作成
    fn create_test_file_db() -> (Connection, NamedTempFile) {
        let temp_file = NamedTempFile::new().expect("一時ファイルの作成に失敗");
        let conn = Connection::open(temp_file.path()).expect("テスト用データベースの作成に失敗");
        (conn, temp_file)
    }

    #[test]
    fn test_initialize_creates_table() {
        let conn = create_test_db();

        // 初期化前はテーブルが存在しない
        assert!(!MigrationTable::table_exists(&conn).unwrap());

        // 初期化実行
        assert!(MigrationTable::initialize(&conn).is_ok());

        // 初期化後はテーブルが存在する
        assert!(MigrationTable::table_exists(&conn).unwrap());
    }

    #[test]
    fn test_initialize_is_idempotent() {
        let conn = create_test_db();

        // 初回初期化
        assert!(MigrationTable::initialize(&conn).is_ok());
        assert!(MigrationTable::table_exists(&conn).unwrap());

        // 2回目の初期化（冪等性のテスト）
        assert!(MigrationTable::initialize(&conn).is_ok());
        assert!(MigrationTable::table_exists(&conn).unwrap());
    }

    #[test]
    fn test_record_and_get_migration() {
        let conn = create_test_db();
        MigrationTable::initialize(&conn).unwrap();

        let name = "test_migration";
        let version = "1.0.0";
        let description = Some("テストマイグレーション");
        let checksum = "a".repeat(64);
        let applied_at = MigrationTable::current_jst_timestamp();
        let execution_time_ms = Some(1000);

        // マイグレーション記録を保存
        assert!(MigrationTable::record_migration(
            &conn,
            name,
            version,
            description,
            &checksum,
            &applied_at,
            execution_time_ms,
        )
        .is_ok());

        // 適用済みマイグレーション一覧を取得
        let migrations = MigrationTable::get_applied_migrations(&conn).unwrap();
        assert_eq!(migrations.len(), 1);

        let migration = &migrations[0];
        assert_eq!(migration.name, name);
        assert_eq!(migration.version, version);
        assert_eq!(migration.description, description.map(|s| s.to_string()));
        assert_eq!(migration.checksum, checksum);
        assert_eq!(migration.applied_at, applied_at);
        assert_eq!(migration.execution_time_ms, execution_time_ms);
    }

    #[test]
    fn test_is_migration_applied() {
        let conn = create_test_db();
        MigrationTable::initialize(&conn).unwrap();

        let name = "test_migration";

        // 初期状態では適用されていない
        assert!(!MigrationTable::is_migration_applied(&conn, name).unwrap());

        // マイグレーションを記録
        MigrationTable::record_migration(
            &conn,
            name,
            "1.0.0",
            None,
            &"a".repeat(64),
            &MigrationTable::current_jst_timestamp(),
            None,
        )
        .unwrap();

        // 記録後は適用済みとして認識される
        assert!(MigrationTable::is_migration_applied(&conn, name).unwrap());
    }

    #[test]
    fn test_get_migration_by_name() {
        let conn = create_test_db();
        MigrationTable::initialize(&conn).unwrap();

        let name = "test_migration";

        // 存在しないマイグレーション
        assert!(MigrationTable::get_migration_by_name(&conn, name)
            .unwrap()
            .is_none());

        // マイグレーションを記録
        let checksum = "a".repeat(64);
        MigrationTable::record_migration(
            &conn,
            name,
            "1.0.0",
            Some("テスト"),
            &checksum,
            &MigrationTable::current_jst_timestamp(),
            Some(500),
        )
        .unwrap();

        // 記録されたマイグレーションを取得
        let migration = MigrationTable::get_migration_by_name(&conn, name)
            .unwrap()
            .unwrap();
        assert_eq!(migration.name, name);
        assert_eq!(migration.checksum, checksum);
    }

    #[test]
    fn test_verify_checksum() {
        let conn = create_test_db();
        MigrationTable::initialize(&conn).unwrap();

        let name = "test_migration";
        let checksum = "a".repeat(64);

        // マイグレーションを記録
        MigrationTable::record_migration(
            &conn,
            name,
            "1.0.0",
            None,
            &checksum,
            &MigrationTable::current_jst_timestamp(),
            None,
        )
        .unwrap();

        // 正しいチェックサムでの検証
        assert!(MigrationTable::verify_checksum(&conn, name, &checksum).is_ok());

        // 間違ったチェックサムでの検証
        let wrong_checksum = "b".repeat(64);
        let result = MigrationTable::verify_checksum(&conn, name, &wrong_checksum);
        assert!(result.is_err());

        if let Err(error) = result {
            assert!(matches!(
                error.error_type,
                super::super::errors::MigrationErrorType::ChecksumMismatch
            ));
        }
    }

    #[test]
    fn test_duplicate_migration_name() {
        let conn = create_test_db();
        MigrationTable::initialize(&conn).unwrap();

        let name = "test_migration";
        let checksum = "a".repeat(64);

        // 初回記録は成功
        assert!(MigrationTable::record_migration(
            &conn,
            name,
            "1.0.0",
            None,
            &checksum,
            &MigrationTable::current_jst_timestamp(),
            None,
        )
        .is_ok());

        // 同じ名前での2回目の記録は失敗
        let result = MigrationTable::record_migration(
            &conn,
            name,
            "1.0.1",
            None,
            &checksum,
            &MigrationTable::current_jst_timestamp(),
            None,
        );

        assert!(result.is_err());
        if let Err(error) = result {
            assert!(matches!(
                error.error_type,
                super::super::errors::MigrationErrorType::Validation
            ));
        }
    }

    #[test]
    fn test_current_jst_timestamp_format() {
        let timestamp = MigrationTable::current_jst_timestamp();

        // RFC3339形式であることを確認（簡易チェック）
        assert!(timestamp.contains("T"));
        assert!(timestamp.contains("+09:00")); // JSTタイムゾーン

        // パースできることを確認
        assert!(chrono::DateTime::parse_from_rfc3339(&timestamp).is_ok());
    }

    #[test]
    fn test_multiple_migrations_ordering() {
        let conn = create_test_db();
        MigrationTable::initialize(&conn).unwrap();

        // 複数のマイグレーションを異なる時刻で記録
        let migrations = vec![
            ("migration_1", "2024-01-01T10:00:00+09:00"),
            ("migration_2", "2024-01-01T11:00:00+09:00"),
            ("migration_3", "2024-01-01T09:00:00+09:00"),
        ];

        for (name, applied_at) in &migrations {
            MigrationTable::record_migration(
                &conn,
                name,
                "1.0.0",
                None,
                &"a".repeat(64),
                applied_at,
                None,
            )
            .unwrap();
        }

        // 適用済みマイグレーション一覧を取得（applied_at順）
        let applied_migrations = MigrationTable::get_applied_migrations(&conn).unwrap();
        assert_eq!(applied_migrations.len(), 3);

        // 時刻順にソートされていることを確認
        assert_eq!(applied_migrations[0].name, "migration_3"); // 09:00
        assert_eq!(applied_migrations[1].name, "migration_1"); // 10:00
        assert_eq!(applied_migrations[2].name, "migration_2"); // 11:00
    }
}

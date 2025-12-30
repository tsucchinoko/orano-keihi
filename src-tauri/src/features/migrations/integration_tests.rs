//! 自動マイグレーションシステム統合テスト
//!
//! このモジュールは、自動マイグレーションシステム全体の統合テストを提供します。
//! 実際のデータベースを使用して、エンドツーエンドの動作を検証します。

#[cfg(test)]
mod integration_tests {
    use crate::features::migrations::auto_migration::{
        AutoMigrationService, MigrationRegistry, MigrationTable,
    };
    use crate::shared::database::connection::create_tables;
    use rusqlite::Connection;
    use std::fs;
    use tempfile::NamedTempFile;

    /// テスト用のファイルデータベース接続を作成
    fn create_test_file_db() -> (Connection, NamedTempFile) {
        let temp_file = NamedTempFile::new().expect("一時ファイルの作成に失敗");
        let conn = Connection::open(temp_file.path()).expect("テスト用データベースの作成に失敗");
        (conn, temp_file)
    }

    /// テスト用のメモリデータベース接続を作成
    fn create_test_memory_db() -> Connection {
        Connection::open_in_memory().expect("テスト用データベースの作成に失敗")
    }

    /// 統合テスト: 完全な自動マイグレーションフロー
    ///
    /// 新しいデータベースから開始して、全てのマイグレーションが正常に適用されることを確認します。
    #[test]
    fn test_complete_auto_migration_flow() {
        let conn = create_test_memory_db();

        // 1. 自動マイグレーションサービスを初期化
        let service = AutoMigrationService::new(&conn).expect("サービス初期化に失敗");

        // 2. 初期状態の確認
        let initial_status = service
            .check_migration_status(&conn)
            .expect("初期状態確認に失敗");
        assert_eq!(initial_status.total_available, 3);
        assert_eq!(initial_status.total_applied, 0);
        assert_eq!(initial_status.pending_migrations.len(), 3);
        assert!(initial_status.last_migration_date.is_none());
        assert_eq!(initial_status.database_version, "0.0.0");

        // 3. 自動マイグレーション実行
        let migration_result = service
            .run_startup_migrations(&conn)
            .expect("自動マイグレーション実行に失敗");

        assert!(migration_result.success);
        assert_eq!(migration_result.applied_migrations.len(), 3);
        assert!(migration_result.backup_path.is_some());
        assert!(migration_result.total_execution_time_ms >= 0);

        // 4. 実行後の状態確認
        let final_status = service
            .check_migration_status(&conn)
            .expect("最終状態確認に失敗");
        assert_eq!(final_status.total_available, 3);
        assert_eq!(final_status.total_applied, 3);
        assert_eq!(final_status.pending_migrations.len(), 0);
        assert!(final_status.last_migration_date.is_some());
        assert_ne!(final_status.database_version, "0.0.0");

        // 5. データベース構造の確認
        verify_database_structure(&conn);

        // 6. 2回目の実行（冪等性の確認）
        let second_run_result = service
            .run_startup_migrations(&conn)
            .expect("2回目の自動マイグレーション実行に失敗");

        assert!(second_run_result.success);
        assert_eq!(second_run_result.applied_migrations.len(), 0);
        assert!(second_run_result.message.contains("既に適用済み"));
    }

    /// 統合テスト: エラーシナリオ - 並行実行制御
    ///
    /// 複数のインスタンスが同時に実行されようとした場合の制御を確認します。
    #[test]
    fn test_concurrent_migration_prevention() {
        let conn = create_test_memory_db();
        let service = AutoMigrationService::new(&conn).expect("サービス初期化に失敗");

        // migration_lockテーブルを初期化
        service.check_migration_status(&conn).expect("初期化に失敗");

        // 手動でマイグレーション実行中フラグを設定
        conn.execute(
            "CREATE TABLE IF NOT EXISTS migration_lock (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                in_progress INTEGER NOT NULL DEFAULT 0,
                started_at TEXT,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .expect("migration_lockテーブル作成に失敗");

        conn.execute(
            "INSERT OR REPLACE INTO migration_lock (id, in_progress, started_at, updated_at) 
             VALUES (1, 1, datetime('now'), datetime('now'))",
            [],
        )
        .expect("実行中フラグ設定に失敗");

        // 並行実行制御エラーが発生することを確認
        let result = service.run_startup_migrations(&conn);
        assert!(result.is_err());

        if let Err(error) = result {
            assert!(error
                .to_string()
                .contains("別のインスタンスがマイグレーション実行中"));
        }
    }

    /// 統合テスト: エラーシナリオ - チェックサム不一致
    ///
    /// マイグレーションのチェックサムが変更された場合のエラー処理を確認します。
    #[test]
    fn test_checksum_mismatch_error() {
        let conn = create_test_memory_db();
        let service = AutoMigrationService::new(&conn).expect("サービス初期化に失敗");

        // 1つのマイグレーションを手動で記録（間違ったチェックサムで）
        MigrationTable::record_migration(
            &conn,
            "001_create_basic_schema",
            "1.0.0",
            Some("基本スキーマ作成"),
            "wrong_checksum_value", // 間違ったチェックサム
            &MigrationTable::current_jst_timestamp(),
            Some(1000),
        )
        .expect("マイグレーション記録に失敗");

        // チェックサム不一致エラーが発生することを確認
        let result = service.run_startup_migrations(&conn);
        assert!(result.is_err());

        if let Err(error) = result {
            assert!(error.to_string().contains("チェックサム"));
        }
    }

    /// 統合テスト: 部分的マイグレーション適用
    ///
    /// 一部のマイグレーションが既に適用されている状態からの実行を確認します。
    #[test]
    fn test_partial_migration_application() {
        let conn = create_test_memory_db();
        let service = AutoMigrationService::new(&conn).expect("サービス初期化に失敗");

        // 基本スキーママイグレーションのみを手動で適用
        let registry =
            MigrationRegistry::register_default_migrations().expect("レジストリ作成に失敗");
        let basic_migration = registry
            .find_migration("001_create_basic_schema")
            .expect("基本マイグレーションが見つからない");

        // 基本テーブルを作成
        create_tables(&conn).expect("基本テーブル作成に失敗");

        // マイグレーション記録を保存
        MigrationTable::record_migration(
            &conn,
            &basic_migration.name,
            &basic_migration.version,
            Some(&basic_migration.description),
            &basic_migration.checksum,
            &MigrationTable::current_jst_timestamp(),
            Some(500),
        )
        .expect("マイグレーション記録に失敗");

        // 残りのマイグレーションが実行されることを確認
        let migration_result = service
            .run_startup_migrations(&conn)
            .expect("部分マイグレーション実行に失敗");

        assert!(migration_result.success);
        assert_eq!(migration_result.applied_migrations.len(), 2); // 残り2つ

        // 最終状態の確認
        let final_status = service
            .check_migration_status(&conn)
            .expect("最終状態確認に失敗");
        assert_eq!(final_status.total_applied, 3);
        assert_eq!(final_status.pending_migrations.len(), 0);
    }

    /// 統合テスト: マイグレーション状態確認コマンド
    ///
    /// Tauriコマンドとしてのマイグレーション状態確認機能を検証します。
    #[test]
    fn test_migration_status_command_integration() {
        let (conn, _temp_file) = create_test_file_db();

        // 自動マイグレーションを実行
        let service = AutoMigrationService::new(&conn).expect("サービス初期化に失敗");
        service
            .run_startup_migrations(&conn)
            .expect("自動マイグレーション実行に失敗");

        // 状態確認を直接実行（Tauriコマンドではなく、サービス経由）
        let status_report = service
            .check_migration_status(&conn)
            .expect("マイグレーション状態取得に失敗");

        // 詳細情報の検証
        assert_eq!(status_report.total_available, 3);
        assert_eq!(status_report.total_applied, 3);
        assert_eq!(status_report.pending_migrations.len(), 0);
        assert!(status_report.last_migration_date.is_some());

        // 適用済みマイグレーション情報の確認
        let applied_migrations = MigrationTable::get_applied_migrations(&conn)
            .expect("適用済みマイグレーション取得に失敗");

        assert_eq!(applied_migrations.len(), 3);

        // マイグレーション名の確認
        let migration_names: Vec<&String> = applied_migrations.iter().map(|m| &m.name).collect();
        assert!(migration_names.contains(&&"001_create_basic_schema".to_string()));
        assert!(migration_names.contains(&&"002_add_user_authentication".to_string()));
        assert!(migration_names.contains(&&"003_migrate_receipt_url".to_string()));

        // 適用済みマイグレーション情報の確認
        for applied_migration in &applied_migrations {
            assert!(applied_migration.applied_at.contains("+09:00")); // JSTタイムゾーン
            assert!(applied_migration.execution_time_ms.is_some());
        }

        println!("マイグレーション状態確認テスト完了");
    }

    /// 統合テスト: バックアップ機能
    ///
    /// マイグレーション実行時のバックアップ作成機能を検証します。
    #[test]
    fn test_backup_functionality() {
        let (conn, _temp_file) = create_test_file_db();

        // テストデータを挿入
        conn.execute(
            "CREATE TABLE test_data (id INTEGER PRIMARY KEY, value TEXT)",
            [],
        )
        .expect("テストテーブル作成に失敗");
        conn.execute("INSERT INTO test_data (value) VALUES ('test')", [])
            .expect("テストデータ挿入に失敗");

        // 自動マイグレーションを実行
        let service = AutoMigrationService::new(&conn).expect("サービス初期化に失敗");
        let migration_result = service
            .run_startup_migrations(&conn)
            .expect("自動マイグレーション実行に失敗");

        // バックアップが作成されていることを確認
        assert!(migration_result.backup_path.is_some());
        let backup_path = migration_result.backup_path.unwrap();
        assert!(backup_path.starts_with("database_backup_"));
        assert!(backup_path.ends_with(".db"));

        // バックアップファイルが存在することを確認
        assert!(
            fs::metadata(&backup_path).is_ok(),
            "バックアップファイルが存在しません: {}",
            backup_path
        );

        // バックアップファイルのサイズが0より大きいことを確認
        let backup_size = fs::metadata(&backup_path)
            .expect("バックアップファイル情報取得に失敗")
            .len();
        assert!(backup_size > 0, "バックアップファイルが空です");

        // クリーンアップ
        fs::remove_file(&backup_path).ok();
    }

    /// 統合テスト: JST時刻記録
    ///
    /// マイグレーション実行時刻がJST（日本標準時）で記録されることを確認します。
    #[test]
    fn test_jst_timestamp_recording() {
        let conn = create_test_memory_db();
        let service = AutoMigrationService::new(&conn).expect("サービス初期化に失敗");

        // 自動マイグレーションを実行
        service
            .run_startup_migrations(&conn)
            .expect("自動マイグレーション実行に失敗");

        // 適用済みマイグレーション一覧を取得
        let applied_migrations = MigrationTable::get_applied_migrations(&conn)
            .expect("適用済みマイグレーション取得に失敗");

        // 全てのマイグレーションがJSTタイムゾーンで記録されていることを確認
        for migration in applied_migrations {
            assert!(
                migration.applied_at.contains("+09:00"),
                "JSTタイムゾーンで記録されていません: {}",
                migration.applied_at
            );

            // RFC3339形式でパースできることを確認
            assert!(
                chrono::DateTime::parse_from_rfc3339(&migration.applied_at).is_ok(),
                "RFC3339形式ではありません: {}",
                migration.applied_at
            );
        }
    }

    /// 統合テスト: データベース構造検証
    ///
    /// マイグレーション後のデータベース構造が期待通りであることを確認します。
    #[test]
    fn test_database_structure_after_migration() {
        let conn = create_test_memory_db();
        let service = AutoMigrationService::new(&conn).expect("サービス初期化に失敗");

        // 自動マイグレーションを実行
        service
            .run_startup_migrations(&conn)
            .expect("自動マイグレーション実行に失敗");

        // データベース構造を検証
        verify_database_structure(&conn);
    }

    /// データベース構造を検証するヘルパー関数
    fn verify_database_structure(conn: &Connection) {
        // 基本テーブルの存在確認
        let basic_tables = vec!["expenses", "subscriptions", "categories"];
        for table_name in basic_tables {
            let table_exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table_name],
                    |row| row.get(0),
                )
                .expect(&format!("{}テーブル存在確認に失敗", table_name));
            assert_eq!(table_exists, 1, "{}テーブルが存在しません", table_name);
        }

        // ユーザー認証テーブルの存在確認
        let auth_tables = vec!["users", "sessions"];
        for table_name in auth_tables {
            let table_exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table_name],
                    |row| row.get(0),
                )
                .expect(&format!("{}テーブル存在確認に失敗", table_name));
            assert_eq!(table_exists, 1, "{}テーブルが存在しません", table_name);
        }

        // migrationsテーブルの存在確認
        let migrations_table_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='migrations'",
                [],
                |row| row.get(0),
            )
            .expect("migrationsテーブル存在確認に失敗");
        assert_eq!(
            migrations_table_exists, 1,
            "migrationsテーブルが存在しません"
        );

        // expensesテーブルのスキーマ確認（receipt_urlカラムが存在することを確認）
        let column_info: Vec<String> = conn
            .prepare("PRAGMA table_info(expenses)")
            .expect("expensesテーブル情報取得に失敗")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("カラム情報取得に失敗")
            .collect::<Result<Vec<_>, _>>()
            .expect("カラム情報変換に失敗");

        assert!(
            column_info.contains(&"receipt_url".to_string()),
            "receipt_urlカラムが存在しません"
        );
        assert!(
            !column_info.contains(&"receipt_path".to_string()),
            "古いreceipt_pathカラムが残っています"
        );

        // 初期データの確認
        let category_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
            .expect("カテゴリ数取得に失敗");
        assert_eq!(category_count, 6, "初期カテゴリが正しく挿入されていません");

        let default_user_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM users WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("デフォルトユーザー確認に失敗");
        assert_eq!(
            default_user_count, 1,
            "デフォルトユーザーが作成されていません"
        );

        // インデックスの存在確認
        let index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name LIKE 'idx_migrations_%'",
                [],
                |row| row.get(0),
            )
            .expect("インデックス数取得に失敗");
        assert!(
            index_count >= 3,
            "migrationsテーブルのインデックスが不足しています"
        );
    }

    /// 統合テスト: エラー回復シナリオ
    ///
    /// マイグレーション実行中にエラーが発生した場合の回復処理を確認します。
    #[test]
    fn test_error_recovery_scenario() {
        let conn = create_test_memory_db();

        // migration_lockテーブルを手動で作成し、実行中状態にする
        conn.execute(
            "CREATE TABLE migration_lock (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                in_progress INTEGER NOT NULL DEFAULT 1,
                started_at TEXT,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .expect("migration_lockテーブル作成に失敗");

        conn.execute(
            "INSERT INTO migration_lock (id, in_progress, started_at, updated_at) 
             VALUES (1, 1, datetime('now'), datetime('now'))",
            [],
        )
        .expect("実行中フラグ設定に失敗");

        // サービス初期化（この時点では実行中フラグが立っている）
        let service = AutoMigrationService::new(&conn).expect("サービス初期化に失敗");

        // 並行実行制御エラーが発生することを確認
        let result = service.run_startup_migrations(&conn);
        assert!(result.is_err());

        // 実行中フラグを手動でクリア（回復処理をシミュレート）
        conn.execute(
            "UPDATE migration_lock SET in_progress = 0, started_at = NULL WHERE id = 1",
            [],
        )
        .expect("実行中フラグクリアに失敗");

        // 再実行が成功することを確認
        let recovery_result = service
            .run_startup_migrations(&conn)
            .expect("回復後のマイグレーション実行に失敗");

        assert!(recovery_result.success);
        assert_eq!(recovery_result.applied_migrations.len(), 3);
    }

    /// 統合テスト: 大量データでのパフォーマンス
    ///
    /// 大量のマイグレーション記録がある状態でのパフォーマンスを確認します。
    #[test]
    fn test_performance_with_large_migration_history() {
        let conn = create_test_memory_db();
        let service = AutoMigrationService::new(&conn).expect("サービス初期化に失敗");

        // 大量のダミーマイグレーション記録を作成
        for i in 1..=100 {
            MigrationTable::record_migration(
                &conn,
                &format!("dummy_migration_{:03}", i),
                "1.0.0",
                Some("ダミーマイグレーション"),
                &format!("{:064}", i), // 64文字のダミーチェックサム
                &MigrationTable::current_jst_timestamp(),
                Some(100),
            )
            .expect("ダミーマイグレーション記録に失敗");
        }

        // パフォーマンス測定
        let start_time = std::time::Instant::now();
        let status = service
            .check_migration_status(&conn)
            .expect("大量データでの状態確認に失敗");
        let elapsed = start_time.elapsed();

        // 基本的な動作確認
        assert_eq!(status.total_available, 3); // デフォルトマイグレーション数
        assert_eq!(status.total_applied, 100); // ダミーマイグレーション数

        // パフォーマンス確認（1秒以内で完了することを期待）
        assert!(
            elapsed.as_secs() < 1,
            "大量データでの処理時間が長すぎます: {:?}",
            elapsed
        );

        println!("大量データ（100件）での状態確認処理時間: {:?}", elapsed);
    }
}

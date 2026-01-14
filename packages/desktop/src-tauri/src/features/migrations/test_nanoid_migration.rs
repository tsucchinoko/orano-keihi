/// nanoIdマイグレーションの統合テスト
///
/// このテストは、実際のデータベースでnanoIdマイグレーションを実行し、
/// すべてのテーブルが正しく移行されることを確認します。

#[cfg(test)]
mod nanoid_migration_tests {
    use crate::features::migrations::service::{
        migrate_user_authentication, migrate_user_id_to_nanoid,
    };
    use crate::shared::utils::nanoid::is_valid_nanoid;
    use rusqlite::Connection;

    /// テスト用のデータベースを作成する（ユーザー認証機能付き）
    fn create_test_db_with_users() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

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

        // ユーザー認証マイグレーションを実行
        let result = migrate_user_authentication(&conn).unwrap();
        assert!(
            result.success,
            "ユーザー認証マイグレーション失敗: {}",
            result.message
        );

        // テストデータを挿入
        conn.execute(
            "INSERT INTO users (id, google_id, email, name, created_at, updated_at)
             VALUES (2, 'test_google_id', 'test@example.com', 'テストユーザー', '2024-01-01T00:00:00+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO expenses (date, amount, category, description, user_id, created_at, updated_at)
             VALUES ('2024-01-01', 1000.0, 'テスト', 'テスト経費1', 1, '2024-01-01T00:00:00+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO expenses (date, amount, category, description, user_id, created_at, updated_at)
             VALUES ('2024-01-02', 2000.0, 'テスト', 'テスト経費2', 2, '2024-01-02T00:00:00+09:00', '2024-01-02T00:00:00+09:00')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO subscriptions (name, amount, billing_cycle, start_date, category, user_id, created_at, updated_at)
             VALUES ('テストサブスク1', 500.0, 'monthly', '2024-01-01', 'テスト', 1, '2024-01-01T00:00:00+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO subscriptions (name, amount, billing_cycle, start_date, category, user_id, created_at, updated_at)
             VALUES ('テストサブスク2', 1000.0, 'annual', '2024-01-01', 'テスト', 2, '2024-01-01T00:00:00+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO sessions (id, user_id, expires_at, created_at)
             VALUES ('session1', 1, '2024-12-31T23:59:59+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO sessions (id, user_id, expires_at, created_at)
             VALUES ('session2', 2, '2024-12-31T23:59:59+09:00', '2024-01-01T00:00:00+09:00')",
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_nanoid_migration_complete_flow() {
        println!("\n=== nanoIdマイグレーション統合テスト開始 ===\n");

        let conn = create_test_db_with_users();

        // マイグレーション前のデータ数を記録
        let users_count_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .unwrap();
        let expenses_count_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))
            .unwrap();
        let subscriptions_count_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM subscriptions", [], |row| row.get(0))
            .unwrap();
        let sessions_count_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .unwrap();

        println!("マイグレーション前のデータ数:");
        println!("  users: {}", users_count_before);
        println!("  expenses: {}", expenses_count_before);
        println!("  subscriptions: {}", subscriptions_count_before);
        println!("  sessions: {}", sessions_count_before);

        // マイグレーション前のユーザーIDを記録
        let mut stmt = conn.prepare("SELECT id FROM users ORDER BY id").unwrap();
        let old_user_ids: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        println!("\nマイグレーション前のユーザーID: {:?}", old_user_ids);

        // nanoIdマイグレーションを実行
        println!("\nnanoIdマイグレーションを実行中...");
        let result = migrate_user_id_to_nanoid(&conn).unwrap();

        println!("\nマイグレーション結果:");
        println!("  成功: {}", result.success);
        println!("  メッセージ: {}", result.message);
        println!("  バックアップパス: {:?}", result.backup_path);

        assert!(result.success, "マイグレーション失敗: {}", result.message);

        // マイグレーション後のデータ数を確認
        let users_count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .unwrap();
        let expenses_count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))
            .unwrap();
        let subscriptions_count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM subscriptions", [], |row| row.get(0))
            .unwrap();
        let sessions_count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .unwrap();

        println!("\nマイグレーション後のデータ数:");
        println!("  users: {}", users_count_after);
        println!("  expenses: {}", expenses_count_after);
        println!("  subscriptions: {}", subscriptions_count_after);
        println!("  sessions: {}", sessions_count_after);

        // データ数が保持されていることを確認
        assert_eq!(users_count_before, users_count_after);
        assert_eq!(expenses_count_before, expenses_count_after);
        assert_eq!(subscriptions_count_before, subscriptions_count_after);
        assert_eq!(sessions_count_before, sessions_count_after);

        // すべてのユーザーIDがnanoId形式であることを確認
        let mut stmt = conn.prepare("SELECT id FROM users").unwrap();
        let new_user_ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        println!("\nマイグレーション後のユーザーID:");
        for id in &new_user_ids {
            println!("  {}", id);
            assert!(is_valid_nanoid(id), "無効なnanoId形式: {}", id);
        }

        // usersテーブルのidカラムがTEXT型であることを確認
        let table_info: Vec<(String, String)> = conn
            .prepare("PRAGMA table_info(users)")
            .unwrap()
            .query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let id_column = table_info.iter().find(|(name, _)| name == "id");
        assert!(id_column.is_some(), "idカラムが見つかりません");
        let (_, col_type) = id_column.unwrap();
        assert_eq!(col_type, "TEXT", "idカラムの型が不正です: {}", col_type);

        println!("\nusersテーブルのidカラム型: {}", col_type);

        // 外部キー参照を持つテーブルのuser_idカラムがTEXT型であることを確認
        let tables_to_check = ["expenses", "subscriptions", "sessions"];
        for table in &tables_to_check {
            let table_info: Vec<(String, String)> = conn
                .prepare(&format!("PRAGMA table_info({table})"))
                .unwrap()
                .query_map([], |row| {
                    Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

            let user_id_column = table_info.iter().find(|(name, _)| name == "user_id");
            assert!(
                user_id_column.is_some(),
                "{}テーブルにuser_idカラムが見つかりません",
                table
            );
            let (_, col_type) = user_id_column.unwrap();
            assert_eq!(
                col_type, "TEXT",
                "{}テーブルのuser_idカラムの型が不正です: {}",
                table, col_type
            );

            println!("{}テーブルのuser_idカラム型: {}", table, col_type);
        }

        // 外部キー整合性を確認（孤立したレコードがないこと）
        for table in &tables_to_check {
            let orphaned_count: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM {table} t 
                         LEFT JOIN users u ON t.user_id = u.id 
                         WHERE u.id IS NULL"
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();

            assert_eq!(
                orphaned_count, 0,
                "{}テーブルに孤立したレコードが {} 件存在します",
                table, orphaned_count
            );

            println!("{}テーブルの外部キー整合性: OK", table);
        }

        // インデックスが正しく作成されていることを確認
        let index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name='users'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        println!("\nusersテーブルのインデックス数: {}", index_count);
        assert!(
            index_count >= 2,
            "usersテーブルに必要なインデックスが不足しています"
        );

        println!("\n=== nanoIdマイグレーション統合テスト完了 ===\n");
    }

    #[test]
    fn test_nanoid_migration_with_empty_database() {
        println!("\n=== 空のデータベースでのnanoIdマイグレーションテスト開始 ===\n");

        let conn = Connection::open_in_memory().unwrap();

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

        // ユーザー認証マイグレーションを実行（デフォルトユーザーのみ）
        let result = migrate_user_authentication(&conn).unwrap();
        assert!(
            result.success,
            "ユーザー認証マイグレーション失敗: {}",
            result.message
        );

        // nanoIdマイグレーションを実行
        println!("nanoIdマイグレーションを実行中...");
        let result = migrate_user_id_to_nanoid(&conn).unwrap();

        println!("\nマイグレーション結果:");
        println!("  成功: {}", result.success);
        println!("  メッセージ: {}", result.message);

        assert!(result.success, "マイグレーション失敗: {}", result.message);

        // デフォルトユーザーのIDがnanoId形式であることを確認
        let user_id: String = conn
            .query_row("SELECT id FROM users WHERE id = 1", [], |row| row.get(0))
            .unwrap_or_else(|_| {
                // id=1が存在しない場合は、最初のユーザーを取得
                conn.query_row("SELECT id FROM users LIMIT 1", [], |row| row.get(0))
                    .unwrap()
            });

        println!("\nデフォルトユーザーのID: {}", user_id);
        assert!(is_valid_nanoid(&user_id), "無効なnanoId形式: {}", user_id);

        println!("\n=== 空のデータベースでのnanoIdマイグレーションテスト完了 ===\n");
    }

    #[test]
    fn test_nanoid_migration_preserves_relationships() {
        println!("\n=== nanoIdマイグレーションのリレーションシップ保持テスト開始 ===\n");

        let conn = create_test_db_with_users();

        // マイグレーション前のリレーションシップを記録
        let expenses_by_user: Vec<(i64, i64)> = conn
            .prepare("SELECT id, user_id FROM expenses ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        println!("マイグレーション前の経費とユーザーの関係:");
        for (expense_id, user_id) in &expenses_by_user {
            println!("  経費ID: {} -> ユーザーID: {}", expense_id, user_id);
        }

        // nanoIdマイグレーションを実行
        println!("\nnanoIdマイグレーションを実行中...");
        let result = migrate_user_id_to_nanoid(&conn).unwrap();
        assert!(result.success, "マイグレーション失敗: {}", result.message);

        // マイグレーション後のリレーションシップを確認
        println!("\nマイグレーション後の経費とユーザーの関係:");
        for (expense_id, old_user_id) in &expenses_by_user {
            let (new_user_id, user_email): (String, String) = conn
                .query_row(
                    "SELECT e.user_id, u.email 
                     FROM expenses e 
                     INNER JOIN users u ON e.user_id = u.id 
                     WHERE e.id = ?",
                    [expense_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();

            println!(
                "  経費ID: {} -> 新ユーザーID: {} (メール: {})",
                expense_id, new_user_id, user_email
            );

            // ユーザーIDがnanoId形式であることを確認
            assert!(
                is_valid_nanoid(&new_user_id),
                "無効なnanoId形式: {}",
                new_user_id
            );

            // リレーションシップが保持されていることを確認
            // （旧ユーザーID=1はデフォルトユーザー、旧ユーザーID=2はテストユーザー）
            if *old_user_id == 1 {
                assert_eq!(
                    user_email, "default@example.com",
                    "デフォルトユーザーとの関係が保持されていません"
                );
            } else if *old_user_id == 2 {
                assert_eq!(
                    user_email, "test@example.com",
                    "テストユーザーとの関係が保持されていません"
                );
            }
        }

        println!("\n=== nanoIdマイグレーションのリレーションシップ保持テスト完了 ===\n");
    }
}

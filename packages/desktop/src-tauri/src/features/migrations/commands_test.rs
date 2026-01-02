//! マイグレーションコマンドのテスト
//!
//! 新しく実装したマイグレーション状態確認コマンドのテストを行います。

#[cfg(test)]
mod tests {
    use super::super::commands::*;

    #[test]
    fn test_migration_info_creation() {
        let migration_info = MigrationInfo {
            name: "test_migration".to_string(),
            version: "1.0.0".to_string(),
            description: "テストマイグレーション".to_string(),
            checksum: "abc123".to_string(),
            is_applied: true,
            applied_at: Some("2024-01-01T00:00:00+09:00".to_string()),
            execution_time_ms: Some(1000),
        };

        assert_eq!(migration_info.name, "test_migration");
        assert!(migration_info.is_applied);
        assert!(migration_info.applied_at.is_some());
        assert_eq!(migration_info.execution_time_ms, Some(1000));
    }

    #[test]
    fn test_database_stats_with_migrations_count() {
        let stats = DatabaseStats {
            expenses_count: 10,
            subscriptions_count: 5,
            receipt_cache_count: 3,
            categories_count: 8,
            users_count: 2,
            sessions_count: 1,
            database_size_bytes: 1024000,
            page_count: 250,
            page_size: 4096,
            migrations_count: Some(3),
        };

        assert_eq!(stats.expenses_count, 10);
        assert_eq!(stats.migrations_count, Some(3));
        assert_eq!(stats.database_size_bytes, 1024000);
    }

    #[test]
    fn test_detailed_migration_info_structure() {
        use super::super::auto_migration::{AppliedMigration, MigrationStatusReport};

        let status_report = MigrationStatusReport::new(
            3,
            2,
            vec!["pending_migration".to_string()],
            Some("2024-01-01T00:00:00+09:00".to_string()),
            "1.0.0".to_string(),
        );

        let migration_info = MigrationInfo {
            name: "test_migration".to_string(),
            version: "1.0.0".to_string(),
            description: "テストマイグレーション".to_string(),
            checksum: "abc123".to_string(),
            is_applied: true,
            applied_at: Some("2024-01-01T00:00:00+09:00".to_string()),
            execution_time_ms: Some(1000),
        };

        let applied_migration = AppliedMigration::new(
            1,
            "applied_migration".to_string(),
            "1.0.0".to_string(),
            Some("適用済みマイグレーション".to_string()),
            "def456".to_string(),
            "2024-01-01T00:00:00+09:00".to_string(),
            Some(1500),
            "2024-01-01T00:00:00+09:00".to_string(),
        );

        let database_stats = DatabaseStats {
            expenses_count: 5,
            subscriptions_count: 3,
            receipt_cache_count: 2,
            categories_count: 4,
            users_count: 1,
            sessions_count: 1,
            database_size_bytes: 512000,
            page_count: 125,
            page_size: 4096,
            migrations_count: Some(2),
        };

        let detailed_info = DetailedMigrationInfo {
            status_report,
            available_migrations: vec![migration_info],
            applied_migrations: vec![applied_migration],
            integrity_status: "データベースの整合性に問題はありません".to_string(),
            database_stats,
        };

        assert_eq!(detailed_info.status_report.total_available, 3);
        assert_eq!(detailed_info.status_report.total_applied, 2);
        assert_eq!(detailed_info.available_migrations.len(), 1);
        assert_eq!(detailed_info.applied_migrations.len(), 1);
        assert!(detailed_info.integrity_status.contains("問題はありません"));
        assert_eq!(detailed_info.database_stats.migrations_count, Some(2));
    }
}

//! マイグレーション機能モジュール
//!
//! このモジュールは、データベーススキーマのマイグレーション、
//! バックアップの作成と復元、マイグレーション状態の確認を提供します。

pub mod commands;
pub mod service;

// 公開インターフェース
pub use commands::{
    check_database_integrity, check_migration_status, create_manual_backup,
    drop_receipt_path_column_command, execute_comprehensive_data_migration_command,
    execute_receipt_url_migration, execute_user_authentication_migration, get_database_stats,
    list_backup_files_command, restore_database_from_backup, DatabaseStats,
};

pub use service::{
    create_backup, drop_receipt_path_column, execute_comprehensive_data_migration,
    is_receipt_url_migration_complete, is_user_authentication_migration_complete,
    list_backup_files, migrate_receipt_path_to_url, migrate_user_authentication,
    restore_from_backup, run_migrations, DataMigrationResult, MigrationResult, MigrationStatus,
    RestoreResult,
};

/// マイグレーション機能の初期化
///
/// このモジュールは他のモジュールから独立して動作するため、
/// 特別な初期化処理は不要です。
pub fn init() {
    // 将来的にマイグレーション設定やログ設定が必要になった場合に使用
    println!("マイグレーション機能モジュールを初期化しました");
}

/// マイグレーション機能のバージョン情報
pub const VERSION: &str = "1.0.0";

/// サポートされているマイグレーション一覧
pub const SUPPORTED_MIGRATIONS: &[&str] = &[
    "receipt_path_to_receipt_url", // receipt_pathからreceipt_urlへの移行
    "drop_receipt_path_column",    // receipt_pathカラムの削除
    "add_user_authentication",     // ユーザー認証機能の追加
];

/// マイグレーション機能の説明
pub const DESCRIPTION: &str =
    "データベーススキーマのマイグレーション、バックアップ、復元機能を提供します";

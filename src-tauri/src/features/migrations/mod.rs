//! マイグレーション機能モジュール
//!
//! このモジュールは、データベーススキーマのマイグレーション、
//! バックアップの作成と復元、マイグレーション状態の確認を提供します。

pub mod auto_migration;
pub mod batch_processor;
pub mod commands;
pub mod database_update_commands;
pub mod database_updater;
pub mod r2_migration_commands;
pub mod r2_user_directory_migration;
pub mod service;

#[cfg(test)]
mod batch_processor_test;

#[cfg(test)]
mod commands_test;

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod error_scenario_tests;

#[cfg(test)]
mod r2_migration_commands_test;

// 公開インターフェース
pub use auto_migration::{
    AppliedMigration, AutoMigrationResult, AutoMigrationService, MigrationDefinition,
    MigrationError, MigrationErrorType, MigrationExecutionResult, MigrationExecutor,
    MigrationRegistry, MigrationStatusReport, MigrationTable,
};

pub use batch_processor::{
    BatchProcessor, BatchProcessorStats, MigrationItem, MigrationResult, R2FileInfo,
};

pub use commands::{
    check_auto_migration_status, check_database_integrity, check_migration_status,
    create_manual_backup, drop_receipt_path_column_command,
    execute_comprehensive_data_migration_command, execute_receipt_url_migration,
    execute_user_authentication_migration, get_database_stats, get_detailed_migration_info,
    list_backup_files_command, restore_database_from_backup, DatabaseStats, DetailedMigrationInfo,
    MigrationInfo,
};

pub use database_update_commands::{
    check_database_url_integrity, detect_legacy_receipt_urls, execute_database_update,
    get_database_statistics, update_specific_receipt_urls, DatabaseIntegrityResult,
    DatabaseUpdateParams, LegacyUrlDetectionResult,
};

pub use database_updater::{
    DatabaseStatistics, DatabaseUpdateResult, DatabaseUpdater, UrlUpdateItem,
};

pub use r2_migration_commands::{
    get_r2_migration_status, pause_r2_migration, resume_r2_migration, start_r2_migration,
    stop_r2_migration, validate_r2_migration_integrity, R2MigrationResult, R2MigrationStatus,
    StartR2MigrationParams, ValidationResult,
};

pub use r2_user_directory_migration::{
    create_migration_item, create_migration_log_entry, get_migration_progress,
    update_migration_item_status, update_migration_log_status, MigrationProgress,
};

pub use service::{
    create_backup, drop_receipt_path_column, execute_comprehensive_data_migration,
    is_receipt_url_migration_complete, is_user_authentication_migration_complete,
    list_backup_files, migrate_receipt_path_to_url, migrate_user_authentication,
    restore_from_backup, run_migrations, DataMigrationResult, MigrationStatus, RestoreResult,
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
    "r2_migration_schema",         // R2移行用データベーススキーマ
];

/// マイグレーション機能の説明
pub const DESCRIPTION: &str =
    "データベーススキーマのマイグレーション、バックアップ、復元機能を提供します";

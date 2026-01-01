//! マイグレーション機能モジュール
//!
//! このモジュールは、データベーススキーマのマイグレーション、
//! バックアップの作成と復元、マイグレーション状態の確認を提供します。

pub mod auto_migration;
pub mod batch_processor;
pub mod commands;
pub mod database_update_commands;
pub mod database_updater;
pub mod error_handler;
pub mod errors;
pub mod logging;
pub mod r2_migration_commands;
pub mod r2_user_directory_migration;
pub mod security_audit;
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

pub use error_handler::{
    get_global_error_handler, handle_migration_error, init_global_error_handler,
    ComprehensiveErrorHandler, ErrorHandlingResult, ErrorStatistics, RetryResult,
};

pub use errors::{ErrorAction, MigrationErrorHandler, RetryStrategy};

pub use logging::{
    get_global_logger, init_global_logger, log_migration_info, LogLevel, LogStatistics,
    StructuredLogEntry, StructuredLogger,
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

pub use security_audit::{
    get_global_security_logger, init_global_security_logger, log_migration_security_error,
    log_security_event, InvestigationStatus, SecurityAuditEntry, SecurityAuditLogger,
    SecurityAuditStatistics, SecurityEventType,
};

pub use service::{
    create_backup, drop_receipt_path_column, execute_comprehensive_data_migration,
    is_receipt_url_migration_complete, is_user_authentication_migration_complete,
    list_backup_files, migrate_receipt_path_to_url, migrate_user_authentication,
    restore_from_backup, run_migrations, DataMigrationResult, MigrationStatus, RestoreResult,
};

// 統合エラーハンドリングとログ機能の初期化
use crate::shared::errors::AppResult;
use log::info;

/// 移行機能の初期化
///
/// エラーハンドリング、ログ機能、セキュリティ監査機能を初期化します。
pub fn initialize_migration_system() -> AppResult<()> {
    info!("移行システムを初期化中...");

    // 構造化ログ機能を初期化
    logging::init_global_logger(Some(1000));
    info!("構造化ログ機能を初期化しました");

    // セキュリティ監査ログ機能を初期化
    if let Some(structured_logger) = logging::get_global_logger() {
        security_audit::init_global_security_logger(structured_logger, Some(5000));
        info!("セキュリティ監査ログ機能を初期化しました");
    }

    // 包括的エラーハンドラーを初期化
    let structured_logger = logging::get_global_logger();
    error_handler::init_global_error_handler(structured_logger);
    info!("包括的エラーハンドラーを初期化しました");

    info!("移行システムの初期化が完了しました");
    Ok(())
}

/// 移行システムの統計情報を取得
pub fn get_migration_system_statistics() -> MigrationSystemStatistics {
    let mut stats = MigrationSystemStatistics::default();

    // エラー統計を取得
    if let Some(error_handler) = error_handler::get_global_error_handler() {
        if let Ok(handler) = error_handler.lock() {
            stats.error_statistics = Some(handler.get_error_statistics());
        }
    }

    // ログ統計を取得
    if let Some(logger) = logging::get_global_logger() {
        if let Ok(logger) = logger.lock() {
            stats.log_statistics = Some(logger.get_log_statistics());
        }
    }

    // セキュリティ監査統計を取得
    if let Some(security_logger) = security_audit::get_global_security_logger() {
        if let Ok(logger) = security_logger.lock() {
            stats.security_audit_statistics = Some(logger.get_audit_statistics());
        }
    }

    stats
}

/// 移行システム統計
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MigrationSystemStatistics {
    /// エラー統計
    pub error_statistics: Option<error_handler::ErrorStatistics>,
    /// ログ統計
    pub log_statistics: Option<logging::LogStatistics>,
    /// セキュリティ監査統計
    pub security_audit_statistics: Option<security_audit::SecurityAuditStatistics>,
}

/// マイグレーション機能の初期化（従来の関数）
///
/// このモジュールは他のモジュールから独立して動作するため、
/// 特別な初期化処理は不要です。
pub fn init() {
    // 新しい統合初期化関数を呼び出し
    if let Err(e) = initialize_migration_system() {
        log::error!("移行システムの初期化に失敗しました: {}", e);
    }
}

/// マイグレーション機能のバージョン情報
pub const VERSION: &str = "1.1.0";

/// サポートされているマイグレーション一覧
pub const SUPPORTED_MIGRATIONS: &[&str] = &[
    "receipt_path_to_receipt_url", // receipt_pathからreceipt_urlへの移行
    "drop_receipt_path_column",    // receipt_pathカラムの削除
    "add_user_authentication",     // ユーザー認証機能の追加
    "r2_migration_schema",         // R2移行用データベーススキーマ
    "r2_user_directory_migration", // R2ユーザーディレクトリ移行
];

/// マイグレーション機能の説明
pub const DESCRIPTION: &str =
    "データベーススキーマのマイグレーション、バックアップ、復元機能、包括的エラーハンドリング、構造化ログ、セキュリティ監査機能を提供します";

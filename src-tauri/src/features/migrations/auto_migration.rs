//! 自動マイグレーションシステム
//!
//! このモジュールは、アプリケーション起動時に実行されていないマイグレーションを
//! 自動で適用するシステムを提供します。

pub mod errors;
pub mod executor;
pub mod models;
pub mod registry;
pub mod service;
pub mod table;

// 公開インターフェース
pub use models::{
    AppliedMigration, AutoMigrationResult, MigrationDefinition, MigrationExecutionResult,
    MigrationStatusReport,
};

pub use errors::{MigrationError, MigrationErrorType};

pub use executor::MigrationExecutor;
pub use registry::MigrationRegistry;
pub use service::AutoMigrationService;
pub use table::MigrationTable;

/// 自動マイグレーションシステムのバージョン
pub const VERSION: &str = "1.0.0";

/// 自動マイグレーションシステムの説明
pub const DESCRIPTION: &str = "アプリケーション起動時の自動マイグレーション実行システム";

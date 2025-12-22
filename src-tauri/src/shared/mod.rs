/// 共有エラー型とエラーハンドリング
pub mod errors;

/// 共有データベース接続管理
pub mod database;

/// 共有設定管理
pub mod config;

/// 共有ユーティリティ関数
pub mod utils;

// 便利な再エクスポート
pub use config::{
    get_database_filename, get_environment, initialize_application, initialize_logging_system,
    load_environment_variables, log_initialization_complete, Environment, EnvironmentConfig,
    InitializationResult,
};
pub use database::{create_tables, get_database_path, initialize_database};
pub use errors::{AppError, AppResult, ErrorSeverity};

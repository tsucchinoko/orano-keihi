/// 共有エラー型とエラーハンドリング
pub mod errors;

/// 共有データベース接続管理
pub mod database;

/// 共有設定管理
pub mod config;

/// 共有ユーティリティ関数
pub mod utils;

// 便利な再エクスポート
pub use errors::{AppError, AppResult, ErrorSeverity};
pub use database::{initialize_database, get_database_path, create_tables};
pub use config::{
    Environment, EnvironmentConfig, 
    get_environment, get_database_filename,
    load_environment_variables, initialize_logging_system,
    InitializationResult, initialize_application, log_initialization_complete
};

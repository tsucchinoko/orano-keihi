/// 環境設定関連のモジュール
pub mod environment;
/// アプリケーション初期化関連のモジュール
pub mod initialization;

// 便利な再エクスポート
pub use environment::{get_database_filename, get_environment, Environment};
pub use initialization::{initialize_application, log_initialization_complete};

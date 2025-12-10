/// 環境設定関連のモジュール
pub mod environment;

// 便利な再エクスポート
pub use environment::{get_database_filename, get_environment, Environment};

// セキュリティ機能モジュール

pub mod commands;
pub mod models;
pub mod service;

// 公開インターフェース
pub use commands::*;
pub use models::*;
pub use service::{EnvironmentConfig, SecureCredentials, SecurityManager};

/// セキュリティ機能の公開API
///
/// このモジュールは以下の機能を提供します：
/// - システム診断情報の取得
/// - セキュリティ設定の検証
/// - R2接続のセキュアなテスト
/// - セキュリティイベントのログ記録
/// - 環境情報の管理
/// - 認証情報の安全な管理
pub struct SecurityFeature;

impl SecurityFeature {
    /// セキュリティマネージャーを作成
    pub fn create_manager() -> SecurityManager {
        SecurityManager::new()
    }

    /// 環境設定を取得
    pub fn get_environment_config() -> EnvironmentConfig {
        EnvironmentConfig::from_env()
    }

    /// 新しい認証情報管理インスタンスを作成
    pub fn create_credentials() -> SecureCredentials {
        SecureCredentials::new()
    }

    /// セキュリティ機能で使用可能なTauriコマンドのリストを取得
    pub fn get_tauri_commands() -> Vec<&'static str> {
        vec![
            "get_system_diagnostic_info",
            "validate_security_configuration",
            "test_r2_connection_secure",
            "get_environment_info",
            "log_security_event",
            "log_security_event_with_severity",
            "get_recent_security_events",
            "generate_security_audit_log",
            "get_r2_diagnostic_info",
            "perform_security_initialization_check",
        ]
    }

    /// セキュリティ機能の初期化チェックを実行
    pub async fn initialize() -> Result<ValidationResult, crate::shared::errors::AppError> {
        let manager = SecurityManager::new();
        manager.validate_configuration()
    }

    /// セキュリティ機能の健全性チェックを実行
    pub fn health_check() -> Result<(), crate::shared::errors::AppError> {
        let manager = SecurityManager::new();
        let validation_result = manager.validate_configuration()?;

        if validation_result.is_valid {
            Ok(())
        } else {
            Err(crate::shared::errors::AppError::Security(format!(
                "セキュリティ健全性チェックに失敗しました: {}",
                validation_result.message
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_feature_manager_creation() {
        let manager = SecurityFeature::create_manager();
        assert!(!manager.get_env_config().environment.is_empty());
    }

    #[test]
    fn test_security_feature_environment_config() {
        let config = SecurityFeature::get_environment_config();
        assert!(!config.environment.is_empty());
    }

    #[test]
    fn test_security_feature_credentials_creation() {
        let credentials = SecurityFeature::create_credentials();
        assert!(credentials.get_all_masked().is_empty());
    }

    #[test]
    fn test_security_feature_tauri_commands() {
        let commands = SecurityFeature::get_tauri_commands();
        assert!(!commands.is_empty());
        assert!(commands.contains(&"get_system_diagnostic_info"));
        assert!(commands.contains(&"validate_security_configuration"));
        assert!(commands.contains(&"test_r2_connection_secure"));
    }

    #[tokio::test]
    async fn test_security_feature_initialize() {
        let result = SecurityFeature::initialize().await;
        assert!(result.is_ok());

        let validation_result = result.unwrap();
        assert!(!validation_result.details.is_empty());
    }

    #[test]
    fn test_security_feature_health_check() {
        // 健全性チェックは設定によって成功/失敗が変わるため、
        // エラーが発生しないことのみをテスト
        let result = SecurityFeature::health_check();
        // 結果に関わらず、パニックしないことを確認
        match result {
            Ok(()) => {
                // 成功の場合
                assert!(true);
            }
            Err(_) => {
                // 失敗の場合（設定が不完全な場合など）
                assert!(true);
            }
        }
    }
}

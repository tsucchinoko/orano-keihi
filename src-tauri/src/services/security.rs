// セキュリティ機能モジュール

use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::env;

/// 認証情報を安全に管理するための構造体
#[derive(Debug, Clone)]
pub struct SecureCredentials {
    /// マスクされた認証情報（ログ出力用）
    masked_credentials: HashMap<String, String>,
    /// 実際の認証情報（内部使用のみ）
    actual_credentials: HashMap<String, String>,
}

impl Default for SecureCredentials {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureCredentials {
    /// 新しいSecureCredentialsインスタンスを作成
    pub fn new() -> Self {
        Self {
            masked_credentials: HashMap::new(),
            actual_credentials: HashMap::new(),
        }
    }

    /// 認証情報を追加（自動的にマスク処理）
    pub fn add_credential(&mut self, key: &str, value: &str) {
        // 実際の値を保存
        self.actual_credentials
            .insert(key.to_string(), value.to_string());

        // マスクされた値を保存（最初の4文字と最後の4文字のみ表示）
        let masked_value = if value.len() > 8 {
            format!("{}****{}", &value[..4], &value[value.len() - 4..])
        } else if value.len() > 4 {
            format!("{}****", &value[..2])
        } else {
            "****".to_string()
        };

        self.masked_credentials
            .insert(key.to_string(), masked_value);

        info!(
            "認証情報を追加しました: {} = {}",
            key,
            self.masked_credentials.get(key).unwrap()
        );
    }

    /// 実際の認証情報を取得
    pub fn get_credential(&self, key: &str) -> Option<&String> {
        self.actual_credentials.get(key)
    }

    /// マスクされた認証情報を取得（ログ出力用）
    pub fn get_masked_credential(&self, key: &str) -> Option<&String> {
        self.masked_credentials.get(key)
    }

    /// すべてのマスクされた認証情報を取得
    pub fn get_all_masked(&self) -> &HashMap<String, String> {
        &self.masked_credentials
    }

    /// 認証情報の検証
    pub fn validate_all(&self) -> Result<(), String> {
        let required_keys = [
            "R2_ACCOUNT_ID",
            "R2_ACCESS_KEY",
            "R2_SECRET_KEY",
            "R2_BUCKET_NAME",
        ];

        for key in &required_keys {
            if let Some(value) = self.actual_credentials.get(*key) {
                if value.is_empty() {
                    let error_msg = format!("必須の認証情報が空です: {key}");
                    error!("{error_msg}");
                    return Err(error_msg);
                }
            } else {
                let error_msg = format!("必須の認証情報が見つかりません: {key}");
                error!("{error_msg}");
                return Err(error_msg);
            }
        }

        info!("すべての認証情報の検証が完了しました");
        Ok(())
    }
}

/// 環境別設定管理
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    pub environment: String,
    pub debug_mode: bool,
    pub log_level: String,
}

impl EnvironmentConfig {
    /// 環境変数から設定を読み込み
    pub fn from_env() -> Self {
        // コンパイル時埋め込み値を優先し、見つからない場合は実行時環境変数を使用
        let environment = option_env!("EMBEDDED_ENVIRONMENT")
            .map(|s| {
                info!("コンパイル時埋め込み環境設定を使用: {s}");
                s.to_string()
            })
            .or_else(|| {
                env::var("ENVIRONMENT").ok().map(|s| {
                    info!("実行時環境変数を使用: ENVIRONMENT={s}");
                    s
                })
            })
            .unwrap_or_else(|| {
                info!("デフォルト環境設定を使用: development");
                "development".to_string()
            });

        let debug_mode = if environment == "production" {
            // 本番環境では強制的にデバッグモードを無効化
            false
        } else {
            env::var("DEBUG")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false)
        };
        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| {
            if environment == "production" {
                "info".to_string()
            } else {
                "debug".to_string()
            }
        });

        info!(
            "環境設定を読み込みました: environment={environment}, debug_mode={debug_mode}, log_level={log_level}"
        );

        Self {
            environment,
            debug_mode,
            log_level,
        }
    }

    /// 本番環境かどうかを判定
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    /// 開発環境かどうかを判定
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    /// デバッグモードが有効かどうかを判定
    pub fn is_debug_enabled(&self) -> bool {
        self.debug_mode
    }
}

/// セキュリティマネージャー
#[derive(Clone)]
pub struct SecurityManager {
    credentials: SecureCredentials,
    env_config: EnvironmentConfig,
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityManager {
    /// 新しいSecurityManagerインスタンスを作成
    pub fn new() -> Self {
        info!("セキュリティマネージャーを初期化しています...");

        let mut credentials = SecureCredentials::new();
        let env_config = EnvironmentConfig::from_env();

        // 環境変数から認証情報を読み込み
        // コンパイル時埋め込み値を優先し、見つからない場合は実行時環境変数を使用

        // R2_ACCOUNT_ID
        let account_id = option_env!("EMBEDDED_R2_ACCOUNT_ID")
            .map(|s| s.to_string())
            .or_else(|| env::var("R2_ACCOUNT_ID").ok())
            .unwrap_or_default();
        if !account_id.is_empty() {
            credentials.add_credential("R2_ACCOUNT_ID", &account_id);
        }

        // R2_ACCESS_KEY
        let access_key = option_env!("EMBEDDED_R2_ACCESS_KEY")
            .map(|s| s.to_string())
            .or_else(|| env::var("R2_ACCESS_KEY").ok())
            .unwrap_or_default();
        if !access_key.is_empty() {
            credentials.add_credential("R2_ACCESS_KEY", &access_key);
        }

        // R2_SECRET_KEY
        let secret_key = option_env!("EMBEDDED_R2_SECRET_KEY")
            .map(|s| s.to_string())
            .or_else(|| env::var("R2_SECRET_KEY").ok())
            .unwrap_or_default();
        if !secret_key.is_empty() {
            credentials.add_credential("R2_SECRET_KEY", &secret_key);
        }

        // R2_BUCKET_NAME
        let bucket_name = option_env!("EMBEDDED_R2_BUCKET_NAME")
            .map(|s| s.to_string())
            .or_else(|| env::var("R2_BUCKET_NAME").ok())
            .unwrap_or_default();
        if !bucket_name.is_empty() {
            credentials.add_credential("R2_BUCKET_NAME", &bucket_name);
        }

        info!("セキュリティマネージャーの初期化が完了しました");

        Self {
            credentials,
            env_config,
        }
    }

    /// 認証情報を取得
    pub fn get_credentials(&self) -> &SecureCredentials {
        &self.credentials
    }

    /// 環境設定を取得
    pub fn get_env_config(&self) -> &EnvironmentConfig {
        &self.env_config
    }

    /// システム診断情報を取得
    pub fn get_diagnostic_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();

        info.insert(
            "environment".to_string(),
            self.env_config.environment.clone(),
        );
        info.insert(
            "debug_mode".to_string(),
            self.env_config.debug_mode.to_string(),
        );
        info.insert("log_level".to_string(), self.env_config.log_level.clone());

        // マスクされた認証情報を追加
        for (key, value) in self.credentials.get_all_masked() {
            info.insert(format!("credential_{}", key.to_lowercase()), value.clone());
        }

        // システム情報を追加
        info.insert(
            "rust_version".to_string(),
            env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string()),
        );
        info.insert("target_arch".to_string(), env::consts::ARCH.to_string());
        info.insert("target_os".to_string(), env::consts::OS.to_string());

        debug!("診断情報を生成しました: {info:?}");
        info
    }

    /// 設定の検証
    pub fn validate_configuration(&self) -> Result<(), String> {
        info!("設定の検証を開始します...");

        // 認証情報の検証
        self.credentials.validate_all()?;

        // 環境設定の検証
        if self.env_config.environment.is_empty() {
            let error_msg = "環境設定が空です".to_string();
            error!("{error_msg}");
            return Err(error_msg);
        }

        info!("設定の検証が完了しました");
        Ok(())
    }

    /// セキュリティ監査ログを出力
    pub fn log_security_event(&self, event_type: &str, details: &str) {
        let masked_info = self.credentials.get_all_masked();
        warn!(
            "セキュリティイベント: type={event_type}, details={details}, credentials={masked_info:?}"
        );
    }

    /// 最近のセキュリティイベントを取得（簡易実装）
    pub fn get_recent_security_events(&self, _limit: usize) -> Vec<String> {
        // 簡易実装：実際のログファイルから読み取る代わりに、
        // 一般的なセキュリティイベントの例を返す
        vec![
            "app_init_success: アプリケーション初期化完了".to_string(),
            "database_init_success: データベース初期化完了".to_string(),
            "r2_connection_test_success: R2接続テスト成功".to_string(),
            "config_validation_success: 設定検証成功".to_string(),
            "security_manager_init: セキュリティマネージャー初期化".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_credentials_masking() {
        let mut credentials = SecureCredentials::new();
        credentials.add_credential("TEST_KEY", "abcdefghijklmnop");

        assert_eq!(
            credentials.get_credential("TEST_KEY"),
            Some(&"abcdefghijklmnop".to_string())
        );
        assert_eq!(
            credentials.get_masked_credential("TEST_KEY"),
            Some(&"abcd****mnop".to_string())
        );
    }

    #[test]
    fn test_short_credential_masking() {
        let mut credentials = SecureCredentials::new();
        credentials.add_credential("SHORT", "abc");

        assert_eq!(
            credentials.get_masked_credential("SHORT"),
            Some(&"****".to_string())
        );
    }

    #[test]
    fn test_credential_validation() {
        let mut credentials = SecureCredentials::new();
        credentials.add_credential("R2_ACCOUNT_ID", "test_account");
        credentials.add_credential("R2_ACCESS_KEY", "test_key");
        credentials.add_credential("R2_SECRET_KEY", "test_secret");
        credentials.add_credential("R2_BUCKET_NAME", "test_bucket");

        assert!(credentials.validate_all().is_ok());
    }

    #[test]
    fn test_missing_credential_validation() {
        let credentials = SecureCredentials::new();
        assert!(credentials.validate_all().is_err());
    }

    #[test]
    fn test_environment_config() {
        let config = EnvironmentConfig::from_env();
        assert!(!config.environment.is_empty());
    }
}

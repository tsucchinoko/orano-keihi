// セキュリティサービス

use super::models::*;
use crate::shared::errors::AppError;
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
    pub fn validate_all(&self) -> Result<ValidationResult, AppError> {
        let required_keys = [
            "R2_ACCOUNT_ID",
            "R2_ACCESS_KEY_ID",
            "R2_SECRET_ACCESS_KEY",
            "R2_BUCKET_NAME",
        ];

        let mut details = Vec::new();
        let mut has_errors = false;

        for key in &required_keys {
            if let Some(value) = self.actual_credentials.get(*key) {
                if value.is_empty() {
                    let message = format!("必須の認証情報が空です: {key}");
                    error!("{message}");
                    details.push(ValidationDetail::new(
                        key.to_string(),
                        ValidationStatus::Error,
                        message,
                    ));
                    has_errors = true;
                } else {
                    details.push(ValidationDetail::new(
                        key.to_string(),
                        ValidationStatus::Success,
                        "認証情報が正常に設定されています".to_string(),
                    ));
                }
            } else {
                let message = format!("必須の認証情報が見つかりません: {key}");
                error!("{message}");
                details.push(ValidationDetail::new(
                    key.to_string(),
                    ValidationStatus::Error,
                    message,
                ));
                has_errors = true;
            }
        }

        if has_errors {
            Ok(ValidationResult::failure(
                "認証情報の検証に失敗しました".to_string(),
                details,
            ))
        } else {
            info!("すべての認証情報の検証が完了しました");
            Ok(ValidationResult::success(
                "すべての認証情報が正常に設定されています".to_string(),
                details,
            ))
        }
    }
}

/// 環境別設定管理
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    pub environment: String,
    pub debug_mode: bool,
    pub log_level: String,
    pub config_source: ConfigSource,
}

impl EnvironmentConfig {
    /// 環境変数から設定を読み込み
    pub fn from_env() -> Self {
        // コンパイル時埋め込み値を優先し、見つからない場合は実行時環境変数を使用
        let (environment, env_source) = option_env!("EMBEDDED_ENVIRONMENT")
            .map(|s| {
                info!("コンパイル時埋め込み環境設定を使用: {s}");
                (s.to_string(), ConfigSource::Embedded)
            })
            .or_else(|| {
                env::var("ENVIRONMENT").ok().map(|s| {
                    info!("実行時環境変数を使用: ENVIRONMENT={s}");
                    (s, ConfigSource::Environment)
                })
            })
            .unwrap_or_else(|| {
                info!("デフォルト環境設定を使用: development");
                ("development".to_string(), ConfigSource::Default)
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
            config_source: env_source,
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

    /// 環境情報を取得
    pub fn to_environment_info(&self) -> EnvironmentInfo {
        EnvironmentInfo {
            environment: self.environment.clone(),
            debug_mode: self.debug_mode,
            log_level: self.log_level.clone(),
            is_production: self.is_production(),
            is_development: self.is_development(),
            config_source: self.config_source.clone(),
        }
    }
}

/// セキュリティマネージャー
#[derive(Clone)]
pub struct SecurityManager {
    credentials: SecureCredentials,
    env_config: EnvironmentConfig,
    security_events: Vec<SecurityEvent>,
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

        // R2_ACCESS_KEY_ID
        let access_key = option_env!("EMBEDDED_R2_ACCESS_KEY_ID")
            .map(|s| s.to_string())
            .or_else(|| env::var("R2_ACCESS_KEY_ID").ok())
            .unwrap_or_default();
        if !access_key.is_empty() {
            credentials.add_credential("R2_ACCESS_KEY_ID", &access_key);
        }

        // R2_SECRET_ACCESS_KEY
        let secret_key = option_env!("EMBEDDED_R2_SECRET_ACCESS_KEY")
            .map(|s| s.to_string())
            .or_else(|| env::var("R2_SECRET_ACCESS_KEY").ok())
            .unwrap_or_default();
        if !secret_key.is_empty() {
            credentials.add_credential("R2_SECRET_ACCESS_KEY", &secret_key);
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
            security_events: Vec::new(),
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
    pub fn get_diagnostic_info(&self) -> DiagnosticInfo {
        let system_info = SystemInfo::new(
            env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string()),
            env::consts::ARCH.to_string(),
            env::consts::OS.to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        );

        let mut diagnostic_info = DiagnosticInfo::new(
            self.env_config.environment.clone(),
            self.env_config.debug_mode,
            self.env_config.log_level.clone(),
            self.credentials.get_all_masked().clone(),
            system_info,
        );

        // 設定検証を実行
        match self.credentials.validate_all() {
            Ok(validation_result) => {
                if validation_result.is_valid {
                    diagnostic_info.set_validation_status(ValidationStatus::Success);
                } else {
                    diagnostic_info.set_validation_status(ValidationStatus::Error);
                }
            }
            Err(_) => {
                diagnostic_info.set_validation_status(ValidationStatus::Error);
            }
        }

        debug!("診断情報を生成しました: {diagnostic_info:?}");
        diagnostic_info
    }

    /// 設定の検証
    pub fn validate_configuration(&self) -> Result<ValidationResult, AppError> {
        info!("設定の検証を開始します...");

        // 認証情報の検証
        let credential_result = self.credentials.validate_all()?;

        // 環境設定の検証
        let mut env_details = Vec::new();
        if self.env_config.environment.is_empty() {
            env_details.push(ValidationDetail::new(
                "environment".to_string(),
                ValidationStatus::Error,
                "環境設定が空です".to_string(),
            ));
        } else {
            env_details.push(ValidationDetail::new(
                "environment".to_string(),
                ValidationStatus::Success,
                format!("環境設定: {}", self.env_config.environment),
            ));
        }

        // 結果をマージ
        let mut all_details = credential_result.details;
        all_details.extend(env_details);

        let has_errors = all_details
            .iter()
            .any(|d| d.status == ValidationStatus::Error);

        if has_errors {
            let result =
                ValidationResult::failure("設定の検証に失敗しました".to_string(), all_details);
            warn!("設定の検証に失敗しました: {result:?}");
            Ok(result)
        } else {
            info!("設定の検証が完了しました");
            Ok(ValidationResult::success(
                "すべての設定が正常です".to_string(),
                all_details,
            ))
        }
    }

    /// セキュリティイベントをログに記録
    pub fn log_security_event(&mut self, event_type: &str, details: &str) {
        let event = SecurityEvent::new(
            event_type.to_string(),
            details.to_string(),
            EventSeverity::Info,
            None,
        );

        warn!(
            "セキュリティイベント: type={event_type}, details={details}, credentials={:?}",
            self.credentials.get_all_masked()
        );

        self.security_events.push(event);
    }

    /// セキュリティイベントをログに記録（重要度指定）
    pub fn log_security_event_with_severity(
        &mut self,
        event_type: &str,
        details: &str,
        severity: EventSeverity,
    ) {
        let event = SecurityEvent::new(
            event_type.to_string(),
            details.to_string(),
            severity.clone(),
            None,
        );

        match severity {
            EventSeverity::Info => {
                info!("セキュリティイベント: type={event_type}, details={details}")
            }
            EventSeverity::Warning => {
                warn!("セキュリティイベント: type={event_type}, details={details}")
            }
            EventSeverity::Error | EventSeverity::Critical => {
                error!("セキュリティイベント: type={event_type}, details={details}")
            }
        }

        self.security_events.push(event);
    }

    /// 最近のセキュリティイベントを取得
    pub fn get_recent_security_events(&self, limit: usize) -> Vec<SecurityEvent> {
        self.security_events
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// セキュリティ監査ログを生成
    pub fn generate_audit_log(&self, period_start: String, period_end: String) -> SecurityAuditLog {
        use chrono::Utc;
        use chrono_tz::Asia::Tokyo;

        let now_jst = Utc::now().with_timezone(&Tokyo);
        let generated_at = now_jst.to_rfc3339();

        SecurityAuditLog {
            entries: self.security_events.clone(),
            generated_at,
            period_start,
            period_end,
            total_events: self.security_events.len(),
        }
    }

    /// 本番環境かどうかを判定
    pub fn is_production(&self) -> bool {
        self.env_config.is_production()
    }

    /// 開発環境かどうかを判定
    pub fn is_development(&self) -> bool {
        self.env_config.is_development()
    }

    /// 環境情報を取得
    pub fn get_environment_info(&self) -> EnvironmentInfo {
        self.env_config.to_environment_info()
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
        credentials.add_credential("R2_ACCESS_KEY_ID", "test_key");
        credentials.add_credential("R2_SECRET_ACCESS_KEY", "test_secret");
        credentials.add_credential("R2_BUCKET_NAME", "test_bucket");

        let result = credentials.validate_all().unwrap();
        assert!(result.is_valid);
    }

    #[test]
    fn test_missing_credential_validation() {
        let credentials = SecureCredentials::new();
        let result = credentials.validate_all().unwrap();
        assert!(!result.is_valid);
    }

    #[test]
    fn test_environment_config() {
        let config = EnvironmentConfig::from_env();
        assert!(!config.environment.is_empty());
    }

    #[test]
    fn test_security_manager_creation() {
        let manager = SecurityManager::new();
        assert!(!manager.env_config.environment.is_empty());
    }

    #[test]
    fn test_security_event_logging() {
        let mut manager = SecurityManager::new();
        manager.log_security_event("test_event", "テストイベントの詳細");

        let events = manager.get_recent_security_events(10);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "test_event");
        assert_eq!(events[0].details, "テストイベントの詳細");
    }

    #[test]
    fn test_diagnostic_info_generation() {
        let manager = SecurityManager::new();
        let diagnostic_info = manager.get_diagnostic_info();

        assert!(!diagnostic_info.environment.is_empty());
        assert!(!diagnostic_info.system_info.app_version.is_empty());
    }
}

// R2設定管理モジュール

use super::ConfigError;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R2Config {
    pub account_id: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket_name: String,
    pub region: String,
}

impl R2Config {
    /// 環境変数から設定を読み込み
    pub fn from_env() -> Result<Self, ConfigError> {
        // 環境変数から設定を読み込み
        let account_id = env::var("R2_ACCOUNT_ID").map_err(|_| ConfigError::MissingAccountId)?;

        let access_key = env::var("R2_ACCESS_KEY").map_err(|_| ConfigError::MissingAccessKey)?;

        let secret_key = env::var("R2_SECRET_KEY").map_err(|_| ConfigError::MissingSecretKey)?;

        let bucket_name = env::var("R2_BUCKET_NAME").map_err(|_| ConfigError::MissingBucketName)?;

        let region = env::var("R2_REGION").unwrap_or_else(|_| "auto".to_string());

        Ok(Self {
            account_id,
            access_key,
            secret_key,
            bucket_name,
            region,
        })
    }

    /// 設定の検証
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.account_id.is_empty() {
            return Err(ConfigError::MissingAccountId);
        }

        if self.access_key.is_empty() {
            return Err(ConfigError::MissingAccessKey);
        }

        if self.secret_key.is_empty() {
            return Err(ConfigError::MissingSecretKey);
        }

        if self.bucket_name.is_empty() {
            return Err(ConfigError::MissingBucketName);
        }

        Ok(())
    }

    /// R2エンドポイントURLを生成
    pub fn endpoint_url(&self) -> String {
        format!("https://{}.r2.cloudflarestorage.com", self.account_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = R2Config {
            account_id: "test_account".to_string(),
            access_key: "test_key".to_string(),
            secret_key: "test_secret".to_string(),
            bucket_name: "test_bucket".to_string(),
            region: "auto".to_string(),
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_empty_account_id_validation() {
        let config = R2Config {
            account_id: "".to_string(),
            access_key: "test_key".to_string(),
            secret_key: "test_secret".to_string(),
            bucket_name: "test_bucket".to_string(),
            region: "auto".to_string(),
        };

        assert!(matches!(
            config.validate(),
            Err(ConfigError::MissingAccountId)
        ));
    }

    #[test]
    fn test_endpoint_url_generation() {
        let config = R2Config {
            account_id: "test_account".to_string(),
            access_key: "test_key".to_string(),
            secret_key: "test_secret".to_string(),
            bucket_name: "test_bucket".to_string(),
            region: "auto".to_string(),
        };

        let expected_url = "https://test_account.r2.cloudflarestorage.com";
        assert_eq!(config.endpoint_url(), expected_url);
    }
}

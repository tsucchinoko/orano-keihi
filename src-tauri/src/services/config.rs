// R2設定管理モジュール

use super::{ConfigError, security::SecurityManager};
use log::{debug, error, info, warn};
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
    /// 環境変数から設定を読み込み（セキュリティマネージャー使用）
    pub fn from_env() -> Result<Self, ConfigError> {
        info!("R2設定を環境変数から読み込み中...");
        
        // セキュリティマネージャーを使用して安全に認証情報を取得
        let security_manager = SecurityManager::new();
        let credentials = security_manager.get_credentials();
        
        // 設定の検証
        security_manager.validate_configuration()
            .map_err(|e| ConfigError::LoadFailed(e))?;

        let account_id = credentials.get_credential("R2_ACCOUNT_ID")
            .ok_or(ConfigError::MissingAccountId)?
            .clone();

        let access_key = credentials.get_credential("R2_ACCESS_KEY")
            .ok_or(ConfigError::MissingAccessKey)?
            .clone();

        let secret_key = credentials.get_credential("R2_SECRET_KEY")
            .ok_or(ConfigError::MissingSecretKey)?
            .clone();

        let bucket_name = credentials.get_credential("R2_BUCKET_NAME")
            .ok_or(ConfigError::MissingBucketName)?
            .clone();

        let region = env::var("R2_REGION").unwrap_or_else(|_| "auto".to_string());

        let config = Self {
            account_id,
            access_key,
            secret_key,
            bucket_name,
            region,
        };

        info!("R2設定の読み込みが完了しました");
        debug!("設定詳細: account_id={}, bucket_name={}, region={}", 
               credentials.get_masked_credential("R2_ACCOUNT_ID").unwrap_or(&"****".to_string()),
               &config.bucket_name,
               &config.region);

        Ok(config)
    }

    /// 環境変数から設定を読み込み（従来版、後方互換性のため）
    pub fn from_env_legacy() -> Result<Self, ConfigError> {
        warn!("従来の設定読み込み方法を使用しています。セキュリティ機能が制限されます。");
        
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

    /// 設定の検証（セキュリティ強化版）
    pub fn validate(&self) -> Result<(), ConfigError> {
        info!("R2設定の検証を開始します...");
        
        if self.account_id.is_empty() {
            error!("アカウントIDが空です");
            return Err(ConfigError::MissingAccountId);
        }

        if self.access_key.is_empty() {
            error!("アクセスキーが空です");
            return Err(ConfigError::MissingAccessKey);
        }

        if self.secret_key.is_empty() {
            error!("シークレットキーが空です");
            return Err(ConfigError::MissingSecretKey);
        }

        if self.bucket_name.is_empty() {
            error!("バケット名が空です");
            return Err(ConfigError::MissingBucketName);
        }

        // セキュリティチェック
        if self.access_key.len() < 16 {
            warn!("アクセスキーが短すぎる可能性があります");
        }

        if self.secret_key.len() < 32 {
            warn!("シークレットキーが短すぎる可能性があります");
        }

        // バケット名の形式チェック
        if !self.bucket_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            warn!("バケット名に無効な文字が含まれている可能性があります");
        }

        info!("R2設定の検証が完了しました");
        Ok(())
    }

    /// R2エンドポイントURLを生成
    pub fn endpoint_url(&self) -> String {
        let url = format!("https://{}.r2.cloudflarestorage.com", self.account_id);
        debug!("R2エンドポイントURL: {}", url);
        url
    }

    /// 環境別のバケット名を取得
    pub fn get_environment_bucket_name(&self) -> String {
        // 開発環境でも本番と同じバケット名を使用（一時的な修正）
        let bucket_name = self.bucket_name.clone();
        
        let env = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        info!("バケット名: {} (環境: {})", bucket_name, env);
        bucket_name
    }

    /// デバッグ情報を取得（認証情報はマスク）
    pub fn get_debug_info(&self) -> std::collections::HashMap<String, String> {
        let mut info = std::collections::HashMap::new();
        
        // マスクされた認証情報
        info.insert("account_id".to_string(), self.mask_account_id());
        info.insert("access_key".to_string(), self.mask_access_key());
        info.insert("bucket_name".to_string(), self.bucket_name.clone());
        info.insert("region".to_string(), self.region.clone());
        info.insert("endpoint_url".to_string(), self.endpoint_url());
        
        debug!("R2設定デバッグ情報: {:?}", info);
        info
    }

    /// アカウントIDをマスク
    fn mask_account_id(&self) -> String {
        if self.account_id.len() > 8 {
            format!("{}****{}", &self.account_id[..4], &self.account_id[self.account_id.len()-4..])
        } else {
            "****".to_string()
        }
    }

    /// アクセスキーをマスク
    fn mask_access_key(&self) -> String {
        if self.access_key.len() > 8 {
            format!("{}****{}", &self.access_key[..4], &self.access_key[self.access_key.len()-4..])
        } else {
            "****".to_string()
        }
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

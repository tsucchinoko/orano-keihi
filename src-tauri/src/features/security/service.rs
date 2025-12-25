use crate::features::security::encryption::{EncryptionError, TokenEncryption};
use crate::features::security::models::{SecurityConfig, SecurityError, TokenInfo};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// セキュリティサービス
/// 認証トークンの暗号化、セキュアな保存、アクセス制御を管理する
#[derive(Clone)]
pub struct SecurityService {
    /// トークン暗号化サービス
    token_encryption: TokenEncryption,
    /// セキュリティ設定
    config: SecurityConfig,
    /// アクティブなトークンのキャッシュ
    token_cache: Arc<Mutex<HashMap<String, TokenInfo>>>,
}

impl SecurityService {
    /// 新しいSecurityServiceを作成する
    ///
    /// # 引数
    /// * `config` - セキュリティ設定
    ///
    /// # 戻り値
    /// SecurityServiceインスタンス
    pub fn new(config: SecurityConfig) -> Result<Self, SecurityError> {
        let token_encryption = TokenEncryption::new(config.encryption_key.clone())
            .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;

        Ok(Self {
            token_encryption,
            config,
            token_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// 認証トークンを暗号化して保存する
    ///
    /// # 引数
    /// * `token_id` - トークンID
    /// * `token` - 暗号化するトークン
    ///
    /// # 戻り値
    /// 暗号化されたトークン
    pub fn encrypt_and_store_token(
        &self,
        token_id: &str,
        token: &str,
    ) -> Result<String, SecurityError> {
        log::debug!("トークンを暗号化して保存: token_id={token_id}");

        // トークンを暗号化
        let encrypted_token = self
            .token_encryption
            .encrypt_token(token)
            .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;

        // トークン情報をキャッシュに保存
        let token_info = TokenInfo {
            token_id: token_id.to_string(),
            encrypted_token: encrypted_token.clone(),
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 0,
        };

        {
            let mut cache = self.token_cache.lock().unwrap();
            cache.insert(token_id.to_string(), token_info);
        }

        log::info!("トークンを暗号化して保存しました: token_id={token_id}");
        Ok(encrypted_token)
    }

    /// 暗号化されたトークンを復号化する
    ///
    /// # 引数
    /// * `token_id` - トークンID
    /// * `encrypted_token` - 暗号化されたトークン
    ///
    /// # 戻り値
    /// 復号化されたトークン
    pub fn decrypt_token(
        &self,
        token_id: &str,
        encrypted_token: &str,
    ) -> Result<String, SecurityError> {
        log::debug!("トークンを復号化: token_id={token_id}");

        // トークンを復号化
        let decrypted_token = self
            .token_encryption
            .decrypt_token(encrypted_token)
            .map_err(|e| SecurityError::DecryptionError(e.to_string()))?;

        // アクセス情報を更新
        {
            let mut cache = self.token_cache.lock().unwrap();
            if let Some(token_info) = cache.get_mut(token_id) {
                token_info.last_accessed = chrono::Utc::now();
                token_info.access_count += 1;
            }
        }

        log::debug!("トークンを復号化しました: token_id={token_id}");
        Ok(decrypted_token)
    }

    /// 複数のトークンを一括暗号化する
    ///
    /// # 引数
    /// * `tokens` - 暗号化するトークンのマップ
    ///
    /// # 戻り値
    /// 暗号化されたトークンのマップ
    pub fn encrypt_multiple_tokens(
        &self,
        tokens: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, SecurityError> {
        log::debug!("複数のトークンを一括暗号化: count={}", tokens.len());

        let encrypted_tokens = self
            .token_encryption
            .encrypt_tokens(tokens)
            .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;

        // キャッシュに保存
        {
            let mut cache = self.token_cache.lock().unwrap();
            for (token_id, encrypted_token) in &encrypted_tokens {
                let token_info = TokenInfo {
                    token_id: token_id.clone(),
                    encrypted_token: encrypted_token.clone(),
                    created_at: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                };
                cache.insert(token_id.clone(), token_info);
            }
        }

        log::info!(
            "複数のトークンを一括暗号化しました: count={}",
            encrypted_tokens.len()
        );
        Ok(encrypted_tokens)
    }

    /// 複数のトークンを一括復号化する
    ///
    /// # 引数
    /// * `encrypted_tokens` - 暗号化されたトークンのマップ
    ///
    /// # 戻り値
    /// 復号化されたトークンのマップ
    pub fn decrypt_multiple_tokens(
        &self,
        encrypted_tokens: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, SecurityError> {
        log::debug!(
            "複数のトークンを一括復号化: count={}",
            encrypted_tokens.len()
        );

        let decrypted_tokens = self
            .token_encryption
            .decrypt_tokens(encrypted_tokens)
            .map_err(|e| SecurityError::DecryptionError(e.to_string()))?;

        // アクセス情報を更新
        {
            let mut cache = self.token_cache.lock().unwrap();
            for token_id in encrypted_tokens.keys() {
                if let Some(token_info) = cache.get_mut(token_id) {
                    token_info.last_accessed = chrono::Utc::now();
                    token_info.access_count += 1;
                }
            }
        }

        log::info!(
            "複数のトークンを一括復号化しました: count={}",
            decrypted_tokens.len()
        );
        Ok(decrypted_tokens)
    }

    /// APIリクエストの認証を検証する
    ///
    /// # 引数
    /// * `token` - 認証トークン
    ///
    /// # 戻り値
    /// 検証結果
    pub fn verify_api_request(&self, token: &str) -> Result<bool, SecurityError> {
        log::debug!("APIリクエストの認証を検証");

        if token.is_empty() {
            log::warn!("空のトークンでAPIリクエスト認証を試行");
            return Ok(false);
        }

        // トークンの形式を検証（Base64エンコードされた暗号化トークンかチェック）
        match self.token_encryption.decrypt_token(token) {
            Ok(_) => {
                log::debug!("APIリクエストの認証が成功しました");
                Ok(true)
            }
            Err(e) => {
                log::warn!("APIリクエストの認証に失敗しました: {e}");
                Ok(false)
            }
        }
    }

    /// 不正アクセスを検出して処理する
    ///
    /// # 引数
    /// * `request_info` - リクエスト情報
    /// * `token` - 認証トークン（オプション）
    ///
    /// # 戻り値
    /// 処理結果
    pub fn detect_unauthorized_access(
        &self,
        request_info: &str,
        token: Option<&str>,
    ) -> Result<(), SecurityError> {
        log::warn!("不正アクセスを検出しました: {request_info}");

        // 不正アクセスの詳細をログに記録
        if let Some(token) = token {
            if !token.is_empty() {
                // トークンが提供されている場合、その有効性をチェック
                match self.verify_api_request(token) {
                    Ok(true) => {
                        log::info!("有効なトークンでの不正アクセス試行: {request_info}");
                    }
                    Ok(false) => {
                        log::warn!("無効なトークンでの不正アクセス試行: {request_info}");
                    }
                    Err(e) => {
                        log::error!("トークン検証エラー during 不正アクセス検出: {e}");
                    }
                }
            } else {
                log::warn!("空のトークンでの不正アクセス試行: {request_info}");
            }
        } else {
            log::warn!("トークンなしでの不正アクセス試行: {request_info}");
        }

        // セキュリティイベントとして記録（実装は後で追加）
        // TODO: セキュリティイベントログシステムとの統合

        Ok(())
    }

    /// トークンを無効化する
    ///
    /// # 引数
    /// * `token_id` - 無効化するトークンID
    ///
    /// # 戻り値
    /// 処理結果
    pub fn invalidate_token(&self, token_id: &str) -> Result<(), SecurityError> {
        log::debug!("トークンを無効化: token_id={token_id}");

        {
            let mut cache = self.token_cache.lock().unwrap();
            cache.remove(token_id);
        }

        log::info!("トークンを無効化しました: token_id={token_id}");
        Ok(())
    }

    /// すべてのトークンを無効化する
    ///
    /// # 戻り値
    /// 無効化されたトークン数
    pub fn invalidate_all_tokens(&self) -> Result<usize, SecurityError> {
        log::debug!("すべてのトークンを無効化");

        let count = {
            let mut cache = self.token_cache.lock().unwrap();
            let count = cache.len();
            cache.clear();
            count
        };

        log::info!("すべてのトークンを無効化しました: count={count}");
        Ok(count)
    }

    /// トークン情報を取得する
    ///
    /// # 引数
    /// * `token_id` - トークンID
    ///
    /// # 戻り値
    /// トークン情報
    pub fn get_token_info(&self, token_id: &str) -> Result<Option<TokenInfo>, SecurityError> {
        let cache = self.token_cache.lock().unwrap();
        Ok(cache.get(token_id).cloned())
    }

    /// アクティブなトークン数を取得する
    ///
    /// # 戻り値
    /// アクティブなトークン数
    pub fn get_active_token_count(&self) -> usize {
        let cache = self.token_cache.lock().unwrap();
        cache.len()
    }

    /// セキュリティ統計情報を取得する
    ///
    /// # 戻り値
    /// セキュリティ統計情報
    pub fn get_security_stats(&self) -> Result<HashMap<String, serde_json::Value>, SecurityError> {
        let cache = self.token_cache.lock().unwrap();

        let mut stats = HashMap::new();
        stats.insert(
            "active_tokens".to_string(),
            serde_json::Value::Number(cache.len().into()),
        );

        let total_access_count: u64 = cache.values().map(|info| info.access_count).sum();
        stats.insert(
            "total_access_count".to_string(),
            serde_json::Value::Number(total_access_count.into()),
        );

        if let Some(oldest_token) = cache.values().min_by_key(|info| info.created_at) {
            stats.insert(
                "oldest_token_age_seconds".to_string(),
                serde_json::Value::Number(
                    (chrono::Utc::now() - oldest_token.created_at)
                        .num_seconds()
                        .into(),
                ),
            );
        }

        Ok(stats)
    }

    /// 期限切れトークンをクリーンアップする
    ///
    /// # 引数
    /// * `max_age_hours` - 最大保持時間（時間）
    ///
    /// # 戻り値
    /// 削除されたトークン数
    pub fn cleanup_expired_tokens(&self, max_age_hours: i64) -> Result<usize, SecurityError> {
        log::debug!("期限切れトークンをクリーンアップ: max_age_hours={max_age_hours}");

        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut removed_count = 0;

        {
            let mut cache = self.token_cache.lock().unwrap();
            cache.retain(|_token_id, token_info| {
                if token_info.created_at < cutoff_time {
                    removed_count += 1;
                    false
                } else {
                    true
                }
            });
        }

        log::info!("期限切れトークンをクリーンアップしました: removed_count={removed_count}");
        Ok(removed_count)
    }

    /// セキュリティ設定を取得する
    ///
    /// # 戻り値
    /// セキュリティ設定
    pub fn get_config(&self) -> &SecurityConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_security_service() -> SecurityService {
        let config = SecurityConfig {
            encryption_key: "test_encryption_key_32_bytes_long".to_string(),
            max_token_age_hours: 24,
            enable_audit_logging: true,
        };

        SecurityService::new(config).unwrap()
    }

    #[test]
    fn test_encrypt_and_store_token() {
        let service = setup_test_security_service();
        let token_id = "test_token_id";
        let token = "test_session_token";

        let encrypted_token = service.encrypt_and_store_token(token_id, token).unwrap();
        assert!(!encrypted_token.is_empty());

        let token_info = service.get_token_info(token_id).unwrap();
        assert!(token_info.is_some());
        assert_eq!(token_info.unwrap().token_id, token_id);
    }

    #[test]
    fn test_decrypt_token() {
        let service = setup_test_security_service();
        let token_id = "test_token_id";
        let original_token = "test_session_token";

        let encrypted_token = service
            .encrypt_and_store_token(token_id, original_token)
            .unwrap();
        let decrypted_token = service.decrypt_token(token_id, &encrypted_token).unwrap();

        assert_eq!(original_token, decrypted_token);
    }

    #[test]
    fn test_verify_api_request() {
        let service = setup_test_security_service();
        let token = "test_session_token";

        let encrypted_token = service.token_encryption.encrypt_token(token).unwrap();
        let is_valid = service.verify_api_request(&encrypted_token).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_verify_invalid_api_request() {
        let service = setup_test_security_service();
        let invalid_token = "invalid_token";

        let is_valid = service.verify_api_request(invalid_token).unwrap();
        assert!(!is_valid);
    }

    #[test]
    fn test_invalidate_token() {
        let service = setup_test_security_service();
        let token_id = "test_token_id";
        let token = "test_session_token";

        service.encrypt_and_store_token(token_id, token).unwrap();
        assert!(service.get_token_info(token_id).unwrap().is_some());

        service.invalidate_token(token_id).unwrap();
        assert!(service.get_token_info(token_id).unwrap().is_none());
    }

    #[test]
    fn test_encrypt_multiple_tokens() {
        let service = setup_test_security_service();

        let mut tokens = HashMap::new();
        tokens.insert("session".to_string(), "session_token_123".to_string());
        tokens.insert("access".to_string(), "access_token_456".to_string());

        let encrypted_tokens = service.encrypt_multiple_tokens(&tokens).unwrap();
        assert_eq!(encrypted_tokens.len(), 2);

        let decrypted_tokens = service.decrypt_multiple_tokens(&encrypted_tokens).unwrap();
        assert_eq!(tokens, decrypted_tokens);
    }

    #[test]
    fn test_get_security_stats() {
        let service = setup_test_security_service();

        service.encrypt_and_store_token("token1", "value1").unwrap();
        service.encrypt_and_store_token("token2", "value2").unwrap();

        let stats = service.get_security_stats().unwrap();

        if let Some(serde_json::Value::Number(count)) = stats.get("active_tokens") {
            assert_eq!(count.as_u64().unwrap(), 2);
        } else {
            panic!("active_tokens stat not found or wrong type");
        }
    }
}

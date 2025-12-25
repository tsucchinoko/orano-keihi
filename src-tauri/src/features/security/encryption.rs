use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 暗号化エラー
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("暗号化エラー: {0}")]
    EncryptionFailed(String),

    #[error("復号化エラー: {0}")]
    DecryptionFailed(String),

    #[error("キー生成エラー: {0}")]
    KeyGenerationFailed(String),

    #[error("Base64エンコードエラー: {0}")]
    Base64Error(String),

    #[error("データ形式エラー: {0}")]
    FormatError(String),
}

/// 暗号化されたデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// 暗号化されたデータ（Base64エンコード）
    pub ciphertext: String,
    /// ナンス（Base64エンコード）
    pub nonce: String,
    /// 暗号化アルゴリズム
    pub algorithm: String,
}

/// トークン暗号化サービス
#[derive(Clone)]
pub struct TokenEncryption {
    /// 暗号化キー
    encryption_key: Vec<u8>,
}

impl TokenEncryption {
    /// 新しいTokenEncryptionを作成する
    ///
    /// # 引数
    /// * `key` - 暗号化キー（32バイト）
    ///
    /// # 戻り値
    /// TokenEncryptionインスタンス
    pub fn new(key: String) -> Result<Self, EncryptionError> {
        // キーを32バイトに調整
        let mut key_bytes = key.as_bytes().to_vec();
        key_bytes.resize(32, 0); // 32バイトに調整（不足分は0で埋める）

        Ok(Self {
            encryption_key: key_bytes,
        })
    }

    /// データを暗号化する
    ///
    /// # 引数
    /// * `plaintext` - 暗号化するデータ
    ///
    /// # 戻り値
    /// 暗号化されたデータ
    pub fn encrypt(&self, plaintext: &str) -> Result<EncryptedData, EncryptionError> {
        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
            .map_err(|e| EncryptionError::KeyGenerationFailed(e.to_string()))?;

        // ランダムなナンス（12バイト）を生成
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // データを暗号化
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

        // Base64エンコード
        let ciphertext_b64 = general_purpose::STANDARD.encode(&ciphertext);
        let nonce_b64 = general_purpose::STANDARD.encode(&nonce_bytes);

        Ok(EncryptedData {
            ciphertext: ciphertext_b64,
            nonce: nonce_b64,
            algorithm: "AES-256-GCM".to_string(),
        })
    }

    /// データを復号化する
    ///
    /// # 引数
    /// * `encrypted_data` - 暗号化されたデータ
    ///
    /// # 戻り値
    /// 復号化されたデータ
    pub fn decrypt(&self, encrypted_data: &EncryptedData) -> Result<String, EncryptionError> {
        // アルゴリズムを確認
        if encrypted_data.algorithm != "AES-256-GCM" {
            return Err(EncryptionError::FormatError(format!(
                "サポートされていないアルゴリズム: {}",
                encrypted_data.algorithm
            )));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
            .map_err(|e| EncryptionError::KeyGenerationFailed(e.to_string()))?;

        // Base64デコード
        let ciphertext = general_purpose::STANDARD
            .decode(&encrypted_data.ciphertext)
            .map_err(|e| EncryptionError::Base64Error(format!("暗号文デコードエラー: {e}")))?;

        let nonce_bytes = general_purpose::STANDARD
            .decode(&encrypted_data.nonce)
            .map_err(|e| EncryptionError::Base64Error(format!("ナンスデコードエラー: {e}")))?;

        if nonce_bytes.len() != 12 {
            return Err(EncryptionError::FormatError(
                "ナンスのサイズが正しくありません".to_string(),
            ));
        }

        let nonce = Nonce::from_slice(&nonce_bytes);

        // 復号化
        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        String::from_utf8(plaintext)
            .map_err(|e| EncryptionError::DecryptionFailed(format!("UTF-8変換エラー: {e}")))
    }

    /// トークンを暗号化してBase64文字列として返す
    ///
    /// # 引数
    /// * `token` - 暗号化するトークン
    ///
    /// # 戻り値
    /// Base64エンコードされた暗号化トークン
    pub fn encrypt_token(&self, token: &str) -> Result<String, EncryptionError> {
        let encrypted_data = self.encrypt(token)?;

        // JSON形式でシリアライズしてBase64エンコード
        let json_data = serde_json::to_string(&encrypted_data)
            .map_err(|e| EncryptionError::FormatError(format!("JSON変換エラー: {e}")))?;

        Ok(general_purpose::STANDARD.encode(json_data.as_bytes()))
    }

    /// Base64文字列からトークンを復号化する
    ///
    /// # 引数
    /// * `encrypted_token` - Base64エンコードされた暗号化トークン
    ///
    /// # 戻り値
    /// 復号化されたトークン
    pub fn decrypt_token(&self, encrypted_token: &str) -> Result<String, EncryptionError> {
        // Base64デコード
        let json_bytes = general_purpose::STANDARD
            .decode(encrypted_token)
            .map_err(|e| EncryptionError::Base64Error(format!("Base64デコードエラー: {e}")))?;

        let json_data = String::from_utf8(json_bytes)
            .map_err(|e| EncryptionError::FormatError(format!("UTF-8変換エラー: {e}")))?;

        // JSONデシリアライズ
        let encrypted_data: EncryptedData = serde_json::from_str(&json_data)
            .map_err(|e| EncryptionError::FormatError(format!("JSON解析エラー: {e}")))?;

        self.decrypt(&encrypted_data)
    }

    /// 複数のトークンを一括暗号化する
    ///
    /// # 引数
    /// * `tokens` - 暗号化するトークンのマップ
    ///
    /// # 戻り値
    /// 暗号化されたトークンのマップ
    pub fn encrypt_tokens(
        &self,
        tokens: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, EncryptionError> {
        let mut encrypted_tokens = HashMap::new();

        for (key, token) in tokens {
            let encrypted_token = self.encrypt_token(token)?;
            encrypted_tokens.insert(key.clone(), encrypted_token);
        }

        Ok(encrypted_tokens)
    }

    /// 複数のトークンを一括復号化する
    ///
    /// # 引数
    /// * `encrypted_tokens` - 暗号化されたトークンのマップ
    ///
    /// # 戻り値
    /// 復号化されたトークンのマップ
    pub fn decrypt_tokens(
        &self,
        encrypted_tokens: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, EncryptionError> {
        let mut tokens = HashMap::new();

        for (key, encrypted_token) in encrypted_tokens {
            let token = self.decrypt_token(encrypted_token)?;
            tokens.insert(key.clone(), token);
        }

        Ok(tokens)
    }
}

/// セキュアなランダムキーを生成する
///
/// # 引数
/// * `length` - キーの長さ（バイト）
///
/// # 戻り値
/// Base64エンコードされたランダムキー
pub fn generate_secure_key(length: usize) -> String {
    let mut key_bytes = vec![0u8; length];
    OsRng.fill_bytes(&mut key_bytes);
    general_purpose::STANDARD.encode(&key_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_encryption() -> TokenEncryption {
        TokenEncryption::new("test_encryption_key_32_bytes_long".to_string()).unwrap()
    }

    #[test]
    fn test_encrypt_decrypt() {
        let encryption = setup_test_encryption();
        let plaintext = "test_token_data";

        let encrypted_data = encryption.encrypt(plaintext).unwrap();
        let decrypted = encryption.decrypt(&encrypted_data).unwrap();

        assert_eq!(plaintext, decrypted);
        assert_eq!(encrypted_data.algorithm, "AES-256-GCM");
    }

    #[test]
    fn test_encrypt_decrypt_token() {
        let encryption = setup_test_encryption();
        let token = "session_token_12345";

        let encrypted_token = encryption.encrypt_token(token).unwrap();
        let decrypted_token = encryption.decrypt_token(&encrypted_token).unwrap();

        assert_eq!(token, decrypted_token);
        assert!(!encrypted_token.is_empty());
    }

    #[test]
    fn test_encrypt_decrypt_multiple_tokens() {
        let encryption = setup_test_encryption();

        let mut tokens = HashMap::new();
        tokens.insert("session".to_string(), "session_token_123".to_string());
        tokens.insert("access".to_string(), "access_token_456".to_string());
        tokens.insert("refresh".to_string(), "refresh_token_789".to_string());

        let encrypted_tokens = encryption.encrypt_tokens(&tokens).unwrap();
        let decrypted_tokens = encryption.decrypt_tokens(&encrypted_tokens).unwrap();

        assert_eq!(tokens, decrypted_tokens);
    }

    #[test]
    fn test_generate_secure_key() {
        let key1 = generate_secure_key(32);
        let key2 = generate_secure_key(32);

        assert_ne!(key1, key2);
        assert!(!key1.is_empty());
        assert!(!key2.is_empty());
    }

    #[test]
    fn test_invalid_algorithm() {
        let encryption = setup_test_encryption();

        let invalid_data = EncryptedData {
            ciphertext: "test".to_string(),
            nonce: "test".to_string(),
            algorithm: "INVALID".to_string(),
        };

        let result = encryption.decrypt(&invalid_data);
        assert!(result.is_err());
    }
}

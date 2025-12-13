// R2クライアントモジュール

use super::{config::R2Config, R2Error};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::config::{Credentials, SharedCredentialsProvider};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::{Client, Config};
use std::time::Duration;

#[derive(Clone)]
pub struct R2Client {
    client: Client,
    bucket_name: String,
    config: R2Config,
}

impl R2Client {
    /// R2クライアントを初期化
    pub async fn new(config: R2Config) -> Result<Self, R2Error> {
        // 設定を検証
        config
            .validate()
            .map_err(|_e| R2Error::InvalidCredentials)?;

        // 認証情報を設定
        let credentials =
            Credentials::new(&config.access_key, &config.secret_key, None, None, "r2");

        // S3互換設定を構築
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(&config.endpoint_url())
            .region(Region::new(config.region.clone()))
            .credentials_provider(SharedCredentialsProvider::new(credentials))
            .load()
            .await;

        let s3_config = Config::from(&aws_config);
        let client = Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket_name: config.bucket_name.clone(),
            config,
        })
    }

    /// ファイルをR2にアップロード
    pub async fn upload_file(
        &self,
        key: &str,
        file_data: Vec<u8>,
        content_type: &str,
    ) -> Result<String, R2Error> {
        let _put_object_output = self
            .client
            .put_object()
            .bucket(&self.bucket_name)
            .key(key)
            .body(file_data.into())
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| R2Error::UploadFailed(format!("アップロードエラー: {}", e)))?;

        // アップロード成功時のHTTPS URLを生成
        let url = format!(
            "https://{}/{}/{}",
            self.config.endpoint_url().replace("https://", ""),
            self.bucket_name,
            key
        );

        Ok(url)
    }

    /// リトライ機能付きファイルアップロード
    pub async fn upload_file_with_retry(
        &self,
        key: &str,
        file_data: Vec<u8>,
        content_type: &str,
        max_retries: u32,
    ) -> Result<String, R2Error> {
        let mut attempts = 0;

        loop {
            match self.upload_file(key, file_data.clone(), content_type).await {
                Ok(url) => return Ok(url),
                Err(_e) if attempts < max_retries => {
                    attempts += 1;
                    // 指数バックオフ（2^attempts秒待機）
                    let delay = Duration::from_secs(2_u64.pow(attempts));
                    tokio::time::sleep(delay).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Presigned URLを生成（ダウンロード用）
    pub async fn generate_presigned_url(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> Result<String, R2Error> {
        let presigning_config = PresigningConfig::expires_in(expires_in)
            .map_err(|e| R2Error::NetworkError(format!("Presigned URL設定エラー: {}", e)))?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|e| R2Error::NetworkError(format!("Presigned URL生成エラー: {}", e)))?;

        Ok(presigned_request.uri().to_string())
    }

    /// ファイルをR2から削除
    pub async fn delete_file(&self, key: &str) -> Result<(), R2Error> {
        self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await
            .map_err(|e| R2Error::NetworkError(format!("削除エラー: {}", e)))?;

        Ok(())
    }

    /// 接続テスト
    pub async fn test_connection(&self) -> Result<(), R2Error> {
        // バケットの存在確認を行う
        self.client
            .head_bucket()
            .bucket(&self.bucket_name)
            .send()
            .await
            .map_err(|e| R2Error::ConnectionFailed(format!("接続テスト失敗: {}", e)))?;

        Ok(())
    }

    /// ファイルキーを生成（予測困難にする）
    pub fn generate_file_key(expense_id: i64, filename: &str) -> String {
        let timestamp = chrono::Utc::now().timestamp();
        let uuid = uuid::Uuid::new_v4();
        format!(
            "receipts/{}/{}-{}-{}",
            expense_id, timestamp, uuid, filename
        )
    }

    /// ファイル形式を検証
    pub fn validate_file_format(filename: &str) -> Result<(), R2Error> {
        let extension = std::path::Path::new(filename)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .ok_or_else(|| R2Error::UploadFailed("ファイル拡張子が取得できません".to_string()))?;

        if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "pdf") {
            return Err(R2Error::UploadFailed(
                "サポートされていないファイル形式です（PNG、JPG、JPEG、PDFのみ対応）".to_string(),
            ));
        }

        Ok(())
    }

    /// ファイルサイズを検証
    pub fn validate_file_size(file_size: u64) -> Result<(), R2Error> {
        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB

        if file_size > MAX_FILE_SIZE {
            return Err(R2Error::UploadFailed(
                "ファイルサイズが10MBを超えています".to_string(),
            ));
        }

        Ok(())
    }

    /// Content-Typeを推定
    pub fn get_content_type(filename: &str) -> String {
        let extension = std::path::Path::new(filename)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        match extension.as_str() {
            "png" => "image/png".to_string(),
            "jpg" | "jpeg" => "image/jpeg".to_string(),
            "pdf" => "application/pdf".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_key_generation() {
        let expense_id = 123;
        let filename = "receipt.pdf";

        let key1 = R2Client::generate_file_key(expense_id, filename);
        let key2 = R2Client::generate_file_key(expense_id, filename);

        // 異なるキーが生成されることを確認
        assert_ne!(key1, key2);

        // 正しい形式であることを確認
        assert!(key1.starts_with("receipts/123/"));
        assert!(key1.ends_with("-receipt.pdf"));
    }

    #[test]
    fn test_file_key_format() {
        let expense_id = 456;
        let filename = "test.jpg";

        let key = R2Client::generate_file_key(expense_id, filename);

        // キーの形式を確認
        let parts: Vec<&str> = key.split('/').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "receipts");
        assert_eq!(parts[1], "456");

        // ファイル名部分の確認
        let file_part = parts[2];
        assert!(file_part.contains("test.jpg"));
    }

    #[test]
    fn test_file_format_validation() {
        // 有効なファイル形式
        assert!(R2Client::validate_file_format("test.pdf").is_ok());
        assert!(R2Client::validate_file_format("test.png").is_ok());
        assert!(R2Client::validate_file_format("test.jpg").is_ok());
        assert!(R2Client::validate_file_format("test.jpeg").is_ok());

        // 無効なファイル形式
        assert!(R2Client::validate_file_format("test.txt").is_err());
        assert!(R2Client::validate_file_format("test.doc").is_err());
        assert!(R2Client::validate_file_format("test").is_err());
    }

    #[test]
    fn test_file_size_validation() {
        // 有効なファイルサイズ（10MB以下）
        assert!(R2Client::validate_file_size(1024).is_ok()); // 1KB
        assert!(R2Client::validate_file_size(1024 * 1024).is_ok()); // 1MB
        assert!(R2Client::validate_file_size(10 * 1024 * 1024).is_ok()); // 10MB

        // 無効なファイルサイズ（10MB超過）
        assert!(R2Client::validate_file_size(10 * 1024 * 1024 + 1).is_err()); // 10MB + 1byte
        assert!(R2Client::validate_file_size(20 * 1024 * 1024).is_err()); // 20MB
    }

    #[test]
    fn test_content_type_detection() {
        assert_eq!(R2Client::get_content_type("test.pdf"), "application/pdf");
        assert_eq!(R2Client::get_content_type("test.png"), "image/png");
        assert_eq!(R2Client::get_content_type("test.jpg"), "image/jpeg");
        assert_eq!(R2Client::get_content_type("test.jpeg"), "image/jpeg");
        assert_eq!(R2Client::get_content_type("test.unknown"), "application/octet-stream");
    }
}

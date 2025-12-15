// R2クライアントモジュール

use super::{config::R2Config, security::SecurityManager, R2Error};
use crate::R2ConnectionCache;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::config::{Credentials, SharedCredentialsProvider};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::{Client, Config};
use futures::future::join_all;
use log::{debug, error, info, warn};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Semaphore};
use uuid::Uuid;

/// 複数ファイルアップロード用の構造体
#[derive(Debug, Clone)]
pub struct MultipleFileUpload {
    pub file_key: String,
    pub file_data: Vec<u8>,
    pub content_type: String,
    pub expense_id: i64,
    pub filename: String,
}

/// アップロード結果の構造体
#[derive(Debug, Clone, serde::Serialize)]
pub struct UploadResult {
    pub file_key: String,
    pub success: bool,
    pub url: Option<String>,
    pub error: Option<String>,
    pub file_size: u64,
    #[serde(serialize_with = "serialize_duration")]
    pub duration: Duration,
}

/// Duration を milliseconds として serialize する
fn serialize_duration<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(duration.as_millis() as u64)
}

/// アップロードプログレスの構造体
#[derive(Debug, Clone, serde::Serialize)]
pub struct UploadProgress {
    pub file_index: usize,
    pub file_key: String,
    pub status: UploadStatus,
    pub bytes_uploaded: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
}

/// アップロードステータス
#[derive(Debug, Clone, serde::Serialize)]
pub enum UploadStatus {
    Started,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// パフォーマンス統計の構造体
#[derive(Debug, Clone, serde::Serialize)]
pub struct PerformanceStats {
    pub latency_ms: u64,
    pub throughput_bps: u64,
    pub connection_status: String,
    pub last_measured: String,
}

#[derive(Clone)]
pub struct R2Client {
    client: Client,
    bucket_name: String,
    config: R2Config,
}

impl R2Client {
    /// R2クライアントを初期化（セキュリティ強化版）
    pub async fn new(config: R2Config) -> Result<Self, R2Error> {
        info!("R2クライアントを初期化しています...");

        // セキュリティマネージャーでログ記録
        let security_manager = SecurityManager::new();
        security_manager.log_security_event("r2_client_init", "R2クライアント初期化開始");

        // 設定を検証
        config.validate().map_err(|e| {
            error!("R2設定の検証に失敗しました: {:?}", e);
            security_manager.log_security_event("config_validation_failed", &format!("{:?}", e));
            R2Error::InvalidCredentials
        })?;

        // 認証情報を設定（ログには出力しない）
        debug!("認証情報を設定中...");
        let credentials =
            Credentials::new(&config.access_key, &config.secret_key, None, None, "r2");

        // S3互換設定を構築
        debug!(
            "AWS設定を構築中... エンドポイント: {}",
            config.endpoint_url()
        );
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(&config.endpoint_url())
            .region(Region::new(config.region.clone()))
            .credentials_provider(SharedCredentialsProvider::new(credentials))
            .load()
            .await;

        let s3_config = Config::from(&aws_config);
        let client = Client::from_conf(s3_config);

        // 環境別バケット名を使用
        let bucket_name = config.get_environment_bucket_name();

        info!(
            "R2クライアントの初期化が完了しました。バケット: {}",
            bucket_name
        );
        security_manager.log_security_event(
            "r2_client_init_success",
            &format!("バケット: {}", bucket_name),
        );

        Ok(Self {
            client,
            bucket_name,
            config,
        })
    }

    /// ファイルをR2にアップロード（ログ強化版）
    pub async fn upload_file(
        &self,
        key: &str,
        file_data: Vec<u8>,
        content_type: &str,
    ) -> Result<String, R2Error> {
        let file_size = file_data.len();
        info!(
            "ファイルアップロード開始: key={}, size={} bytes, content_type={}",
            key, file_size, content_type
        );

        let start_time = std::time::Instant::now();

        let _put_object_output = self
            .client
            .put_object()
            .bucket(&self.bucket_name)
            .key(key)
            .body(file_data.into())
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| {
                // 詳細なエラー情報を取得
                let error_msg = format!("アップロードエラー: {}", e);
                let error_debug = format!("Debug: {:?}", e);

                error!(
                    "ファイルアップロード失敗: key={}, bucket={}, error={}",
                    key, self.bucket_name, error_msg
                );
                error!("詳細エラー情報: {}", error_debug);

                // セキュリティログ記録
                let security_manager = SecurityManager::new();
                security_manager.log_security_event(
                    "upload_failed",
                    &format!("key={}, error={}", key, error_msg),
                );

                R2Error::UploadFailed(error_msg)
            })?;

        let duration = start_time.elapsed();

        // アップロード成功時のHTTPS URLを生成
        let url = format!(
            "https://{}/{}/{}",
            self.config.endpoint_url().replace("https://", ""),
            self.bucket_name,
            key
        );

        info!(
            "ファイルアップロード成功: key={}, url={}, duration={:?}",
            key, url, duration
        );

        // セキュリティログ記録
        let security_manager = SecurityManager::new();
        security_manager.log_security_event(
            "upload_success",
            &format!("key={}, size={} bytes", key, file_size),
        );

        Ok(url)
    }

    /// リトライ機能付きファイルアップロード（ログ強化版）
    pub async fn upload_file_with_retry(
        &self,
        key: &str,
        file_data: Vec<u8>,
        content_type: &str,
        max_retries: u32,
    ) -> Result<String, R2Error> {
        let mut attempts = 0;
        info!(
            "リトライ機能付きアップロード開始: key={}, max_retries={}",
            key, max_retries
        );

        loop {
            match self.upload_file(key, file_data.clone(), content_type).await {
                Ok(url) => {
                    if attempts > 0 {
                        info!(
                            "リトライ後にアップロード成功: key={}, attempts={}",
                            key, attempts
                        );
                    }
                    return Ok(url);
                }
                Err(_e) if attempts < max_retries => {
                    attempts += 1;
                    // 指数バックオフ（2^attempts秒待機）
                    let delay = Duration::from_secs(2_u64.pow(attempts));
                    warn!(
                        "アップロード失敗、リトライします: key={}, attempt={}/{}, delay={:?}s",
                        key, attempts, max_retries, delay
                    );

                    tokio::time::sleep(delay).await;
                    continue;
                }
                Err(e) => {
                    error!(
                        "アップロード最終失敗: key={}, total_attempts={}",
                        key,
                        attempts + 1
                    );

                    // セキュリティログ記録
                    let security_manager = SecurityManager::new();
                    security_manager.log_security_event(
                        "upload_final_failure",
                        &format!("key={}, attempts={}", key, attempts + 1),
                    );

                    return Err(e);
                }
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

    /// 接続テスト（詳細ログ付き）
    pub async fn test_connection(&self) -> Result<(), R2Error> {
        info!("R2接続テストを開始します: bucket={}", self.bucket_name);

        let start_time = std::time::Instant::now();

        // バケットの存在確認を行う
        let _result = self
            .client
            .head_bucket()
            .bucket(&self.bucket_name)
            .send()
            .await
            .map_err(|e| {
                let error_msg = format!("接続テスト失敗: {}", e);
                error!(
                    "R2接続テスト失敗: bucket={}, error={}",
                    self.bucket_name, error_msg
                );

                // セキュリティログ記録
                let security_manager = SecurityManager::new();
                security_manager.log_security_event(
                    "connection_test_failed",
                    &format!("bucket={}, error={}", self.bucket_name, error_msg),
                );

                R2Error::ConnectionFailed(error_msg)
            })?;

        let duration = start_time.elapsed();
        info!(
            "R2接続テスト成功: bucket={}, duration={:?}",
            self.bucket_name, duration
        );

        // セキュリティログ記録
        let security_manager = SecurityManager::new();
        security_manager.log_security_event(
            "connection_test_success",
            &format!("bucket={}, duration={:?}", self.bucket_name, duration),
        );

        Ok(())
    }

    /// 詳細な診断情報を取得
    pub fn get_diagnostic_info(&self) -> std::collections::HashMap<String, String> {
        let mut info = std::collections::HashMap::new();

        info.insert("bucket_name".to_string(), self.bucket_name.clone());
        info.insert("endpoint_url".to_string(), self.config.endpoint_url());
        info.insert("region".to_string(), self.config.region.clone());

        // 設定のデバッグ情報を追加
        let config_debug = self.config.get_debug_info();
        for (key, value) in config_debug {
            info.insert(format!("config_{}", key), value);
        }

        debug!("R2クライアント診断情報: {:?}", info);
        info
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

    /// 複数ファイルを並列でアップロードする
    pub async fn upload_multiple_files(
        &self,
        files: Vec<MultipleFileUpload>,
        max_concurrent: usize,
        progress_sender: Option<mpsc::UnboundedSender<UploadProgress>>,
        cancel_token: Option<Arc<tokio_util::sync::CancellationToken>>,
    ) -> Result<Vec<UploadResult>, R2Error> {
        info!(
            "並列アップロード開始: {} ファイル, 最大同時実行数: {}",
            files.len(),
            max_concurrent
        );

        let start_time = Instant::now();
        let total_files = files.len();
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let client = Arc::new(self.clone());

        // 各ファイルのアップロードタスクを作成
        let upload_tasks: Vec<_> = files
            .into_iter()
            .enumerate()
            .map(|(index, file_upload)| {
                let semaphore = semaphore.clone();
                let client = client.clone();
                let progress_sender = progress_sender.clone();
                let cancel_token = cancel_token.clone();

                tokio::spawn(async move {
                    // セマフォを取得（同時実行数制限）
                    let _permit = semaphore.acquire().await.map_err(|_| {
                        R2Error::UploadFailed("セマフォ取得に失敗しました".to_string())
                    })?;

                    // キャンセルチェック
                    if let Some(token) = &cancel_token {
                        if token.is_cancelled() {
                            return Ok(UploadResult {
                                file_key: file_upload.file_key.clone(),
                                success: false,
                                url: None,
                                error: Some("アップロードがキャンセルされました".to_string()),
                                file_size: file_upload.file_data.len() as u64,
                                duration: Duration::from_secs(0),
                            });
                        }
                    }

                    let upload_start = Instant::now();

                    // プログレス通知（開始）
                    if let Some(sender) = &progress_sender {
                        let _ = sender.send(UploadProgress {
                            file_index: index,
                            file_key: file_upload.file_key.clone(),
                            status: UploadStatus::Started,
                            bytes_uploaded: 0,
                            total_bytes: file_upload.file_data.len() as u64,
                            speed_bps: 0,
                        });
                    }

                    // ファイルアップロード実行
                    let result = client
                        .upload_file_with_retry(
                            &file_upload.file_key,
                            file_upload.file_data.clone(),
                            &file_upload.content_type,
                            3, // 最大3回リトライ
                        )
                        .await;

                    let duration = upload_start.elapsed();
                    let file_size = file_upload.file_data.len() as u64;

                    let upload_result = match result {
                        Ok(url) => {
                            // プログレス通知（完了）
                            if let Some(sender) = &progress_sender {
                                let speed_bps = if duration.as_secs() > 0 {
                                    file_size / duration.as_secs()
                                } else {
                                    0
                                };

                                let _ = sender.send(UploadProgress {
                                    file_index: index,
                                    file_key: file_upload.file_key.clone(),
                                    status: UploadStatus::Completed,
                                    bytes_uploaded: file_size,
                                    total_bytes: file_size,
                                    speed_bps,
                                });
                            }

                            UploadResult {
                                file_key: file_upload.file_key,
                                success: true,
                                url: Some(url),
                                error: None,
                                file_size,
                                duration,
                            }
                        }
                        Err(e) => {
                            // プログレス通知（エラー）
                            if let Some(sender) = &progress_sender {
                                let _ = sender.send(UploadProgress {
                                    file_index: index,
                                    file_key: file_upload.file_key.clone(),
                                    status: UploadStatus::Failed,
                                    bytes_uploaded: 0,
                                    total_bytes: file_size,
                                    speed_bps: 0,
                                });
                            }

                            UploadResult {
                                file_key: file_upload.file_key,
                                success: false,
                                url: None,
                                error: Some(e.to_string()),
                                file_size,
                                duration,
                            }
                        }
                    };

                    Ok::<UploadResult, R2Error>(upload_result)
                })
            })
            .collect();

        // すべてのタスクを並列実行
        let results = join_all(upload_tasks).await;

        // 結果を収集
        let mut upload_results = Vec::new();
        let mut successful_uploads = 0;
        let mut failed_uploads = 0;

        for task_result in results {
            match task_result {
                Ok(Ok(upload_result)) => {
                    if upload_result.success {
                        successful_uploads += 1;
                    } else {
                        failed_uploads += 1;
                    }
                    upload_results.push(upload_result);
                }
                Ok(Err(e)) => {
                    failed_uploads += 1;
                    error!("アップロードタスクエラー: {}", e);
                    return Err(e);
                }
                Err(e) => {
                    failed_uploads += 1;
                    error!("タスク実行エラー: {}", e);
                    return Err(R2Error::UploadFailed(format!("タスク実行エラー: {}", e)));
                }
            }
        }

        let total_duration = start_time.elapsed();

        info!(
            "並列アップロード完了: 成功={}, 失敗={}, 総時間={:?}",
            successful_uploads, failed_uploads, total_duration
        );

        // セキュリティログ記録
        let security_manager = SecurityManager::new();
        security_manager.log_security_event(
            "parallel_upload_completed",
            &format!(
                "total_files={}, successful={}, failed={}, duration={:?}",
                total_files, successful_uploads, failed_uploads, total_duration
            ),
        );

        Ok(upload_results)
    }

    /// パフォーマンス統計を取得する
    pub async fn get_performance_stats(&self) -> Result<PerformanceStats, R2Error> {
        let start_time = Instant::now();

        // 接続テストでレイテンシを測定
        self.test_connection().await?;
        let latency = start_time.elapsed();

        // 小さなテストファイルでスループットを測定
        let test_data = vec![0u8; 1024]; // 1KB
        let test_key = format!("performance_test_{}", uuid::Uuid::new_v4());

        let upload_start = Instant::now();
        let _url = self
            .upload_file(&test_key, test_data.clone(), "application/octet-stream")
            .await?;
        let upload_duration = upload_start.elapsed();

        // テストファイルを削除
        let _ = self.delete_file(&test_key).await;

        // スループット計算（bytes/sec）
        let throughput_bps = if upload_duration.as_secs_f64() > 0.0 {
            test_data.len() as f64 / upload_duration.as_secs_f64()
        } else {
            0.0
        };

        Ok(PerformanceStats {
            latency_ms: latency.as_millis() as u64,
            throughput_bps: throughput_bps as u64,
            connection_status: "healthy".to_string(),
            last_measured: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// キャッシュ対応のパフォーマンス統計を取得する
    pub async fn get_performance_stats_with_cache(
        &self,
        connection_cache: Arc<Mutex<R2ConnectionCache>>,
    ) -> Result<PerformanceStats, R2Error> {
        let start_time = Instant::now();

        // キャッシュされた接続テスト結果を確認
        let cached_result = {
            let cache = connection_cache.lock().unwrap();
            cache.get_cached_result()
        };

        let latency = if let Some(true) = cached_result {
            // キャッシュされた成功結果がある場合は、接続テストをスキップ
            info!("R2接続テストをスキップ（キャッシュされた成功結果を使用）");
            Duration::from_millis(50) // 推定レイテンシ
        } else {
            // 接続テストを実行してキャッシュを更新
            match self.test_connection().await {
                Ok(_) => {
                    let latency = start_time.elapsed();
                    {
                        let mut cache = connection_cache.lock().unwrap();
                        cache.update_cache(true);
                    }
                    latency
                }
                Err(e) => {
                    {
                        let mut cache = connection_cache.lock().unwrap();
                        cache.update_cache(false);
                    }
                    return Err(e);
                }
            }
        };

        // 小さなテストファイルでスループットを測定（軽量化）
        let test_data = vec![0u8; 512]; // 512バイトに削減
        let test_key = format!("perf_test_{}", Uuid::new_v4());

        let upload_start = Instant::now();
        let _url = self
            .upload_file(&test_key, test_data.clone(), "application/octet-stream")
            .await?;
        let upload_duration = upload_start.elapsed();

        // テストファイルを削除（バックグラウンドで実行）
        let client_clone = self.clone();
        let test_key_clone = test_key.clone();
        tokio::spawn(async move {
            let _ = client_clone.delete_file(&test_key_clone).await;
        });

        // スループット計算（bytes/sec）
        let throughput_bps = if upload_duration.as_secs_f64() > 0.0 {
            test_data.len() as f64 / upload_duration.as_secs_f64()
        } else {
            0.0
        };

        Ok(PerformanceStats {
            latency_ms: latency.as_millis() as u64,
            throughput_bps: throughput_bps as u64,
            connection_status: "healthy".to_string(),
            last_measured: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// アップロード統計を記録する
    pub fn record_upload_stats(&self, file_size: u64, duration: Duration, success: bool) {
        let speed_bps = if duration.as_secs() > 0 {
            file_size / duration.as_secs()
        } else {
            0
        };

        info!(
            "アップロード統計: サイズ={}bytes, 時間={:?}, 速度={}bps, 成功={}",
            file_size, duration, speed_bps, success
        );

        // セキュリティログ記録
        let security_manager = SecurityManager::new();
        security_manager.log_security_event(
            "upload_stats",
            &format!(
                "size={}, duration={:?}, speed={}bps, success={}",
                file_size, duration, speed_bps, success
            ),
        );
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
        assert_eq!(
            R2Client::get_content_type("test.unknown"),
            "application/octet-stream"
        );
    }
}

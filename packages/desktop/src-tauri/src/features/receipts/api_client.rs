// APIサーバーとの通信を行うクライアント

use crate::shared::config::environment::ApiConfig;
use crate::shared::errors::AppError;
use log::{debug, error, info, warn};
use reqwest::{multipart, Client, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// APIクライアント設定
#[derive(Debug, Clone)]
pub struct ApiClientConfig {
    pub base_url: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

impl Default for ApiClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3000".to_string(),
            timeout_seconds: 30,
            max_retries: 3,
        }
    }
}

impl ApiClientConfig {
    /// 環境設定からAPIクライアント設定を作成
    pub fn from_env() -> Self {
        let api_config = ApiConfig::from_env();
        Self {
            base_url: api_config.base_url,
            timeout_seconds: api_config.timeout_seconds,
            max_retries: api_config.max_retries,
        }
    }
}

/// APIサーバーからのファイルアップロードレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub success: bool,
    #[serde(rename = "fileUrl")]
    pub file_url: Option<String>,
    #[serde(rename = "fileKey")]
    pub file_key: String,
    #[serde(rename = "fileSize")]
    pub file_size: u64,
    #[serde(rename = "contentType")]
    pub content_type: String,
    #[serde(rename = "uploadedAt")]
    pub uploaded_at: String,
    pub error: Option<String>,
}

/// APIサーバーからの複数ファイルアップロードレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct MultipleUploadResponse {
    #[serde(rename = "totalFiles")]
    pub total_files: usize,
    #[serde(rename = "successfulUploads")]
    pub successful_uploads: usize,
    #[serde(rename = "failedUploads")]
    pub failed_uploads: usize,
    pub results: Vec<UploadResponse>,
    #[serde(rename = "totalDurationMs")]
    pub total_duration_ms: u64,
}

/// APIサーバーからのエラーレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: String,
    #[serde(rename = "requestId")]
    pub request_id: String,
}

/// ヘルスチェック結果
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub is_healthy: bool,
    pub response_time_ms: u64,
    pub status_code: u16,
    pub error_message: Option<String>,
    pub details: Option<serde_json::Value>,
}

/// APIクライアント
pub struct ApiClient {
    client: Client,
    config: ApiClientConfig,
}

impl ApiClient {
    /// 新しいAPIクライアントを作成
    pub fn new(config: ApiClientConfig) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| AppError::Configuration(format!("HTTPクライアント初期化失敗: {e}")))?;

        Ok(Self { client, config })
    }

    /// 単一ファイルをAPIサーバー経由でアップロード
    pub async fn upload_file(
        &self,
        expense_id: i64,
        file_data: &[u8],
        filename: &str,
        auth_token: &str,
    ) -> Result<UploadResponse, AppError> {
        info!("APIサーバー経由でファイルアップロード開始: expense_id={expense_id}, filename={filename}");

        let url = format!("{}/api/v1/receipts/upload", self.config.base_url);

        // リトライ機能付きでリクエスト送信
        let mut attempts = 0;
        loop {
            // マルチパートフォームデータを構築（リトライごとに再作成）
            let form = multipart::Form::new()
                .part(
                    "file",
                    multipart::Part::bytes(file_data.to_vec())
                        .file_name(filename.to_string())
                        .mime_str(&self.get_content_type(filename))
                        .map_err(|e| AppError::Validation(format!("MIMEタイプ設定エラー: {e}")))?,
                )
                .text("expenseId", expense_id.to_string())
                .text("userId", "1"); // TODO: 実際のユーザーIDを使用

            match self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {auth_token}"))
                .multipart(form)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        let upload_response: UploadResponse =
                            response.json().await.map_err(|e| {
                                AppError::ExternalService(format!("レスポンス解析エラー: {e}"))
                            })?;

                        info!(
                            "ファイルアップロード成功: expense_id={expense_id}, url={:?}",
                            upload_response.file_url
                        );
                        return Ok(upload_response);
                    } else {
                        let error_response = self.handle_error_response(response).await?;
                        return Err(AppError::ExternalService(format!(
                            "APIサーバーエラー: {} - {}",
                            error_response.error.code, error_response.error.message
                        )));
                    }
                }
                Err(e) => {
                    if attempts < self.config.max_retries {
                        attempts += 1;
                        let delay = Duration::from_secs(2_u64.pow(attempts));
                        warn!(
                            "APIリクエスト失敗、リトライします: attempt={attempts}/{}, delay={delay:?}",
                            self.config.max_retries
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    } else {
                        return Err(AppError::ExternalService(format!(
                            "APIサーバーへの接続に失敗しました: {e}"
                        )));
                    }
                }
            }
        }
    }

    /// 複数ファイルをAPIサーバー経由で並列アップロード
    pub async fn upload_multiple_files(
        &self,
        files: Vec<(i64, String, Vec<u8>, String)>, // (expense_id, file_path, file_data, filename)
        auth_token: &str,
    ) -> Result<MultipleUploadResponse, AppError> {
        info!(
            "APIサーバー経由で複数ファイル並列アップロード開始: {} ファイル",
            files.len()
        );

        let url = format!("{}/api/v1/receipts/upload/multiple", self.config.base_url);

        // リトライ機能付きでリクエスト送信
        let mut attempts = 0;
        loop {
            // マルチパートフォームデータを構築（リトライごとに再作成）
            let mut form = multipart::Form::new();

            for (i, (expense_id, _file_path, file_data, filename)) in files.iter().enumerate() {
                form = form
                    .part(
                        format!("files[{i}]"),
                        multipart::Part::bytes(file_data.clone())
                            .file_name(filename.clone())
                            .mime_str(&self.get_content_type(filename))
                            .map_err(|e| {
                                AppError::Validation(format!("MIMEタイプ設定エラー: {e}"))
                            })?,
                    )
                    .text(format!("expenseIds[{i}]"), expense_id.to_string());
            }

            form = form.text("userId", "1"); // TODO: 実際のユーザーIDを使用

            match self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {auth_token}"))
                .multipart(form)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        let upload_response: MultipleUploadResponse =
                            response.json().await.map_err(|e| {
                                AppError::ExternalService(format!("レスポンス解析エラー: {e}"))
                            })?;

                        info!(
                            "複数ファイルアップロード成功: 成功={}, 失敗={}",
                            upload_response.successful_uploads, upload_response.failed_uploads
                        );
                        return Ok(upload_response);
                    } else {
                        let error_response = self.handle_error_response(response).await?;
                        return Err(AppError::ExternalService(format!(
                            "APIサーバーエラー: {} - {}",
                            error_response.error.code, error_response.error.message
                        )));
                    }
                }
                Err(e) => {
                    if attempts < self.config.max_retries {
                        attempts += 1;
                        let delay = Duration::from_secs(2_u64.pow(attempts));
                        warn!(
                            "APIリクエスト失敗、リトライします: attempt={attempts}/{}, delay={delay:?}",
                            self.config.max_retries
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    } else {
                        return Err(AppError::ExternalService(format!(
                            "APIサーバーへの接続に失敗しました: {e}"
                        )));
                    }
                }
            }
        }
    }

    /// ファイルをAPIサーバー経由で削除
    pub async fn delete_file(&self, file_key: &str, auth_token: &str) -> Result<bool, AppError> {
        info!("APIサーバー経由でファイル削除開始: file_key={file_key}");

        let url = format!("{}/api/v1/receipts/{file_key}", self.config.base_url);

        // リトライ機能付きでリクエスト送信
        let mut attempts = 0;
        loop {
            match self
                .client
                .delete(&url)
                .header("Authorization", format!("Bearer {auth_token}"))
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        info!("ファイル削除成功: file_key={file_key}");
                        return Ok(true);
                    } else if response.status().as_u16() == 404 {
                        warn!("削除対象ファイルが見つかりません: file_key={file_key}");
                        return Ok(true); // 既に削除済みとして成功扱い
                    } else {
                        let error_response = self.handle_error_response(response).await?;
                        return Err(AppError::ExternalService(format!(
                            "APIサーバーエラー: {} - {}",
                            error_response.error.code, error_response.error.message
                        )));
                    }
                }
                Err(e) => {
                    if attempts < self.config.max_retries {
                        attempts += 1;
                        let delay = Duration::from_secs(2_u64.pow(attempts));
                        warn!(
                            "APIリクエスト失敗、リトライします: attempt={attempts}/{}, delay={delay:?}",
                            self.config.max_retries
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    } else {
                        return Err(AppError::ExternalService(format!(
                            "APIサーバーへの接続に失敗しました: {e}"
                        )));
                    }
                }
            }
        }
    }

    /// APIサーバーのヘルスチェック（詳細版）
    pub async fn health_check_detailed(&self) -> Result<HealthCheckResult, AppError> {
        debug!("APIサーバー詳細ヘルスチェック開始");

        let url = format!("{}/api/v1/health", self.config.base_url);
        let start_time = std::time::Instant::now();

        match self.client.get(&url).send().await {
            Ok(response) => {
                let duration = start_time.elapsed();
                let status_code = response.status().as_u16();

                if response.status().is_success() {
                    // レスポンスボディを解析してより詳細な情報を取得
                    let health_info = response
                        .json::<serde_json::Value>()
                        .await
                        .unwrap_or_else(|_| serde_json::json!({"status": "ok"}));

                    debug!("APIサーバー詳細ヘルスチェック成功: duration={:?}", duration);
                    Ok(HealthCheckResult {
                        is_healthy: true,
                        response_time_ms: duration.as_millis() as u64,
                        status_code,
                        error_message: None,
                        details: Some(health_info),
                    })
                } else {
                    warn!(
                        "APIサーバー詳細ヘルスチェック失敗: HTTP {}, duration={:?}",
                        status_code, duration
                    );
                    Ok(HealthCheckResult {
                        is_healthy: false,
                        response_time_ms: duration.as_millis() as u64,
                        status_code,
                        error_message: Some(format!("HTTPステータス: {}", status_code)),
                        details: None,
                    })
                }
            }
            Err(e) => {
                let duration = start_time.elapsed();
                error!(
                    "APIサーバー詳細ヘルスチェックエラー: {}, duration={:?}",
                    e, duration
                );

                // エラーの種類に基づいて詳細な情報を提供
                let error_message = if e.is_timeout() {
                    "接続タイムアウト".to_string()
                } else if e.is_connect() {
                    "接続失敗（サーバーが起動していない可能性があります）".to_string()
                } else if e.is_request() {
                    "リクエスト送信失敗".to_string()
                } else {
                    format!("不明なエラー: {}", e)
                };

                Ok(HealthCheckResult {
                    is_healthy: false,
                    response_time_ms: duration.as_millis() as u64,
                    status_code: 0,
                    error_message: Some(error_message),
                    details: None,
                })
            }
        }
    }

    /// APIサーバーのヘルスチェック
    pub async fn health_check(&self) -> Result<bool, AppError> {
        let result = self.health_check_detailed().await?;
        Ok(result.is_healthy)
    }

    /// エラーレスポンスを処理し、詳細なエラー情報を提供
    async fn handle_error_response(&self, response: Response) -> Result<ErrorResponse, AppError> {
        let status = response.status();
        let status_code = status.as_u16();

        // レスポンスヘッダーからリクエストIDを取得
        let request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let response_text = response
            .text()
            .await
            .unwrap_or_else(|_| "レスポンス読み取り失敗".to_string());

        // JSONエラーレスポンスの解析を試行
        if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&response_text) {
            // APIサーバーからの構造化エラーレスポンス
            info!(
                "APIサーバーから構造化エラーレスポンスを受信: code={}, message={}",
                error_response.error.code, error_response.error.message
            );
            Ok(error_response)
        } else {
            // JSONでない場合は汎用エラーレスポンスを作成
            let (error_code, user_message) = match status_code {
                400 => ("BAD_REQUEST", "リクエストの形式が正しくありません"),
                401 => (
                    "UNAUTHORIZED",
                    "認証に失敗しました。再度ログインしてください",
                ),
                403 => ("FORBIDDEN", "この操作を実行する権限がありません"),
                404 => ("NOT_FOUND", "指定されたリソースが見つかりません"),
                413 => (
                    "PAYLOAD_TOO_LARGE",
                    "ファイルサイズが制限を超えています（最大10MB）",
                ),
                415 => (
                    "UNSUPPORTED_MEDIA_TYPE",
                    "サポートされていないファイル形式です",
                ),
                429 => (
                    "TOO_MANY_REQUESTS",
                    "リクエストが多すぎます。しばらく待ってから再試行してください",
                ),
                500 => ("INTERNAL_SERVER_ERROR", "サーバー内部エラーが発生しました"),
                502 => ("BAD_GATEWAY", "APIサーバーとの通信でエラーが発生しました"),
                503 => ("SERVICE_UNAVAILABLE", "APIサーバーが一時的に利用できません"),
                504 => (
                    "GATEWAY_TIMEOUT",
                    "APIサーバーからの応答がタイムアウトしました",
                ),
                _ => ("UNKNOWN_ERROR", "不明なエラーが発生しました"),
            };

            warn!(
                "APIサーバーから非構造化エラーレスポンス: status={}, body={}",
                status_code, response_text
            );

            Ok(ErrorResponse {
                error: ErrorDetail {
                    code: error_code.to_string(),
                    message: user_message.to_string(),
                    details: Some(serde_json::json!({
                        "http_status": status_code,
                        "raw_response": response_text,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    })),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    request_id,
                },
            })
        }
    }

    /// ファイル名からContent-Typeを取得
    fn get_content_type(&self, filename: &str) -> String {
        let extension = std::path::Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "pdf" => "application/pdf",
            "txt" => "text/plain",
            _ => "application/octet-stream",
        }
        .to_string()
    }
}

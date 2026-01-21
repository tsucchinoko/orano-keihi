use crate::shared::config::environment::ApiConfig;
/// 汎用APIクライアント
///
/// APIサーバーとの通信を行う汎用的なクライアント
/// サブスクリプション、経費、その他のAPIエンドポイントで使用可能
use crate::shared::errors::AppError;
use log::{debug, info, warn};
use reqwest::{Client, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
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
            base_url: "http://localhost:8787".to_string(),
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

/// 汎用APIクライアント
pub struct ApiClient {
    client: Client,
    config: ApiClientConfig,
}

impl ApiClient {
    /// 新しいAPIクライアントを作成
    pub fn new() -> Result<Self, AppError> {
        let config = ApiClientConfig::from_env();
        Self::new_with_config(config)
    }

    /// 設定を指定してAPIクライアントを作成
    pub fn new_with_config(config: ApiClientConfig) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| AppError::Configuration(format!("HTTPクライアント初期化失敗: {e}")))?;

        Ok(Self { client, config })
    }

    /// APIサーバーがlocalhostかどうかを判定
    pub fn is_localhost(&self) -> bool {
        self.config.base_url.contains("localhost") || self.config.base_url.contains("127.0.0.1")
    }

    /// GETリクエストを送信
    pub async fn get<T>(&self, endpoint: &str, auth_token: Option<&str>) -> Result<T, AppError>
    where
        T: DeserializeOwned,
    {
        info!("GETリクエスト送信: endpoint={endpoint}");

        let url = format!("{}{endpoint}", self.config.base_url);
        let mut request = self.client.get(&url);

        // 認証トークンがある場合は追加
        if let Some(token) = auth_token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        self.send_request_with_retry(request, "GET", endpoint).await
    }

    /// POSTリクエストを送信
    pub async fn post<B, T>(
        &self,
        endpoint: &str,
        body: &B,
        auth_token: Option<&str>,
    ) -> Result<T, AppError>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        info!("POSTリクエスト送信: endpoint={endpoint}");

        let url = format!("{}{endpoint}", self.config.base_url);
        let mut request = self.client.post(&url).json(body);

        // 認証トークンがある場合は追加
        if let Some(token) = auth_token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        self.send_request_with_retry(request, "POST", endpoint)
            .await
    }

    /// PUTリクエストを送信
    pub async fn put<B, T>(
        &self,
        endpoint: &str,
        body: &B,
        auth_token: Option<&str>,
    ) -> Result<T, AppError>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        info!("PUTリクエスト送信: endpoint={endpoint}");

        let url = format!("{}{endpoint}", self.config.base_url);
        let mut request = self.client.put(&url).json(body);

        // 認証トークンがある場合は追加
        if let Some(token) = auth_token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        self.send_request_with_retry(request, "PUT", endpoint).await
    }

    /// PATCHリクエストを送信
    pub async fn patch<B, T>(
        &self,
        endpoint: &str,
        body: &B,
        auth_token: Option<&str>,
    ) -> Result<T, AppError>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        info!("PATCHリクエスト送信: endpoint={endpoint}");

        let url = format!("{}{endpoint}", self.config.base_url);
        let mut request = self.client.patch(&url).json(body);

        // 認証トークンがある場合は追加
        if let Some(token) = auth_token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        self.send_request_with_retry(request, "PATCH", endpoint)
            .await
    }

    /// DELETEリクエストを送信
    pub async fn delete(&self, endpoint: &str, auth_token: Option<&str>) -> Result<(), AppError> {
        let url = format!("{}{endpoint}", self.config.base_url);
        info!("DELETEリクエスト送信: endpoint={endpoint}, url={url}");

        let mut request = self.client.delete(&url);

        // 認証トークンがある場合は追加
        if let Some(token) = auth_token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        // DELETEリクエストは通常レスポンスボディがないため、成功ステータスのみチェック
        let mut attempts = 0;
        loop {
            match request.try_clone() {
                Some(cloned_request) => match cloned_request.send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            info!("DELETEリクエスト成功: endpoint={endpoint}");
                            return Ok(());
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
                },
                None => {
                    return Err(AppError::ExternalService(
                        "リクエストのクローンに失敗しました".to_string(),
                    ));
                }
            }
        }
    }

    /// ボディ付きDELETEリクエストを送信
    pub async fn delete_with_body<T>(
        &self,
        endpoint: &str,
        body: &serde_json::Value,
        auth_token: Option<&str>,
    ) -> Result<T, AppError>
    where
        T: DeserializeOwned,
    {
        info!("ボディ付きDELETEリクエスト送信: endpoint={endpoint}");

        let url = format!("{}{endpoint}", self.config.base_url);
        let mut request = self.client.delete(&url).json(body);

        // 認証トークンがある場合は追加
        if let Some(token) = auth_token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        self.send_request_with_retry(request, "DELETE", endpoint)
            .await
    }

    /// リトライ機能付きでリクエストを送信
    async fn send_request_with_retry<T>(
        &self,
        request: reqwest::RequestBuilder,
        method: &str,
        endpoint: &str,
    ) -> Result<T, AppError>
    where
        T: DeserializeOwned,
    {
        let mut attempts = 0;
        loop {
            match request.try_clone() {
                Some(cloned_request) => match cloned_request.send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            let result: T = response.json().await.map_err(|e| {
                                AppError::ExternalService(format!("レスポンス解析エラー: {e}"))
                            })?;

                            info!("{method}リクエスト成功: endpoint={endpoint}");
                            return Ok(result);
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
                },
                None => {
                    return Err(AppError::ExternalService(
                        "リクエストのクローンに失敗しました".to_string(),
                    ));
                }
            }
        }
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
            debug!(
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
                413 => ("PAYLOAD_TOO_LARGE", "データサイズが制限を超えています"),
                415 => (
                    "UNSUPPORTED_MEDIA_TYPE",
                    "サポートされていないデータ形式です",
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
                "APIサーバーから非構造化エラーレスポンス: status={status_code}, body={response_text}"
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
}

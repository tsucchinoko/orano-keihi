/// 領収書関連のAPIコマンド
/// APIサーバー経由で領収書の取得・操作を行う
use crate::features::auth::middleware::AuthMiddleware;
use crate::features::receipts::api_client::{ApiClient, ApiClientConfig};
use crate::shared::api_client::ApiClient as SharedApiClient;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use tauri::State;

/// 領収書取得のレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct ReceiptResponse {
    pub data: String, // Base64エンコードされた画像データ
    pub content_type: String,
    pub file_size: u64,
}

/// アップロードレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub success: bool,
    pub file_key: String,
    pub file_url: String,
    pub file_size: u64,
    pub uploaded_at: String,
}

/// 複数アップロードレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct MultipleUploadResponse {
    pub success: bool,
    pub results: Vec<UploadResult>,
    pub total_files: usize,
    pub successful_uploads: usize,
    pub failed_uploads: usize,
}

/// アップロード結果
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResult {
    pub file_name: String,
    pub success: bool,
    pub file_key: Option<String>,
    pub file_url: Option<String>,
    pub error: Option<String>,
}

/// ヘルスチェックレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub environment: String,
}

/// APIサーバー経由で領収書を取得する
///
/// # 引数
/// * `receipt_url` - 領収書URL
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 領収書データ（Base64エンコード）、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_receipt_via_api(
    receipt_url: String,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<String, String> {
    info!("APIサーバー経由で領収書取得開始: receipt_url={receipt_url}");

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/api/receipts/get")
        .await
        .map_err(|e| {
            error!("認証エラー: {e}");
            format!("認証エラー: {e}")
        })?;

    debug!("認証成功 - ユーザーID: {}", user.id);

    // URLの基本検証
    if !receipt_url.starts_with("https://") {
        return Err("無効な領収書URLです".to_string());
    }

    // URLからファイルキーを抽出
    let file_key = extract_file_key_from_url(&receipt_url)?;
    debug!("抽出されたファイルキー: {file_key}");

    // APIクライアントを作成
    let api_client = SharedApiClient::new().map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // APIサーバーから領収書を取得
    let endpoint = format!("/api/v1/receipts/{}/data", file_key);

    debug!("APIエンドポイント: {endpoint}");

    let response = api_client
        .get::<ReceiptResponse>(&endpoint, session_token.as_deref())
        .await
        .map_err(|e| {
            error!("APIリクエストエラー: {e}");
            format!("領収書の取得に失敗しました: {e}")
        })?;

    info!(
        "領収書取得成功 - ユーザーID: {}, ファイルサイズ: {} bytes",
        user.id, response.file_size
    );

    Ok(response.data)
}

/// APIサーバー経由で領収書をアップロードする
///
/// # 引数
/// * `expense_id` - 経費ID
/// * `file_path` - ファイルパス
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// アップロード結果、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn upload_receipt_via_api(
    expense_id: i64,
    file_path: String,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<String, String> {
    info!(
        "APIサーバー経由で領収書アップロード開始: expense_id={expense_id}, file_path={file_path}"
    );

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/api/receipts/upload")
        .await
        .map_err(|e| {
            error!("認証エラー: {e}");
            format!("認証エラー: {e}")
        })?;

    debug!("認証成功 - ユーザーID: {}", user.id);

    // セッショントークンが必要
    let token = session_token.ok_or_else(|| {
        error!("セッショントークンが提供されていません");
        "セッショントークンが必要です".to_string()
    })?;

    // ファイルの存在確認
    if !std::path::Path::new(&file_path).exists() {
        return Err("指定されたファイルが存在しません".to_string());
    }

    // ファイルを読み込み
    let file_data = tokio::fs::read(&file_path).await.map_err(|e| {
        error!("ファイル読み込みエラー: {e}");
        format!("ファイル読み込みエラー: {e}")
    })?;

    // ファイル名を取得
    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "ファイル名を取得できません".to_string())?;

    // APIクライアントを作成
    let config = ApiClientConfig::from_env();
    let api_client = ApiClient::new(config).map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // ファイルをアップロード（ユーザーIDを渡す）
    match api_client
        .upload_file(expense_id, &file_data, filename, &user.id, &token)
        .await
    {
        Ok(response) => {
            let file_url = response.file_url.unwrap_or_else(|| "".to_string());
            info!("ファイルアップロード成功: file_url={file_url}");
            Ok(file_url)
        }
        Err(e) => {
            error!("ファイルアップロードエラー: {e}");
            Err(format!("ファイルアップロードエラー: {e}"))
        }
    }
}

/// APIサーバー経由で複数の領収書をアップロードする
///
/// # 引数
/// * `file_paths` - ファイルパスのリスト
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// アップロード結果、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn upload_multiple_receipts_via_api(
    file_paths: Vec<String>,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<MultipleUploadResponse, String> {
    info!(
        "APIサーバー経由で複数領収書アップロード開始: ファイル数={}",
        file_paths.len()
    );

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/api/receipts/upload/multiple")
        .await
        .map_err(|e| {
            error!("認証エラー: {e}");
            format!("認証エラー: {e}")
        })?;

    debug!("認証成功 - ユーザーID: {}", user.id);

    // 現在は未実装
    warn!("APIサーバー経由の複数ファイルアップロードは現在開発中です");

    let results: Vec<UploadResult> = file_paths
        .iter()
        .map(|path| UploadResult {
            file_name: std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            success: false,
            file_key: None,
            file_url: None,
            error: Some("APIサーバー経由のアップロードは現在サポートされていません".to_string()),
        })
        .collect();

    Ok(MultipleUploadResponse {
        success: false,
        results,
        total_files: file_paths.len(),
        successful_uploads: 0,
        failed_uploads: file_paths.len(),
    })
}

/// APIサーバーのヘルスチェック
///
/// # 戻り値
/// ヘルスチェック結果、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn check_api_server_health() -> Result<HealthCheckResponse, String> {
    info!("APIサーバーヘルスチェック開始");

    // APIクライアントを作成
    let api_client = SharedApiClient::new().map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // ヘルスチェックエンドポイントを呼び出し
    let response = api_client
        .get::<HealthCheckResponse>("/api/v1/health", None)
        .await
        .map_err(|e| {
            error!("ヘルスチェックエラー: {e}");
            format!("APIサーバーへの接続に失敗しました: {e}")
        })?;

    info!("APIサーバーヘルスチェック成功: status={}", response.status);

    Ok(response)
}

/// APIサーバーの詳細ヘルスチェック
///
/// # 戻り値
/// 詳細ヘルスチェック結果、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn check_api_server_health_detailed() -> Result<serde_json::Value, String> {
    info!("APIサーバー詳細ヘルスチェック開始");

    // APIクライアントを作成
    let api_client = SharedApiClient::new().map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // 詳細ヘルスチェックエンドポイントを呼び出し
    let response = api_client
        .get::<serde_json::Value>("/api/v1/health/detailed", None)
        .await
        .map_err(|e| {
            error!("詳細ヘルスチェックエラー: {e}");
            format!("APIサーバーへの接続に失敗しました: {e}")
        })?;

    info!("APIサーバー詳細ヘルスチェック成功");

    Ok(response)
}

/// フォールバックファイルの同期
///
/// # 戻り値
/// 同期結果、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn sync_fallback_files() -> Result<serde_json::Value, String> {
    info!("フォールバックファイル同期開始");

    // 現在は未実装
    warn!("フォールバックファイル同期は現在開発中です");

    Ok(serde_json::json!({
        "success": false,
        "message": "フォールバックファイル同期は現在サポートされていません",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// フォールバックファイル数の取得
///
/// # 戻り値
/// ファイル数、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_fallback_file_count() -> Result<i32, String> {
    info!("フォールバックファイル数取得開始");

    // 現在は未実装
    warn!("フォールバックファイル数取得は現在開発中です");

    Ok(0)
}

/// APIサーバー経由で領収書を削除する
///
/// # 引数
/// * `receipt_url` - 領収書URL
/// * `session_token` - セッショントークン
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 削除成功の場合はtrue、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_receipt_via_api(
    receipt_url: String,
    session_token: Option<String>,
    auth_middleware: State<'_, AuthMiddleware>,
) -> Result<bool, String> {
    info!("APIサーバー経由で領収書削除開始: receipt_url={receipt_url}");

    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/api/receipts/delete")
        .await
        .map_err(|e| {
            error!("認証エラー: {e}");
            format!("認証エラー: {e}")
        })?;

    debug!("認証成功 - ユーザーID: {}", user.id);

    // セッショントークンが必要
    let token = session_token.ok_or_else(|| {
        error!("セッショントークンが提供されていません");
        "セッショントークンが必要です".to_string()
    })?;

    // URLの基本検証
    if !receipt_url.starts_with("https://") {
        return Err("無効な領収書URLです".to_string());
    }

    debug!(
        "使用するセッショントークン: {}****",
        &token[..8.min(token.len())]
    );

    // APIクライアントを作成
    let api_client = SharedApiClient::new().map_err(|e| {
        error!("APIクライアント作成エラー: {e}");
        format!("APIクライアント作成エラー: {e}")
    })?;

    // 削除リクエストのペイロード
    let payload = serde_json::json!({
        "receiptUrl": receipt_url
    });

    debug!(
        "削除リクエストペイロード: {}",
        serde_json::to_string_pretty(&payload).unwrap_or_default()
    );

    // APIサーバーに削除リクエストを送信
    let endpoint = "/api/v1/receipts/delete-by-url";

    debug!("APIエンドポイント: {endpoint}");

    let response = api_client
        .delete_with_body::<serde_json::Value>(endpoint, &payload, Some(&token))
        .await
        .map_err(|e| {
            error!("APIリクエストエラー: {e}");
            format!("領収書の削除に失敗しました: {e}")
        })?;

    info!(
        "APIレスポンス受信: {}",
        serde_json::to_string_pretty(&response).unwrap_or_default()
    );

    // レスポンスから成功フラグを取得
    let success = response
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    info!("レスポンス解析結果: success={success}");

    if success {
        info!(
            "領収書削除成功 - ユーザーID: {}, receipt_url: {receipt_url}",
            user.id
        );
        Ok(true)
    } else {
        let error_message = response
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("不明なエラーが発生しました");

        error!("領収書削除失敗: {error_message}");
        Err(format!("領収書の削除に失敗しました: {error_message}"))
    }
}

/// URLからファイルキーを抽出する
///
/// # 引数
/// * `url` - 領収書URL
///
/// # 戻り値
/// ファイルキー、または失敗時はエラーメッセージ
fn extract_file_key_from_url(url: &str) -> Result<String, String> {
    // R2 URLの形式: https://{account_id}.r2.cloudflarestorage.com/{bucket_name}/{file_key}
    // または: https://r2.cloudflarestorage.com/{bucket_name}/{file_key}

    let url_parts: Vec<&str> = url.split('/').collect();
    if url_parts.len() < 5 {
        return Err("URLの形式が正しくありません".to_string());
    }

    // バケット名（url_parts[3]）を除いて、その後の部分をファイルキーとして取得
    let file_key_parts = &url_parts[4..];
    let file_key = file_key_parts.join("/");

    if file_key.is_empty() {
        return Err("ファイルキーの抽出に失敗しました".to_string());
    }

    debug!("URLからファイルキーを抽出: url={url}, file_key={file_key}");

    Ok(file_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_file_key_from_url() {
        // 正常なURL（バケット名を除いたファイルキーを取得）
        let url = "https://d6392b1230a419b37b30f45fc13de9cf.r2.cloudflarestorage.com/orano-keihi-dev/users/2/receipts/6/test.png";
        let result = extract_file_key_from_url(url);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "users/2/receipts/6/test.png");

        // 別の形式のURL
        let url2 = "https://r2.cloudflarestorage.com/bucket-name/path/to/file.jpg";
        let result2 = extract_file_key_from_url(url2);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), "path/to/file.jpg");

        // より複雑なファイルキー
        let url3 = "https://d6392b1230a419b37b30f45fc13de9cf.r2.cloudflarestorage.com/orano-keihi-dev/users/2/receipts/6/1766576410-52dd0bc2-4e34-4d20-9ae4-2f69d0ccb255-christmas-amidakuji-result-2025-12-24T10-15-07.png";
        let result3 = extract_file_key_from_url(url3);
        assert!(result3.is_ok());
        assert_eq!(result3.unwrap(), "users/2/receipts/6/1766576410-52dd0bc2-4e34-4d20-9ae4-2f69d0ccb255-christmas-amidakuji-result-2025-12-24T10-15-07.png");

        // 無効なURL（パーツが不足）
        let invalid_url = "https://example.com/file.jpg";
        let result4 = extract_file_key_from_url(invalid_url);
        assert!(result4.is_err());
    }

    #[test]
    fn test_upload_result_serialization() {
        let result = UploadResult {
            file_name: "test.jpg".to_string(),
            success: true,
            file_key: Some("users/1/receipts/test.jpg".to_string()),
            file_url: Some("https://example.com/test.jpg".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"file_name\":\"test.jpg\""));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_multiple_upload_response_serialization() {
        let response = MultipleUploadResponse {
            success: true,
            results: vec![],
            total_files: 0,
            successful_uploads: 0,
            failed_uploads: 0,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"total_files\":0"));
    }
}

// ユーザー認証付きR2コマンド
// R2ユーザーディレクトリ移行機能のための認証付きコマンド

use super::service::R2Client;
use crate::features::auth::middleware::AuthMiddleware;
use crate::features::expenses::repository as expense_operations;
use crate::shared::config::environment::R2Config;
use crate::shared::errors::AppError;
use crate::AppState;
use log::{debug, error, info};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::State;

/// ユーザー認証付きファイルアップロード
///
/// # 引数
/// * `session_token` - セッショントークン
/// * `expense_id` - 経費ID
/// * `file_path` - ローカルファイルパス
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// アップロードされたファイルのURL
#[tauri::command]
pub async fn upload_receipt_with_auth(
    session_token: String,
    expense_id: i64,
    file_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!(
        "ユーザー認証付きファイルアップロード開始: expense_id={expense_id}, file_path={file_path}"
    );

    // 認証ミドルウェアを初期化
    let auth_middleware = create_auth_middleware(&state).await?;

    // 認証を実行
    let user = auth_middleware
        .authenticate_request(Some(&session_token), "/receipts/upload")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    info!("認証成功: user_id={}", user.id);

    // アップロード処理を実行
    let result = upload_receipt_with_auth_internal(user.id, expense_id, file_path, state).await;

    match result {
        Ok(url) => {
            info!(
                "ユーザー認証付きファイルアップロード成功: user_id={}, expense_id={expense_id}, url={url}",
                user.id
            );
            Ok(url)
        }
        Err(app_error) => {
            let user_message = app_error.user_message();
            error!(
                "ユーザー認証付きファイルアップロード失敗: user_id={}, expense_id={expense_id}, error={app_error}",
                user.id
            );
            Err(user_message.to_string())
        }
    }
}

/// 内部的なアップロード処理
async fn upload_receipt_with_auth_internal(
    user_id: i64,
    expense_id: i64,
    file_path: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    // ファイルパスの検証
    let source_path = Path::new(&file_path);
    if !source_path.exists() {
        return Err(AppError::NotFound(format!(
            "指定されたファイルが存在しません: {file_path}"
        )));
    }

    // ファイル名を取得
    let filename = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            AppError::Validation(format!("ファイル名の取得に失敗しました: {file_path}"))
        })?;

    debug!("ファイル名を取得しました: {filename}");

    // ファイル形式の事前検証
    R2Client::validate_file_format(filename)?;

    // ファイルサイズの事前検証
    let metadata = fs::metadata(source_path)
        .map_err(|e| AppError::ExternalService(format!("ファイル情報取得失敗: {e}")))?;

    let file_size = metadata.len();
    debug!("ファイルサイズ: {file_size} bytes");

    R2Client::validate_file_size(file_size)?;

    // ファイルを読み込み
    let file_data = fs::read(source_path)
        .map_err(|e| AppError::ExternalService(format!("ファイル読み込み失敗: {e}")))?;

    info!("ファイルを読み込みました: {} bytes", file_data.len());

    // R2設定を読み込み
    let config = R2Config::from_env().ok_or_else(|| {
        AppError::Configuration(
            "R2設定読み込み失敗: 必要な環境変数が設定されていません".to_string(),
        )
    })?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await?;

    // Content-Typeを取得
    let content_type = R2Client::get_content_type(filename);
    debug!("Content-Type: {content_type}");

    // ユーザー認証付きでR2にアップロード
    let receipt_url = client
        .upload_file_with_user_auth(user_id, expense_id, filename, file_data, &content_type)
        .await?;

    info!("R2アップロードが成功しました: {receipt_url}");

    // データベースにreceipt_urlを保存
    {
        let db = state
            .db
            .lock()
            .map_err(|e| AppError::Database(format!("データベースロック取得エラー: {e}")))?;

        expense_operations::set_receipt_url(&db, expense_id, receipt_url.clone(), user_id)?;
    }

    info!(
        "データベースへの保存が完了しました: user_id={user_id}, expense_id={expense_id}, receipt_url={receipt_url}"
    );

    Ok(receipt_url)
}

/// ユーザー認証付きファイル取得
///
/// # 引数
/// * `session_token` - セッショントークン
/// * `receipt_url` - 領収書URL
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// ファイルデータ（Base64エンコード）
#[tauri::command]
pub async fn get_receipt_with_auth(
    session_token: String,
    receipt_url: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!("ユーザー認証付きファイル取得開始: receipt_url={receipt_url}");

    // 認証ミドルウェアを初期化
    let auth_middleware = create_auth_middleware(&state).await?;

    // 認証を実行
    let user = auth_middleware
        .authenticate_request(Some(&session_token), "/receipts/get")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    info!("認証成功: user_id={}", user.id);

    // ファイル取得処理を実行
    let result = get_receipt_with_auth_internal(user.id, receipt_url.clone(), state).await;

    match result {
        Ok(base64_data) => {
            info!(
                "ユーザー認証付きファイル取得成功: user_id={}, receipt_url={receipt_url}",
                user.id
            );
            Ok(base64_data)
        }
        Err(app_error) => {
            let user_message = app_error.user_message();
            error!(
                "ユーザー認証付きファイル取得失敗: user_id={}, receipt_url={receipt_url}, error={app_error}",
                user.id
            );
            Err(user_message.to_string())
        }
    }
}

/// 内部的なファイル取得処理
async fn get_receipt_with_auth_internal(
    user_id: i64,
    receipt_url: String,
    _state: State<'_, AppState>,
) -> Result<String, AppError> {
    // URLの検証
    if !receipt_url.starts_with("https://") {
        return Err(AppError::Validation(
            "領収書URLの形式が正しくありません（HTTPS URLである必要があります）".to_string(),
        ));
    }

    // R2設定を読み込み
    let config = R2Config::from_env().ok_or_else(|| {
        AppError::Configuration(
            "R2設定読み込み失敗: 必要な環境変数が設定されていません".to_string(),
        )
    })?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await?;

    // URLからパスを抽出
    let file_path = client.extract_path_from_url(&receipt_url)?;

    // ユーザー認証付きでファイルを取得
    let file_data = client
        .download_file_with_user_auth(user_id, &file_path)
        .await?;

    // Base64エンコードして返却
    use base64::{engine::general_purpose, Engine as _};
    let base64_data = general_purpose::STANDARD.encode(&file_data);

    Ok(base64_data)
}

/// ユーザー認証付きファイル削除
///
/// # 引数
/// * `session_token` - セッショントークン
/// * `receipt_url` - 領収書URL
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 削除成功の場合はtrue
#[tauri::command]
pub async fn delete_receipt_with_auth(
    session_token: String,
    receipt_url: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    info!("ユーザー認証付きファイル削除開始: receipt_url={receipt_url}");

    // 認証ミドルウェアを初期化
    let auth_middleware = create_auth_middleware(&state).await?;

    // 認証を実行
    let user = auth_middleware
        .authenticate_request(Some(&session_token), "/receipts/delete")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    info!("認証成功: user_id={}", user.id);

    // ファイル削除処理を実行
    let result = delete_receipt_with_auth_internal(user.id, receipt_url.clone(), state).await;

    match result {
        Ok(_) => {
            info!(
                "ユーザー認証付きファイル削除成功: user_id={}, receipt_url={receipt_url}",
                user.id
            );
            Ok(true)
        }
        Err(app_error) => {
            let user_message = app_error.user_message();
            error!(
                "ユーザー認証付きファイル削除失敗: user_id={}, receipt_url={receipt_url}, error={app_error}",
                user.id
            );
            Err(user_message.to_string())
        }
    }
}

/// 内部的なファイル削除処理
async fn delete_receipt_with_auth_internal(
    user_id: i64,
    receipt_url: String,
    _state: State<'_, AppState>,
) -> Result<(), AppError> {
    // URLの検証
    if !receipt_url.starts_with("https://") {
        return Err(AppError::Validation(
            "領収書URLの形式が正しくありません（HTTPS URLである必要があります）".to_string(),
        ));
    }

    // R2設定を読み込み
    let config = R2Config::from_env().ok_or_else(|| {
        AppError::Configuration(
            "R2設定読み込み失敗: 必要な環境変数が設定されていません".to_string(),
        )
    })?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await?;

    // URLからパスを抽出
    let file_path = client.extract_path_from_url(&receipt_url)?;

    // ユーザー認証付きでファイルを削除
    client
        .delete_file_with_user_auth(user_id, &file_path)
        .await?;

    Ok(())
}

/// ファイルダウンロード（Presigned URL使用）
///
/// # 引数
/// * `session_token` - セッショントークン
/// * `receipt_url` - 領収書URL
/// * `expires_in_seconds` - 有効期限（秒）
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// Presigned URL
#[tauri::command]
pub async fn download_receipt_with_auth(
    session_token: String,
    receipt_url: String,
    expires_in_seconds: Option<u64>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!("ユーザー認証付きファイルダウンロード開始: receipt_url={receipt_url}");

    // 認証ミドルウェアを初期化
    let auth_middleware = create_auth_middleware(&state).await?;

    // 認証を実行
    let user = auth_middleware
        .authenticate_request(Some(&session_token), "/receipts/download")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    info!("認証成功: user_id={}", user.id);

    // ダウンロード処理を実行
    let result = download_receipt_with_auth_internal(
        user.id,
        receipt_url.clone(),
        expires_in_seconds,
        state,
    )
    .await;

    match result {
        Ok(presigned_url) => {
            info!(
                "ユーザー認証付きファイルダウンロード成功: user_id={}, receipt_url={receipt_url}",
                user.id
            );
            Ok(presigned_url)
        }
        Err(app_error) => {
            let user_message = app_error.user_message();
            error!(
                "ユーザー認証付きファイルダウンロード失敗: user_id={}, receipt_url={receipt_url}, error={app_error}",
                user.id
            );
            Err(user_message.to_string())
        }
    }
}

/// 内部的なダウンロード処理
async fn download_receipt_with_auth_internal(
    user_id: i64,
    receipt_url: String,
    expires_in_seconds: Option<u64>,
    _state: State<'_, AppState>,
) -> Result<String, AppError> {
    // URLの検証
    if !receipt_url.starts_with("https://") {
        return Err(AppError::Validation(
            "領収書URLの形式が正しくありません（HTTPS URLである必要があります）".to_string(),
        ));
    }

    // R2設定を読み込み
    let config = R2Config::from_env().ok_or_else(|| {
        AppError::Configuration(
            "R2設定読み込み失敗: 必要な環境変数が設定されていません".to_string(),
        )
    })?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await?;

    // URLからパスを抽出
    let file_path = client.extract_path_from_url(&receipt_url)?;

    // 有効期限を設定（デフォルト1時間）
    let expires_in = Duration::from_secs(expires_in_seconds.unwrap_or(3600));

    // ユーザー認証付きでPresigned URLを生成
    let presigned_url = client
        .generate_presigned_url_with_user_auth(user_id, &file_path, expires_in)
        .await?;

    Ok(presigned_url)
}

/// URLからパス抽出（ユーティリティ）
///
/// # 引数
/// * `session_token` - セッショントークン
/// * `url` - 抽出対象のURL
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 抽出されたパス
#[tauri::command]
pub async fn extract_path_from_url_with_auth(
    session_token: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!("URLからパス抽出開始: url={url}");

    // 認証ミドルウェアを初期化
    let auth_middleware = create_auth_middleware(&state).await?;

    // 認証を実行
    let user = auth_middleware
        .authenticate_request(Some(&session_token), "/receipts/extract-path")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

    info!("認証成功: user_id={}", user.id);

    // R2設定を読み込み
    let config = R2Config::from_env()
        .ok_or_else(|| "R2設定読み込み失敗: 必要な環境変数が設定されていません".to_string())?;

    // R2クライアントを初期化
    let client = R2Client::new(config)
        .await
        .map_err(|e| format!("R2クライアント初期化エラー: {e}"))?;

    // URLからパスを抽出
    let path = client
        .extract_path_from_url(&url)
        .map_err(|e| format!("パス抽出エラー: {e}"))?;

    info!(
        "URLからパス抽出成功: user_id={}, url={url}, path={path}",
        user.id
    );

    Ok(path)
}

/// 認証ミドルウェアを作成するヘルパー関数
async fn create_auth_middleware(state: &State<'_, AppState>) -> Result<AuthMiddleware, String> {
    // 認証サービスを取得
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| "認証サービスが初期化されていません".to_string())?;

    let auth_service = Arc::new(auth_service.clone());

    // セキュリティサービスを取得（SecurityManagerはSecurityServiceのエイリアス）
    let security_service = Arc::new(state.security_manager.clone());

    Ok(AuthMiddleware::new(auth_service, security_service))
}

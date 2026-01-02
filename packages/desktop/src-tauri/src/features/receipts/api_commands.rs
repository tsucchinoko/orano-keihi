// APIサーバー経由でのファイルアップロードコマンド

use super::{
    api_client::{ApiClient, ApiClientConfig},
    models::{MultipleFileUploadInput, MultipleUploadResult, SingleUploadResult},
};
use crate::features::expenses::repository as expense_operations;
use crate::features::security::models::SecurityConfig;
use crate::features::security::service::SecurityManager;
use crate::shared::errors::AppError;
use crate::AppState;
use log::{debug, error, info, warn};
use std::fs;
use std::path::Path;
use tauri::State;

/// APIサーバー経由で領収書ファイルをアップロードする
///
/// # 引数
/// * `expense_id` - 経費ID
/// * `file_path` - 元のファイルパス
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// アップロードされた領収書のHTTPS URL、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn upload_receipt_via_api(
    expense_id: i64,
    file_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!("APIサーバー経由での領収書アップロードを開始します: expense_id={expense_id}, file_path={file_path}");

    let security_config = SecurityConfig {
        encryption_key: "default_key_32_bytes_long_enough".to_string(),
        max_token_age_hours: 24,
        enable_audit_logging: true,
    };
    let _security_manager =
        SecurityManager::new(security_config).expect("SecurityManager初期化失敗");

    // 統一エラーハンドリングを使用してアップロード処理を実行
    let result = upload_receipt_via_api_internal(expense_id, file_path, state).await;

    match result {
        Ok(url) => {
            info!("APIサーバー経由での領収書アップロード成功: expense_id={expense_id}, url={url}");
            Ok(url)
        }
        Err(app_error) => {
            let user_message = app_error.user_message();
            error!("APIサーバー経由での領収書アップロード失敗: expense_id={expense_id}, error={app_error}");
            Err(user_message.to_string())
        }
    }
}

/// 内部的なAPIサーバー経由アップロード処理
async fn upload_receipt_via_api_internal(
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

    // ファイルサイズの事前検証
    let metadata = fs::metadata(source_path)
        .map_err(|e| AppError::ExternalService(format!("ファイル情報取得失敗: {e}")))?;

    let file_size = metadata.len();
    debug!("ファイルサイズ: {file_size} bytes");

    // ファイルサイズ制限チェック（10MB）
    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
    if file_size > MAX_FILE_SIZE {
        return Err(AppError::Validation(format!(
            "ファイルサイズが制限を超えています: {file_size} bytes (最大: {MAX_FILE_SIZE} bytes)"
        )));
    }

    // ファイルを読み込み
    let file_data = fs::read(source_path)
        .map_err(|e| AppError::ExternalService(format!("ファイル読み込み失敗: {e}")))?;

    info!("ファイルを読み込みました: {} bytes", file_data.len());

    // APIクライアント設定を読み込み
    let api_config = ApiClientConfig::from_env();
    debug!("APIクライアント設定: base_url={}", api_config.base_url);

    // APIクライアントを初期化
    let api_client = ApiClient::new(api_config)?;

    // APIサーバーのヘルスチェック
    if let Err(e) = api_client.health_check().await {
        return Err(AppError::ExternalService(format!(
            "APIサーバーが利用できません: {e}"
        )));
    }

    // 認証トークンを取得（TODO: 実際の認証システムと連携）
    let auth_token = get_auth_token().await?;

    // 現在のreceipt_urlを保存（ロールバック用）
    let original_receipt_url = {
        let db = state
            .db
            .lock()
            .map_err(|e| AppError::Database(format!("データベースロック取得エラー: {e}")))?;

        expense_operations::get_receipt_url(&db, expense_id, 1i64)?
    };

    // APIサーバー経由でファイルをアップロード
    let upload_response = api_client
        .upload_file(expense_id, &file_path, file_data, filename, &auth_token)
        .await?;

    if !upload_response.success {
        return Err(AppError::ExternalService(format!(
            "APIサーバーでのアップロードに失敗しました: {:?}",
            upload_response.error
        )));
    }

    let receipt_url = upload_response.file_url.ok_or_else(|| {
        AppError::ExternalService("APIサーバーからファイルURLが返されませんでした".to_string())
    })?;

    info!("APIサーバー経由でのアップロードが成功しました: {receipt_url}");

    // データベースにreceipt_urlを保存（失敗時は状態を保持）
    let db_result = {
        let db = state
            .db
            .lock()
            .map_err(|e| AppError::Database(format!("データベースロック取得エラー: {e}")))?;

        expense_operations::set_receipt_url(&db, expense_id, receipt_url.clone(), 1i64)
    };

    match db_result {
        Ok(_) => {
            info!(
                "データベースへの保存が完了しました: expense_id={expense_id}, receipt_url={receipt_url}"
            );
            Ok(receipt_url)
        }
        Err(db_error) => {
            // データベース保存に失敗した場合、APIサーバー経由でファイルを削除してロールバック
            warn!(
                "データベース保存に失敗しました。APIサーバー経由でファイルを削除してロールバックします: {db_error}"
            );

            if let Err(delete_error) = api_client
                .delete_file(&upload_response.file_key, &auth_token)
                .await
            {
                error!("ロールバック中のAPIサーバー経由ファイル削除に失敗しました: {delete_error}");
            }

            // 元のreceipt_urlを復元（もしあれば）
            if let Some(original_url) = original_receipt_url {
                let db = state.db.lock().map_err(|e| {
                    AppError::Database(format!("ロールバック時のデータベースロック取得エラー: {e}"))
                })?;

                if let Err(restore_error) =
                    expense_operations::set_receipt_url(&db, expense_id, original_url, 1i64)
                {
                    error!("元のreceipt_urlの復元に失敗しました: {restore_error}");
                }
            }

            Err(db_error)
        }
    }
}

/// APIサーバー経由で複数ファイルを並列アップロードする
///
/// # 引数
/// * `files` - アップロードするファイルのリスト
/// * `max_concurrent` - 最大同時実行数（デフォルト: 3）
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// アップロード結果、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn upload_multiple_receipts_via_api(
    files: Vec<MultipleFileUploadInput>,
    max_concurrent: Option<usize>,
    state: State<'_, AppState>,
) -> Result<MultipleUploadResult, String> {
    let start_time = std::time::Instant::now();
    let _max_concurrent = max_concurrent.unwrap_or(3); // デフォルト3並列

    info!(
        "APIサーバー経由での複数ファイル並列アップロード開始: {} ファイル",
        files.len()
    );

    let security_config = SecurityConfig {
        encryption_key: "default_key_32_bytes_long_enough".to_string(),
        max_token_age_hours: 24,
        enable_audit_logging: true,
    };
    let _security_manager =
        SecurityManager::new(security_config).expect("SecurityManager初期化失敗");

    // APIクライアント設定を読み込み
    let api_config = ApiClientConfig::from_env();
    let api_client = ApiClient::new(api_config).map_err(|e| {
        let error_msg = format!("APIクライアントの初期化に失敗しました: {e}");
        error!("{error_msg}");
        error_msg
    })?;

    // APIサーバーのヘルスチェック
    if let Err(e) = api_client.health_check().await {
        let error_msg = format!("APIサーバーが利用できません: {e}");
        error!("{error_msg}");
        return Err(error_msg);
    }

    // 認証トークンを取得
    let auth_token = get_auth_token().await.map_err(|e| {
        let error_msg = format!("認証トークンの取得に失敗しました: {e}");
        error!("{error_msg}");
        error_msg
    })?;

    // ファイルを読み込んで準備
    let mut upload_files = Vec::new();

    for file_input in files {
        let source_path = Path::new(&file_input.file_path);

        // ファイル存在チェック
        if !source_path.exists() {
            warn!("ファイルが存在しません: {}", file_input.file_path);
            continue;
        }

        // ファイル名を取得
        let filename = source_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("ファイル名の取得に失敗しました: {}", file_input.file_path))?;

        // ファイルサイズの事前検証
        let metadata = fs::metadata(source_path).map_err(|e| {
            format!(
                "ファイル情報の取得に失敗しました: {} - {}",
                file_input.file_path, e
            )
        })?;

        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
        if metadata.len() > MAX_FILE_SIZE {
            warn!("ファイルサイズエラー: {filename} - サイズ制限超過");
            continue;
        }

        // ファイルを読み込み
        let file_data = fs::read(source_path).map_err(|e| {
            format!(
                "ファイルの読み込みに失敗しました: {} - {}",
                file_input.file_path, e
            )
        })?;

        upload_files.push((
            file_input.expense_id,
            file_input.file_path.clone(),
            file_data,
            filename.to_string(),
        ));
    }

    if upload_files.is_empty() {
        return Err("アップロード可能なファイルがありません".to_string());
    }

    // APIサーバー経由で複数ファイルをアップロード
    let upload_response = api_client
        .upload_multiple_files(upload_files.clone(), &auth_token)
        .await
        .map_err(|e| {
            let error_msg = format!("APIサーバー経由での並列アップロードに失敗しました: {e}");
            error!("{error_msg}");
            error_msg
        })?;

    // データベースに結果を保存
    let mut single_results = Vec::new();
    let mut successful_uploads = 0;
    let mut failed_uploads = 0;

    for (i, (expense_id, _file_path, _file_data, _filename)) in upload_files.iter().enumerate() {
        if let Some(result) = upload_response.results.get(i) {
            if result.success {
                if let Some(url) = &result.file_url {
                    // データベースにreceipt_urlを保存
                    let db = state
                        .db
                        .lock()
                        .map_err(|e| format!("データベースロックエラー: {e}"))?;

                    if let Err(e) =
                        expense_operations::set_receipt_url(&db, *expense_id, url.clone(), 1i64)
                    {
                        error!(
                            "データベース保存エラー: expense_id={}, error={}",
                            expense_id, e
                        );
                        failed_uploads += 1;
                        single_results.push(SingleUploadResult {
                            expense_id: *expense_id,
                            success: false,
                            url: None,
                            error: Some(format!("データベース保存エラー: {e}")),
                            file_size: result.file_size,
                            duration_ms: 0, // APIサーバーから取得できない場合は0
                        });
                    } else {
                        successful_uploads += 1;
                        single_results.push(SingleUploadResult {
                            expense_id: *expense_id,
                            success: true,
                            url: Some(url.clone()),
                            error: None,
                            file_size: result.file_size,
                            duration_ms: 0,
                        });
                    }
                }
            } else {
                failed_uploads += 1;
                single_results.push(SingleUploadResult {
                    expense_id: *expense_id,
                    success: false,
                    url: None,
                    error: result.error.clone(),
                    file_size: result.file_size,
                    duration_ms: 0,
                });
            }
        }
    }

    let total_duration = start_time.elapsed();

    info!(
        "APIサーバー経由での複数ファイル並列アップロード完了: 成功={successful_uploads}, 失敗={failed_uploads}, 総時間={total_duration:?}"
    );

    Ok(MultipleUploadResult {
        total_files: upload_files.len(),
        successful_uploads,
        failed_uploads,
        results: single_results,
        total_duration_ms: total_duration.as_millis() as u64,
    })
}

/// APIサーバーのヘルスチェックを実行する
///
/// # 引数
/// * `_state` - アプリケーション状態
///
/// # 戻り値
/// 接続成功時はtrue、失敗時はエラーメッセージ
#[tauri::command]
pub async fn check_api_server_health(_state: State<'_, AppState>) -> Result<bool, String> {
    info!("APIサーバーヘルスチェックを開始します");

    let _security_manager = SecurityManager::new(SecurityConfig {
        encryption_key: "default_key_32_bytes_long_enough".to_string(),
        max_token_age_hours: 24,
        enable_audit_logging: true,
    })
    .unwrap_or_else(|_| panic!("SecurityManager初期化失敗"));

    // APIクライアント設定を読み込み
    let api_config = ApiClientConfig::from_env();
    let api_client = ApiClient::new(api_config).map_err(|e| {
        let error_msg = format!("APIクライアントの初期化に失敗しました: {e}");
        error!("{error_msg}");
        error_msg
    })?;

    // ヘルスチェックを実行
    api_client.health_check().await.map_err(|e| {
        let error_msg = format!("APIサーバーヘルスチェックに失敗しました: {e}");
        error!("{error_msg}");
        error_msg
    })?;

    info!("APIサーバーヘルスチェックが成功しました");
    Ok(true)
}

/// 認証トークンを取得する（TODO: 実際の認証システムと連携）
async fn get_auth_token() -> Result<String, AppError> {
    // TODO: 実際の認証システムからトークンを取得
    // 現在はダミートークンを返す
    Ok("dummy_auth_token".to_string())
}

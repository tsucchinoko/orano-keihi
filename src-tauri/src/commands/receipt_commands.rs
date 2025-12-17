use crate::db::{expense_operations, subscription_operations};
use crate::services::{
    config::R2Config,
    r2_client::{MultipleFileUpload, PerformanceStats, R2Client},
    security::SecurityManager,
    AppError, ErrorHandler,
};
use crate::AppState;
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use log::{debug, error, info, warn};
use std::fs;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};
use tokio::sync::mpsc;

/// 領収書ファイルを保存する
///
/// # 引数
/// * `expense_id` - 経費ID
/// * `file_path` - 元のファイルパス
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 保存された領収書のファイルパス、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn save_receipt(
    expense_id: i64,
    file_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // ファイルパスの検証
    let source_path = Path::new(&file_path);
    if !source_path.exists() {
        return Err("指定されたファイルが存在しません".to_string());
    }

    // ファイルサイズの検証（10MB制限）
    let metadata =
        fs::metadata(source_path).map_err(|e| format!("ファイル情報の取得に失敗しました: {e}"))?;

    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
    if metadata.len() > MAX_FILE_SIZE {
        return Err("ファイルサイズが10MBを超えています".to_string());
    }

    // ファイル形式の検証（PNG/JPG/JPEG/PDF）
    let extension = source_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .ok_or_else(|| "ファイル拡張子が取得できません".to_string())?;

    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "pdf") {
        return Err("サポートされていないファイル形式です（PNG、JPG、PDFのみ対応）".to_string());
    }

    // アプリデータディレクトリを取得
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("アプリデータディレクトリの取得に失敗しました: {e}"))?;

    // receiptsディレクトリを作成
    let receipts_dir = app_data_dir.join("receipts");
    fs::create_dir_all(&receipts_dir)
        .map_err(|e| format!("receiptsディレクトリの作成に失敗しました: {e}"))?;

    // ユニークなファイル名を生成（expense_id_timestamp.ext）
    // JSTでタイムスタンプを取得
    let timestamp = Utc::now().with_timezone(&Tokyo).timestamp();
    let filename = format!("{expense_id}_{timestamp}.{extension}");
    let dest_path = receipts_dir.join(&filename);

    // ファイルをコピー
    fs::copy(source_path, &dest_path)
        .map_err(|e| format!("ファイルのコピーに失敗しました: {e}"))?;

    // データベースに領収書パスを保存
    let receipt_path_str = dest_path
        .to_str()
        .ok_or_else(|| "ファイルパスの変換に失敗しました".to_string())?
        .to_string();

    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    expense_operations::set_receipt_url(&db, expense_id, receipt_path_str.clone())
        .map_err(|e| format!("データベースへの保存に失敗しました: {e}"))?;

    Ok(receipt_path_str)
}

/// サブスクリプションの領収書ファイルを保存する
///
/// # 引数
/// * `subscription_id` - サブスクリプションID
/// * `file_path` - 元のファイルパス
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 保存された領収書のファイルパス、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn save_subscription_receipt(
    subscription_id: i64,
    file_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // ファイルパスの検証
    let source_path = Path::new(&file_path);
    if !source_path.exists() {
        return Err("指定されたファイルが存在しません".to_string());
    }

    // ファイルサイズの検証（10MB制限）
    let metadata =
        fs::metadata(source_path).map_err(|e| format!("ファイル情報の取得に失敗しました: {e}"))?;

    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
    if metadata.len() > MAX_FILE_SIZE {
        return Err("ファイルサイズが10MBを超えています".to_string());
    }

    // ファイル形式の検証（PNG/JPG/JPEG/PDF）
    let extension = source_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .ok_or_else(|| "ファイル拡張子が取得できません".to_string())?;

    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "pdf") {
        return Err("サポートされていないファイル形式です（PNG、JPG、PDFのみ対応）".to_string());
    }

    // アプリデータディレクトリを取得
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("アプリデータディレクトリの取得に失敗しました: {e}"))?;

    // receiptsディレクトリを作成
    let receipts_dir = app_data_dir.join("receipts");
    fs::create_dir_all(&receipts_dir)
        .map_err(|e| format!("receiptsディレクトリの作成に失敗しました: {e}"))?;

    // ユニークなファイル名を生成（subscription_id_timestamp.ext）
    // JSTでタイムスタンプを取得
    let timestamp = Utc::now().with_timezone(&Tokyo).timestamp();
    let filename = format!("sub_{subscription_id}_{timestamp}.{extension}");
    let dest_path = receipts_dir.join(&filename);

    // ファイルをコピー
    fs::copy(source_path, &dest_path)
        .map_err(|e| format!("ファイルのコピーに失敗しました: {e}"))?;

    // データベースに領収書パスを保存
    let receipt_path_str = dest_path
        .to_str()
        .ok_or_else(|| "ファイルパスの変換に失敗しました".to_string())?
        .to_string();

    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    subscription_operations::set_receipt_path(&db, subscription_id, receipt_path_str.clone())
        .map_err(|e| format!("データベースへの保存に失敗しました: {e}"))?;

    Ok(receipt_path_str)
}

/// 経費の領収書を削除する
///
/// # 引数
/// * `expense_id` - 経費ID
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 成功時はtrue、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_receipt(expense_id: i64, state: State<'_, AppState>) -> Result<bool, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 現在の領収書パスを取得
    let current_receipt_path = expense_operations::get_receipt_url(&db, expense_id)
        .map_err(|e| format!("領収書URLの取得に失敗しました: {e}"))?;

    if let Some(receipt_path) = current_receipt_path {
        // ファイルが存在する場合は削除
        let path = Path::new(&receipt_path);
        if path.exists() {
            fs::remove_file(path)
                .map_err(|e| format!("領収書ファイルの削除に失敗しました: {e}"))?;
        }
    }

    // データベースから領収書パスを削除（空文字に設定）
    expense_operations::set_receipt_url(&db, expense_id, "".to_string())
        .map_err(|e| format!("データベースからの領収書URL削除に失敗しました: {e}"))?;

    Ok(true)
}

/// サブスクリプションの領収書を削除する
///
/// # 引数
/// * `subscription_id` - サブスクリプションID
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 成功時はtrue、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_subscription_receipt(
    subscription_id: i64,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {e}"))?;

    // 現在の領収書パスを取得
    let current_receipt_path = subscription_operations::get_receipt_path(&db, subscription_id)
        .map_err(|e| format!("領収書パスの取得に失敗しました: {e}"))?;

    if let Some(receipt_path) = current_receipt_path {
        // ファイルが存在する場合は削除
        let path = Path::new(&receipt_path);
        if path.exists() {
            fs::remove_file(path)
                .map_err(|e| format!("領収書ファイルの削除に失敗しました: {e}"))?;
        }
    }

    // データベースから領収書パスを削除（空文字に設定）
    subscription_operations::set_receipt_path(&db, subscription_id, "".to_string())
        .map_err(|e| format!("データベースからの領収書パス削除に失敗しました: {e}"))?;

    Ok(true)
}
/// 領収書ファイルをR2にアップロードする（統一エラーハンドリング版）
///
/// # 引数
/// * `expense_id` - 経費ID
/// * `file_path` - 元のファイルパス
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// アップロードされた領収書のHTTPS URL、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn upload_receipt_to_r2(
    expense_id: i64,
    file_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!("R2への領収書アップロードを開始します: expense_id={expense_id}, file_path={file_path}");

    let security_manager = SecurityManager::new();
    security_manager.log_security_event(
        "receipt_upload_started",
        &format!("expense_id={expense_id}, file_path={file_path}"),
    );

    // 統一エラーハンドリングを使用してアップロード処理を実行
    let result = upload_receipt_internal(expense_id, file_path, state).await;

    match result {
        Ok(url) => {
            info!("領収書アップロード成功: expense_id={expense_id}, url={url}");
            security_manager.log_security_event(
                "receipt_upload_success",
                &format!("expense_id={expense_id}, url={url}"),
            );
            Ok(url)
        }
        Err(app_error) => {
            let user_message = ErrorHandler::handle_error(app_error);
            Err(user_message)
        }
    }
}

/// 内部的なアップロード処理（統一エラーハンドリング使用）
async fn upload_receipt_internal(
    expense_id: i64,
    file_path: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    // ファイルパスの検証
    let source_path = Path::new(&file_path);
    if !source_path.exists() {
        return Err(ErrorHandler::file_operation_error(
            "ファイル存在確認",
            &file_path,
            std::io::Error::new(std::io::ErrorKind::NotFound, "ファイルが存在しません"),
        ));
    }

    // ファイル名を取得
    let filename = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| AppError::FileOperationError {
            details: format!("ファイル名の取得に失敗しました: {file_path}"),
            user_message: "ファイル名を取得できませんでした。ファイルパスを確認してください。"
                .to_string(),
            retry_possible: false,
        })?;

    debug!("ファイル名を取得しました: {filename}");

    // ファイル形式の事前検証
    R2Client::validate_file_format(filename).map_err(|_| {
        ErrorHandler::invalid_file_format_error(filename, &["PNG", "JPG", "JPEG", "PDF"])
    })?;

    // ファイルサイズの事前検証
    let metadata = fs::metadata(source_path)
        .map_err(|e| ErrorHandler::file_operation_error("ファイル情報取得", &file_path, e))?;

    let file_size = metadata.len();
    debug!("ファイルサイズ: {file_size} bytes");

    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
    if file_size > MAX_FILE_SIZE {
        return Err(ErrorHandler::file_size_error(file_size, MAX_FILE_SIZE));
    }

    // ファイルを読み込み
    let file_data = fs::read(source_path)
        .map_err(|e| ErrorHandler::file_operation_error("ファイル読み込み", &file_path, e))?;

    info!("ファイルを読み込みました: {} bytes", file_data.len());

    // R2設定を読み込み
    let config = R2Config::from_env().map_err(AppError::from)?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await.map_err(AppError::from)?;

    // ファイルキーを生成
    let file_key = R2Client::generate_file_key(expense_id, filename);
    debug!("ファイルキーを生成しました: {file_key}");

    // Content-Typeを取得
    let content_type = R2Client::get_content_type(filename);
    debug!("Content-Type: {content_type}");

    // 現在のreceipt_urlを保存（ロールバック用）
    let original_receipt_url = {
        let db = state.db.lock().map_err(|e| AppError::DatabaseError {
            details: format!("データベースロック取得エラー: {e}"),
            user_message: "データベースへのアクセス中にエラーが発生しました。".to_string(),
            retry_possible: true,
        })?;

        expense_operations::get_receipt_url(&db, expense_id)
            .map_err(|e| ErrorHandler::database_error("receipt_url取得", e))?
    };

    // リトライ機能付きでR2にアップロード（最大3回リトライ）
    let receipt_url = client
        .upload_file_with_retry(&file_key, file_data, &content_type, 3)
        .await
        .map_err(AppError::from)?;

    info!("R2アップロードが成功しました: {receipt_url}");

    // データベースにreceipt_urlを保存（失敗時は状態を保持）
    let db_result = {
        let db = state.db.lock().map_err(|e| AppError::DatabaseError {
            details: format!("データベースロック取得エラー: {e}"),
            user_message: "データベースへのアクセス中にエラーが発生しました。".to_string(),
            retry_possible: true,
        })?;

        expense_operations::set_receipt_url(&db, expense_id, receipt_url.clone())
    };

    match db_result {
        Ok(_) => {
            info!(
                "データベースへの保存が完了しました: expense_id={expense_id}, receipt_url={receipt_url}"
            );
            Ok(receipt_url)
        }
        Err(db_error) => {
            // データベース保存に失敗した場合、R2からファイルを削除してロールバック
            warn!(
                "データベース保存に失敗しました。R2からファイルを削除してロールバックします: {db_error}"
            );

            if let Err(delete_error) = client.delete_file(&file_key).await {
                error!("ロールバック中のR2ファイル削除に失敗しました: {delete_error}");
            }

            // 元のreceipt_urlを復元（もしあれば）
            if let Some(original_url) = original_receipt_url {
                let db = state.db.lock().map_err(|e| AppError::DatabaseError {
                    details: format!("ロールバック時のデータベースロック取得エラー: {e}"),
                    user_message: "データベースへのアクセス中にエラーが発生しました。".to_string(),
                    retry_possible: true,
                })?;

                if let Err(restore_error) =
                    expense_operations::set_receipt_url(&db, expense_id, original_url)
                {
                    error!("元のreceipt_urlの復元に失敗しました: {restore_error}");
                }
            }

            Err(ErrorHandler::database_error("receipt_url保存", db_error))
        }
    }
}

/// R2から領収書を取得する（統一エラーハンドリング版）
///
/// # 引数
/// * `receipt_url` - 領収書のHTTPS URL
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// ファイルデータ（Base64エンコード）、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_receipt_from_r2(
    receipt_url: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!("R2からの領収書取得を開始します: receipt_url={receipt_url}");

    let security_manager = SecurityManager::new();
    security_manager.log_security_event(
        "receipt_download_started",
        &format!("receipt_url={receipt_url}"),
    );

    // 統一エラーハンドリングを使用して取得処理を実行
    let result = get_receipt_internal(receipt_url.clone(), app, state).await;

    match result {
        Ok(base64_data) => {
            info!("領収書取得成功: receipt_url={receipt_url}");
            security_manager.log_security_event(
                "receipt_download_success",
                &format!("receipt_url={receipt_url}"),
            );
            Ok(base64_data)
        }
        Err(app_error) => {
            let user_message = ErrorHandler::handle_error(app_error);
            Err(user_message)
        }
    }
}

/// 内部的な取得処理（統一エラーハンドリング使用）
async fn get_receipt_internal(
    receipt_url: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    // URLの検証
    if !receipt_url.starts_with("https://") {
        return Err(AppError::InvalidFileFormat {
            details: format!("無効なreceipt_URL: {receipt_url}"),
            user_message: "領収書URLの形式が正しくありません（HTTPS URLである必要があります）。"
                .to_string(),
            retry_possible: false,
        });
    }

    // キャッシュマネージャーを初期化
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::InternalError {
            details: format!("アプリデータディレクトリの取得に失敗しました: {e}"),
            user_message: "アプリケーションの設定取得中にエラーが発生しました。".to_string(),
            retry_possible: false,
        })?;

    let cache_dir = app_data_dir.join("receipt_cache");
    let cache_manager = crate::services::cache_manager::CacheManager::new(cache_dir, 100); // 100MB制限

    // まずキャッシュから取得を試行
    let cached_result = {
        let db = state.db.lock().map_err(|e| AppError::DatabaseError {
            details: format!("キャッシュ確認時のデータベースロック取得エラー: {e}"),
            user_message: "データベースへのアクセス中にエラーが発生しました。".to_string(),
            retry_possible: true,
        })?;

        cache_manager.get_cached_file(&receipt_url, &db)
    };

    match cached_result {
        Ok(Some(cached_data)) => {
            // キャッシュヒット - Base64エンコードして返却
            debug!("キャッシュヒット: receipt_url={receipt_url}");
            use base64::{engine::general_purpose, Engine as _};
            let base64_data = general_purpose::STANDARD.encode(&cached_data);
            return Ok(base64_data);
        }
        Ok(None) => {
            // キャッシュミス - R2から取得
            debug!("キャッシュミス、R2から取得します: receipt_url={receipt_url}");
        }
        Err(e) => {
            // キャッシュエラーはログに記録するが、R2からの取得を続行
            warn!("キャッシュ取得エラー（R2から取得を続行）: {e}");
        }
    }

    // R2から取得
    let file_data = download_from_r2_internal(&receipt_url).await?;

    // 取得したファイルをキャッシュに保存（エラーは無視）
    {
        let db = state.db.lock().map_err(|e| AppError::DatabaseError {
            details: format!("キャッシュ保存時のデータベースロック取得エラー: {e}"),
            user_message: "データベースへのアクセス中にエラーが発生しました。".to_string(),
            retry_possible: true,
        })?;

        if let Err(e) = cache_manager.cache_file(&receipt_url, file_data.clone(), &db) {
            warn!("キャッシュ保存エラー（無視して続行）: {e}");
        }

        // キャッシュサイズ管理（エラーは無視）
        if let Err(e) = cache_manager.manage_cache_size(&db) {
            warn!("キャッシュサイズ管理エラー（無視して続行）: {e}");
        }
    }

    // Base64エンコードして返却
    use base64::{engine::general_purpose, Engine as _};
    let base64_data = general_purpose::STANDARD.encode(&file_data);
    Ok(base64_data)
}

/// R2からファイルをダウンロードする内部関数（統一エラーハンドリング版）
async fn download_from_r2_internal(receipt_url: &str) -> Result<Vec<u8>, AppError> {
    // URLからファイルキーを抽出
    let url_parts: Vec<&str> = receipt_url.split('/').collect();
    if url_parts.len() < 4 {
        return Err(AppError::InvalidFileFormat {
            details: format!("無効なreceipt_URL形式: {receipt_url}"),
            user_message: "領収書URLの形式が正しくありません。".to_string(),
            retry_possible: false,
        });
    }

    // R2設定を読み込み
    let config = R2Config::from_env().map_err(AppError::from)?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await.map_err(AppError::from)?;

    // ファイルキーを抽出（receipts/expense_id/filename形式を想定）
    let file_key = if url_parts.len() >= 6 {
        // https://account_id.r2.cloudflarestorage.com/bucket_name/receipts/expense_id/filename
        url_parts[url_parts.len() - 3..].join("/")
    } else {
        return Err(AppError::InvalidFileFormat {
            details: format!("URLからファイルキーを抽出できません: {receipt_url}"),
            user_message: "領収書URLからファイル情報を取得できませんでした。".to_string(),
            retry_possible: false,
        });
    };

    // Presigned URLを生成（1時間有効）
    let presigned_url = client
        .generate_presigned_url(&file_key, Duration::from_secs(3600))
        .await
        .map_err(AppError::from)?;

    // リトライ機能付きでHTTPクライアントでファイルをダウンロード
    let mut attempts = 0;
    const MAX_RETRIES: u32 = 3;

    loop {
        match reqwest::get(&presigned_url).await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes().await {
                        Ok(file_data) => {
                            if attempts > 0 {
                                info!(
                                    "リトライ後にダウンロード成功: file_key={file_key}, attempts={attempts}"
                                );
                            }
                            return Ok(file_data.to_vec());
                        }
                        Err(e) => {
                            if attempts < MAX_RETRIES {
                                attempts += 1;
                                let delay = Duration::from_secs(2_u64.pow(attempts));
                                warn!("ファイルデータ取得失敗、リトライします: file_key={file_key}, attempt={attempts}/{MAX_RETRIES}, delay={delay:?}s");
                                tokio::time::sleep(delay).await;
                                continue;
                            } else {
                                return Err(AppError::DownloadFailed {
                                    details: format!("ファイルデータの取得に失敗しました: {e}"),
                                    user_message: "ファイルのダウンロード中にエラーが発生しました。しばらく時間をおいて再試行してください。".to_string(),
                                    retry_possible: true,
                                });
                            }
                        }
                    }
                } else if response.status().as_u16() == 404 {
                    return Err(AppError::FileNotFound {
                        details: format!("ファイルが見つかりません: {file_key}"),
                        user_message: "指定された領収書ファイルが見つかりません。ファイルが削除されている可能性があります。".to_string(),
                        retry_possible: false,
                    });
                } else {
                    return Err(AppError::DownloadFailed {
                        details: format!("ファイルダウンロードエラー: HTTP {}", response.status()),
                        user_message: "ファイルのダウンロード中にエラーが発生しました。しばらく時間をおいて再試行してください。".to_string(),
                        retry_possible: true,
                    });
                }
            }
            Err(e) => {
                if attempts < MAX_RETRIES {
                    attempts += 1;
                    let delay = Duration::from_secs(2_u64.pow(attempts));
                    warn!(
                        "ダウンロード失敗、リトライします: file_key={file_key}, attempt={attempts}/{MAX_RETRIES}, delay={delay:?}s"
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                } else {
                    return Err(AppError::NetworkError {
                        details: format!("ファイルダウンロードに失敗しました: {e}"),
                        user_message: "ネットワークエラーが発生しました。インターネット接続を確認してください。".to_string(),
                        retry_possible: true,
                    });
                }
            }
        }
    }
}



/// R2から領収書を削除する（統一エラーハンドリング版）
///
/// # 引数
/// * `expense_id` - 経費ID
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 成功時はtrue、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_receipt_from_r2(
    expense_id: i64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    info!("R2からの領収書削除を開始します: expense_id={expense_id}");

    let security_manager = SecurityManager::new();
    security_manager.log_security_event(
        "receipt_delete_started",
        &format!("expense_id={expense_id}"),
    );

    // 統一エラーハンドリングを使用して削除処理を実行
    let result = delete_receipt_internal(expense_id, app, state).await;

    match result {
        Ok(success) => {
            info!("領収書削除成功: expense_id={expense_id}");
            security_manager.log_security_event(
                "receipt_delete_success",
                &format!("expense_id={expense_id}"),
            );
            Ok(success)
        }
        Err(app_error) => {
            let user_message = ErrorHandler::handle_error(app_error);
            Err(user_message)
        }
    }
}

/// 内部的な削除処理（統一エラーハンドリング使用）
async fn delete_receipt_internal(
    expense_id: i64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    // 現在のreceipt_urlを取得
    let current_receipt_url = {
        let db = state.db.lock().map_err(|e| AppError::DatabaseError {
            details: format!("データベースロック取得エラー: {e}"),
            user_message: "データベースへのアクセス中にエラーが発生しました。".to_string(),
            retry_possible: true,
        })?;

        expense_operations::get_receipt_url(&db, expense_id)
            .map_err(|e| ErrorHandler::database_error("receipt_url取得", e))?
    };

    if let Some(receipt_url) = current_receipt_url {
        // キャッシュマネージャーを初期化
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| AppError::InternalError {
                details: format!("アプリデータディレクトリの取得に失敗しました: {e}"),
                user_message: "アプリケーションの設定取得中にエラーが発生しました。".to_string(),
                retry_possible: false,
            })?;

        let cache_dir = app_data_dir.join("receipt_cache");
        let cache_manager = crate::services::cache_manager::CacheManager::new(cache_dir, 100);

        // トランザクション的な削除処理：R2→キャッシュ→DB順
        // 1. R2からファイルを削除
        delete_from_r2_with_retry_internal(&receipt_url).await?;

        info!("R2からのファイル削除が成功しました: {receipt_url}");

        // 2. キャッシュからも削除（エラーは無視）
        {
            let db = state.db.lock().map_err(|e| AppError::DatabaseError {
                details: format!("キャッシュ削除時のデータベースロック取得エラー: {e}"),
                user_message: "データベースへのアクセス中にエラーが発生しました。".to_string(),
                retry_possible: true,
            })?;

            if let Err(e) = cache_manager.delete_cache_file(&receipt_url, &db) {
                warn!("キャッシュ削除エラー（無視して続行）: {e}");
            }
        }

        // 3. データベースからreceipt_urlを削除
        {
            let db = state.db.lock().map_err(|e| AppError::DatabaseError {
                details: format!("データベースロック取得エラー: {e}"),
                user_message: "データベースへのアクセス中にエラーが発生しました。".to_string(),
                retry_possible: true,
            })?;

            expense_operations::set_receipt_url(&db, expense_id, "".to_string())
                .map_err(|e| ErrorHandler::database_error("receipt_url削除", e))?;
        }

        // 削除操作のログ記録
        let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();
        info!(
            "領収書削除完了: expense_id={expense_id}, receipt_url={receipt_url}, timestamp={now}"
        );

        let security_manager = SecurityManager::new();
        security_manager.log_security_event(
            "receipt_delete_completed",
            &format!("expense_id={expense_id}, receipt_url={receipt_url}, timestamp={now}"),
        );
    } else {
        // receipt_urlが存在しない場合は何もしない
        info!("削除対象の領収書URLが存在しません: expense_id={expense_id}");
    }

    Ok(true)
}

/// R2からファイルを削除する内部関数（統一エラーハンドリング版）
async fn delete_from_r2_with_retry_internal(receipt_url: &str) -> Result<(), AppError> {
    // URLからファイルキーを抽出
    let url_parts: Vec<&str> = receipt_url.split('/').collect();
    if url_parts.len() < 4 {
        return Err(AppError::InvalidFileFormat {
            details: format!("無効なreceipt_URL形式: {receipt_url}"),
            user_message: "領収書URLの形式が正しくありません。".to_string(),
            retry_possible: false,
        });
    }

    // ファイルキーを抽出
    let file_key = if url_parts.len() >= 6 {
        url_parts[url_parts.len() - 3..].join("/")
    } else {
        return Err(AppError::InvalidFileFormat {
            details: format!("URLからファイルキーを抽出できません: {receipt_url}"),
            user_message: "領収書URLからファイル情報を取得できませんでした。".to_string(),
            retry_possible: false,
        });
    };

    // R2設定を読み込み
    let config = R2Config::from_env().map_err(AppError::from)?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await.map_err(AppError::from)?;

    // リトライ機能付きでR2からファイルを削除
    let mut attempts = 0;
    const MAX_RETRIES: u32 = 3;

    loop {
        match client.delete_file(&file_key).await {
            Ok(_) => {
                if attempts > 0 {
                    info!("リトライ後にR2削除成功: file_key={file_key}, attempts={attempts}");
                }
                return Ok(());
            }
            Err(r2_error) => {
                if attempts < MAX_RETRIES {
                    attempts += 1;
                    let delay = Duration::from_secs(2_u64.pow(attempts));
                    warn!(
                        "R2削除失敗、リトライします: file_key={file_key}, attempt={attempts}/{MAX_RETRIES}, delay={delay:?}s"
                    );

                    tokio::time::sleep(delay).await;
                    continue;
                } else {
                    error!(
                        "R2削除最終失敗: file_key={}, total_attempts={}",
                        file_key,
                        attempts + 1
                    );
                    return Err(AppError::from(r2_error));
                }
            }
        }
    }
}

/// R2からファイルを削除する内部関数（リトライ機能付き）
///
/// # 引数
/// * `receipt_url` - 領収書のHTTPS URL
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラーメッセージ
pub async fn delete_from_r2_with_retry(receipt_url: &str) -> Result<(), String> {
    // URLからファイルキーを抽出
    let url_parts: Vec<&str> = receipt_url.split('/').collect();
    if url_parts.len() < 4 {
        return Err("無効なreceipt_URLです".to_string());
    }

    // ファイルキーを抽出
    let file_key = if url_parts.len() >= 6 {
        url_parts[url_parts.len() - 3..].join("/")
    } else {
        return Err("URLからファイルキーを抽出できません".to_string());
    };

    // R2設定を読み込み
    let config =
        R2Config::from_env().map_err(|e| format!("R2設定の読み込みに失敗しました: {e}"))?;

    // R2クライアントを初期化
    let client = R2Client::new(config)
        .await
        .map_err(|e| format!("R2クライアントの初期化に失敗しました: {e}"))?;

    // リトライ機能付きでR2からファイルを削除
    let mut attempts = 0;
    const MAX_RETRIES: u32 = 3;

    loop {
        match client.delete_file(&file_key).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                if attempts < MAX_RETRIES {
                    attempts += 1;
                    let delay = Duration::from_secs(2_u64.pow(attempts));
                    tokio::time::sleep(delay).await;
                    continue;
                } else {
                    return Err(format!("R2削除エラー（最大リトライ回数に到達）: {e}"));
                }
            }
        }
    }
}

/// R2接続をテストする
///
/// # 引数
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 接続成功時はtrue、失敗時はエラーメッセージ（セキュリティ強化版）
#[tauri::command]
pub async fn test_r2_connection(_state: State<'_, AppState>) -> Result<bool, String> {
    info!("R2接続テストを開始します");

    let security_manager = SecurityManager::new();
    security_manager.log_security_event("r2_connection_test_started", "従来のR2接続テスト開始");

    // 環境変数からR2設定を読み込み
    let config = R2Config::from_env().map_err(|e| {
        let error_msg = format!("R2設定の読み込みに失敗しました: {e}");
        error!("{error_msg}");
        security_manager.log_security_event("r2_config_load_failed", &error_msg);
        error_msg
    })?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await.map_err(|e| {
        let error_msg = format!("R2クライアントの初期化に失敗しました: {e}");
        error!("{error_msg}");
        security_manager.log_security_event("r2_client_init_failed", &error_msg);
        error_msg
    })?;

    // 接続テストを実行
    client.test_connection().await.map_err(|e| {
        let error_msg = format!("R2接続テストに失敗しました: {e}");
        error!("{error_msg}");
        security_manager.log_security_event("r2_connection_test_failed", &error_msg);
        error_msg
    })?;

    info!("R2接続テストが成功しました");
    security_manager.log_security_event("r2_connection_test_success", "従来のR2接続テスト成功");
    Ok(true)
}

/// オフライン時に領収書をキャッシュから取得する
///
/// # 引数
/// * `receipt_url` - 領収書のHTTPS URL
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// キャッシュされたファイルデータ（Base64エンコード）、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_receipt_offline(
    receipt_url: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // URLの検証
    if !receipt_url.starts_with("https://") {
        return Err("無効なreceipt_URLです（HTTPS URLである必要があります）".to_string());
    }

    // キャッシュマネージャーを初期化
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("アプリデータディレクトリの取得に失敗しました: {e}"))?;

    let cache_dir = app_data_dir.join("receipt_cache");
    let cache_manager = crate::services::cache_manager::CacheManager::new(cache_dir, 100);

    // オフライン時のキャッシュから取得
    let cached_result = {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {e}"))?;
        cache_manager.get_offline_cached_file(&receipt_url, &db)
    };

    match cached_result {
        Ok(Some(cached_data)) => {
            // キャッシュヒット - Base64エンコードして返却
            use base64::{Engine as _, engine::general_purpose};
            let base64_data = general_purpose::STANDARD.encode(&cached_data);
            Ok(base64_data)
        }
        Ok(None) => {
            Err("オフライン時：領収書がキャッシュに見つかりません。オンライン時に一度表示してください。".to_string())
        }
        Err(e) => {
            Err(format!("キャッシュ取得エラー: {e}"))
        }
    }
}

/// オンライン復帰時にキャッシュを同期する
///
/// # 引数
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 同期されたキャッシュ数、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn sync_cache_on_online(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    // キャッシュマネージャーを初期化
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("アプリデータディレクトリの取得に失敗しました: {e}"))?;

    let cache_dir = app_data_dir.join("receipt_cache");
    let cache_manager = crate::services::cache_manager::CacheManager::new(cache_dir, 100);

    // キャッシュ同期を実行（同期版を使用）
    let sync_result: Result<usize, String> = {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {e}"))?;

        // 古いキャッシュをクリーンアップ
        let cleaned_count = cache_manager
            .cleanup_old_cache(&db)
            .map_err(|e| format!("キャッシュクリーンアップエラー: {e}"))?;

        // キャッシュサイズを管理
        cache_manager
            .manage_cache_size(&db)
            .map_err(|e| format!("キャッシュサイズ管理エラー: {e}"))?;

        println!("キャッシュ同期完了: {cleaned_count}個のファイルをクリーンアップしました");

        Ok(cleaned_count)
    };

    match sync_result {
        Ok(synced_count) => Ok(synced_count),
        Err(e) => Err(format!("キャッシュ同期エラー: {e}")),
    }
}

/// キャッシュ統計情報を取得する
///
/// # 引数
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// キャッシュ統計情報、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_cache_stats(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CacheStats, String> {
    // キャッシュマネージャーを初期化
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("アプリデータディレクトリの取得に失敗しました: {e}"))?;

    let cache_dir = app_data_dir.join("receipt_cache");
    let cache_manager = crate::services::cache_manager::CacheManager::new(cache_dir, 100);

    // キャッシュサイズを計算（同期版を使用）
    let current_size = cache_manager
        .calculate_cache_size_sync()
        .map_err(|e| format!("キャッシュサイズ計算エラー: {e}"))?;

    // データベースからキャッシュ数を取得
    let cache_count = {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {e}"))?;

        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM receipt_cache", [], |row| row.get(0))
            .map_err(|e| format!("キャッシュ数取得エラー: {e}"))?;

        count as usize
    };

    Ok(CacheStats {
        total_files: cache_count,
        total_size_bytes: current_size,
        max_size_bytes: cache_manager.max_cache_size,
        cache_hit_rate: 0.0, // 実装を簡略化
    })
}

/// キャッシュ統計情報の構造体
#[derive(serde::Serialize)]
pub struct CacheStats {
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub max_size_bytes: u64,
    pub cache_hit_rate: f64,
}

/// 複数ファイルアップロード用の入力構造体
#[derive(serde::Deserialize)]
pub struct MultipleFileUploadInput {
    pub expense_id: i64,
    pub file_path: String,
}

/// 複数ファイルアップロード結果の構造体
#[derive(serde::Serialize)]
pub struct MultipleUploadResult {
    pub total_files: usize,
    pub successful_uploads: usize,
    pub failed_uploads: usize,
    pub results: Vec<SingleUploadResult>,
    pub total_duration_ms: u64,
}

/// 単一アップロード結果の構造体
#[derive(serde::Serialize)]
pub struct SingleUploadResult {
    pub expense_id: i64,
    pub success: bool,
    pub url: Option<String>,
    pub error: Option<String>,
    pub file_size: u64,
    pub duration_ms: u64,
}

/// 複数ファイルを並列でR2にアップロードする
///
/// # 引数
/// * `files` - アップロードするファイルのリスト
/// * `max_concurrent` - 最大同時実行数（デフォルト: 3）
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// アップロード結果、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn upload_multiple_receipts_to_r2(
    files: Vec<MultipleFileUploadInput>,
    max_concurrent: Option<usize>,
    state: State<'_, AppState>,
) -> Result<MultipleUploadResult, String> {
    let start_time = std::time::Instant::now();
    let max_concurrent = max_concurrent.unwrap_or(3); // デフォルト3並列

    info!(
        "複数ファイル並列アップロード開始: {} ファイル, 最大同時実行数: {}",
        files.len(),
        max_concurrent
    );

    let security_manager = SecurityManager::new();
    security_manager.log_security_event(
        "multiple_upload_started",
        &format!(
            "files_count={}, max_concurrent={}",
            files.len(),
            max_concurrent
        ),
    );

    // R2設定を読み込み
    let config = R2Config::from_env().map_err(|e| {
        let error_msg = format!("R2設定の読み込みに失敗しました: {e}");
        error!("{error_msg}");
        security_manager.log_security_event("r2_config_load_failed", &format!("error={e:?}"));
        error_msg
    })?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await.map_err(|e| {
        let error_msg = format!("R2クライアントの初期化に失敗しました: {e}");
        error!("{error_msg}");
        security_manager.log_security_event("r2_client_init_failed", &format!("error={e}"));
        error_msg
    })?;

    // ファイルを読み込んでMultipleFileUpload構造体に変換
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

        // ファイル形式の事前検証
        if let Err(e) = R2Client::validate_file_format(filename) {
            warn!("ファイル形式エラー: {filename} - {e}");
            continue;
        }

        // ファイルサイズの事前検証
        let metadata = fs::metadata(source_path).map_err(|e| {
            format!(
                "ファイル情報の取得に失敗しました: {} - {}",
                file_input.file_path, e
            )
        })?;

        if let Err(e) = R2Client::validate_file_size(metadata.len()) {
            warn!("ファイルサイズエラー: {filename} - {e}");
            continue;
        }

        // ファイルを読み込み
        let file_data = fs::read(source_path).map_err(|e| {
            format!(
                "ファイルの読み込みに失敗しました: {} - {}",
                file_input.file_path, e
            )
        })?;

        // ファイルキーを生成
        let file_key = R2Client::generate_file_key(file_input.expense_id, filename);
        let content_type = R2Client::get_content_type(filename);

        upload_files.push(MultipleFileUpload {
            file_key,
            file_data,
            content_type,
            expense_id: file_input.expense_id,
        });
    }

    if upload_files.is_empty() {
        return Err("アップロード可能なファイルがありません".to_string());
    }

    // プログレス通知用チャンネル（オプション）
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();

    // プログレス受信タスクを起動（バックグラウンドで実行）
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            debug!("アップロードプログレス: {progress:?}");
            // 必要に応じてフロントエンドに通知
        }
    });

    // 並列アップロード実行
    let upload_results = client
        .upload_multiple_files(
            upload_files.clone(),
            max_concurrent,
            Some(progress_tx),
            None, // キャンセルトークンは今回は使用しない
        )
        .await
        .map_err(|e| {
            let error_msg = format!("並列アップロードに失敗しました: {e}");
            error!("{error_msg}");
            security_manager.log_security_event("parallel_upload_failed", &format!("error={e}"));
            error_msg
        })?;

    // データベースに結果を保存
    let mut single_results = Vec::new();
    let mut successful_uploads = 0;
    let mut failed_uploads = 0;

    for (upload_file, upload_result) in upload_files.iter().zip(upload_results.iter()) {
        if upload_result.success {
            if let Some(url) = &upload_result.url {
                // データベースにreceipt_urlを保存
                let db = state
                    .db
                    .lock()
                    .map_err(|e| format!("データベースロックエラー: {e}"))?;

                if let Err(e) =
                    expense_operations::set_receipt_url(&db, upload_file.expense_id, url.clone())
                {
                    error!(
                        "データベース保存エラー: expense_id={}, error={}",
                        upload_file.expense_id, e
                    );
                    failed_uploads += 1;
                    single_results.push(SingleUploadResult {
                        expense_id: upload_file.expense_id,
                        success: false,
                        url: None,
                        error: Some(format!("データベース保存エラー: {e}")),
                        file_size: upload_result.file_size,
                        duration_ms: upload_result.duration.as_millis() as u64,
                    });
                } else {
                    successful_uploads += 1;
                    single_results.push(SingleUploadResult {
                        expense_id: upload_file.expense_id,
                        success: true,
                        url: Some(url.clone()),
                        error: None,
                        file_size: upload_result.file_size,
                        duration_ms: upload_result.duration.as_millis() as u64,
                    });
                }
            }
        } else {
            failed_uploads += 1;
            single_results.push(SingleUploadResult {
                expense_id: upload_file.expense_id,
                success: false,
                url: None,
                error: upload_result.error.clone(),
                file_size: upload_result.file_size,
                duration_ms: upload_result.duration.as_millis() as u64,
            });
        }
    }

    let total_duration = start_time.elapsed();

    info!(
        "複数ファイル並列アップロード完了: 成功={successful_uploads}, 失敗={failed_uploads}, 総時間={total_duration:?}"
    );

    security_manager.log_security_event(
        "multiple_upload_completed",
        &format!(
            "total_files={}, successful={}, failed={}, duration={:?}",
            upload_files.len(),
            successful_uploads,
            failed_uploads,
            total_duration
        ),
    );

    Ok(MultipleUploadResult {
        total_files: upload_files.len(),
        successful_uploads,
        failed_uploads,
        results: single_results,
        total_duration_ms: total_duration.as_millis() as u64,
    })
}

/// アップロードをキャンセルする（将来の実装用）
///
/// # 引数
/// * `upload_id` - アップロードID（将来の実装で使用）
///
/// # 戻り値
/// キャンセル成功時はtrue、失敗時はエラーメッセージ
#[tauri::command]
pub async fn cancel_upload(upload_id: String) -> Result<bool, String> {
    // 現在の実装では簡単なログ出力のみ
    info!("アップロードキャンセル要求: upload_id={upload_id}");

    // 将来的にはアクティブなアップロードタスクを管理し、
    // キャンセルトークンを使用してタスクを停止する実装を追加

    Ok(true)
}

/// R2パフォーマンス統計を取得する
///
/// # 引数
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// パフォーマンス統計、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_r2_performance_stats(
    state: State<'_, AppState>,
) -> Result<PerformanceStats, String> {
    info!("R2パフォーマンス統計取得開始");

    let security_manager = &state.security_manager;
    security_manager.log_security_event(
        "performance_stats_requested",
        "R2パフォーマンス統計取得開始",
    );

    // R2設定を読み込み
    let config = R2Config::from_env().map_err(|e| {
        let error_msg = format!("R2設定の読み込みに失敗しました: {e}");
        error!("{error_msg}");
        security_manager.log_security_event("r2_config_load_failed", &format!("error={e:?}"));
        error_msg
    })?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await.map_err(|e| {
        let error_msg = format!("R2クライアントの初期化に失敗しました: {e}");
        error!("{error_msg}");
        security_manager.log_security_event("r2_client_init_failed", &format!("error={e}"));
        error_msg
    })?;

    // キャッシュされた接続テスト結果を確認
    let connection_cache = state.r2_connection_cache.clone();
    let cached_result = {
        let cache = connection_cache.lock().unwrap();
        cache.get_cached_result()
    };

    // キャッシュが有効で接続が失敗している場合は、統計取得をスキップ
    if let Some(false) = cached_result {
        let error_msg = "R2接続が利用できません（キャッシュされた結果）".to_string();
        security_manager
            .log_security_event("performance_stats_skipped", "cached_connection_failed");
        return Err(error_msg);
    }

    // パフォーマンス統計を取得（接続テストを含む）
    let stats = client
        .get_performance_stats_with_cache(connection_cache)
        .await
        .map_err(|e| {
            let error_msg = format!("パフォーマンス統計の取得に失敗しました: {e}");
            error!("{error_msg}");
            security_manager.log_security_event("performance_stats_failed", &format!("error={e}"));
            error_msg
        })?;

    info!(
        "R2パフォーマンス統計取得完了: レイテンシ={}ms, スループット={}bps",
        stats.latency_ms, stats.throughput_bps
    );

    security_manager.log_security_event(
        "performance_stats_success",
        &format!(
            "latency={}ms, throughput={}bps",
            stats.latency_ms, stats.throughput_bps
        ),
    );

    Ok(stats)
}

// ========== 統合テストとデバッグ機能 ==========

/// R2接続の詳細テスト（統合テスト機能）
///
/// # 引数
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 詳細なテスト結果、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn test_r2_connection_detailed(
    _state: State<'_, AppState>,
) -> Result<R2ConnectionTestResult, String> {
    info!("R2接続詳細テストを開始します");

    let security_manager = SecurityManager::new();
    security_manager.log_security_event("r2_detailed_test_started", "R2接続詳細テスト開始");

    let start_time = std::time::Instant::now();
    let mut test_result = R2ConnectionTestResult {
        overall_success: false,
        config_validation: TestStepResult::default(),
        client_initialization: TestStepResult::default(),
        bucket_access: TestStepResult::default(),
        upload_test: TestStepResult::default(),
        download_test: TestStepResult::default(),
        delete_test: TestStepResult::default(),
        performance_metrics: None,
        total_duration_ms: 0,
        environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
    };

    // 1. 設定検証テスト
    info!("ステップ 1: R2設定検証");
    let config_result = test_config_validation().await;
    test_result.config_validation = config_result.clone();

    if !config_result.success {
        test_result.total_duration_ms = start_time.elapsed().as_millis() as u64;
        security_manager.log_security_event("r2_detailed_test_failed", "設定検証で失敗");
        return Ok(test_result);
    }

    // 2. クライアント初期化テスト
    info!("ステップ 2: R2クライアント初期化");
    let (client_result, client_opt) = test_client_initialization().await;
    test_result.client_initialization = client_result.clone();

    if !client_result.success || client_opt.is_none() {
        test_result.total_duration_ms = start_time.elapsed().as_millis() as u64;
        security_manager.log_security_event("r2_detailed_test_failed", "クライアント初期化で失敗");
        return Ok(test_result);
    }

    let client = client_opt.unwrap();

    // 3. バケットアクセステスト
    info!("ステップ 3: バケットアクセステスト");
    let bucket_result = test_bucket_access(&client).await;
    test_result.bucket_access = bucket_result.clone();

    if !bucket_result.success {
        test_result.total_duration_ms = start_time.elapsed().as_millis() as u64;
        security_manager.log_security_event("r2_detailed_test_failed", "バケットアクセスで失敗");
        return Ok(test_result);
    }

    // 4. アップロードテスト
    info!("ステップ 4: アップロードテスト");
    let (upload_result, test_key_opt) = test_upload_functionality(&client).await;
    test_result.upload_test = upload_result.clone();

    if !upload_result.success {
        test_result.total_duration_ms = start_time.elapsed().as_millis() as u64;
        security_manager.log_security_event("r2_detailed_test_failed", "アップロードテストで失敗");
        return Ok(test_result);
    }

    // 5. ダウンロードテスト
    info!("ステップ 5: ダウンロードテスト");
    let download_result = if let Some(test_key) = &test_key_opt {
        test_download_functionality(&client, test_key).await
    } else {
        TestStepResult {
            success: false,
            message: "テストキーが利用できません".to_string(),
            duration_ms: 0,
            details: None,
        }
    };
    test_result.download_test = download_result.clone();

    // 6. 削除テスト
    info!("ステップ 6: 削除テスト");
    let delete_result = if let Some(test_key) = &test_key_opt {
        test_delete_functionality(&client, test_key).await
    } else {
        TestStepResult {
            success: false,
            message: "テストキーが利用できません".to_string(),
            duration_ms: 0,
            details: None,
        }
    };
    test_result.delete_test = delete_result.clone();

    // 7. パフォーマンス測定
    info!("ステップ 7: パフォーマンス測定");
    if let Ok(perf_stats) = client.get_performance_stats().await {
        test_result.performance_metrics = Some(perf_stats);
    }

    // 全体の成功判定
    test_result.overall_success = test_result.config_validation.success
        && test_result.client_initialization.success
        && test_result.bucket_access.success
        && test_result.upload_test.success
        && test_result.download_test.success
        && test_result.delete_test.success;

    test_result.total_duration_ms = start_time.elapsed().as_millis() as u64;

    let result_status = if test_result.overall_success {
        "成功"
    } else {
        "失敗"
    };
    info!(
        "R2接続詳細テスト完了: {} (総時間: {}ms)",
        result_status, test_result.total_duration_ms
    );

    security_manager.log_security_event(
        "r2_detailed_test_completed",
        &format!(
            "success={}, duration={}ms",
            test_result.overall_success, test_result.total_duration_ms
        ),
    );

    Ok(test_result)
}

/// R2使用量監視情報を取得する
///
/// # 引数
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 使用量監視情報、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_r2_usage_monitoring(state: State<'_, AppState>) -> Result<R2UsageInfo, String> {
    info!("R2使用量監視情報取得開始");

    let security_manager = SecurityManager::new();
    security_manager.log_security_event("r2_usage_monitoring_started", "R2使用量監視情報取得開始");

    // データベースから統計情報を取得
    let db_stats = {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {e}"))?;

        // 領収書数を取得
        let total_receipts: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM expenses WHERE receipt_url IS NOT NULL AND receipt_url != ''",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("領収書数取得エラー: {e}"))?;

        // 今月のアップロード数を取得（JSTベース）
        let now_jst = Utc::now().with_timezone(&Tokyo);
        let current_month = now_jst.format("%Y-%m").to_string();

        let monthly_uploads: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM expenses WHERE receipt_url IS NOT NULL AND receipt_url != '' AND date LIKE ?",
                [format!("{current_month}%")],
                |row| row.get(0),
            )
            .map_err(|e| format!("月間アップロード数取得エラー: {e}"))?;

        // 今日のアップロード数を取得（JSTベース）
        let today = now_jst.format("%Y-%m-%d").to_string();

        let daily_uploads: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM expenses WHERE receipt_url IS NOT NULL AND receipt_url != '' AND date = ?",
                [today],
                |row| row.get(0),
            )
            .map_err(|e| format!("日間アップロード数取得エラー: {e}"))?;

        (
            total_receipts as u64,
            monthly_uploads as u64,
            daily_uploads as u64,
        )
    };

    // キャッシュ統計を取得（簡易実装）
    let cache_stats = {
        let app_data_dir = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME").map(|h| format!("{h}/.local/share")))
            .unwrap_or_else(|_| "/tmp".to_string());

        let cache_dir = std::path::PathBuf::from(app_data_dir).join("receipt_cache");
        let cache_manager = crate::services::cache_manager::CacheManager::new(cache_dir, 100);

        match cache_manager.calculate_cache_size_sync() {
            Ok(current_size) => {
                let db = state
                    .db
                    .lock()
                    .map_err(|e| format!("データベースロックエラー: {e}"))?;

                let cache_count: i64 = db
                    .query_row("SELECT COUNT(*) FROM receipt_cache", [], |row| row.get(0))
                    .unwrap_or(0);

                Some(crate::commands::receipt_commands::CacheStats {
                    total_files: cache_count as usize,
                    total_size_bytes: current_size,
                    max_size_bytes: cache_manager.max_cache_size,
                    cache_hit_rate: 0.0,
                })
            }
            Err(e) => {
                warn!("キャッシュ統計取得エラー: {e}");
                None
            }
        }
    };

    // R2設定情報を取得
    let config =
        R2Config::from_env().map_err(|e| format!("R2設定の読み込みに失敗しました: {e}"))?;

    // 推定ストレージ使用量を計算（概算）
    let estimated_storage_mb = db_stats.0 * 2; // 1ファイル平均2MBと仮定

    let usage_info = R2UsageInfo {
        total_files: db_stats.0,
        estimated_storage_mb,
        monthly_uploads: db_stats.1,
        daily_uploads: db_stats.2,
        cache_stats,
        bucket_name: config.get_environment_bucket_name(),
        region: config.region.clone(),
        last_updated: Utc::now().with_timezone(&Tokyo).to_rfc3339(),
        cost_estimate_usd: calculate_estimated_cost(db_stats.0, estimated_storage_mb),
    };

    info!(
        "R2使用量監視情報取得完了: ファイル数={}, 推定容量={}MB",
        usage_info.total_files, usage_info.estimated_storage_mb
    );

    security_manager.log_security_event(
        "r2_usage_monitoring_success",
        &format!(
            "files={}, storage={}MB",
            usage_info.total_files, usage_info.estimated_storage_mb
        ),
    );

    Ok(usage_info)
}

/// 開発者向けデバッグ情報を取得する
///
/// # 引数
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// デバッグ情報、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_r2_debug_info(state: State<'_, AppState>) -> Result<R2DebugInfo, String> {
    info!("R2デバッグ情報取得開始");

    let security_manager = SecurityManager::new();
    security_manager.log_security_event("r2_debug_info_started", "R2デバッグ情報取得開始");

    // 環境変数情報を取得（認証情報はマスク）
    let mut env_vars = std::collections::HashMap::new();

    // 安全な環境変数のみ表示
    let safe_env_vars = ["ENVIRONMENT", "R2_REGION", "R2_BUCKET_NAME"];

    for var_name in &safe_env_vars {
        if let Ok(value) = std::env::var(var_name) {
            env_vars.insert(var_name.to_string(), value);
        }
    }

    // 認証情報はマスクして表示
    if let Ok(account_id) = std::env::var("R2_ACCOUNT_ID") {
        env_vars.insert("R2_ACCOUNT_ID".to_string(), mask_credential(&account_id));
    }

    if let Ok(access_key) = std::env::var("R2_ACCESS_KEY") {
        env_vars.insert("R2_ACCESS_KEY".to_string(), mask_credential(&access_key));
    }

    env_vars.insert("R2_SECRET_KEY".to_string(), "****".to_string()); // 完全にマスク

    // R2設定情報を取得
    let config_info = match R2Config::from_env() {
        Ok(config) => Some(config.get_debug_info()),
        Err(e) => {
            warn!("R2設定取得エラー: {e}");
            None
        }
    };

    // システム情報を取得
    let system_info = get_system_info();

    // データベース統計を取得
    let db_stats = {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {e}"))?;

        get_database_stats(&db).map_err(|e| format!("データベース統計取得エラー: {e}"))?
    };

    // 最近のエラーログを取得（セキュリティログから）
    let recent_errors = security_manager.get_recent_security_events(10);

    let debug_info = R2DebugInfo {
        environment_variables: env_vars,
        r2_config: config_info,
        system_info,
        database_stats: db_stats,
        recent_errors,
        timestamp: Utc::now().with_timezone(&Tokyo).to_rfc3339(),
    };

    info!("R2デバッグ情報取得完了");

    security_manager.log_security_event("r2_debug_info_success", "R2デバッグ情報取得完了");

    Ok(debug_info)
}

// ========== 内部ヘルパー関数 ==========

/// 設定検証テスト
async fn test_config_validation() -> TestStepResult {
    let start_time = std::time::Instant::now();

    match R2Config::from_env() {
        Ok(config) => match config.validate() {
            Ok(_) => TestStepResult {
                success: true,
                message: "設定検証成功".to_string(),
                duration_ms: start_time.elapsed().as_millis() as u64,
                details: Some(format!(
                    "バケット: {}, リージョン: {}",
                    config.get_environment_bucket_name(),
                    config.region
                )),
            },
            Err(e) => TestStepResult {
                success: false,
                message: format!("設定検証エラー: {e}"),
                duration_ms: start_time.elapsed().as_millis() as u64,
                details: None,
            },
        },
        Err(e) => TestStepResult {
            success: false,
            message: format!("設定読み込みエラー: {e}"),
            duration_ms: start_time.elapsed().as_millis() as u64,
            details: None,
        },
    }
}

/// クライアント初期化テスト
async fn test_client_initialization() -> (TestStepResult, Option<R2Client>) {
    let start_time = std::time::Instant::now();

    match R2Config::from_env() {
        Ok(config) => match R2Client::new(config).await {
            Ok(client) => (
                TestStepResult {
                    success: true,
                    message: "クライアント初期化成功".to_string(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    details: Some("R2クライアントが正常に初期化されました".to_string()),
                },
                Some(client),
            ),
            Err(e) => (
                TestStepResult {
                    success: false,
                    message: format!("クライアント初期化エラー: {e}"),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    details: None,
                },
                None,
            ),
        },
        Err(e) => (
            TestStepResult {
                success: false,
                message: format!("設定読み込みエラー: {e}"),
                duration_ms: start_time.elapsed().as_millis() as u64,
                details: None,
            },
            None,
        ),
    }
}

/// バケットアクセステスト
async fn test_bucket_access(client: &R2Client) -> TestStepResult {
    let start_time = std::time::Instant::now();

    match client.test_connection().await {
        Ok(_) => TestStepResult {
            success: true,
            message: "バケットアクセス成功".to_string(),
            duration_ms: start_time.elapsed().as_millis() as u64,
            details: Some("バケットへの接続が確認されました".to_string()),
        },
        Err(e) => TestStepResult {
            success: false,
            message: format!("バケットアクセスエラー: {e}"),
            duration_ms: start_time.elapsed().as_millis() as u64,
            details: None,
        },
    }
}

/// アップロード機能テスト
async fn test_upload_functionality(client: &R2Client) -> (TestStepResult, Option<String>) {
    let start_time = std::time::Instant::now();

    // テスト用の小さなファイルを作成
    let test_data = b"R2 integration test file";
    let test_key = format!("test/integration_test_{}.txt", uuid::Uuid::new_v4());

    match client
        .upload_file(&test_key, test_data.to_vec(), "text/plain")
        .await
    {
        Ok(url) => (
            TestStepResult {
                success: true,
                message: "アップロードテスト成功".to_string(),
                duration_ms: start_time.elapsed().as_millis() as u64,
                details: Some(format!("テストファイルがアップロードされました: {url}")),
            },
            Some(test_key),
        ),
        Err(e) => (
            TestStepResult {
                success: false,
                message: format!("アップロードテストエラー: {e}"),
                duration_ms: start_time.elapsed().as_millis() as u64,
                details: None,
            },
            None,
        ),
    }
}

/// ダウンロード機能テスト
async fn test_download_functionality(client: &R2Client, test_key: &str) -> TestStepResult {
    let start_time = std::time::Instant::now();

    match client
        .generate_presigned_url(test_key, Duration::from_secs(300))
        .await
    {
        Ok(presigned_url) => {
            // Presigned URLでダウンロードテスト
            match reqwest::get(&presigned_url).await {
                Ok(response) => {
                    if response.status().is_success() {
                        TestStepResult {
                            success: true,
                            message: "ダウンロードテスト成功".to_string(),
                            duration_ms: start_time.elapsed().as_millis() as u64,
                            details: Some(
                                "Presigned URLでのダウンロードが成功しました".to_string(),
                            ),
                        }
                    } else {
                        TestStepResult {
                            success: false,
                            message: format!("ダウンロードHTTPエラー: {}", response.status()),
                            duration_ms: start_time.elapsed().as_millis() as u64,
                            details: None,
                        }
                    }
                }
                Err(e) => TestStepResult {
                    success: false,
                    message: format!("ダウンロードリクエストエラー: {e}"),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    details: None,
                },
            }
        }
        Err(e) => TestStepResult {
            success: false,
            message: format!("Presigned URL生成エラー: {e}"),
            duration_ms: start_time.elapsed().as_millis() as u64,
            details: None,
        },
    }
}

/// 削除機能テスト
async fn test_delete_functionality(client: &R2Client, test_key: &str) -> TestStepResult {
    let start_time = std::time::Instant::now();

    match client.delete_file(test_key).await {
        Ok(_) => TestStepResult {
            success: true,
            message: "削除テスト成功".to_string(),
            duration_ms: start_time.elapsed().as_millis() as u64,
            details: Some("テストファイルが正常に削除されました".to_string()),
        },
        Err(e) => TestStepResult {
            success: false,
            message: format!("削除テストエラー: {e}"),
            duration_ms: start_time.elapsed().as_millis() as u64,
            details: None,
        },
    }
}

/// 認証情報をマスクする
fn mask_credential(credential: &str) -> String {
    if credential.len() > 8 {
        format!(
            "{}****{}",
            &credential[..4],
            &credential[credential.len() - 4..]
        )
    } else {
        "****".to_string()
    }
}

/// システム情報を取得する
fn get_system_info() -> std::collections::HashMap<String, String> {
    let mut info = std::collections::HashMap::new();

    info.insert("os".to_string(), std::env::consts::OS.to_string());
    info.insert("arch".to_string(), std::env::consts::ARCH.to_string());
    info.insert("family".to_string(), std::env::consts::FAMILY.to_string());

    if let Ok(hostname) = std::env::var("HOSTNAME") {
        info.insert("hostname".to_string(), hostname);
    }

    info.insert(
        "rust_version".to_string(),
        std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string()),
    );
    info.insert(
        "cargo_version".to_string(),
        std::env::var("CARGO_VERSION").unwrap_or_else(|_| "unknown".to_string()),
    );

    info
}

/// データベース統計を取得する
fn get_database_stats(
    db: &rusqlite::Connection,
) -> Result<std::collections::HashMap<String, String>, rusqlite::Error> {
    let mut stats = std::collections::HashMap::new();

    // テーブル数を取得
    let table_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
        [],
        |row| row.get(0),
    )?;
    stats.insert("table_count".to_string(), table_count.to_string());

    // 経費テーブルの行数
    let expense_count: i64 = db.query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))?;
    stats.insert("expense_count".to_string(), expense_count.to_string());

    // 領収書付き経費の数
    let receipt_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM expenses WHERE receipt_url IS NOT NULL AND receipt_url != ''",
        [],
        |row| row.get(0),
    )?;
    stats.insert("receipt_count".to_string(), receipt_count.to_string());

    // キャッシュテーブルの行数（存在する場合）
    let cache_count: i64 = db
        .query_row("SELECT COUNT(*) FROM receipt_cache", [], |row| row.get(0))
        .unwrap_or(0);
    stats.insert("cache_count".to_string(), cache_count.to_string());

    Ok(stats)
}

/// 推定コストを計算する（概算）
fn calculate_estimated_cost(file_count: u64, storage_mb: u64) -> f64 {
    // Cloudflare R2の料金体系に基づく概算
    // ストレージ: $0.015/GB/月
    // Class A操作（PUT/POST）: $4.50/百万リクエスト
    // Class B操作（GET/HEAD）: $0.36/百万リクエスト

    let storage_gb = storage_mb as f64 / 1024.0;
    let storage_cost = storage_gb * 0.015; // 月額ストレージコスト

    let put_operations = file_count as f64;
    let put_cost = (put_operations / 1_000_000.0) * 4.50;

    // 月間100回のGET操作を仮定
    let get_operations = file_count as f64 * 100.0;
    let get_cost = (get_operations / 1_000_000.0) * 0.36;

    storage_cost + put_cost + get_cost
}

// ========== データ構造体 ==========

/// R2接続テスト結果
#[derive(serde::Serialize)]
pub struct R2ConnectionTestResult {
    pub overall_success: bool,
    pub config_validation: TestStepResult,
    pub client_initialization: TestStepResult,
    pub bucket_access: TestStepResult,
    pub upload_test: TestStepResult,
    pub download_test: TestStepResult,
    pub delete_test: TestStepResult,
    pub performance_metrics: Option<PerformanceStats>,
    pub total_duration_ms: u64,
    pub environment: String,
}

/// テストステップ結果
#[derive(serde::Serialize, Clone)]
pub struct TestStepResult {
    pub success: bool,
    pub message: String,
    pub duration_ms: u64,
    pub details: Option<String>,
}

impl Default for TestStepResult {
    fn default() -> Self {
        Self {
            success: false,
            message: "未実行".to_string(),
            duration_ms: 0,
            details: None,
        }
    }
}

/// R2使用量情報
#[derive(serde::Serialize)]
pub struct R2UsageInfo {
    pub total_files: u64,
    pub estimated_storage_mb: u64,
    pub monthly_uploads: u64,
    pub daily_uploads: u64,
    pub cache_stats: Option<CacheStats>,
    pub bucket_name: String,
    pub region: String,
    pub last_updated: String,
    pub cost_estimate_usd: f64,
}

/// R2デバッグ情報
#[derive(serde::Serialize)]
pub struct R2DebugInfo {
    pub environment_variables: std::collections::HashMap<String, String>,
    pub r2_config: Option<std::collections::HashMap<String, String>>,
    pub system_info: std::collections::HashMap<String, String>,
    pub database_stats: std::collections::HashMap<String, String>,
    pub recent_errors: Vec<String>,
    pub timestamp: String,
}

// 領収書機能のTauriコマンドハンドラー

use super::{
    cache::CacheManager,
    models::{
        CacheStats, MultipleFileUploadInput, MultipleUploadResult, PerformanceStats,
        R2ConnectionTestResult, R2DebugInfo, R2UsageInfo, SingleUploadResult, TestStepResult,
    },
    service::R2Client,
};
use crate::features::expenses::repository as expense_operations;
use crate::services::{config::R2Config, security::SecurityManager};
use crate::shared::errors::AppError;
use crate::AppState;
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use log::{debug, error, info, warn};
use std::fs;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};
use tokio::sync::mpsc;

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
            let user_message = app_error.user_message();
            error!("領収書アップロード失敗: expense_id={expense_id}, error={app_error}");
            security_manager.log_security_event(
                "receipt_upload_failed",
                &format!("expense_id={expense_id}, error={app_error}"),
            );
            Err(user_message.to_string())
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
        return Err(AppError::NotFound(format!(
            "指定されたファイルが存在しません: {file_path}"
        )));
    }

    // ファイル名を取得
    let filename = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| AppError::Validation(format!("ファイル名の取得に失敗しました: {file_path}")))?;

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
    let config = R2Config::from_env()
        .map_err(|e| AppError::Configuration(format!("R2設定読み込み失敗: {e}")))?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await?;

    // ファイルキーを生成
    let file_key = R2Client::generate_file_key(expense_id, filename);
    debug!("ファイルキーを生成しました: {file_key}");

    // Content-Typeを取得
    let content_type = R2Client::get_content_type(filename);
    debug!("Content-Type: {content_type}");

    // 現在のreceipt_urlを保存（ロールバック用）
    let original_receipt_url = {
        let db = state
            .db
            .lock()
            .map_err(|e| AppError::Database(format!("データベースロック取得エラー: {e}")))?;

        expense_operations::get_receipt_url(&db, expense_id)?
    };

    // リトライ機能付きでR2にアップロード（最大3回リトライ）
    let receipt_url = client
        .upload_file_with_retry(&file_key, file_data, &content_type, 3)
        .await?;

    info!("R2アップロードが成功しました: {receipt_url}");

    // データベースにreceipt_urlを保存（失敗時は状態を保持）
    let db_result = {
        let db = state
            .db
            .lock()
            .map_err(|e| AppError::Database(format!("データベースロック取得エラー: {e}")))?;

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
                let db = state
                    .db
                    .lock()
                    .map_err(|e| AppError::Database(format!("ロールバック時のデータベースロック取得エラー: {e}")))?;

                if let Err(restore_error) =
                    expense_operations::set_receipt_url(&db, expense_id, original_url)
                {
                    error!("元のreceipt_urlの復元に失敗しました: {restore_error}");
                }
            }

            Err(db_error)
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
            let user_message = app_error.user_message();
            error!("領収書取得失敗: receipt_url={receipt_url}, error={app_error}");
            security_manager.log_security_event(
                "receipt_download_failed",
                &format!("receipt_url={receipt_url}, error={app_error}"),
            );
            Err(user_message.to_string())
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
        return Err(AppError::Validation(
            "領収書URLの形式が正しくありません（HTTPS URLである必要があります）".to_string(),
        ));
    }

    // キャッシュマネージャーを初期化
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Configuration(format!("アプリデータディレクトリの取得に失敗しました: {e}")))?;

    let cache_dir = app_data_dir.join("receipt_cache");
    let cache_manager = CacheManager::new(cache_dir, 100); // 100MB制限

    // まずキャッシュから取得を試行
    let cached_result = {
        let db = state
            .db
            .lock()
            .map_err(|e| AppError::Database(format!("キャッシュ確認時のデータベースロック取得エラー: {e}")))?;

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
        let db = state
            .db
            .lock()
            .map_err(|e| AppError::Database(format!("キャッシュ保存時のデータベースロック取得エラー: {e}")))?;

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
        return Err(AppError::Validation(format!(
            "無効なreceipt_URL形式: {receipt_url}"
        )));
    }

    // R2設定を読み込み
    let config = R2Config::from_env()
        .map_err(|e| AppError::Configuration(format!("R2設定読み込み失敗: {e}")))?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await?;

    // ファイルキーを抽出（receipts/expense_id/filename形式を想定）
    let file_key = if url_parts.len() >= 6 {
        // https://account_id.r2.cloudflarestorage.com/bucket_name/receipts/expense_id/filename
        url_parts[url_parts.len() - 3..].join("/")
    } else {
        return Err(AppError::Validation(format!(
            "URLからファイルキーを抽出できません: {receipt_url}"
        )));
    };

    // Presigned URLを生成（1時間有効）
    let presigned_url = client
        .generate_presigned_url(&file_key, Duration::from_secs(3600))
        .await?;

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
                                return Err(AppError::ExternalService(format!(
                                    "ファイルのダウンロード中にエラーが発生しました: {e}"
                                )));
                            }
                        }
                    }
                } else if response.status().as_u16() == 404 {
                    return Err(AppError::NotFound(format!(
                        "指定された領収書ファイルが見つかりません: {file_key}"
                    )));
                } else {
                    return Err(AppError::ExternalService(format!(
                        "ファイルのダウンロード中にエラーが発生しました: HTTP {}",
                        response.status()
                    )));
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
                    return Err(AppError::ExternalService(format!(
                        "ネットワークエラーが発生しました: {e}"
                    )));
                }
            }
        }
    }
}

/// R2から領収書を削除する（統一エラーハンドリング版）
///
/// # 引数
/// * `receipt_url` - 領収書のHTTPS URL
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 成功時はtrue、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_receipt_from_r2(
    receipt_url: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    info!("R2からの領収書削除を開始します: receipt_url={receipt_url}");

    let security_manager = SecurityManager::new();
    security_manager.log_security_event(
        "receipt_delete_started",
        &format!("receipt_url={receipt_url}"),
    );

    // 統一エラーハンドリングを使用して削除処理を実行
    let result = delete_receipt_internal(receipt_url.clone(), app, state).await;

    match result {
        Ok(success) => {
            info!("領収書削除成功: receipt_url={receipt_url}");
            security_manager.log_security_event(
                "receipt_delete_success",
                &format!("receipt_url={receipt_url}"),
            );
            Ok(success)
        }
        Err(app_error) => {
            let user_message = app_error.user_message();
            error!("領収書削除失敗: receipt_url={receipt_url}, error={app_error}");
            security_manager.log_security_event(
                "receipt_delete_failed",
                &format!("receipt_url={receipt_url}, error={app_error}"),
            );
            Err(user_message.to_string())
        }
    }
}

/// 内部的な削除処理（統一エラーハンドリング使用）
async fn delete_receipt_internal(
    receipt_url: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    // URLからファイルキーを抽出
    let url_parts: Vec<&str> = receipt_url.split('/').collect();
    if url_parts.len() < 4 {
        return Err(AppError::Validation(format!(
            "無効なreceipt_URL形式: {receipt_url}"
        )));
    }

    // ファイルキーを抽出
    let file_key = if url_parts.len() >= 6 {
        url_parts[url_parts.len() - 3..].join("/")
    } else {
        return Err(AppError::Validation(format!(
            "URLからファイルキーを抽出できません: {receipt_url}"
        )));
    };

    // R2設定を読み込み
    let config = R2Config::from_env()
        .map_err(|e| AppError::Configuration(format!("R2設定読み込み失敗: {e}")))?;

    // R2クライアントを初期化
    let client = R2Client::new(config).await?;

    // キャッシュマネージャーを初期化
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Configuration(format!("アプリデータディレクトリの取得に失敗しました: {e}")))?;

    let cache_dir = app_data_dir.join("receipt_cache");
    let cache_manager = CacheManager::new(cache_dir, 100);

    // トランザクション的な削除処理：R2→キャッシュ→DB順
    // 1. R2からファイルを削除
    client.delete_file(&file_key).await?;

    info!("R2からのファイル削除が成功しました: {receipt_url}");

    // 2. キャッシュからも削除（エラーは無視）
    {
        let db = state
            .db
            .lock()
            .map_err(|e| AppError::Database(format!("キャッシュ削除時のデータベースロック取得エラー: {e}")))?;

        if let Err(e) = cache_manager.delete_cache_file(&receipt_url, &db) {
            warn!("キャッシュ削除エラー（無視して続行）: {e}");
        }
    }

    // 削除操作のログ記録
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();
    info!(
        "領収書削除完了: receipt_url={receipt_url}, timestamp={now}"
    );

    let security_manager = SecurityManager::new();
    security_manager.log_security_event(
        "receipt_delete_completed",
        &format!("receipt_url={receipt_url}, timestamp={now}"),
    );

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
    let cache_manager = CacheManager::new(cache_dir, 100);

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
            use base64::{engine::general_purpose, Engine as _};
            let base64_data = general_purpose::STANDARD.encode(&cached_data);
            Ok(base64_data)
        }
        Ok(None) => {
            Err("オフライン時：領収書がキャッシュに見つかりません。オンライン時に一度表示してください。".to_string())
        }
        Err(e) => Err(format!("キャッシュ取得エラー: {e}")),
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
    let cache_manager = CacheManager::new(cache_dir, 100);

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
    let cache_manager = CacheManager::new(cache_dir, 100);

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

        upload_files.push(super::models::MultipleFileUpload {
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
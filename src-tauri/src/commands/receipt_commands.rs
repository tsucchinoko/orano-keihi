use crate::db::{expense_operations, subscription_operations};
use crate::services::{config::R2Config, r2_client::R2Client};
use crate::AppState;
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

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

    expense_operations::set_receipt_path(&db, expense_id, receipt_path_str.clone())
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
    let current_receipt_path = expense_operations::get_receipt_path(&db, expense_id)
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
    expense_operations::set_receipt_path(&db, expense_id, "".to_string())
        .map_err(|e| format!("データベースからの領収書パス削除に失敗しました: {e}"))?;

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
/// 領収書ファイルをR2にアップロードする
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
    // ファイルパスの検証
    let source_path = Path::new(&file_path);
    if !source_path.exists() {
        return Err("指定されたファイルが存在しません".to_string());
    }

    // ファイル名を取得
    let filename = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "ファイル名の取得に失敗しました".to_string())?;

    // ファイル形式の事前検証
    R2Client::validate_file_format(filename).map_err(|e| format!("ファイル形式エラー: {}", e))?;

    // ファイルサイズの事前検証
    let metadata = fs::metadata(source_path)
        .map_err(|e| format!("ファイル情報の取得に失敗しました: {}", e))?;

    R2Client::validate_file_size(metadata.len())
        .map_err(|e| format!("ファイルサイズエラー: {}", e))?;

    // ファイルを読み込み
    let file_data =
        fs::read(source_path).map_err(|e| format!("ファイルの読み込みに失敗しました: {}", e))?;

    // R2設定を読み込み
    let config =
        R2Config::from_env().map_err(|e| format!("R2設定の読み込みに失敗しました: {}", e))?;

    // R2クライアントを初期化
    let client = R2Client::new(config)
        .await
        .map_err(|e| format!("R2クライアントの初期化に失敗しました: {}", e))?;

    // ファイルキーを生成
    let file_key = R2Client::generate_file_key(expense_id, filename);

    // Content-Typeを取得
    let content_type = R2Client::get_content_type(filename);

    // リトライ機能付きでR2にアップロード（最大3回リトライ）
    let receipt_url = client
        .upload_file_with_retry(&file_key, file_data, &content_type, 3)
        .await
        .map_err(|e| format!("R2アップロードに失敗しました: {}", e))?;

    // データベースにreceipt_urlを保存
    let db = state
        .db
        .lock()
        .map_err(|e| format!("データベースロックエラー: {}", e))?;

    expense_operations::set_receipt_url(&db, expense_id, receipt_url.clone())
        .map_err(|e| format!("データベースへの保存に失敗しました: {}", e))?;

    Ok(receipt_url)
}

/// R2から領収書を取得する（キャッシュ対応）
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
    // URLの検証
    if !receipt_url.starts_with("https://") {
        return Err("無効なreceipt_URLです（HTTPS URLである必要があります）".to_string());
    }

    // キャッシュマネージャーを初期化
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("アプリデータディレクトリの取得に失敗しました: {}", e))?;

    let cache_dir = app_data_dir.join("receipt_cache");
    let cache_manager = crate::services::cache_manager::CacheManager::new(cache_dir, 100); // 100MB制限

    // まずキャッシュから取得を試行
    let cached_result = {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {}", e))?;
        cache_manager.get_cached_file(&receipt_url, &db)
    };

    match cached_result {
        Ok(Some(cached_data)) => {
            // キャッシュヒット - Base64エンコードして返却
            use base64::{engine::general_purpose, Engine as _};
            let base64_data = general_purpose::STANDARD.encode(&cached_data);
            return Ok(base64_data);
        }
        Ok(None) => {
            // キャッシュミス - R2から取得
        }
        Err(e) => {
            // キャッシュエラーはログに記録するが、R2からの取得を続行
            eprintln!("キャッシュ取得エラー: {}", e);
        }
    }

    // R2から取得
    let file_data = match download_from_r2(&receipt_url).await {
        Ok(data) => data,
        Err(e) => {
            // R2からの取得に失敗した場合のフォールバック
            return Err(format!(
                "領収書の取得に失敗しました: {}。ネットワーク接続を確認してください。",
                e
            ));
        }
    };

    // 取得したファイルをキャッシュに保存（エラーは無視）
    {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {}", e))?;

        if let Err(e) = cache_manager.cache_file(&receipt_url, file_data.clone(), &db) {
            eprintln!("キャッシュ保存エラー: {}", e);
        }

        // キャッシュサイズ管理（エラーは無視）
        if let Err(e) = cache_manager.manage_cache_size(&db) {
            eprintln!("キャッシュサイズ管理エラー: {}", e);
        }
    }

    // Base64エンコードして返却
    use base64::{engine::general_purpose, Engine as _};
    let base64_data = general_purpose::STANDARD.encode(&file_data);
    Ok(base64_data)
}

/// R2からファイルをダウンロードする内部関数
///
/// # 引数
/// * `receipt_url` - 領収書のHTTPS URL
///
/// # 戻り値
/// ファイルデータ、または失敗時はエラーメッセージ
async fn download_from_r2(receipt_url: &str) -> Result<Vec<u8>, String> {
    // URLからファイルキーを抽出
    let url_parts: Vec<&str> = receipt_url.split('/').collect();
    if url_parts.len() < 4 {
        return Err("無効なreceipt_URLです".to_string());
    }

    // R2設定を読み込み
    let config =
        R2Config::from_env().map_err(|e| format!("R2設定の読み込みに失敗しました: {}", e))?;

    // R2クライアントを初期化
    let client = R2Client::new(config)
        .await
        .map_err(|e| format!("R2クライアントの初期化に失敗しました: {}", e))?;

    // ファイルキーを抽出（receipts/expense_id/filename形式を想定）
    let file_key = if url_parts.len() >= 6 {
        // https://account_id.r2.cloudflarestorage.com/bucket_name/receipts/expense_id/filename
        url_parts[url_parts.len() - 3..].join("/")
    } else {
        return Err("URLからファイルキーを抽出できません".to_string());
    };

    // Presigned URLを生成（1時間有効）
    let presigned_url = client
        .generate_presigned_url(&file_key, Duration::from_secs(3600))
        .await
        .map_err(|e| format!("Presigned URL生成に失敗しました: {}", e))?;

    // リトライ機能付きでHTTPクライアントでファイルをダウンロード
    let mut attempts = 0;
    const MAX_RETRIES: u32 = 3;

    loop {
        match reqwest::get(&presigned_url).await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes().await {
                        Ok(file_data) => return Ok(file_data.to_vec()),
                        Err(e) => {
                            if attempts < MAX_RETRIES {
                                attempts += 1;
                                let delay = Duration::from_secs(2_u64.pow(attempts));
                                tokio::time::sleep(delay).await;
                                continue;
                            } else {
                                return Err(format!("ファイルデータの取得に失敗しました: {}", e));
                            }
                        }
                    }
                } else if response.status().as_u16() == 404 {
                    return Err("領収書ファイルが見つかりません".to_string());
                } else {
                    return Err(format!("ファイルダウンロードエラー: {}", response.status()));
                }
            }
            Err(e) => {
                if attempts < MAX_RETRIES {
                    attempts += 1;
                    let delay = Duration::from_secs(2_u64.pow(attempts));
                    tokio::time::sleep(delay).await;
                    continue;
                } else {
                    return Err(format!("ファイルダウンロードに失敗しました: {}", e));
                }
            }
        }
    }
}

/// R2から領収書を削除する（キャッシュ対応）
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
    // 現在のreceipt_urlを取得
    let current_receipt_url = {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {}", e))?;

        expense_operations::get_receipt_url(&db, expense_id)
            .map_err(|e| format!("receipt_urlの取得に失敗しました: {}", e))?
    };

    if let Some(receipt_url) = current_receipt_url {
        // キャッシュマネージャーを初期化
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("アプリデータディレクトリの取得に失敗しました: {}", e))?;

        let cache_dir = app_data_dir.join("receipt_cache");
        let cache_manager = crate::services::cache_manager::CacheManager::new(cache_dir, 100);

        // R2からファイルを削除（トランザクション的な削除処理：R2→DB順）
        let deletion_result = delete_from_r2_with_retry(&receipt_url).await;

        match deletion_result {
            Ok(_) => {
                // R2削除成功 - キャッシュからも削除
                {
                    let db = state
                        .db
                        .lock()
                        .map_err(|e| format!("データベースロックエラー: {}", e))?;

                    if let Err(e) = cache_manager.delete_cache_file(&receipt_url, &db) {
                        eprintln!("キャッシュ削除エラー: {}", e);
                    }
                } // dbのスコープを終了

                // データベースからreceipt_urlを削除
                {
                    let db = state
                        .db
                        .lock()
                        .map_err(|e| format!("データベースロックエラー: {}", e))?;

                    expense_operations::set_receipt_url(&db, expense_id, "".to_string()).map_err(
                        |e| format!("データベースからのreceipt_url削除に失敗しました: {}", e),
                    )?;
                }

                // 削除操作のログ記録
                let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();
                println!(
                    "領収書削除完了: expense_id={}, receipt_url={}, timestamp={}",
                    expense_id, receipt_url, now
                );
            }
            Err(e) => {
                // R2削除失敗 - データベースの状態は変更しない
                return Err(format!(
                    "R2からのファイル削除に失敗しました。データベースの状態は保持されます: {}",
                    e
                ));
            }
        }
    } else {
        // receipt_urlが存在しない場合は何もしない
        println!(
            "削除対象の領収書URLが存在しません: expense_id={}",
            expense_id
        );
    }

    Ok(true)
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
        R2Config::from_env().map_err(|e| format!("R2設定の読み込みに失敗しました: {}", e))?;

    // R2クライアントを初期化
    let client = R2Client::new(config)
        .await
        .map_err(|e| format!("R2クライアントの初期化に失敗しました: {}", e))?;

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
                    return Err(format!("R2削除エラー（最大リトライ回数に到達）: {}", e));
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
/// 接続成功時はtrue、失敗時はエラーメッセージ
#[tauri::command]
pub async fn test_r2_connection(_state: State<'_, AppState>) -> Result<bool, String> {
    // 環境変数からR2設定を読み込み
    let config =
        R2Config::from_env().map_err(|e| format!("R2設定の読み込みに失敗しました: {}", e))?;

    // R2クライアントを初期化
    let client = R2Client::new(config)
        .await
        .map_err(|e| format!("R2クライアントの初期化に失敗しました: {}", e))?;

    // 接続テストを実行
    client
        .test_connection()
        .await
        .map_err(|e| format!("R2接続テストに失敗しました: {}", e))?;

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
        .map_err(|e| format!("アプリデータディレクトリの取得に失敗しました: {}", e))?;

    let cache_dir = app_data_dir.join("receipt_cache");
    let cache_manager = crate::services::cache_manager::CacheManager::new(cache_dir, 100);

    // オフライン時のキャッシュから取得
    let cached_result = {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {}", e))?;
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
            Err(format!("キャッシュ取得エラー: {}", e))
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
        .map_err(|e| format!("アプリデータディレクトリの取得に失敗しました: {}", e))?;

    let cache_dir = app_data_dir.join("receipt_cache");
    let cache_manager = crate::services::cache_manager::CacheManager::new(cache_dir, 100);

    // キャッシュ同期を実行（同期版を使用）
    let sync_result: Result<usize, String> = {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {}", e))?;

        // 古いキャッシュをクリーンアップ
        let cleaned_count = cache_manager
            .cleanup_old_cache(&db)
            .map_err(|e| format!("キャッシュクリーンアップエラー: {}", e))?;

        // キャッシュサイズを管理
        cache_manager
            .manage_cache_size(&db)
            .map_err(|e| format!("キャッシュサイズ管理エラー: {}", e))?;

        println!(
            "キャッシュ同期完了: {}個のファイルをクリーンアップしました",
            cleaned_count
        );

        Ok(cleaned_count)
    };

    match sync_result {
        Ok(synced_count) => Ok(synced_count),
        Err(e) => Err(format!("キャッシュ同期エラー: {}", e)),
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
        .map_err(|e| format!("アプリデータディレクトリの取得に失敗しました: {}", e))?;

    let cache_dir = app_data_dir.join("receipt_cache");
    let cache_manager = crate::services::cache_manager::CacheManager::new(cache_dir, 100);

    // キャッシュサイズを計算（同期版を使用）
    let current_size = cache_manager
        .calculate_cache_size_sync()
        .map_err(|e| format!("キャッシュサイズ計算エラー: {}", e))?;

    // データベースからキャッシュ数を取得
    let cache_count = {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {}", e))?;

        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM receipt_cache", [], |row| row.get(0))
            .map_err(|e| format!("キャッシュ数取得エラー: {}", e))?;

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

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
    R2Client::validate_file_format(filename)
        .map_err(|e| format!("ファイル形式エラー: {}", e))?;

    // ファイルサイズの事前検証
    let metadata = fs::metadata(source_path)
        .map_err(|e| format!("ファイル情報の取得に失敗しました: {}", e))?;
    
    R2Client::validate_file_size(metadata.len())
        .map_err(|e| format!("ファイルサイズエラー: {}", e))?;

    // ファイルを読み込み
    let file_data = fs::read(source_path)
        .map_err(|e| format!("ファイルの読み込みに失敗しました: {}", e))?;

    // R2設定を読み込み
    let config = R2Config::from_env()
        .map_err(|e| format!("R2設定の読み込みに失敗しました: {}", e))?;

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

/// R2から領収書を取得する
///
/// # 引数
/// * `receipt_url` - 領収書のHTTPS URL
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// ファイルデータ（Base64エンコード）、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_receipt_from_r2(
    receipt_url: String,
    _state: State<'_, AppState>,
) -> Result<String, String> {
    // URLからファイルキーを抽出
    let url_parts: Vec<&str> = receipt_url.split('/').collect();
    if url_parts.len() < 2 {
        return Err("無効なreceipt_URLです".to_string());
    }

    // R2設定を読み込み
    let config = R2Config::from_env()
        .map_err(|e| format!("R2設定の読み込みに失敗しました: {}", e))?;

    // R2クライアントを初期化
    let client = R2Client::new(config)
        .await
        .map_err(|e| format!("R2クライアントの初期化に失敗しました: {}", e))?;

    // Presigned URLを生成（1時間有効）
    let file_key = url_parts[url_parts.len() - 2..].join("/");
    let presigned_url = client
        .generate_presigned_url(&file_key, Duration::from_secs(3600))
        .await
        .map_err(|e| format!("Presigned URL生成に失敗しました: {}", e))?;

    // HTTPクライアントでファイルをダウンロード
    let response = reqwest::get(&presigned_url)
        .await
        .map_err(|e| format!("ファイルダウンロードに失敗しました: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("ファイルダウンロードエラー: {}", response.status()));
    }

    let file_data = response
        .bytes()
        .await
        .map_err(|e| format!("ファイルデータの取得に失敗しました: {}", e))?;

    // Base64エンコードして返却
    use base64::{Engine as _, engine::general_purpose};
    let base64_data = general_purpose::STANDARD.encode(&file_data);
    Ok(base64_data)
}

/// R2から領収書を削除する
///
/// # 引数
/// * `expense_id` - 経費ID
/// * `state` - アプリケーション状態
///
/// # 戻り値
/// 成功時はtrue、失敗時はエラーメッセージ
#[tauri::command]
pub async fn delete_receipt_from_r2(
    expense_id: i64,
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
        // URLからファイルキーを抽出
        let url_parts: Vec<&str> = receipt_url.split('/').collect();
        if url_parts.len() >= 2 {
            let file_key = url_parts[url_parts.len() - 2..].join("/");

            // R2設定を読み込み
            let config = R2Config::from_env()
                .map_err(|e| format!("R2設定の読み込みに失敗しました: {}", e))?;

            // R2クライアントを初期化
            let client = R2Client::new(config)
                .await
                .map_err(|e| format!("R2クライアントの初期化に失敗しました: {}", e))?;

            // R2からファイルを削除
            client
                .delete_file(&file_key)
                .await
                .map_err(|e| format!("R2からのファイル削除に失敗しました: {}", e))?;
        }
    }

    // データベースからreceipt_urlを削除（空文字列に設定）
    {
        let db = state
            .db
            .lock()
            .map_err(|e| format!("データベースロックエラー: {}", e))?;
        
        expense_operations::set_receipt_url(&db, expense_id, "".to_string())
            .map_err(|e| format!("データベースからのreceipt_url削除に失敗しました: {}", e))?;
    }

    Ok(true)
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
    let config = R2Config::from_env()
        .map_err(|e| format!("R2設定の読み込みに失敗しました: {}", e))?;

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
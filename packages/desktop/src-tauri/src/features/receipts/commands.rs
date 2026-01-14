// 領収書機能のTauriコマンドハンドラー

use super::{cache::CacheManager, models::CacheStats};
use crate::AppState;
use tauri::{AppHandle, Manager, State};

/// オフライン時に領収書をキャッシュから取得する
///
/// # 引数
/// * `receipt_url` - 領収書のHTTPS URL
/// * `session_token` - セッショントークン
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// キャッシュされたファイルデータ（Base64エンコード）、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_receipt_offline(
    receipt_url: String,
    session_token: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
    auth_middleware: State<'_, crate::features::auth::middleware::AuthMiddleware>,
) -> Result<String, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/receipts/offline")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;

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
        cache_manager.get_offline_cached_file(&receipt_url, &db, &user.id)
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
/// * `session_token` - セッショントークン
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// 同期されたキャッシュ数、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn sync_cache_on_online(
    session_token: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
    auth_middleware: State<'_, crate::features::auth::middleware::AuthMiddleware>,
) -> Result<usize, String> {
    // 認証チェック
    let user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/receipts/sync")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;
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
            .cleanup_old_cache(&db, Some(&user.id))
            .map_err(|e| format!("キャッシュクリーンアップエラー: {e}"))?;

        // キャッシュサイズを管理
        cache_manager
            .manage_cache_size(&db, Some(&user.id))
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
/// * `session_token` - セッショントークン
/// * `app` - Tauriアプリハンドル
/// * `state` - アプリケーション状態
/// * `auth_middleware` - 認証ミドルウェア
///
/// # 戻り値
/// キャッシュ統計情報、または失敗時はエラーメッセージ
#[tauri::command]
pub async fn get_cache_stats(
    session_token: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
    auth_middleware: State<'_, crate::features::auth::middleware::AuthMiddleware>,
) -> Result<CacheStats, String> {
    // 認証チェック
    let _user = auth_middleware
        .authenticate_request(session_token.as_deref(), "/receipts/stats")
        .await
        .map_err(|e| format!("認証エラー: {e}"))?;
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

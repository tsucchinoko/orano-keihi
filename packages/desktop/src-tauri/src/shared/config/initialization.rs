use crate::shared::config::{get_environment, Environment};
use crate::shared::errors::{AppError, AppResult};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// アプリケーション初期化の結果を表す構造体
#[derive(Debug)]
pub struct InitializationResult {
    /// アプリケーションデータディレクトリのパス
    pub app_data_dir: PathBuf,
    /// 実行環境
    pub environment: Environment,
}

/// アプリケーションの初期化を実行する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// 初期化結果、または失敗時はエラー
///
/// # 処理内容
/// 1. アプリケーションデータディレクトリの作成
/// 2. 初回起動の判定
/// 3. データベースファイルの初期化
/// 4. 環境に応じた設定の適用
pub fn initialize_application(app_handle: &AppHandle) -> AppResult<InitializationResult> {
    // 現在の実行環境を取得
    let environment = get_environment();

    // アプリケーションデータディレクトリを取得・作成
    let app_data_dir = ensure_app_data_directory(app_handle)?;

    Ok(InitializationResult {
        app_data_dir,
        environment,
    })
}

/// アプリケーションデータディレクトリを確実に作成する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// アプリケーションデータディレクトリのパス、または失敗時はエラー
fn ensure_app_data_directory(app_handle: &AppHandle) -> AppResult<PathBuf> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| {
        AppError::configuration(format!("アプリデータディレクトリの取得に失敗: {e}"))
    })?;

    // ディレクトリが存在しない場合は作成
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir).map_err(|e| {
            AppError::configuration(format!("アプリデータディレクトリの作成に失敗: {e}"))
        })?;

        log::info!("アプリケーションデータディレクトリを作成しました: {app_data_dir:?}");
    }

    Ok(app_data_dir)
}

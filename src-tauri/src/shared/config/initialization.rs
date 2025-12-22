use crate::shared::config::{get_database_filename, get_environment, Environment};
use crate::shared::errors::{AppError, AppResult};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// アプリケーション初期化の結果を表す構造体
#[derive(Debug)]
pub struct InitializationResult {
    /// 初回起動かどうか
    pub is_first_run: bool,
    /// アプリケーションデータディレクトリのパス
    pub app_data_dir: PathBuf,
    /// データベースファイルのパス
    pub database_path: PathBuf,
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

    // データベースファイルパスを構築
    let db_filename = get_database_filename(environment.clone());
    let database_path = app_data_dir.join(db_filename);

    // 初回起動かどうかを判定（データベースファイルの存在で判定）
    let is_first_run = !database_path.exists();

    // 初回起動の場合、初期化ログを出力
    if is_first_run {
        log_first_run_initialization(&environment, &app_data_dir, &database_path);
    }

    Ok(InitializationResult {
        is_first_run,
        app_data_dir,
        database_path,
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

/// 初回起動時の初期化ログを出力する
///
/// # 引数
/// * `environment` - 実行環境
/// * `app_data_dir` - アプリケーションデータディレクトリ
/// * `database_path` - データベースファイルパス
fn log_first_run_initialization(
    environment: &Environment,
    app_data_dir: &PathBuf,
    database_path: &PathBuf,
) {
    log::info!("=== アプリケーション初回起動 ===");
    log::info!("実行環境: {environment:?}");
    log::info!("アプリデータディレクトリ: {app_data_dir:?}");
    log::info!("データベースファイル: {database_path:?}");
    log::info!("初期化を開始します...");
}

/// 初期化完了ログを出力する
///
/// # 引数
/// * `result` - 初期化結果
pub fn log_initialization_complete(result: &InitializationResult) {
    if result.is_first_run {
        log::info!("=== 初期化完了 ===");
        log::info!("初回起動の初期化が正常に完了しました");
    } else {
        log::info!("アプリケーション起動完了（既存データベースを使用）");
    }
    log::info!("環境: {:?}", result.environment);
    log::info!("データベース: {:?}", result.database_path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_first_run_initialization() {
        let environment = Environment::Development;
        let app_data_dir = PathBuf::from("/tmp/test_app");
        let database_path = PathBuf::from("/tmp/test_app/dev_expenses.db");

        // ログ出力関数が正常に実行されることを確認（パニックしない）
        log_first_run_initialization(&environment, &app_data_dir, &database_path);
    }

    #[test]
    fn test_initialization_result_creation() {
        let result = InitializationResult {
            is_first_run: true,
            app_data_dir: PathBuf::from("/tmp/test"),
            database_path: PathBuf::from("/tmp/test/expenses.db"),
            environment: Environment::Production,
        };

        assert!(result.is_first_run);
        assert_eq!(result.environment, Environment::Production);
    }

    #[test]
    fn test_log_initialization_complete() {
        let result = InitializationResult {
            is_first_run: true,
            app_data_dir: PathBuf::from("/tmp/test"),
            database_path: PathBuf::from("/tmp/test/expenses.db"),
            environment: Environment::Development,
        };

        // ログ出力関数が正常に実行されることを確認（パニックしない）
        log_initialization_complete(&result);
    }
}

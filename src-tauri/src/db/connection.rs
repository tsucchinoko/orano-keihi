use crate::config::{initialize_application, log_initialization_complete};
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use tauri::AppHandle;

/// アプリデータディレクトリ内のデータベースファイルパスを取得する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// データベースファイルのパス、または失敗時はエラーメッセージ
///
/// # 動作
/// 新しい初期化システムを使用してデータベースパスを取得する。
/// この関数は後方互換性のために残されているが、
/// 新しいコードでは initialize_database を直接使用することを推奨する。
pub fn get_db_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let init_result = initialize_application(app_handle)?;
    Ok(init_result.database_path)
}

/// データベース接続を初期化し、マイグレーションを実行する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// データベース接続、または失敗時はエラーメッセージ
///
/// # 処理内容
/// 1. アプリケーション全体の初期化を実行
/// 2. データベース接続を開く
/// 3. 初期化完了ログを出力
pub fn initialize_database(app_handle: &AppHandle) -> Result<Connection, String> {
    // アプリケーション全体の初期化を実行
    let init_result = initialize_application(app_handle)?;

    // データベース接続を開く
    let conn = Connection::open(&init_result.database_path)
        .map_err(|e| format!("データベースのオープンに失敗しました: {e}"))?;

    // 初期化完了ログを出力
    log_initialization_complete(&init_result);

    Ok(conn)
}

use crate::config::{get_database_filename, get_environment};
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// アプリデータディレクトリ内のデータベースファイルパスを取得する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// データベースファイルのパス、または失敗時はエラーメッセージ
///
/// # 動作
/// 実行環境（開発/プロダクション）に応じて適切なデータベースファイル名を選択し、
/// アプリデータディレクトリ内のパスを返す。ディレクトリが存在しない場合は自動作成する。
pub fn get_db_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("アプリデータディレクトリの取得に失敗しました: {e}"))?;

    // アプリデータディレクトリが存在しない場合は作成
    std::fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("アプリデータディレクトリの作成に失敗しました: {e}"))?;

    // 現在の実行環境を取得
    let env = get_environment();

    // 環境に応じたデータベースファイル名を取得
    let db_filename = get_database_filename(env);

    Ok(app_data_dir.join(db_filename))
}

/// データベース接続を初期化し、マイグレーションを実行する
///
/// # 引数
/// * `app_handle` - Tauriアプリケーションハンドル
///
/// # 戻り値
/// データベース接続、または失敗時はエラーメッセージ
pub fn initialize_database(app_handle: &AppHandle) -> Result<Connection, String> {
    let db_path = get_db_path(app_handle)?;

    let conn = Connection::open(&db_path)
        .map_err(|e| format!("データベースのオープンに失敗しました: {e}"))?;

    // マイグレーションを実行
    crate::db::migrations::run_migrations(&conn)
        .map_err(|e| format!("マイグレーションの実行に失敗しました: {e}"))?;

    Ok(conn)
}

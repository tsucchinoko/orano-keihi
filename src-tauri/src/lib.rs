mod db;
mod models;

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::Manager;

/// アプリケーション状態（データベース接続を保持）
pub struct AppState {
    pub db: Mutex<Connection>,
}

/// サンプルコマンド：挨拶メッセージを返す
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // アプリ起動時にデータベースを初期化
            let db_conn = db::initialize_database(app.handle())
                .expect("データベースの初期化に失敗しました");
            
            // データベース接続をアプリ状態に保存
            app.manage(AppState {
                db: Mutex::new(db_conn),
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("Tauriアプリケーションの実行中にエラーが発生しました");
}

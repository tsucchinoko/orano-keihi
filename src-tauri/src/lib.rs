mod commands;
mod db;
mod models;

use commands::{expense_commands, receipt_commands, subscription_commands};
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::Manager;

/// アプリケーション状態（データベース接続を保持）
pub struct AppState {
    pub db: Mutex<Connection>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
        .invoke_handler(tauri::generate_handler![
            // 経費コマンド
            expense_commands::create_expense,
            expense_commands::get_expenses,
            expense_commands::update_expense,
            expense_commands::delete_expense,
            // サブスクリプションコマンド
            subscription_commands::create_subscription,
            subscription_commands::get_subscriptions,
            subscription_commands::update_subscription,
            subscription_commands::toggle_subscription_status,
            subscription_commands::get_monthly_subscription_total,
            // 領収書コマンド
            receipt_commands::save_receipt,
        ])
        .run(tauri::generate_context!())
        .expect("Tauriアプリケーションの実行中にエラーが発生しました");
}

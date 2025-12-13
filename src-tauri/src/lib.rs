mod commands;
mod config;
pub mod db;
mod models;
mod services;

use commands::{expense_commands, migration_commands, receipt_commands, security_commands, subscription_commands};
use log::{error, info, warn};
use rusqlite::Connection;
use services::security::{EnvironmentConfig, SecurityManager};
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
            // ログシステムを初期化
            initialize_logging_system();

            info!("アプリケーション初期化を開始します...");

            // セキュリティマネージャーを初期化
            let security_manager = SecurityManager::new();
            
            // 環境変数を読み込み（.envファイルがある場合）
            if let Err(_) = dotenv::dotenv() {
                // .envファイルがない場合は無視（本番環境では環境変数が直接設定される）
                warn!(".envファイルが見つかりません。環境変数が直接設定されていることを確認してください。");
            } else {
                info!(".envファイルを読み込みました");
            }

            // セキュリティ設定の検証
            if let Err(e) = security_manager.validate_configuration() {
                error!("セキュリティ設定の検証に失敗しました: {}", e);
                // 本番環境では起動を停止する場合もある
                let env_config = security_manager.get_env_config();
                if env_config.is_production() {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("本番環境でのセキュリティ設定エラー: {}", e)
                    )));
                } else {
                    warn!("開発環境のため、セキュリティ設定エラーを無視して続行します");
                }
            }

            // 診断情報をログ出力
            let diagnostic_info = security_manager.get_diagnostic_info();
            info!("システム診断情報: {:?}", diagnostic_info);

            // アプリ起動時にデータベースを初期化
            info!("データベースを初期化しています...");
            let db_conn = db::initialize_database(app.handle())
                .map_err(|e| {
                    error!("データベースの初期化に失敗しました: {}", e);
                    security_manager.log_security_event("database_init_failed", &format!("{}", e));
                    e
                })?;

            info!("データベースの初期化が完了しました");
            security_manager.log_security_event("database_init_success", "データベース初期化完了");

            // データベース接続をアプリ状態に保存
            app.manage(AppState {
                db: Mutex::new(db_conn),
            });

            info!("アプリケーション初期化が完了しました");
            security_manager.log_security_event("app_init_success", "アプリケーション初期化完了");

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
            receipt_commands::save_subscription_receipt,
            receipt_commands::delete_receipt,
            receipt_commands::delete_subscription_receipt,
            receipt_commands::test_r2_connection,
            // R2領収書コマンド
            receipt_commands::upload_receipt_to_r2,
            receipt_commands::get_receipt_from_r2,
            receipt_commands::delete_receipt_from_r2,
            // キャッシュ関連コマンド
            receipt_commands::get_receipt_offline,
            receipt_commands::sync_cache_on_online,
            receipt_commands::get_cache_stats,
            // マイグレーションコマンド
            migration_commands::check_migration_status,
            migration_commands::execute_receipt_url_migration,
            migration_commands::restore_database_from_backup,
            migration_commands::list_backup_files,
            // セキュリティコマンド
            security_commands::get_system_diagnostic_info,
            security_commands::validate_security_configuration,
            security_commands::test_r2_connection_secure,
            security_commands::get_environment_info,
            security_commands::log_security_event,
            security_commands::get_r2_diagnostic_info,
        ])
        .run(tauri::generate_context!())
        .expect("Tauriアプリケーションの実行中にエラーが発生しました");
}

/// ログシステムを初期化
fn initialize_logging_system() {
    // 環境設定を取得
    let env_config = EnvironmentConfig::from_env();
    
    // ログレベルを設定
    let log_level = match env_config.log_level.to_lowercase().as_str() {
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    };

    // env_loggerを初期化
    env_logger::Builder::from_default_env()
        .filter_level(log_level)
        .format_timestamp_secs()
        .format_module_path(false)
        .format_target(false)
        .init();

    info!("ログシステムを初期化しました: level={}, environment={}", 
          env_config.log_level, env_config.environment);
}

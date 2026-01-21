// 新しい機能モジュール構造
pub mod features;
pub mod shared;

// 新しい機能モジュールからコマンドをインポート
use features::auth::middleware::AuthMiddleware;
use features::security::models::SecurityConfig;
use features::security::service::SecurityManager;
use features::{
    auth::commands as auth_commands,
    expenses::api_commands as expense_commands,
    receipts::{api_commands as receipt_api_commands, commands as receipt_commands},
    security::commands as security_commands,
    subscriptions::api_commands as subscription_commands,
    updater::commands as updater_commands,
};
use log::info;
use rusqlite::Connection;
use shared::config::environment::{initialize_logging_system, load_environment_variables};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

/// R2接続テストのキャッシュ
#[derive(Debug)]
pub struct R2ConnectionCache {
    pub last_test_time: Option<Instant>,
    pub last_test_result: Option<bool>,
    pub cache_duration: Duration,
}

impl Default for R2ConnectionCache {
    fn default() -> Self {
        Self::new()
    }
}

impl R2ConnectionCache {
    pub fn new() -> Self {
        Self {
            last_test_time: None,
            last_test_result: None,
            cache_duration: Duration::from_secs(300), // 5分間キャッシュ
        }
    }

    pub fn is_cache_valid(&self) -> bool {
        if let Some(last_time) = self.last_test_time {
            last_time.elapsed() < self.cache_duration
        } else {
            false
        }
    }

    pub fn update_cache(&mut self, result: bool) {
        self.last_test_time = Some(Instant::now());
        self.last_test_result = Some(result);
    }

    pub fn get_cached_result(&self) -> Option<bool> {
        if self.is_cache_valid() {
            self.last_test_result
        } else {
            None
        }
    }
}

/// アプリケーション状態（データベース接続とセキュリティマネージャーを保持）
pub struct AppState {
    pub db: Mutex<Connection>,
    pub security_manager: SecurityManager,
    pub r2_connection_cache: Arc<Mutex<R2ConnectionCache>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            // 詳細なデバッグログを追加
            eprintln!("=== アプリケーション初期化開始 ===");

            // メニューバーを作成
            use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};

            // アプリメニューを作成（macOS標準）
            let app_submenu = SubmenuBuilder::new(app, "オラの経費")
                .item(&PredefinedMenuItem::about(app, None, None)?)
                .separator()
                .item(&PredefinedMenuItem::hide(app, None)?)
                .item(&PredefinedMenuItem::hide_others(app, None)?)
                .item(&PredefinedMenuItem::show_all(app, None)?)
                .separator()
                .item(&PredefinedMenuItem::quit(app, None)?)
                .build()?;

            // 編集メニューを作成
            let edit_submenu = SubmenuBuilder::new(app, "編集")
                .item(&PredefinedMenuItem::undo(app, None)?)
                .item(&PredefinedMenuItem::redo(app, None)?)
                .separator()
                .item(&PredefinedMenuItem::cut(app, None)?)
                .item(&PredefinedMenuItem::copy(app, None)?)
                .item(&PredefinedMenuItem::paste(app, None)?)
                .item(&PredefinedMenuItem::select_all(app, None)?)
                .build()?;

            // ヘルプサブメニューを作成
            let help_submenu = SubmenuBuilder::new(app, "ヘルプ")
                .item(
                    &MenuItemBuilder::new("アップデートを確認")
                        .id("check_for_updates")
                        .build(app)?,
                )
                .build()?;

            // メインメニューを作成
            let menu = MenuBuilder::new(app)
                .item(&app_submenu)
                .item(&edit_submenu)
                .item(&help_submenu)
                .build()?;

            app.set_menu(menu)?;

            // メニューイベントをリッスン
            let app_handle = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                if event.id() == "check_for_updates" {
                    let app_handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        use log::info;
                        info!("メニューから「アップデートを確認」が選択されました");

                        // 強制チェックを実行
                        use crate::features::updater::service::UpdaterService;
                        let mut service = UpdaterService::new(app_handle.clone());

                        match service.check_for_updates_force().await {
                            Ok(update_info) => {
                                if update_info.available {
                                    info!(
                                        "アップデートが利用可能です: {:?}",
                                        update_info.latest_version
                                    );
                                    // フロントエンドに通知（ダイアログ表示用）
                                    if let Err(e) =
                                        app_handle.emit("show-update-dialog", &update_info)
                                    {
                                        log::error!("アップデート通知の送信に失敗: {e}");
                                    }
                                } else {
                                    info!("最新バージョンです");
                                    // フロントエンドに通知（ダイアログ表示用）
                                    if let Err(e) = app_handle.emit("show-no-update-dialog", ()) {
                                        log::error!("通知の送信に失敗: {e}");
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("アップデートチェックエラー: {e}");
                                // フロントエンドにエラーを通知（ダイアログ表示用）
                                if let Err(emit_error) =
                                    app_handle.emit("show-update-error-dialog", e.to_string())
                                {
                                    log::error!("エラー通知の送信に失敗: {emit_error}");
                                }
                            }
                        }
                    });
                }
            });

            // 環境に応じた.envファイルを読み込み（ログシステム初期化前に実行）
            eprintln!("環境変数を読み込み中...");
            load_environment_variables();
            eprintln!("環境変数の読み込み完了");

            // ログシステムを初期化（.envファイル読み込み後）
            eprintln!("ログシステムを初期化中...");
            initialize_logging_system();
            eprintln!("ログシステムの初期化完了");

            info!("アプリケーション初期化を開始します...");

            // セキュリティマネージャーを初期化（.envファイル読み込み後）
            eprintln!("セキュリティマネージャーを初期化中...");
            let security_config = SecurityConfig {
                encryption_key: "default_key_32_bytes_long_enough".to_string(),
                max_token_age_hours: 24,
                enable_audit_logging: true,
            };

            let security_manager = match SecurityManager::new(security_config.clone()) {
                Ok(manager) => {
                    eprintln!("セキュリティマネージャーの初期化完了");
                    manager
                }
                Err(e) => {
                    eprintln!("SecurityManager初期化失敗: {e}");
                    // プロダクション環境でクラッシュを防ぐため、デフォルト設定で再試行
                    eprintln!("デフォルト設定でSecurityManagerを再作成します...");
                    match SecurityManager::new(security_config.clone()) {
                        Ok(manager) => {
                            eprintln!("デフォルト設定でSecurityManager作成成功");
                            manager
                        }
                        Err(e2) => {
                            eprintln!("デフォルト設定でもSecurityManager作成失敗: {e2}");
                            return Err(format!("SecurityManager初期化失敗: {e2}").into());
                        }
                    }
                }
            };

            info!("システム診断情報を取得中...");

            // データベース接続を初期化
            eprintln!("データベース接続を初期化中...");
            let db_connection = match crate::shared::database::connection::initialize_database(app.handle()) {
                Ok(conn) => {
                    eprintln!("データベース接続の初期化完了");
                    Arc::new(Mutex::new(conn))
                }
                Err(e) => {
                    eprintln!("データベース接続初期化失敗: {e}");
                    return Err(format!("データベース接続初期化失敗: {e}").into());
                }
            };

            // APIサーバーURLを取得
            eprintln!("APIサーバー設定を読み込み中...");
            let api_server_url = crate::get_env_var!("API_SERVER_URL")
                .unwrap_or_else(|e| {
                    eprintln!("エラー: {e}");
                    panic!("API_SERVER_URLが設定されていません。.envファイルまたは環境変数を確認してください。");
                });

            eprintln!("API_SERVER_URL: {api_server_url}");

            // 認証サービスを初期化（APIサーバー経由）
            eprintln!("認証サービスを初期化中...");
            let auth_service = match features::auth::AuthService::new(
                api_server_url.clone(),
                Arc::clone(&db_connection),
                app.handle().clone(),
            ) {
                Ok(service) => {
                    eprintln!("認証サービスの初期化完了");
                    service
                }
                Err(e) => {
                    eprintln!("認証サービス初期化失敗: {e}");
                    return Err(format!("認証サービス初期化失敗: {e}").into());
                }
            };

            // AuthServiceを直接管理（コマンドで使用するため）
            app.manage(auth_service.clone());

            // SecurityServiceを管理（認証ミドルウェアで使用するため）
            // SecurityManagerから設定を取得してSecurityServiceを作成
            let security_service = Arc::new(security_manager.clone());
            app.manage(security_service.clone());

            // 認証ミドルウェアを作成・管理
            let auth_middleware =
                AuthMiddleware::new(Arc::new(auth_service.clone()), security_service.clone());
            app.manage(auth_middleware);


            eprintln!("=== アプリケーション初期化完了 ===");
            info!("アプリケーション初期化が完了しました");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // セキュリティコマンド
            security_commands::get_system_diagnostic_info,
            security_commands::validate_security_configuration,
            security_commands::test_r2_connection_secure,
            security_commands::get_environment_info,
            security_commands::log_security_event,
            security_commands::get_r2_diagnostic_info,
            security_commands::encrypt_and_store_token,
            security_commands::decrypt_token,
            security_commands::encrypt_multiple_tokens,
            security_commands::verify_api_request,
            security_commands::invalidate_token,
            security_commands::invalidate_all_tokens,
            security_commands::get_security_stats,
            security_commands::cleanup_expired_tokens,
            security_commands::detect_unauthorized_access,
            // 認証コマンド
            auth_commands::start_oauth_flow,
            auth_commands::wait_for_auth_completion,
            auth_commands::validate_session,
            auth_commands::logout,
            auth_commands::get_auth_state,
            auth_commands::get_stored_auth_info,
            auth_commands::cleanup_expired_sessions,
            // 経費コマンド（API Server経由）
            expense_commands::create_expense,
            expense_commands::get_expenses,
            expense_commands::update_expense,
            expense_commands::delete_expense,
            expense_commands::delete_expense_receipt,
            // サブスクリプションコマンド（API Server経由）
            subscription_commands::create_subscription,
            subscription_commands::get_subscriptions,
            subscription_commands::update_subscription,
            subscription_commands::toggle_subscription_status,
            subscription_commands::delete_subscription,
            subscription_commands::get_monthly_subscription_total,
            subscription_commands::upload_subscription_receipt_via_api,
            subscription_commands::delete_subscription_receipt_via_api,
            // 領収書コマンド（APIサーバー経由）
            receipt_api_commands::upload_receipt_via_api,
            receipt_api_commands::upload_multiple_receipts_via_api,
            receipt_api_commands::check_api_server_health,
            receipt_api_commands::check_api_server_health_detailed,
            receipt_api_commands::sync_fallback_files,
            receipt_api_commands::get_fallback_file_count,
            receipt_api_commands::get_receipt_via_api,
            receipt_api_commands::delete_receipt_via_api,
            receipt_commands::get_receipt_offline,
            receipt_commands::sync_cache_on_online,
            receipt_commands::get_cache_stats,
            // マイグレーションコマンド
            features::migrations::commands::check_migration_status,
            features::migrations::commands::check_auto_migration_status,
            features::migrations::commands::get_detailed_migration_info,
            features::migrations::commands::execute_user_authentication_migration,
            features::migrations::commands::execute_receipt_url_migration,
            features::migrations::commands::drop_receipt_path_column_command,
            features::migrations::commands::check_database_integrity,
            // データベース更新コマンド
            features::migrations::database_update_commands::detect_legacy_receipt_urls,
            features::migrations::database_update_commands::execute_database_update,
            features::migrations::database_update_commands::get_database_statistics,
            features::migrations::database_update_commands::update_specific_receipt_urls,
            features::migrations::database_update_commands::check_database_url_integrity,
            // アップデートコマンド
            updater_commands::check_for_updates,
            updater_commands::check_for_updates_force,
            updater_commands::download_and_install_update,
            updater_commands::get_app_version,
            updater_commands::get_updater_config,
            updater_commands::update_updater_config,
            updater_commands::skip_version,
            updater_commands::start_auto_update_check,
            updater_commands::stop_auto_update_check,
            updater_commands::restart_application,
        ])
        .run(tauri::generate_context!())
        .expect("Tauriアプリケーションの実行中にエラーが発生しました");
}

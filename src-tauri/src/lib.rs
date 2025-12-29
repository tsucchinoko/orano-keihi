// 新しい機能モジュール構造
pub mod features;
pub mod shared;

// 新しい機能モジュールからコマンドをインポート
use features::auth::middleware::AuthMiddleware;
use features::auth::service::AuthService;
use features::security::models::SecurityConfig;
use features::security::service::{SecurityManager, SecurityService};
use features::{
    auth::commands as auth_commands, expenses::commands as expense_commands,
    migrations::commands as migration_commands, receipts::commands as receipt_commands,
    security::commands as security_commands, subscriptions::commands as subscription_commands,
};
use log::{error, info, warn};
use rusqlite::Connection;
use shared::config::environment::{
    initialize_logging_system, load_environment_variables, GoogleOAuthConfig,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Manager;

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
    pub auth_service: Option<AuthService>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            // 環境に応じた.envファイルを読み込み（ログシステム初期化前に実行）
            load_environment_variables();

            // ログシステムを初期化（.envファイル読み込み後）
            initialize_logging_system();

            info!("アプリケーション初期化を開始します...");

            // セキュリティマネージャーを初期化（.envファイル読み込み後）
            let security_config = SecurityConfig {
                encryption_key: "default_key_32_bytes_long_enough".to_string(),
                max_token_age_hours: 24,
                enable_audit_logging: true,
            };
            let security_manager =
                SecurityManager::new(security_config.clone()).expect("SecurityManager初期化失敗");

            info!("システム診断情報を取得中...");

            // アプリ起動時にデータベースを初期化
            info!("データベースを初期化しています...");
            let _db_conn = shared::database::connection::initialize_database(app.handle())
                .map_err(|e| {
                    error!("データベースの初期化に失敗しました: {e}");
                    e
                })?;

            info!("データベースの初期化が完了しました");

            // 認証サービスを初期化
            info!("認証サービスを初期化しています...");
            let (auth_service, auth_middleware) = match GoogleOAuthConfig::from_env() {
                Some(oauth_config) => {
                    // 認証サービス用の新しいデータベース接続を作成
                    let auth_db_conn = shared::database::connection::initialize_database(
                        app.handle(),
                    )
                    .map_err(|e| {
                        error!("認証サービス用データベース接続の作成に失敗しました: {e}");
                        e
                    })?;

                    match AuthService::new(oauth_config, Arc::new(Mutex::new(auth_db_conn))) {
                        Ok(service) => {
                            info!("認証サービスの初期化が完了しました");

                            // SecurityServiceを作成
                            let security_service = Arc::new(
                                SecurityService::new(security_config.clone())
                                    .expect("SecurityService初期化失敗"),
                            );

                            // AuthMiddlewareを作成
                            let auth_middleware =
                                AuthMiddleware::new(Arc::new(service.clone()), security_service);

                            (Some(service), Some(auth_middleware))
                        }
                        Err(e) => {
                            warn!("認証サービスの初期化に失敗しました: {e}");
                            (None, None)
                        }
                    }
                }
                None => {
                    warn!("Google OAuth設定が見つかりません。認証機能は無効になります。");
                    (None, None)
                }
            };

            // データベース接続を再作成（メインアプリケーション用）
            let db_conn =
                shared::database::connection::initialize_database(app.handle()).map_err(|e| {
                    error!("メインアプリケーション用データベース接続の作成に失敗しました: {e}");
                    e
                })?;

            // 認証サービスが利用可能な場合は、個別に管理
            if let Some(auth_service) = &auth_service {
                app.manage(auth_service.clone());
            }

            // AuthMiddlewareが利用可能な場合は、個別に管理
            if let Some(auth_middleware) = auth_middleware {
                app.manage(auth_middleware);
            }

            app.manage(AppState {
                db: Mutex::new(db_conn),
                security_manager: security_manager.clone(),
                r2_connection_cache: Arc::new(Mutex::new(R2ConnectionCache::new())),
                auth_service,
            });

            info!("アプリケーション初期化が完了しました");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 認証コマンド
            auth_commands::start_oauth_flow,
            auth_commands::wait_for_auth_completion,
            auth_commands::validate_session,
            auth_commands::logout,
            auth_commands::get_auth_state,
            auth_commands::cleanup_expired_sessions,
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
            receipt_commands::test_r2_connection,
            // R2領収書コマンド
            receipt_commands::upload_receipt_to_r2,
            receipt_commands::get_receipt_from_r2,
            receipt_commands::delete_receipt_from_r2,
            // キャッシュ関連コマンド
            receipt_commands::get_receipt_offline,
            receipt_commands::sync_cache_on_online,
            receipt_commands::get_cache_stats,
            // 並列処理とパフォーマンス関連コマンド
            receipt_commands::upload_multiple_receipts_to_r2,
            receipt_commands::get_r2_performance_stats,
            // マイグレーションコマンド
            migration_commands::check_migration_status,
            migration_commands::execute_receipt_url_migration,
            migration_commands::execute_user_authentication_migration,
            migration_commands::execute_comprehensive_data_migration_command,
            migration_commands::restore_database_from_backup,
            migration_commands::drop_receipt_path_column_command,
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

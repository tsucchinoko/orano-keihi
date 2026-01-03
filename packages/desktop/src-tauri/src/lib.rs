// 新しい機能モジュール構造
pub mod features;
pub mod shared;

// 新しい機能モジュールからコマンドをインポート
use features::auth::middleware::AuthMiddleware;
use features::auth::service::AuthService;
use features::security::models::SecurityConfig;
use features::security::service::SecurityManager;
use features::{
    auth::commands as auth_commands,
    expenses::commands as expense_commands,
    receipts::{
        api_commands as receipt_api_commands, auth_commands as receipt_auth_commands,
        commands as receipt_commands,
    },
    security::commands as security_commands,
    subscriptions::{api_commands as subscription_api_commands, commands as subscription_commands},
};
use log::info;
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
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            // 詳細なデバッグログを追加
            eprintln!("=== アプリケーション初期化開始 ===");

            // 環境に応じた.envファイルを読み込み（ログシステム初期化前に実行）
            eprintln!("環境変数を読み込み中...");
            load_environment_variables();
            eprintln!("環境変数の読み込み完了");

            // ログシステムを初期化（.envファイル読み込み後）
            eprintln!("ログシステムを初期化中...");
            initialize_logging_system();
            eprintln!("ログシステムの初期化完了");

            // マイグレーションシステムを初期化
            eprintln!("マイグレーションシステムを初期化中...");
            if let Err(e) = features::migrations::initialize_migration_system() {
                eprintln!("マイグレーションシステムの初期化に失敗しました: {e}");
                return Err(format!("マイグレーションシステムの初期化に失敗しました: {e}").into());
            }
            eprintln!("マイグレーションシステムの初期化完了");

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

            // データベースを初期化（マイグレーション含む）
            eprintln!("データベースを初期化中...");
            let db_conn = match shared::database::connection::initialize_database(app.handle()) {
                Ok(conn) => {
                    eprintln!("データベース初期化完了");
                    conn
                }
                Err(e) => {
                    eprintln!("データベース初期化失敗: {e}");
                    return Err(format!("データベース初期化失敗: {e}").into());
                }
            };

            // Google OAuth設定を読み込み
            eprintln!("Google OAuth設定を読み込み中...");
            let google_config = match GoogleOAuthConfig::from_env() {
                Some(config) => {
                    eprintln!("Google OAuth設定を読み込みました");
                    // 設定を検証
                    if let Err(e) = config.validate() {
                        eprintln!("Google OAuth設定の検証に失敗: {e}");
                        eprintln!("デフォルト設定を使用します");
                        GoogleOAuthConfig {
                            client_id: "dummy_client_id".to_string(),
                            client_secret: "dummy_client_secret".to_string(),
                            redirect_uri: "http://localhost:8080/auth/callback".to_string(),
                            session_encryption_key: "default_32_byte_encryption_key_123"
                                .to_string(),
                        }
                    } else {
                        config
                    }
                }
                None => {
                    eprintln!("Google OAuth設定が見つかりません - デフォルト設定を使用します");
                    GoogleOAuthConfig {
                        client_id: "dummy_client_id".to_string(),
                        client_secret: "dummy_client_secret".to_string(),
                        redirect_uri: "http://localhost:8080/auth/callback".to_string(),
                        session_encryption_key: "default_32_byte_encryption_key_123".to_string(),
                    }
                }
            };

            // 認証サービスを初期化
            eprintln!("認証サービスを初期化中...");
            let auth_service = match AuthService::new(google_config, Arc::new(Mutex::new(db_conn)))
            {
                Ok(service) => {
                    eprintln!("認証サービスの初期化完了");
                    service
                }
                Err(e) => {
                    eprintln!("認証サービス初期化失敗: {e}");
                    return Err(format!("認証サービス初期化失敗: {e}").into());
                }
            };

            // アプリケーション用のデータベース接続を作成
            eprintln!("アプリケーション用データベース接続を作成中...");
            let app_db_conn = match shared::database::connection::initialize_database(app.handle())
            {
                Ok(conn) => {
                    eprintln!("アプリケーション用データベース接続作成完了");
                    conn
                }
                Err(e) => {
                    eprintln!("アプリケーション用データベース接続作成失敗: {e}");
                    return Err(format!("アプリケーション用データベース接続作成失敗: {e}").into());
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

            app.manage(AppState {
                db: Mutex::new(app_db_conn),
                security_manager: security_manager.clone(),
                r2_connection_cache: Arc::new(Mutex::new(R2ConnectionCache::new())),
            });

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
            // サブスクリプションコマンド（APIサーバー経由）
            subscription_api_commands::fetch_subscriptions_via_api,
            subscription_api_commands::create_subscription_via_api,
            subscription_api_commands::update_subscription_via_api,
            subscription_api_commands::toggle_subscription_status_via_api,
            subscription_api_commands::delete_subscription_via_api,
            subscription_api_commands::fetch_monthly_subscription_total_via_api,
            // 領収書コマンド（APIサーバー経由）
            receipt_api_commands::upload_receipt_via_api,
            receipt_api_commands::upload_multiple_receipts_via_api,
            receipt_api_commands::check_api_server_health,
            receipt_api_commands::check_api_server_health_detailed,
            receipt_api_commands::sync_fallback_files,
            receipt_api_commands::get_fallback_file_count,
            receipt_api_commands::get_receipt_via_api,
            // 領収書コマンド（認証付き）
            receipt_auth_commands::upload_receipt_with_auth,
            receipt_auth_commands::get_receipt_with_auth,
            receipt_auth_commands::delete_receipt_with_auth,
            receipt_auth_commands::download_receipt_with_auth,
            receipt_auth_commands::extract_path_from_url_with_auth,
            // サブスクリプション領収書コマンド（認証付き）
            receipt_auth_commands::upload_subscription_receipt_with_auth,
            receipt_auth_commands::delete_subscription_receipt_with_auth,
            // 領収書コマンド（通常）
            receipt_commands::get_receipt_from_r2,
            receipt_commands::delete_receipt_from_r2,
            receipt_commands::get_receipt_offline,
            receipt_commands::sync_cache_on_online,
            receipt_commands::get_cache_stats,
            receipt_commands::upload_multiple_receipts_to_r2,
            receipt_commands::test_r2_connection,
            receipt_commands::get_r2_performance_stats,
            // マイグレーションコマンド
            features::migrations::commands::check_migration_status,
            features::migrations::commands::check_auto_migration_status,
            features::migrations::commands::get_detailed_migration_info,
            features::migrations::commands::execute_user_authentication_migration,
            features::migrations::commands::execute_receipt_url_migration,
            features::migrations::commands::drop_receipt_path_column_command,
            features::migrations::commands::check_database_integrity,
            // R2マイグレーションコマンド
            features::migrations::r2_migration_commands::start_r2_migration,
            features::migrations::r2_migration_commands::get_r2_migration_status,
            features::migrations::r2_migration_commands::pause_r2_migration,
            features::migrations::r2_migration_commands::resume_r2_migration,
            features::migrations::r2_migration_commands::stop_r2_migration,
            features::migrations::r2_migration_commands::validate_r2_migration_integrity,
            // データベース更新コマンド
            features::migrations::database_update_commands::detect_legacy_receipt_urls,
            features::migrations::database_update_commands::execute_database_update,
            features::migrations::database_update_commands::get_database_statistics,
            features::migrations::database_update_commands::update_specific_receipt_urls,
            features::migrations::database_update_commands::check_database_url_integrity,
        ])
        .run(tauri::generate_context!())
        .expect("Tauriアプリケーションの実行中にエラーが発生しました");
}

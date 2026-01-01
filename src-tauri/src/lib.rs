// 新しい機能モジュール構造
pub mod features;
pub mod shared;

// 新しい機能モジュールからコマンドをインポート
use features::auth::service::AuthService;
use features::security::models::SecurityConfig;
use features::security::service::SecurityManager;
use features::{auth::commands as auth_commands, security::commands as security_commands};
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
            if let Err(e) = std::panic::catch_unwind(|| {
                load_environment_variables();
            }) {
                eprintln!("環境変数の読み込みでパニックが発生しました: {e:?}");
                return Err("環境変数の読み込みに失敗しました".into());
            }
            eprintln!("環境変数の読み込み完了");

            // ログシステムを初期化（.envファイル読み込み後）
            eprintln!("ログシステムを初期化中...");
            if let Err(e) = std::panic::catch_unwind(|| {
                initialize_logging_system();
            }) {
                eprintln!("ログシステムの初期化でパニックが発生しました: {e:?}");
                return Err("ログシステムの初期化に失敗しました".into());
            }
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
                    return Err(format!("SecurityManager初期化失敗: {e}").into());
                }
            };

            info!("システム診断情報を取得中...");

            // データベース初期化をスキップ（クラッシュ原因特定のため）
            eprintln!("データベース初期化をスキップします（デバッグモード）");

            // 認証サービス初期化もスキップ
            eprintln!("認証サービス初期化をスキップします（デバッグモード）");

            // Google OAuth設定を読み込み
            let google_config = match GoogleOAuthConfig::from_env() {
                Some(config) => {
                    eprintln!("Google OAuth設定を読み込みました");
                    config
                }
                None => {
                    eprintln!("Google OAuth設定が見つかりません - 認証機能は無効になります");
                    return Err("Google OAuth設定が必要です".into());
                }
            };

            // ダミーのデータベース接続を作成（実際には使用しない）
            let dummy_db_path = app
                .handle()
                .path()
                .app_data_dir()
                .map_err(|e| format!("アプリデータディレクトリの取得に失敗: {e}"))?
                .join("dummy.db");

            let dummy_db_conn = match rusqlite::Connection::open(&dummy_db_path) {
                Ok(conn) => {
                    eprintln!("ダミーデータベース接続作成完了");
                    conn
                }
                Err(e) => {
                    eprintln!("ダミーデータベース接続作成失敗: {e}");
                    return Err(format!("ダミーデータベース接続作成失敗: {e}").into());
                }
            };

            // 認証サービスを初期化
            eprintln!("認証サービスを初期化中...");
            let auth_service =
                match AuthService::new(google_config, Arc::new(Mutex::new(dummy_db_conn))) {
                    Ok(service) => {
                        eprintln!("認証サービスの初期化完了");
                        service
                    }
                    Err(e) => {
                        eprintln!("認証サービス初期化失敗: {e}");
                        return Err(format!("認証サービス初期化失敗: {e}").into());
                    }
                };

            // 最小限の状態でアプリケーション状態を管理
            eprintln!("最小限のアプリケーション状態を設定中...");

            // 新しいダミーデータベース接続を作成（AuthServiceで使用したものとは別）
            let app_dummy_db_conn = match rusqlite::Connection::open(&dummy_db_path) {
                Ok(conn) => {
                    eprintln!("アプリ用ダミーデータベース接続作成完了");
                    conn
                }
                Err(e) => {
                    eprintln!("アプリ用ダミーデータベース接続作成失敗: {e}");
                    return Err(format!("アプリ用ダミーデータベース接続作成失敗: {e}").into());
                }
            };

            // AuthServiceを直接管理（コマンドで使用するため）
            app.manage(auth_service);

            app.manage(AppState {
                db: Mutex::new(app_dummy_db_conn),
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
            // 認証コマンド
            auth_commands::start_oauth_flow,
            auth_commands::wait_for_auth_completion,
            auth_commands::validate_session,
            auth_commands::logout,
            auth_commands::get_auth_state,
            auth_commands::cleanup_expired_sessions,
        ])
        .run(tauri::generate_context!())
        .expect("Tauriアプリケーションの実行中にエラーが発生しました");
}

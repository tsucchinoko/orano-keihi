/// アプリケーションの実行環境を表す列挙型
#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    /// 開発環境
    Development,
    /// プロダクション環境
    Production,
}

/// 環境設定を管理する構造体
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    /// 実行環境
    pub environment: String,
    /// デバッグモードの有効/無効
    pub debug_mode: bool,
    /// ログレベル
    pub log_level: String,
}

impl EnvironmentConfig {
    /// 環境変数から設定を読み込む
    ///
    /// # 戻り値
    /// 環境設定
    pub fn from_env() -> Self {
        let environment = get_environment();
        let debug_mode = environment == Environment::Development;
        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| {
            if debug_mode {
                "debug".to_string()
            } else {
                "info".to_string()
            }
        });

        Self {
            environment: format!("{environment:?}").to_lowercase(),
            debug_mode,
            log_level,
        }
    }

    /// プロダクション環境かどうかを判定
    ///
    /// # 戻り値
    /// プロダクション環境の場合はtrue
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    /// 開発環境かどうかを判定
    ///
    /// # 戻り値
    /// 開発環境の場合はtrue
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }
}

/// 現在の実行環境を判定する
///
/// # 戻り値
/// 現在の実行環境（Development または Production）
///
/// # 判定ロジック
/// 1. 実行時環境変数 ENVIRONMENT を確認
/// 2. デバッグビルドの場合は Development
/// 3. リリースビルドの場合は Production
pub fn get_environment() -> Environment {
    // 実行時環境変数を確認
    if let Ok(env_var) = std::env::var("ENVIRONMENT") {
        let env = match env_var.as_str() {
            "production" => Environment::Production,
            _ => Environment::Development,
        };
        log::debug!("環境判定: 実行時環境変数を使用 -> {env_var} -> {env:?}");
        return env;
    }

    // フォールバック: ビルド設定に基づく判定
    let env = if cfg!(debug_assertions) {
        Environment::Development
    } else {
        Environment::Production
    };
    log::debug!(
        "環境判定: ビルド設定を使用 -> debug_assertions={} -> {env:?}",
        cfg!(debug_assertions)
    );
    env
}

/// 環境に応じたデータベースファイル名を取得する
///
/// # 引数
/// * `env` - 実行環境
///
/// # 戻り値
/// データベースファイル名
///
/// # ファイル名の規則
/// - 開発環境: "dev_expenses.db"
/// - プロダクション環境: "expenses.db"
pub fn get_database_filename(env: Environment) -> &'static str {
    match env {
        Environment::Development => "dev_expenses.db",
        Environment::Production => "expenses.db",
    }
}

/// 環境変数の読み込みを確認する
///
/// # 処理内容
/// 1. 開発環境（pnpm tauri dev）の場合のみ.envファイルを読み込み
/// 2. 本番ビルドでは環境変数は実行時に設定されることを前提とする
///
/// # 注意
/// - 本番環境では.envファイルは読み込まれません（秘匿情報がバイナリに埋め込まれるのを防ぐため）
/// - 本番実行時は環境変数を設定してからアプリケーションを起動してください
pub fn load_environment_variables() {
    // 開発環境かどうかを判定（デバッグビルド）
    let is_development = cfg!(debug_assertions);

    if is_development {
        // 開発環境の場合のみ.envファイルを読み込む
        eprintln!("開発環境: .envファイルを読み込みます");

        match dotenv::dotenv() {
            Ok(path) => {
                eprintln!("環境ファイルを読み込みました: {}", path.display());
            }
            Err(e) => {
                eprintln!("環境ファイルの読み込みに失敗: {e}");
                eprintln!("環境変数が設定されていることを確認してください");
            }
        }
    } else {
        // 本番環境では.envファイルを読み込まない
        eprintln!("本番環境: 環境変数は実行時に設定されます");
    }

    // 読み込み後の環境変数を確認
    if let Ok(env_var) = std::env::var("ENVIRONMENT") {
        eprintln!("ENVIRONMENT環境変数: {env_var}");
    } else {
        eprintln!("ENVIRONMENT環境変数が設定されていません（デフォルト値を使用）");
    }
}

/// ログシステムを初期化する
///
/// # 処理内容
/// 1. 環境設定を取得
/// 2. ログレベルを設定
/// 3. env_loggerを初期化
pub fn initialize_logging_system() {
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

    log::info!(
        "ログシステムを初期化しました: level={}, environment={}",
        env_config.log_level,
        env_config.environment
    );
}

/// R2（Cloudflare R2）の設定を管理する構造体
#[derive(Debug, Clone)]
pub struct R2Config {
    /// R2のアクセスキーID
    pub access_key_id: String,
    /// R2のシークレットアクセスキー
    pub secret_access_key: String,
    /// R2のバケット名
    pub bucket_name: String,
    /// R2のエンドポイントURL
    pub endpoint_url: String,
    /// R2のリージョン
    pub region: String,
}

/// API設定を管理する構造体
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// APIサーバーのベースURL
    pub base_url: String,
    /// APIリクエストのタイムアウト（秒）
    pub timeout_seconds: u64,
    /// APIリクエストの最大リトライ回数
    pub max_retries: u32,
}

impl ApiConfig {
    /// 環境変数からAPI設定を読み込む
    ///
    /// # 戻り値
    /// API設定
    pub fn from_env() -> Self {
        log::debug!("ApiConfig::from_env() - 環境変数の読み込みを開始");

        // 実行時環境変数を使用
        let base_url = std::env::var("API_SERVER_URL")
            .ok()
            .map(|val| {
                log::debug!("実行時API_SERVER_URL が見つかりました: {val}");
                val
            })
            .unwrap_or_else(|| {
                let default_url = "http://localhost:8787";
                log::debug!(
                    "API_SERVER_URL が設定されていないため、デフォルト値を使用: {default_url}"
                );
                default_url.to_string()
            });

        let timeout_seconds = std::env::var("API_TIMEOUT_SECONDS")
            .ok()
            .and_then(|val| val.parse().ok())
            .unwrap_or_else(|| {
                log::debug!("API_TIMEOUT_SECONDS が設定されていないため、デフォルト値30秒を使用");
                30
            });

        let max_retries = std::env::var("API_MAX_RETRIES")
            .ok()
            .and_then(|val| val.parse().ok())
            .unwrap_or_else(|| {
                log::debug!("API_MAX_RETRIES が設定されていないため、デフォルト値3回を使用");
                3
            });

        log::debug!("ApiConfig::from_env() - 設定の読み込みが完了しました");
        log::info!(
            "API設定: base_url={base_url}, timeout={timeout_seconds}s, max_retries={max_retries}"
        );

        Self {
            base_url,
            timeout_seconds,
            max_retries,
        }
    }

    /// API設定が有効かどうかを判定
    ///
    /// # 戻り値
    /// 設定が有効な場合はtrue
    pub fn is_valid(&self) -> bool {
        !self.base_url.is_empty() && self.timeout_seconds > 0
    }

    /// 設定を検証する
    ///
    /// # 戻り値
    /// 設定が有効な場合はOk(())、無効な場合はErr
    pub fn validate(&self) -> Result<(), String> {
        if self.base_url.is_empty() {
            return Err("APIサーバーのベースURLが設定されていません".to_string());
        }

        if self.timeout_seconds == 0 {
            return Err("APIタイムアウトは0より大きい値である必要があります".to_string());
        }

        Ok(())
    }

    /// APIサーバーがlocalhostかどうかを判定
    ///
    /// # 戻り値
    /// localhostの場合はtrue
    pub fn is_localhost(&self) -> bool {
        self.base_url.contains("localhost") || self.base_url.contains("127.0.0.1")
    }

    /// デバッグ情報を取得
    ///
    /// # 戻り値
    /// デバッグ情報のマップ
    pub fn get_debug_info(&self) -> std::collections::HashMap<String, String> {
        let mut info = std::collections::HashMap::new();
        info.insert("base_url".to_string(), self.base_url.clone());
        info.insert(
            "timeout_seconds".to_string(),
            self.timeout_seconds.to_string(),
        );
        info.insert("max_retries".to_string(), self.max_retries.to_string());
        info.insert("is_localhost".to_string(), self.is_localhost().to_string());
        info
    }
}

/// Google OAuth 2.0の設定を管理する構造体（ネイティブアプリ用）
#[derive(Debug, Clone)]
pub struct GoogleOAuthConfig {
    /// GoogleクライアントID
    pub client_id: String,
    /// Googleクライアントシークレット（一時的に使用）
    pub client_secret: String,
    /// OAuth2リダイレクトURI（動的に設定されるため、ベースURIとして使用）
    pub redirect_uri: String,
    /// セッション暗号化キー
    pub session_encryption_key: String,
}

impl GoogleOAuthConfig {
    /// 環境変数からGoogle OAuth設定を読み込む（ネイティブアプリ用）
    ///
    /// # 戻り値
    /// Google OAuth設定、または設定が不完全な場合はNone
    pub fn from_env() -> Option<Self> {
        log::debug!("GoogleOAuthConfig::from_env() - 環境変数の読み込みを開始");

        // 実行時環境変数を使用
        let client_id = std::env::var("GOOGLE_CLIENT_ID").ok().map(|val| {
            log::debug!(
                "実行時GOOGLE_CLIENT_ID が見つかりました: {}****",
                &val[..8.min(val.len())]
            );
            val
        });

        let client_id = match client_id {
            Some(val) => val,
            None => {
                log::error!("GOOGLE_CLIENT_ID が見つかりません");
                return None;
            }
        };

        // ネイティブアプリではクライアントシークレットは不要（PKCE使用）
        // 一時的にクライアントシークレットを使用
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
            .ok()
            .unwrap_or_else(|| {
                log::debug!("GOOGLE_CLIENT_SECRET が設定されていません");
                String::new()
            });

        log::debug!("一時的にクライアントシークレットを使用します（テスト用）");

        let redirect_uri = std::env::var("GOOGLE_REDIRECT_URI")
            .ok()
            .unwrap_or_else(|| {
                log::debug!("GOOGLE_REDIRECT_URI が設定されていないため、デフォルト値を使用");
                "http://127.0.0.1/callback".to_string()
            });

        let session_encryption_key = std::env::var("SESSION_ENCRYPTION_KEY").ok().unwrap_or_else(|| {
            log::warn!("SESSION_ENCRYPTION_KEY が設定されていないため、デフォルト値を使用（本番環境では必ず設定してください）");
            "default_32_byte_encryption_key_123".to_string()
        });

        log::debug!("GoogleOAuthConfig::from_env() - 設定の読み込みが完了しました");
        Some(Self {
            client_id,
            client_secret,
            redirect_uri,
            session_encryption_key,
        })
    }

    /// Google OAuth設定が有効かどうかを判定
    ///
    /// # 戻り値
    /// 設定が有効な場合はtrue
    pub fn is_valid(&self) -> bool {
        !self.client_id.is_empty()
            && !self.redirect_uri.is_empty()
            && !self.session_encryption_key.is_empty()
    }

    /// 設定を検証する
    ///
    /// # 戻り値
    /// 設定が有効な場合はOk(())、無効な場合はErr
    pub fn validate(&self) -> Result<(), String> {
        if !self.is_valid() {
            return Err("Google OAuth設定が不完全です".to_string());
        }

        // セッション暗号化キーの長さをチェック（最低16バイト）
        if self.session_encryption_key.len() < 16 {
            return Err("セッション暗号化キーは最低16文字以上である必要があります".to_string());
        }

        Ok(())
    }

    /// デバッグ情報を取得
    ///
    /// # 戻り値
    /// デバッグ情報のマップ
    pub fn get_debug_info(&self) -> std::collections::HashMap<String, String> {
        let mut info = std::collections::HashMap::new();
        info.insert(
            "client_id".to_string(),
            format!("{}****", &self.client_id[..8.min(self.client_id.len())]),
        );
        info.insert("redirect_uri".to_string(), self.redirect_uri.clone());
        info.insert(
            "session_encryption_key_length".to_string(),
            self.session_encryption_key.len().to_string(),
        );
        info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_database_filename() {
        // 開発環境のデータベースファイル名をテスト
        assert_eq!(
            get_database_filename(Environment::Development),
            "dev_expenses.db"
        );

        // プロダクション環境のデータベースファイル名をテスト
        assert_eq!(
            get_database_filename(Environment::Production),
            "expenses.db"
        );
    }

    #[test]
    fn test_environment_equality() {
        // Environment列挙型の等価性をテスト
        assert_eq!(Environment::Development, Environment::Development);
        assert_eq!(Environment::Production, Environment::Production);
        assert_ne!(Environment::Development, Environment::Production);
    }

    #[test]
    fn test_get_environment() {
        // 現在の環境を取得（実際の値はビルド設定に依存）
        let env = get_environment();

        // デバッグビルドかリリースビルドかのいずれかであることを確認
        assert!(matches!(
            env,
            Environment::Development | Environment::Production
        ));
    }

    #[test]
    fn test_environment_config_from_env() {
        let config = EnvironmentConfig::from_env();

        // 設定が適切に読み込まれることを確認
        assert!(config.environment == "development" || config.environment == "production");
        assert!(!config.log_level.is_empty());
    }

    #[test]
    fn test_environment_config_methods() {
        let dev_config = EnvironmentConfig {
            environment: "development".to_string(),
            debug_mode: true,
            log_level: "debug".to_string(),
        };

        let prod_config = EnvironmentConfig {
            environment: "production".to_string(),
            debug_mode: false,
            log_level: "info".to_string(),
        };

        // 開発環境の判定テスト
        assert!(dev_config.is_development());
        assert!(!dev_config.is_production());

        // プロダクション環境の判定テスト
        assert!(!prod_config.is_development());
        assert!(prod_config.is_production());
    }

    #[test]
    fn test_load_environment_variables() {
        // 環境変数読み込み関数が正常に実行されることを確認（パニックしない）
        load_environment_variables();
    }
}

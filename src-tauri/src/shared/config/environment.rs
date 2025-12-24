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
/// 1. コンパイル時埋め込み環境変数を最優先
/// 2. 実行時環境変数 ENVIRONMENT を確認
/// 3. デバッグビルドの場合は Development
/// 4. リリースビルドの場合は Production
pub fn get_environment() -> Environment {
    // コンパイル時埋め込み環境変数を最優先
    if let Some(embedded_env) = option_env!("EMBEDDED_ENVIRONMENT") {
        let env = match embedded_env {
            "production" => Environment::Production,
            _ => Environment::Development,
        };
        log::debug!("環境判定: コンパイル時埋め込み値を使用 -> {embedded_env} -> {env:?}");
        return env;
    }

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

/// 環境に応じた.envファイルを読み込む
///
/// # 処理内容
/// 1. コンパイル時埋め込み環境変数をチェック
/// 2. 環境に応じた.envファイルを読み込み
/// 3. フォールバック処理
pub fn load_environment_variables() {
    // コンパイル時に埋め込まれた環境設定があるかチェック
    let embedded_env = option_env!("EMBEDDED_ENVIRONMENT");

    if let Some(env) = embedded_env {
        log::info!("コンパイル時埋め込み環境設定を使用: {env}");
        // コンパイル時に埋め込まれた環境変数がある場合は、実行時読み込みをスキップ
        return;
    }

    // まず、ENVIRONMENTが設定されているかチェック
    let environment = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

    // 環境に応じた.envファイルのパスを決定
    let env_file = match environment.as_str() {
        "production" => ".env.production",
        "development" => ".env",
        _ => ".env", // デフォルトは開発環境
    };

    log::info!("環境: {environment}, 読み込み対象: {env_file}");

    // 指定された.envファイルを読み込み
    match dotenv::from_filename(env_file) {
        Ok(_) => {
            log::info!("{env_file}ファイルを読み込みました");
        }
        Err(_) => {
            // 環境固有のファイルがない場合は、デフォルトの.envを試行
            if env_file != ".env" {
                match dotenv::dotenv() {
                    Ok(_) => {
                        log::warn!("{env_file}が見つからないため、デフォルトの.envファイルを読み込みました");
                    }
                    Err(_) => {
                        log::warn!("環境変数ファイルが見つかりません。コンパイル時埋め込み値または直接設定された環境変数を使用します。");
                    }
                }
            } else {
                log::warn!(".envファイルが見つかりません。コンパイル時埋め込み値または直接設定された環境変数を使用します。");
            }
        }
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

impl R2Config {
    /// 環境変数からR2設定を読み込む
    ///
    /// # 戻り値
    /// R2設定、または設定が不完全な場合はNone
    pub fn from_env() -> Option<Self> {
        let access_key_id = std::env::var("R2_ACCESS_KEY_ID").ok()?;
        let secret_access_key = std::env::var("R2_SECRET_ACCESS_KEY").ok()?;
        let bucket_name = std::env::var("R2_BUCKET_NAME").ok()?;
        let region = std::env::var("R2_REGION").unwrap_or_else(|_| "auto".to_string());

        // エンドポイントURLが設定されていない場合は、アカウントIDから自動構築
        let endpoint_url = std::env::var("R2_ENDPOINT_URL").unwrap_or_else(|_| {
            if let Ok(account_id) = std::env::var("R2_ACCOUNT_ID") {
                format!("https://{}.r2.cloudflarestorage.com", account_id)
            } else {
                // アカウントIDも設定されていない場合はデフォルト
                "https://r2.cloudflarestorage.com".to_string()
            }
        });

        Some(Self {
            access_key_id,
            secret_access_key,
            bucket_name,
            endpoint_url,
            region,
        })
    }

    /// R2設定が有効かどうかを判定
    ///
    /// # 戻り値
    /// 設定が有効な場合はtrue
    pub fn is_valid(&self) -> bool {
        !self.access_key_id.is_empty()
            && !self.secret_access_key.is_empty()
            && !self.bucket_name.is_empty()
            && !self.endpoint_url.is_empty()
    }

    /// 設定を検証する
    ///
    /// # 戻り値
    /// 設定が有効な場合はOk(())、無効な場合はErr
    pub fn validate(&self) -> Result<(), String> {
        if !self.is_valid() {
            return Err("R2設定が不完全です".to_string());
        }
        Ok(())
    }

    /// 環境に応じたバケット名を取得
    ///
    /// # 戻り値
    /// バケット名
    pub fn get_environment_bucket_name(&self) -> String {
        self.bucket_name.clone()
    }

    /// デバッグ情報を取得
    ///
    /// # 戻り値
    /// デバッグ情報のマップ
    pub fn get_debug_info(&self) -> std::collections::HashMap<String, String> {
        let mut info = std::collections::HashMap::new();
        info.insert(
            "access_key_id".to_string(),
            format!(
                "{}****",
                &self.access_key_id[..4.min(self.access_key_id.len())]
            ),
        );
        info.insert("bucket_name".to_string(), self.bucket_name.clone());
        info.insert("endpoint_url".to_string(), self.endpoint_url.clone());
        info.insert("region".to_string(), self.region.clone());
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

/// アプリケーションの実行環境を表す列挙型
#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    /// 開発環境
    Development,
    /// プロダクション環境
    Production,
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
        println!("環境判定: コンパイル時埋め込み値を使用 -> {embedded_env} -> {env:?}");
        return env;
    }

    // 実行時環境変数を確認
    if let Ok(env_var) = std::env::var("ENVIRONMENT") {
        let env = match env_var.as_str() {
            "production" => Environment::Production,
            _ => Environment::Development,
        };
        println!("環境判定: 実行時環境変数を使用 -> {env_var} -> {env:?}");
        return env;
    }

    // フォールバック: ビルド設定に基づく判定
    let env = if cfg!(debug_assertions) {
        Environment::Development
    } else {
        Environment::Production
    };
    println!(
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
}

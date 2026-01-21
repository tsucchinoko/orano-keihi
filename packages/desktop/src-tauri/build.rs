use std::env;

fn main() {
    // 必須の環境変数をコンパイル時に埋め込む
    embed_env_var("API_SERVER_URL", true);

    // オプションの環境変数をコンパイル時に埋め込む
    embed_env_var("API_TIMEOUT_SECONDS", false);
    embed_env_var("API_MAX_RETRIES", false);
    embed_env_var("LOG_LEVEL", false);
    embed_env_var("ENVIRONMENT", false);

    // Tauriのビルド処理を実行
    tauri_build::build()
}

/// 環境変数をコンパイル時に埋め込む
///
/// # 引数
/// * `var_name` - 環境変数名
/// * `required` - 必須かどうか
fn embed_env_var(var_name: &str, required: bool) {
    match env::var(var_name) {
        Ok(value) => {
            // 環境変数が設定されている場合は、cargo:rustc-env で埋め込む
            println!("cargo:rustc-env={var_name}={value}");
            eprintln!("ビルド時環境変数を埋め込みました: {var_name}={value}");
        }
        Err(_) => {
            if required {
                // 必須の環境変数が見つからない場合はエラー
                eprintln!("エラー: 必須の環境変数 {var_name} が設定されていません");
            } else {
                // オプションの環境変数が見つからない場合は警告のみ
                eprintln!("警告: オプションの環境変数 {var_name} が設定されていません（デフォルト値を使用します）");
            }
        }
    }
}

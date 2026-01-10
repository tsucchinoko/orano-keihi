use std::env;

fn main() {
    // ビルド時に環境変数を設定
    // 環境変数は外部（スクリプトや `pnpm tauri dev` 実行時の .env ファイル）から提供されることを前提とする
    // 本番ビルド時は script/.env.local から環境変数を読み込む
    // 開発環境（pnpm tauri dev）では .env ファイルが自動的に読み込まれる

    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

    println!("cargo:rustc-env=ENVIRONMENT={}", environment);

    // API設定
    let api_server_url =
        env::var("API_SERVER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let api_timeout = env::var("API_TIMEOUT_SECONDS").unwrap_or_else(|_| "30".to_string());
    let api_max_retries = env::var("API_MAX_RETRIES").unwrap_or_else(|_| "3".to_string());

    println!("cargo:rustc-env=API_SERVER_URL={}", api_server_url);
    println!("cargo:rustc-env=API_TIMEOUT_SECONDS={}", api_timeout);
    println!("cargo:rustc-env=API_MAX_RETRIES={}", api_max_retries);

    // Google OAuth設定
    // 本番環境ではスクリプトから環境変数を設定する必要がある
    let google_client_id =
        env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID environment variable must be set");
    let google_client_secret = env::var("GOOGLE_CLIENT_SECRET")
        .expect("GOOGLE_CLIENT_SECRET environment variable must be set");
    let google_redirect_uri =
        env::var("GOOGLE_REDIRECT_URI").unwrap_or_else(|_| "http://127.0.0.1/callback".to_string());

    println!("cargo:rustc-env=GOOGLE_CLIENT_ID={}", google_client_id);
    println!(
        "cargo:rustc-env=GOOGLE_CLIENT_SECRET={}",
        google_client_secret
    );
    println!(
        "cargo:rustc-env=GOOGLE_REDIRECT_URI={}",
        google_redirect_uri
    );

    // セッション暗号化キー
    let session_key = env::var("SESSION_ENCRYPTION_KEY")
        .expect("SESSION_ENCRYPTION_KEY environment variable must be set");
    println!("cargo:rustc-env=SESSION_ENCRYPTION_KEY={}", session_key);

    // ログレベル
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    println!("cargo:rustc-env=LOG_LEVEL={}", log_level);

    // ビルド情報を出力
    println!("cargo:warning=ビルド環境: {}", environment);
    println!("cargo:warning=APIサーバーURL: {}", api_server_url);
    println!(
        "cargo:warning=Google Client ID: {}...",
        &google_client_id[..std::cmp::min(20, google_client_id.len())]
    );

    tauri_build::build()
}

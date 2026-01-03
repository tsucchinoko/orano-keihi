use std::env;

fn main() {
    // ビルド時に環境変数を設定
    // 本番環境かどうかを判定
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

    // 環境に応じた.envファイルを読み込み（順序が重要）
    if environment == "production" {
        // 本番環境の場合は.env.productionを優先
        if let Err(e) = dotenv::from_filename(".env.production") {
            println!(
                "cargo:warning=.env.productionファイルの読み込みに失敗: {}",
                e
            );
            // フォールバック: デフォルトの.envファイルを試行
            if let Err(e2) = dotenv::dotenv() {
                println!("cargo:warning=.envファイルの読み込みも失敗: {}", e2);
            }
        } else {
            println!("cargo:warning=.env.productionファイルを読み込みました");
        }
    } else {
        // 開発環境の場合は通常の.envファイル
        if let Err(e) = dotenv::dotenv() {
            println!("cargo:warning=.envファイルの読み込みに失敗: {}", e);
        } else {
            println!("cargo:warning=.envファイルを読み込みました");
        }
    }

    println!("cargo:rustc-env=ENVIRONMENT={}", environment);

    // API設定
    let api_server_url = env::var("API_SERVER_URL").unwrap_or_else(|_| {
        if environment == "production" {
            "https://orano-keihi.ccya2211.workers.dev".to_string()
        } else {
            "http://localhost:3000".to_string()
        }
    });

    let api_timeout = env::var("API_TIMEOUT_SECONDS").unwrap_or_else(|_| "30".to_string());
    let api_max_retries = env::var("API_MAX_RETRIES").unwrap_or_else(|_| "3".to_string());

    println!("cargo:rustc-env=API_SERVER_URL={}", api_server_url);
    println!("cargo:rustc-env=API_TIMEOUT_SECONDS={}", api_timeout);
    println!("cargo:rustc-env=API_MAX_RETRIES={}", api_max_retries);

    // Google OAuth設定（必須）
    let google_client_id = env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| {
        "916180622636-3ekaglfm1iut8e1d9nks8cub5u1uha32.apps.googleusercontent.com".to_string()
    });
    let google_client_secret = env::var("GOOGLE_CLIENT_SECRET")
        .unwrap_or_else(|_| "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".to_string());
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
        .unwrap_or_else(|_| "1f77277c2b02375a1875e3f74d8ab70f".to_string());
    println!("cargo:rustc-env=SESSION_ENCRYPTION_KEY={}", session_key);

    // ログレベル
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    println!("cargo:rustc-env=LOG_LEVEL={}", log_level);

    // ビルド情報を出力
    println!("cargo:warning=ビルド環境: {}", environment);
    println!("cargo:warning=APIサーバーURL: {}", api_server_url);
    println!(
        "cargo:warning=Google Client ID: {}...",
        &google_client_id[..20]
    );

    tauri_build::build()
}

use std::env;

fn main() {
    // Tauriのビルドスクリプトを実行
    tauri_build::build();

    // 環境変数をコンパイル時に埋め込み
    // ENVIRONMENT環境変数に基づいて適切な.envファイルを読み込み
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

    let env_file = match environment.as_str() {
        "production" => ".env.production",
        _ => ".env",
    };

    println!("cargo:rerun-if-env-changed=ENVIRONMENT");
    println!("cargo:rerun-if-changed={env_file}");

    // 環境変数ファイルを読み込み
    if dotenv::from_filename(env_file).is_ok() {
        println!("cargo:warning={env_file}ファイルを読み込みました");

        // 必要な環境変数をコンパイル時定数として埋め込み
        if let Ok(account_id) = env::var("R2_ACCOUNT_ID") {
            println!("cargo:rustc-env=EMBEDDED_R2_ACCOUNT_ID={account_id}");
        }
        if let Ok(access_key) = env::var("R2_ACCESS_KEY_ID") {
            println!("cargo:rustc-env=EMBEDDED_R2_ACCESS_KEY_ID={access_key}");
        }
        if let Ok(secret_key) = env::var("R2_SECRET_ACCESS_KEY") {
            println!("cargo:rustc-env=EMBEDDED_R2_SECRET_ACCESS_KEY={secret_key}");
        }
        if let Ok(bucket_name) = env::var("R2_BUCKET_NAME") {
            println!("cargo:rustc-env=EMBEDDED_R2_BUCKET_NAME={bucket_name}");
        }
        if let Ok(region) = env::var("R2_REGION") {
            println!("cargo:rustc-env=EMBEDDED_R2_REGION={region}");
        }

        // Google OAuth設定も埋め込み
        if let Ok(client_id) = env::var("GOOGLE_CLIENT_ID") {
            println!("cargo:rustc-env=EMBEDDED_GOOGLE_CLIENT_ID={client_id}");
        }
        if let Ok(client_secret) = env::var("GOOGLE_CLIENT_SECRET") {
            println!("cargo:rustc-env=EMBEDDED_GOOGLE_CLIENT_SECRET={client_secret}");
        }
        if let Ok(redirect_uri) = env::var("GOOGLE_REDIRECT_URI") {
            println!("cargo:rustc-env=EMBEDDED_GOOGLE_REDIRECT_URI={redirect_uri}");
        }
        if let Ok(encryption_key) = env::var("SESSION_ENCRYPTION_KEY") {
            println!("cargo:rustc-env=EMBEDDED_SESSION_ENCRYPTION_KEY={encryption_key}");
        }

        // 注意: EMBEDDED_ENVIRONMENTは設定しない
        // 実行時に.envファイルから環境変数を読み込むため
    } else {
        println!("cargo:warning={env_file}ファイルが見つかりません");
    }
}

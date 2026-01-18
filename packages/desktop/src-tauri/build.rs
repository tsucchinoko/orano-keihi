fn main() {
    // .envファイルを読み込む（ビルド時）
    // option_env!マクロがビルド時の環境変数を自動的に読み込むため、
    // ここでdotenvを実行することで.envファイルの内容がビルド時に利用可能になる
    if let Err(e) = dotenv::dotenv() {
        eprintln!("警告: .envファイルの読み込みに失敗しました: {e}");
        eprintln!("環境変数が設定されていることを確認してください");
    } else {
        eprintln!("ビルド時: .envファイルを読み込みました");
    }

    // Tauriのビルド処理を実行
    tauri_build::build()
}

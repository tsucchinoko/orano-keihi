fn main() {
    // Tauriのビルド処理を実行
    // 環境変数はすべて実行時に読み込むため、ビルド時の埋め込みは不要
    tauri_build::build()
}

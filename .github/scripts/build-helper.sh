#!/bin/bash

# MacOSビルドヘルパースクリプト
# エラーハンドリングとログ機能を提供

set -euo pipefail  # エラー時に即座に終了、未定義変数でエラー、パイプラインエラーを検出

# ログ関数
log_info() {
    echo "::notice::$1"
    echo "[INFO] $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

log_error() {
    echo "::error::$1"
    echo "[ERROR] $(date '+%Y-%m-%d %H:%M:%S') - $1" >&2
}

log_warning() {
    echo "::warning::$1"
    echo "[WARNING] $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

# システム情報の収集
collect_system_info() {
    log_info "システム情報を収集中..."
    
    echo "=== システム情報 ==="
    echo "OS: $(uname -a)"
    echo "日時: $(date)"
    echo "ユーザー: $(whoami)"
    echo "作業ディレクトリ: $(pwd)"
    
    echo "=== 開発環境情報 ==="
    echo "Rust: $(rustc --version 2>/dev/null || echo '未インストール')"
    echo "Cargo: $(cargo --version 2>/dev/null || echo '未インストール')"
    echo "Deno: $(deno --version 2>/dev/null || echo '未インストール')"
    
    echo "=== システムリソース ==="
    echo "ディスク使用量:"
    df -h
    echo "メモリ使用量:"
    vm_stat 2>/dev/null || echo "メモリ情報取得失敗"
    
    echo "=== 環境変数 ==="
    env | grep -E "(RUST|CARGO|TAURI|DENO)" | sort || echo "関連環境変数なし"
}

# 依存関係の確認
check_dependencies() {
    log_info "依存関係を確認中..."
    
    local missing_deps=()
    
    # Rustの確認
    if ! command -v rustc &> /dev/null; then
        missing_deps+=("rustc")
    fi
    
    if ! command -v cargo &> /dev/null; then
        missing_deps+=("cargo")
    fi
    
    # Denoの確認
    if ! command -v deno &> /dev/null; then
        missing_deps+=("deno")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "不足している依存関係: ${missing_deps[*]}"
        return 1
    fi
    
    log_info "すべての依存関係が利用可能です"
    return 0
}

# フロントエンドビルドの実行
build_frontend() {
    log_info "フロントエンドビルドを開始..."
    
    # package.jsonの存在確認
    if [ ! -f "package.json" ]; then
        log_error "package.jsonが見つかりません"
        return 1
    fi
    
    # deno.jsonの存在確認
    if [ ! -f "deno.json" ]; then
        log_warning "deno.jsonが見つかりません"
    fi
    
    # ビルドの実行
    if deno task build 2>&1 | tee frontend-build.log; then
        log_info "フロントエンドビルドが正常に完了しました"
        return 0
    else
        log_error "フロントエンドビルドが失敗しました"
        return 1
    fi
}

# Tauriビルドの実行
build_tauri() {
    log_info "Tauriビルドを開始..."
    
    # src-tauriディレクトリの確認
    if [ ! -d "src-tauri" ]; then
        log_error "src-tauriディレクトリが見つかりません"
        return 1
    fi
    
    cd src-tauri
    
    # Cargo.tomlの確認
    if [ ! -f "Cargo.toml" ]; then
        log_error "Cargo.tomlが見つかりません"
        cd ..
        return 1
    fi
    
    # tauri.conf.jsonの確認
    if [ ! -f "tauri.conf.json" ]; then
        log_error "tauri.conf.jsonが見つかりません"
        cd ..
        return 1
    fi
    
    # 依存関係のチェック
    log_info "Cargo依存関係をチェック中..."
    if ! cargo check 2>&1 | tee ../tauri-setup.log; then
        log_error "Cargo依存関係のチェックが失敗しました"
        cd ..
        return 1
    fi
    
    # ビルドの実行
    log_info "Tauriアプリケーションをビルド中..."
    if cargo tauri build --verbose 2>&1 | tee ../tauri-build.log; then
        log_info "Tauriビルドが正常に完了しました"
        cd ..
        return 0
    else
        log_error "Tauriビルドが失敗しました"
        cd ..
        return 1
    fi
}

# 成果物の確認
verify_artifacts() {
    log_info "ビルド成果物を確認中..."
    
    local dmg_files
    dmg_files=$(find src-tauri/target/release/bundle -name "*.dmg" -type f 2>/dev/null || true)
    
    if [ -z "$dmg_files" ]; then
        log_error "dmgファイルが生成されませんでした"
        
        # デバッグ情報の出力
        echo "=== バンドルディレクトリの内容 ==="
        find src-tauri/target/release/bundle -type f 2>/dev/null || echo "バンドルディレクトリが見つかりません"
        
        return 1
    fi
    
    log_info "生成されたdmgファイル:"
    echo "$dmg_files"
    
    # ファイルサイズの確認
    for dmg in $dmg_files; do
        local size
        size=$(stat -f%z "$dmg" 2>/dev/null || echo "0")
        if [ "$size" -lt 1000000 ]; then  # 1MB未満の場合は警告
            log_warning "dmgファイルのサイズが小さすぎます: $dmg ($size bytes)"
        else
            log_info "dmgファイルサイズ: $dmg ($size bytes)"
        fi
    done
    
    return 0
}

# エラー時のクリーンアップ
cleanup_on_error() {
    log_error "ビルドプロセスでエラーが発生しました"
    
    # ログファイルの収集
    mkdir -p build-logs
    
    # 各種ログファイルをコピー
    for log_file in frontend-build.log tauri-setup.log tauri-build.log; do
        if [ -f "$log_file" ]; then
            cp "$log_file" build-logs/
            log_info "ログファイルを保存しました: $log_file"
        fi
    done
    
    # システム情報を保存
    collect_system_info > build-logs/system-info.log
    
    # Cargoの詳細ログを収集
    find src-tauri/target -name "*.log" -type f -exec cp {} build-logs/ \; 2>/dev/null || true
    
    log_info "エラーログが build-logs/ ディレクトリに保存されました"
}

# メイン実行関数
main() {
    log_info "MacOSビルドプロセスを開始します"
    
    # エラー時のクリーンアップを設定
    trap cleanup_on_error ERR
    
    # システム情報の収集
    collect_system_info
    
    # 依存関係の確認
    if ! check_dependencies; then
        log_error "依存関係の確認に失敗しました"
        exit 1
    fi
    
    # フロントエンドビルド
    if ! build_frontend; then
        log_error "フロントエンドビルドに失敗しました"
        exit 1
    fi
    
    # Tauriビルド
    if ! build_tauri; then
        log_error "Tauriビルドに失敗しました"
        exit 1
    fi
    
    # 成果物の確認
    if ! verify_artifacts; then
        log_error "成果物の確認に失敗しました"
        exit 1
    fi
    
    log_info "すべてのビルドプロセスが正常に完了しました"
}

# スクリプトが直接実行された場合のみmainを呼び出し
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
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
    echo "Node.js: $(node --version 2>/dev/null || echo '未インストール')"
    echo "pnpm: $(pnpm --version 2>/dev/null || echo '未インストール')"
    
    # pnpm設定情報の表示
    if command -v pnpm &> /dev/null; then
        echo "pnpm設定:"
        pnpm config list 2>/dev/null | head -10 || echo "pnpm設定の取得に失敗"
        echo "pnpmストアパス: $(pnpm store path 2>/dev/null || echo '不明')"
    fi
    
    echo "=== システムリソース ==="
    echo "ディスク使用量:"
    df -h
    echo "メモリ使用量:"
    vm_stat 2>/dev/null || echo "メモリ情報取得失敗"
    
    echo "=== 環境変数 ==="
    env | grep -E "(RUST|CARGO|TAURI|NODE|PNPM)" | sort || echo "関連環境変数なし"
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
    
    # Node.jsとpnpmの確認
    if ! command -v node &> /dev/null; then
        missing_deps+=("node")
    fi
    
    if ! command -v pnpm &> /dev/null; then
        missing_deps+=("pnpm")
        log_error "pnpmがインストールされていません。以下のコマンドでインストールしてください:"
        log_error "npm install -g pnpm"
        log_error "または"
        log_error "curl -fsSL https://get.pnpm.io/install.sh | sh -"
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "不足している依存関係: ${missing_deps[*]}"
        return 1
    fi
    
    # pnpmのバージョン確認
    local pnpm_version
    pnpm_version=$(pnpm --version 2>/dev/null || echo "不明")
    log_info "pnpmバージョン: $pnpm_version"
    
    # 最小バージョン要件の確認（pnpm 8.0以上を推奨）
    if command -v pnpm &> /dev/null; then
        local version_major
        version_major=$(pnpm --version | cut -d. -f1)
        if [ "$version_major" -lt 8 ]; then
            log_warning "pnpmのバージョンが古い可能性があります（現在: $pnpm_version, 推奨: 8.0以上）"
        fi
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
    
    # pnpm-lock.yamlの存在確認
    if [ ! -f "pnpm-lock.yaml" ]; then
        log_error "pnpm-lock.yamlが見つかりません。pnpm installを実行してください。"
        return 1
    fi
    
    # .pnpmrcファイルの確認（存在する場合）
    if [ -f ".pnpmrc" ]; then
        log_info ".pnpmrc設定ファイルが検出されました"
        cat .pnpmrc | head -10  # 設定内容の一部を表示
    fi
    
    # pnpmストアの状態確認
    log_info "pnpmストアの状態を確認中..."
    pnpm store status 2>/dev/null || log_warning "pnpmストアの状態確認に失敗しました"
    
    # 依存関係のインストール
    log_info "pnpm依存関係をインストール中..."
    if ! pnpm install --frozen-lockfile --prefer-offline 2>&1 | tee frontend-install.log; then
        log_error "pnpm依存関係のインストールに失敗しました"
        
        # エラー詳細の出力
        log_error "pnpmインストールエラーの詳細:"
        tail -20 frontend-install.log 2>/dev/null || echo "ログファイルが見つかりません"
        
        # 代替手段の提案
        log_info "代替手段として --no-frozen-lockfile でのインストールを試行中..."
        if ! pnpm install --no-frozen-lockfile 2>&1 | tee frontend-install-fallback.log; then
            log_error "代替インストール方法も失敗しました"
            return 1
        else
            log_warning "代替インストール方法で成功しました（ロックファイルが更新された可能性があります）"
        fi
    fi
    
    # node_modulesの確認
    if [ ! -d "node_modules" ]; then
        log_error "node_modulesディレクトリが作成されませんでした"
        return 1
    fi
    
    # pnpmの特徴的なシンボリックリンク構造の確認
    if [ -d "node_modules/.pnpm" ]; then
        log_info "pnpmのシンボリックリンク構造が正常に作成されました"
    else
        log_warning "pnpmのシンボリックリンク構造が見つかりません"
    fi
    
    # TypeScriptの型チェック（ビルド前）
    log_info "TypeScript型チェックを実行中..."
    if ! pnpm run check 2>&1 | tee frontend-typecheck.log; then
        log_warning "TypeScript型チェックで警告またはエラーが発生しました"
        # 型チェックエラーでもビルドを続行（警告として扱う）
    fi
    
    # ビルドの実行
    log_info "フロントエンドビルドを実行中..."
    if pnpm run build 2>&1 | tee frontend-build.log; then
        log_info "フロントエンドビルドが正常に完了しました"
        
        # ビルド成果物の確認
        if [ -d "build" ] || [ -d "dist" ] || [ -d ".svelte-kit/output" ]; then
            log_info "ビルド成果物が正常に生成されました"
        else
            log_warning "ビルド成果物ディレクトリが見つかりません"
        fi
        
        return 0
    else
        log_error "フロントエンドビルドが失敗しました"
        
        # ビルドエラーの詳細出力
        log_error "ビルドエラーの詳細:"
        tail -30 frontend-build.log 2>/dev/null || echo "ビルドログが見つかりません"
        
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

# 署名設定の確認
check_signing_configuration() {
    log_info "署名設定を確認中..."
    
    # Tauri設定ファイルの署名設定を確認
    if [ -f "src-tauri/tauri.conf.json" ]; then
        local signing_identity
        signing_identity=$(cat src-tauri/tauri.conf.json | grep -o '"signingIdentity"[[:space:]]*:[[:space:]]*"[^"]*"' | cut -d'"' -f4 2>/dev/null || echo "null")
        
        if [ "$signing_identity" != "null" ] && [ -n "$signing_identity" ]; then
            log_info "署名設定が検出されました: $signing_identity"
            
            # 利用可能な署名IDを確認
            if command -v security &> /dev/null; then
                log_info "利用可能な署名ID:"
                security find-identity -v -p codesigning 2>/dev/null || log_warning "署名IDの確認に失敗しました"
            fi
            
            return 0
        else
            log_info "署名設定が無効化されています（開発用ビルド）"
            return 1
        fi
    else
        log_error "tauri.conf.jsonが見つかりません"
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

# 署名済み実行可能ファイルの検証
verify_signed_executable() {
    log_info "署名済み実行可能ファイルの検証を開始..."
    
    local dmg_files
    dmg_files=$(find src-tauri/target/release/bundle -name "*.dmg" -type f 2>/dev/null || true)
    
    if [ -z "$dmg_files" ]; then
        log_error "検証対象のdmgファイルが見つかりません"
        return 1
    fi
    
    local dmg_file
    dmg_file=$(echo "$dmg_files" | head -1)
    log_info "検証対象dmgファイル: $dmg_file"
    
    # dmgファイルの署名確認
    log_info "dmgファイルの署名を確認中..."
    if codesign -dv "$dmg_file" 2>/dev/null; then
        log_info "✅ dmgファイルに署名が検出されました"
        
        # 署名の検証
        if codesign --verify --deep --strict "$dmg_file" 2>/dev/null; then
            log_info "✅ dmgファイルの署名検証に成功しました"
        else
            log_warning "⚠️ dmgファイルの署名検証に失敗しました"
        fi
    else
        log_info "ℹ️ dmgファイルに署名が見つかりません（開発用ビルド）"
    fi
    
    # dmgファイル内のアプリケーションの確認
    log_info "dmgファイル内のアプリケーションを確認中..."
    
    # 一時的なマウントポイントを作成
    local mount_point="/tmp/dmg_verify_$$"
    mkdir -p "$mount_point"
    
    # dmgファイルをマウント
    if hdiutil attach "$dmg_file" -mountpoint "$mount_point" -nobrowse -quiet 2>/dev/null; then
        log_info "dmgファイルをマウントしました"
        
        # アプリケーションファイルを探す
        local app_path
        app_path=$(find "$mount_point" -name "*.app" -type d | head -1)
        
        if [ -n "$app_path" ]; then
            log_info "アプリケーション発見: $app_path"
            
            # アプリケーションの署名確認
            if codesign -dv "$app_path" 2>/dev/null; then
                log_info "✅ アプリケーションに署名が検出されました"
                
                # アプリケーション署名の検証
                if codesign --verify --deep --strict "$app_path" 2>/dev/null; then
                    log_info "✅ アプリケーションの署名検証に成功しました"
                else
                    log_warning "⚠️ アプリケーションの署名検証に失敗しました"
                fi
            else
                log_info "ℹ️ アプリケーションに署名が見つかりません（開発用ビルド）"
            fi
            
            # 実行可能ファイルの確認
            local executable_path="$app_path/Contents/MacOS"
            if [ -d "$executable_path" ]; then
                local main_executable
                main_executable=$(find "$executable_path" -type f -perm +111 | head -1)
                
                if [ -n "$main_executable" ]; then
                    log_info "✅ 実行可能ファイルが見つかりました: $main_executable"
                    
                    # ファイル情報を確認
                    file "$main_executable" 2>/dev/null || log_warning "ファイル情報の取得に失敗しました"
                    
                    return 0
                else
                    log_error "❌ 実行可能ファイルが見つかりません"
                    hdiutil detach "$mount_point" -quiet 2>/dev/null || true
                    rm -rf "$mount_point" 2>/dev/null || true
                    return 1
                fi
            else
                log_error "❌ 実行可能ファイルディレクトリが見つかりません"
                hdiutil detach "$mount_point" -quiet 2>/dev/null || true
                rm -rf "$mount_point" 2>/dev/null || true
                return 1
            fi
        else
            log_error "❌ dmg内にアプリケーションが見つかりません"
            hdiutil detach "$mount_point" -quiet 2>/dev/null || true
            rm -rf "$mount_point" 2>/dev/null || true
            return 1
        fi
        
        # dmgファイルをアンマウント
        hdiutil detach "$mount_point" -quiet 2>/dev/null || log_warning "dmgアンマウントに失敗しました"
    else
        log_error "❌ dmgファイルのマウントに失敗しました"
        rm -rf "$mount_point" 2>/dev/null || true
        return 1
    fi
    
    # 一時ディレクトリを削除
    rm -rf "$mount_point" 2>/dev/null || true
    
    log_info "署名済み実行可能ファイルの検証が完了しました"
    return 0
}

# エラー時のクリーンアップ
cleanup_on_error() {
    log_error "ビルドプロセスでエラーが発生しました"
    
    # ログファイルの収集
    mkdir -p build-logs
    
    # 各種ログファイルをコピー
    for log_file in frontend-install.log frontend-install-fallback.log frontend-typecheck.log frontend-build.log tauri-setup.log tauri-build.log; do
        if [ -f "$log_file" ]; then
            cp "$log_file" build-logs/
            log_info "ログファイルを保存しました: $log_file"
        fi
    done
    
    # pnpm関連の診断情報を収集
    if command -v pnpm &> /dev/null; then
        log_info "pnpm診断情報を収集中..."
        {
            echo "=== pnpm診断情報 ==="
            echo "pnpmバージョン: $(pnpm --version)"
            echo "pnpm設定:"
            pnpm config list 2>/dev/null || echo "設定取得失敗"
            echo "pnpmストア状態:"
            pnpm store status 2>/dev/null || echo "ストア状態取得失敗"
            echo "package.json scripts:"
            cat package.json | grep -A 20 '"scripts"' 2>/dev/null || echo "scripts取得失敗"
        } > build-logs/pnpm-diagnostics.log
    fi
    
    # システム情報を保存
    collect_system_info > build-logs/system-info.log
    
    # Cargoの詳細ログを収集
    find src-tauri/target -name "*.log" -type f -exec cp {} build-logs/ \; 2>/dev/null || true
    
    # node_modulesの状態確認
    if [ -d "node_modules" ]; then
        {
            echo "=== node_modules診断情報 ==="
            echo "node_modulesサイズ: $(du -sh node_modules 2>/dev/null || echo '不明')"
            echo "pnpmシンボリックリンク構造:"
            ls -la node_modules/.pnpm 2>/dev/null | head -10 || echo "pnpm構造なし"
        } > build-logs/node-modules-diagnostics.log
    fi
    
    log_info "エラーログが build-logs/ ディレクトリに保存されました"
    log_info "診断情報:"
    ls -la build-logs/ 2>/dev/null || echo "ログディレクトリの確認に失敗"
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
    
    # 署名設定の確認
    check_signing_configuration || log_info "署名設定は無効化されています（開発用ビルド）"
    
    # 署名済み実行可能ファイルの検証
    if ! verify_signed_executable; then
        log_error "署名済み実行可能ファイルの検証に失敗しました"
        exit 1
    fi
    
    log_info "すべてのビルドプロセスが正常に完了しました"
}

# スクリプトが直接実行された場合のみmainを呼び出し
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
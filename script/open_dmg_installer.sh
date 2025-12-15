#!/bin/bash

# DMGインストーラーを開くスクリプト
# 使用方法: ./script/open_dmg_installer.sh

set -e

# カラー出力用の定数
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ログ出力関数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# DMGファイルのパスを設定
DMG_DIR="src-tauri/target/release/bundle/dmg"
APP_NAME="orano-keihi"

log_info "DMGインストーラーを検索中..."

# DMGファイルを検索
if [ ! -d "$DMG_DIR" ]; then
    log_error "DMGディレクトリが見つかりません: $DMG_DIR"
    log_info "まず 'pnpm tauri:build:dmg' を実行してDMGファイルを生成してください"
    exit 1
fi

# 最新のDMGファイルを検索
DMG_FILE=$(find "$DMG_DIR" -name "*.dmg" -type f | head -n 1)

if [ -z "$DMG_FILE" ]; then
    log_error "DMGファイルが見つかりません"
    log_info "まず 'pnpm tauri:build:dmg' を実行してDMGファイルを生成してください"
    exit 1
fi

log_success "DMGファイルを発見: $DMG_FILE"

# DMGファイルのサイズを表示
FILE_SIZE=$(du -h "$DMG_FILE" | cut -f1)
log_info "ファイルサイズ: $FILE_SIZE"

# DMGファイルを開く
log_info "DMGインストーラーを開いています..."
open "$DMG_FILE"

log_success "DMGインストーラーが開かれました"
log_info "Finderでインストーラーが表示されます"
log_info "アプリケーションフォルダにドラッグ&ドロップしてインストールしてください"
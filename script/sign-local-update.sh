#!/bin/bash

# ローカル環境でアップデートファイルに署名するスクリプト
# 
# ⚠️ 重要な注意事項:
# このスクリプトは開発・テスト用です。
# 本番リリースではGitHub Actionsで自動的に署名されるため、
# 手動での署名は推奨されません。
# 
# GitHub Actionsで署名されたDMGファイルに追加で署名を行うと、
# ファイルが破損する可能性があります。

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${GREEN}===========================================================${NC}"
echo -e "${GREEN}ローカルアップデートファイル署名スクリプト${NC}"
echo -e "${GREEN}===========================================================${NC}"
echo ""
echo -e "${YELLOW}⚠️  注意: このスクリプトは開発・テスト用です${NC}"
echo -e "${YELLOW}   本番リリースではGitHub Actionsで自動署名されます${NC}"
echo ""

# 引数チェック
if [ $# -lt 1 ]; then
    echo -e "${RED}使用方法: $0 <ファイルのパス> [バージョン]${NC}"
    echo ""
    echo "例:"
    echo "  $0 packages/desktop/src-tauri/target/release/bundle/dmg/orano-keihi_0.1.1_aarch64.dmg v0.1.1"
    echo "  $0 packages/desktop/src-tauri/target/release/bundle/macos/orano-keihi.app.tar.gz v0.1.1"
    echo ""
    exit 1
fi

FILE_PATH="$1"
VERSION="${2:-v0.1.1}"

# ファイルの存在確認
if [ ! -f "$FILE_PATH" ]; then
    echo -e "${RED}エラー: ファイルが見つかりません: $FILE_PATH${NC}"
    exit 1
fi

# 署名鍵のパスを取得
if [ -f "packages/desktop/.env.signing" ]; then
    echo -e "${BLUE}📄 .env.signingファイルを読み込み中...${NC}"
    source packages/desktop/.env.signing
fi

SIGNING_KEY="${TAURI_SIGNING_PRIVATE_KEY:-$HOME/.tauri/orano-keihi.key}"

# パスを展開
SIGNING_KEY="${SIGNING_KEY/#\~/$HOME}"

if [ ! -f "$SIGNING_KEY" ]; then
    echo -e "${RED}エラー: 署名鍵が見つかりません: $SIGNING_KEY${NC}"
    echo ""
    echo -e "${YELLOW}署名鍵を生成するには:${NC}"
    echo "  tauri signer generate -w ~/.tauri/orano-keihi.key"
    echo ""
    exit 1
fi

echo -e "${BLUE}📝 署名情報:${NC}"
echo "  ファイル: $FILE_PATH"
echo "  バージョン: $VERSION"
echo "  署名鍵: $SIGNING_KEY"
echo ""

# ファイル名を取得
FILENAME=$(basename "$FILE_PATH")
DIRNAME=$(dirname "$FILE_PATH")

# 署名ファイルのパス
SIG_FILE="${FILE_PATH}.sig"

echo -e "${YELLOW}🔐 署名を生成中...${NC}"

# tauri signerを使用して署名
if command -v pnpm &> /dev/null; then
    cd packages/desktop
    if [ -n "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" ]; then
        pnpm tauri signer sign "../../$FILE_PATH" -f "$SIGNING_KEY" -p "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD"
    else
        pnpm tauri signer sign "../../$FILE_PATH" -f "$SIGNING_KEY"
    fi
    cd ../..
elif command -v tauri &> /dev/null; then
    if [ -n "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" ]; then
        tauri signer sign "$FILE_PATH" -f "$SIGNING_KEY" -p "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD"
    else
        tauri signer sign "$FILE_PATH" -f "$SIGNING_KEY"
    fi
else
    echo -e "${RED}エラー: tauri CLIが見つかりません${NC}"
    echo "インストールするには: cargo install tauri-cli"
    exit 1
fi

if [ ! -f "$SIG_FILE" ]; then
    echo -e "${RED}エラー: 署名ファイルの生成に失敗しました${NC}"
    exit 1
fi

echo -e "${GREEN}✓ 署名ファイルを生成しました: $SIG_FILE${NC}"
echo ""

# 署名を読み取る
SIGNATURE=$(cat "$SIG_FILE")
echo -e "${BLUE}📋 署名:${NC}"
echo "$SIGNATURE"
echo ""

# アーキテクチャを判定
if [[ "$FILENAME" == *"aarch64"* ]]; then
    ARCH="aarch64"
    TARGET="darwin"
elif [[ "$FILENAME" == *"x64"* ]] || [[ "$FILENAME" == *"x86_64"* ]]; then
    ARCH="x86_64"
    TARGET="darwin"
else
    echo -e "${RED}エラー: アーキテクチャを判定できません${NC}"
    exit 1
fi

MANIFEST_FILE="update-manifests/${TARGET}-${ARCH}.json"

echo -e "${YELLOW}📄 マニフェストファイルを更新中: $MANIFEST_FILE${NC}"

# マニフェストファイルを更新
if [ -f "$MANIFEST_FILE" ]; then
    # 一時ファイルを作成
    TMP_FILE=$(mktemp)
    
    # jqを使用してマニフェストを更新
    if command -v jq &> /dev/null; then
        jq --arg sig "$SIGNATURE" \
           --arg url "https://github.com/tsucchinoko/orano-keihi/releases/download/${VERSION}/${FILENAME}" \
           '.platforms["'${TARGET}'-'${ARCH}'"].signature = $sig | .platforms["'${TARGET}'-'${ARCH}'"].url = $url' \
           "$MANIFEST_FILE" > "$TMP_FILE"
        
        mv "$TMP_FILE" "$MANIFEST_FILE"
        echo -e "${GREEN}✓ マニフェストファイルを更新しました${NC}"
    else
        echo -e "${YELLOW}⚠ jqがインストールされていないため、手動で更新してください${NC}"
        echo ""
        echo "以下の内容を $MANIFEST_FILE に追加してください:"
        echo ""
        echo "  \"signature\": \"$SIGNATURE\","
        echo "  \"url\": \"https://github.com/tsucchinoko/orano-keihi/releases/download/${VERSION}/${FILENAME}\""
        echo ""
    fi
else
    echo -e "${RED}エラー: マニフェストファイルが見つかりません: $MANIFEST_FILE${NC}"
fi

echo ""
echo -e "${GREEN}===========================================================${NC}"
echo -e "${GREEN}署名完了！${NC}"
echo -e "${GREEN}===========================================================${NC}"
echo ""
echo -e "${BLUE}次のステップ:${NC}"
echo "  1. マニフェストファイルを確認: $MANIFEST_FILE"
echo "  2. API Serverをデプロイ"
echo "  3. アプリから「アップデートを確認」をテスト"
echo ""

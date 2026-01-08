#!/bin/bash

# GitHub Releasesにマニフェストファイルを手動でアップロードするスクリプト

set -e

# 色付きの出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}===========================================================${NC}"
echo -e "${GREEN}GitHub Releasesへのマニフェストファイルアップロード${NC}"
echo -e "${GREEN}===========================================================${NC}"
echo ""

# 引数チェック
if [ $# -lt 1 ]; then
    echo -e "${RED}エラー: リリースタグを指定してください${NC}"
    echo ""
    echo "使用方法:"
    echo "  $0 <release-tag>"
    echo ""
    echo "例:"
    echo "  $0 v0.1.1"
    echo "  $0 v0.1.1-20260107"
    exit 1
fi

RELEASE_TAG=$1
REPO="tsucchinoko/orano-keihi"
MANIFEST_DIR="update-manifests"

# GitHub CLIがインストールされているか確認
if ! command -v gh &> /dev/null; then
    echo -e "${RED}エラー: GitHub CLI (gh) がインストールされていません${NC}"
    echo ""
    echo "インストール方法:"
    echo "  macOS: brew install gh"
    echo "  その他: https://cli.github.com/manual/installation"
    exit 1
fi

# 認証確認
if ! gh auth status &> /dev/null; then
    echo -e "${YELLOW}GitHub CLIの認証が必要です${NC}"
    echo "以下のコマンドで認証してください:"
    echo "  gh auth login"
    exit 1
fi

# マニフェストディレクトリの存在確認
if [ ! -d "$MANIFEST_DIR" ]; then
    echo -e "${RED}エラー: ${MANIFEST_DIR} ディレクトリが見つかりません${NC}"
    echo "まず、マニフェストファイルを生成してください:"
    echo "  node script/generate-update-manifest.cjs"
    exit 1
fi

# リリースの存在確認
echo -e "${YELLOW}リリース ${RELEASE_TAG} の存在を確認中...${NC}"
if ! gh release view "$RELEASE_TAG" --repo "$REPO" &> /dev/null; then
    echo -e "${RED}エラー: リリース ${RELEASE_TAG} が見つかりません${NC}"
    echo ""
    echo "利用可能なリリース:"
    gh release list --repo "$REPO" --limit 5
    exit 1
fi

echo -e "${GREEN}✓ リリースが見つかりました${NC}"
echo ""

# マニフェストファイルのアップロード
echo -e "${YELLOW}マニフェストファイルをアップロード中...${NC}"
echo ""

UPLOADED_COUNT=0
FAILED_COUNT=0

for manifest_file in "$MANIFEST_DIR"/*.json; do
    if [ -f "$manifest_file" ]; then
        filename=$(basename "$manifest_file")
        echo -e "  📤 ${filename} をアップロード中..."
        
        if gh release upload "$RELEASE_TAG" "$manifest_file" --repo "$REPO" --clobber; then
            echo -e "  ${GREEN}✓ ${filename} のアップロードが完了しました${NC}"
            UPLOADED_COUNT=$((UPLOADED_COUNT + 1))
        else
            echo -e "  ${RED}✗ ${filename} のアップロードに失敗しました${NC}"
            FAILED_COUNT=$((FAILED_COUNT + 1))
        fi
        echo ""
    fi
done

# 結果サマリー
echo -e "${GREEN}===========================================================${NC}"
echo -e "${GREEN}アップロード完了${NC}"
echo -e "${GREEN}===========================================================${NC}"
echo ""
echo -e "成功: ${GREEN}${UPLOADED_COUNT}${NC} ファイル"
if [ $FAILED_COUNT -gt 0 ]; then
    echo -e "失敗: ${RED}${FAILED_COUNT}${NC} ファイル"
fi
echo ""

# リリースURLの表示
RELEASE_URL="https://github.com/${REPO}/releases/tag/${RELEASE_TAG}"
echo -e "リリースURL: ${RELEASE_URL}"
echo ""

# アップロードされたファイルの確認
echo -e "${YELLOW}アップロードされたファイルを確認中...${NC}"
gh release view "$RELEASE_TAG" --repo "$REPO" --json assets --jq '.assets[] | select(.name | endswith(".json")) | "  - \(.name) (\(.size / 1024 | floor) KB)"'
echo ""

if [ $FAILED_COUNT -eq 0 ]; then
    echo -e "${GREEN}🎉 すべてのマニフェストファイルが正常にアップロードされました！${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  一部のファイルのアップロードに失敗しました${NC}"
    exit 1
fi

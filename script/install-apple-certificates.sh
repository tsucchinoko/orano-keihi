#!/bin/bash

# Apple開発者証明書チェーンをインストールするスクリプト

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${GREEN}===========================================================${NC}"
echo -e "${GREEN}Apple証明書チェーンインストールスクリプト${NC}"
echo -e "${GREEN}===========================================================${NC}"
echo ""

echo -e "${BLUE}このスクリプトは以下の証明書をインストールします:${NC}"
echo "  1. Apple Inc. Root Certificate"
echo "  2. Developer ID Certification Authority (G2)"
echo ""
echo -e "${YELLOW}管理者パスワードの入力が必要です${NC}"
echo ""

# 一時ディレクトリを作成
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

echo -e "${YELLOW}📥 証明書をダウンロード中...${NC}"

# Apple Root CA
echo "  - Apple Inc. Root Certificate"
curl -sO https://www.apple.com/appleca/AppleIncRootCertificate.cer

# Developer ID G2
echo "  - Developer ID Certification Authority (G2)"
curl -sO https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer

echo -e "${GREEN}✅ ダウンロード完了${NC}"
echo ""

echo -e "${YELLOW}📦 証明書をインストール中...${NC}"

# Apple Root CA
echo "  - Apple Inc. Root Certificate"
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain AppleIncRootCertificate.cer

# Developer ID G2
echo "  - Developer ID Certification Authority (G2)"
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain DeveloperIDG2CA.cer

echo -e "${GREEN}✅ インストール完了${NC}"
echo ""

# 一時ファイルを削除
cd -
rm -rf "$TEMP_DIR"

echo -e "${BLUE}🔍 インストールされた証明書を確認中...${NC}"

if security find-certificate -c "Apple Root CA" -a | grep -q "labl"; then
    echo -e "${GREEN}  ✅ Apple Root CA: インストール済み${NC}"
else
    echo -e "${YELLOW}  ⚠️  Apple Root CA: 見つかりません${NC}"
fi

if security find-certificate -c "Developer ID Certification Authority" -a | grep -q "labl"; then
    echo -e "${GREEN}  ✅ Developer ID CA: インストール済み${NC}"
else
    echo -e "${YELLOW}  ⚠️  Developer ID CA: 見つかりません${NC}"
fi

echo ""
echo -e "${GREEN}===========================================================${NC}"
echo -e "${GREEN}完了！${NC}"
echo -e "${GREEN}===========================================================${NC}"
echo ""
echo -e "${BLUE}次のステップ:${NC}"
echo "  ビルドを実行してください:"
echo "  ./script/build-and-sign-local.sh v0.1.2"
echo ""

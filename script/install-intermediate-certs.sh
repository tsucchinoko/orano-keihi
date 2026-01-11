#!/bin/bash

# Apple中間証明書をインストールするスクリプト

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}===========================================================${NC}"
echo -e "${BLUE}Apple中間証明書のインストール${NC}"
echo -e "${BLUE}===========================================================${NC}"
echo ""

# 一時ディレクトリを作成
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

echo -e "${YELLOW}1. 中間証明書をダウンロード中...${NC}"

# Developer ID Certification Authority
echo -e "${BLUE}   Developer ID Certification Authority をダウンロード中...${NC}"
curl -o DeveloperIDG2CA.cer "https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer"

# Apple Worldwide Developer Relations Certification Authority
echo -e "${BLUE}   Apple Worldwide Developer Relations CA をダウンロード中...${NC}"
curl -o AppleWWDRCAG3.cer "https://www.apple.com/certificateauthority/AppleWWDRCAG3.cer"

echo ""
echo -e "${YELLOW}2. 証明書をキーチェーンにインストール中...${NC}"

# システムキーチェーンにインストール
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain DeveloperIDG2CA.cer
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain AppleWWDRCAG3.cer

echo ""
echo -e "${GREEN}✅ 中間証明書のインストールが完了しました${NC}"
echo ""

# クリーンアップ
cd -
rm -rf "$TEMP_DIR"

echo -e "${YELLOW}確認:${NC}"
security find-certificate -c "Developer ID Certification Authority"
security find-certificate -c "Apple Worldwide Developer Relations Certification Authority"

echo ""
echo -e "${GREEN}===========================================================${NC}"
echo -e "${GREEN}インストール完了！${NC}"
echo -e "${GREEN}===========================================================${NC}"
echo ""
echo "次のステップ:"
echo "1. Xcodeを再起動（開いている場合）"
echo "2. 再度ビルドを実行"
echo ""

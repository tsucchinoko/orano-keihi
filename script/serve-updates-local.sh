#!/bin/bash

# ローカルでアップデートサーバーを起動するスクリプト（テスト用）

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${GREEN}===========================================================${NC}"
echo -e "${GREEN}ローカルアップデートサーバーを起動${NC}"
echo -e "${GREEN}===========================================================${NC}"
echo ""

PORT=8080

echo -e "${YELLOW}📝 マニフェストファイルを配信中...${NC}"
echo -e "${BLUE}URL: http://localhost:${PORT}${NC}"
echo ""
echo -e "マニフェストファイル:"
echo -e "  - http://localhost:${PORT}/darwin-aarch64.json"
echo -e "  - http://localhost:${PORT}/darwin-x86_64.json"
echo -e "  - http://localhost:${PORT}/windows-x86_64.json"
echo ""
echo -e "${YELLOW}Ctrl+C で停止${NC}"
echo ""

# Pythonの簡易HTTPサーバーを使用
cd update-manifests
python3 -m http.server $PORT

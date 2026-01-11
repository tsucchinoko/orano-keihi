#!/bin/bash

# キーチェーンアクセス権限を修正するスクリプト

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}===========================================================${NC}"
echo -e "${BLUE}キーチェーンアクセス権限の修正${NC}"
echo -e "${BLUE}===========================================================${NC}"
echo ""

# 1. キーチェーンをロック解除
echo -e "${YELLOW}1. キーチェーンをロック解除${NC}"
security unlock-keychain login.keychain

# 2. 秘密鍵のアクセス制御を確認
echo ""
echo -e "${YELLOW}2. 証明書と秘密鍵の状態を確認${NC}"
security find-identity -v -p codesigning

# 3. codesignツールにアクセス許可を与える
echo ""
echo -e "${YELLOW}3. codesignへのアクセス許可を設定${NC}"
echo -e "${BLUE}   キーチェーンアクセス.appが開きます${NC}"
echo -e "${BLUE}   以下の手順を実行してください:${NC}"
echo ""
echo "   1. 左側で「ログイン」キーチェーンを選択"
echo "   2. 「自分の証明書」カテゴリを選択"
echo "   3. 「Developer ID Application: Daichi Tsuchiya」を見つける"
echo "   4. その下の鍵アイコン（秘密鍵）をダブルクリック"
echo "   5. 「アクセス制御」タブを選択"
echo "   6. 「すべてのアプリケーションにこの項目へのアクセスを許可」を選択"
echo "   7. または「以下のアプリケーションにこの項目へのアクセスを許可」を選択して"
echo "      「+」ボタンから /usr/bin/codesign を追加"
echo ""

# キーチェーンアクセスを開く
open -a "Keychain Access"

echo ""
echo -e "${YELLOW}上記の設定を完了したら、Enterキーを押してください...${NC}"
read -r

echo ""
echo -e "${GREEN}✅ キーチェーン設定が完了しました${NC}"
echo ""
echo -e "${YELLOW}次のステップ:${NC}"
echo "1. ビルドを再実行してください"
echo "2. それでも失敗する場合は、次のコマンドを試してください:"
echo "   security set-key-partition-list -S apple-tool:,apple: -s -k \"\$USER_PASSWORD\" ~/Library/Keychains/login.keychain-db"
echo ""

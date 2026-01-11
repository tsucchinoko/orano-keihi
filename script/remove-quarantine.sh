#!/bin/bash

# ダウンロードしたDMGファイルから検疫属性を削除するスクリプト
# テスト環境でのみ使用してください

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${YELLOW}===========================================================${NC}"
echo -e "${YELLOW}検疫属性削除スクリプト (テスト用)${NC}"
echo -e "${YELLOW}===========================================================${NC}"
echo ""

# 引数チェック
if [ $# -lt 1 ]; then
    echo -e "${RED}使用方法: $0 <DMGファイルパス>${NC}"
    echo ""
    echo "例:"
    echo "  $0 ~/Downloads/orano-keihi_0.1.1_aarch64.dmg"
    echo ""
    echo -e "${YELLOW}注意: このスクリプトはテスト環境でのみ使用してください${NC}"
    echo -e "${YELLOW}本番環境では、Apple Developer証明書で署名する必要があります${NC}"
    echo ""
    exit 1
fi

DMG_FILE="$1"

# ファイルの存在確認
if [ ! -f "$DMG_FILE" ]; then
    echo -e "${RED}エラー: ファイルが見つかりません: $DMG_FILE${NC}"
    exit 1
fi

echo -e "${BLUE}📋 対象ファイル:${NC}"
echo "  $DMG_FILE"
echo ""

# 現在の拡張属性を表示
echo -e "${BLUE}🔍 現在の拡張属性:${NC}"
if xattr "$DMG_FILE" 2>/dev/null; then
    HAS_QUARANTINE=$(xattr "$DMG_FILE" 2>/dev/null | grep -c "com.apple.quarantine" || true)

    if [ "$HAS_QUARANTINE" -gt 0 ]; then
        echo ""
        echo -e "${YELLOW}⚠️  com.apple.quarantine 属性が検出されました${NC}"
        echo ""

        # 検疫属性を削除
        echo -e "${YELLOW}🔓 検疫属性を削除中...${NC}"
        xattr -d com.apple.quarantine "$DMG_FILE"

        echo -e "${GREEN}✅ 検疫属性を削除しました${NC}"
        echo ""

        # 削除後の拡張属性を表示
        echo -e "${BLUE}🔍 削除後の拡張属性:${NC}"
        if xattr "$DMG_FILE" 2>/dev/null | grep -q "com.apple.quarantine"; then
            echo -e "${RED}⚠️  検疫属性がまだ残っています${NC}"
        else
            echo -e "${GREEN}✅ 検疫属性が削除されました${NC}"
        fi

        echo ""
        echo -e "${GREEN}===========================================================${NC}"
        echo -e "${GREEN}処理完了！${NC}"
        echo -e "${GREEN}===========================================================${NC}"
        echo ""
        echo -e "${BLUE}次のステップ:${NC}"
        echo "  1. DMGファイルをダブルクリックしてマウント"
        echo "  2. アプリケーションフォルダにドラッグ&ドロップ"
        echo ""
    else
        echo -e "${GREEN}✅ com.apple.quarantine 属性は付いていません${NC}"
        echo ""
    fi
else
    echo -e "${GREEN}✅ 拡張属性はありません${NC}"
    echo ""
fi

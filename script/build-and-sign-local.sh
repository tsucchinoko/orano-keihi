#!/bin/bash

# ローカル環境でビルド・署名・マニフェスト生成を行うスクリプト
# 手動でのテストリリース作成用

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${GREEN}===========================================================${NC}"
echo -e "${GREEN}ローカルビルド・署名・マニフェスト生成スクリプト${NC}"
echo -e "${GREEN}===========================================================${NC}"
echo ""

# 引数チェック
if [ $# -lt 1 ]; then
    echo -e "${RED}使用方法: $0 <バージョン> [プラットフォーム]${NC}"
    echo ""
    echo "例:"
    echo "  $0 v0.1.1                    # 全プラットフォーム"
    echo "  $0 v0.1.1 macos             # macOSのみ"
    echo "  $0 v0.1.1 windows           # Windowsのみ"
    echo ""
    exit 1
fi

VERSION="$1"
PLATFORM="${2:-all}"

# バージョンの検証
if [[ ! "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}エラー: バージョンはv0.1.0形式で指定してください${NC}"
    exit 1
fi

echo -e "${BLUE}📋 ビルド設定:${NC}"
echo "  バージョン: $VERSION"
echo "  プラットフォーム: $PLATFORM"
echo ""

# 環境変数を読み込み
if [ -f "script/load-env.sh" ]; then
    echo -e "${BLUE}📄 環境変数を読み込み中: script/load-env.sh${NC}"
    source script/load-env.sh
else
    echo -e "${RED}エラー: script/load-env.sh が見つかりません${NC}"
    echo -e "${YELLOW}script/load-env.example.sh をコピーして作成してください:${NC}"
    echo "  cp script/load-env.example.sh script/load-env.sh"
    echo ""
    exit 1
fi

# 署名鍵のパスを取得
if [ -f "packages/desktop/.env.signing" ]; then
    echo -e "${BLUE}📄 .env.signingファイルを読み込み中...${NC}"
    source packages/desktop/.env.signing
fi

SIGNING_KEY="${TAURI_SIGNING_PRIVATE_KEY:-$HOME/.tauri/orano-keihi.key}"
SIGNING_KEY="${SIGNING_KEY/#\~/$HOME}"

if [ ! -f "$SIGNING_KEY" ]; then
    echo -e "${RED}エラー: 署名鍵が見つかりません: $SIGNING_KEY${NC}"
    echo ""
    echo -e "${YELLOW}署名鍵を生成するには:${NC}"
    echo "  pnpm tauri signer generate -w ~/.tauri/orano-keihi.key"
    echo ""
    exit 1
fi

echo -e "${BLUE}🔑 署名鍵: $SIGNING_KEY${NC}"
echo ""

# 依存関係のインストール
echo -e "${YELLOW}📦 依存関係をインストール中...${NC}"
pnpm install

# フロントエンドビルド
echo -e "${YELLOW}🏗️  フロントエンドをビルド中...${NC}"
pnpm build

# プラットフォーム別ビルド
build_macos() {
    echo -e "${YELLOW}🍎 macOSアプリケーションをビルド中...${NC}"

    cd packages/desktop

    # 環境変数を設定してビルド
    ENVIRONMENT=production pnpm tauri:build:dmg:signed

    cd ../..

    # ビルド結果の確認
    DMG_FILE=$(find packages/desktop/src-tauri/target/release/bundle/dmg -name "*.dmg" | head -n 1)

    if [ -z "$DMG_FILE" ]; then
        echo -e "${RED}エラー: DMGファイルが見つかりません${NC}"
        return 1
    fi

    echo -e "${GREEN}✅ macOSビルド完了: $DMG_FILE${NC}"

    # DMGファイルのチェックサムを記録（署名前）
    echo -e "${BLUE}📋 DMGファイルのチェックサムを確認中...${NC}"
    CHECKSUM_BEFORE=$(shasum -a 256 "$DMG_FILE" | awk '{print $1}')
    echo -e "${BLUE}  署名前: $CHECKSUM_BEFORE${NC}"

    # Tauri署名を生成
    echo -e "${YELLOW}🔐 Tauri署名を生成中...${NC}"

    cd packages/desktop
    if [ -n "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" ]; then
        pnpm tauri signer sign "../../$DMG_FILE" -f "$SIGNING_KEY" -p "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD"
    else
        pnpm tauri signer sign "../../$DMG_FILE" -f "$SIGNING_KEY"
    fi
    cd ../..

    # DMGファイルのチェックサムを確認（署名後）
    CHECKSUM_AFTER=$(shasum -a 256 "$DMG_FILE" | awk '{print $1}')
    echo -e "${BLUE}  署名後: $CHECKSUM_AFTER${NC}"

    if [ "$CHECKSUM_BEFORE" != "$CHECKSUM_AFTER" ]; then
        echo -e "${RED}⚠️  警告: DMGファイルが署名処理によって変更されました${NC}"
    else
        echo -e "${GREEN}✅ DMGファイルは変更されていません${NC}"
    fi

    SIG_FILE="${DMG_FILE}.sig"
    if [ -f "$SIG_FILE" ]; then
        echo -e "${GREEN}✅ 署名ファイル生成完了: $SIG_FILE${NC}"
    else
        echo -e "${RED}エラー: 署名ファイルの生成に失敗しました${NC}"
        return 1
    fi

    # ファイル情報を表示
    echo -e "${BLUE}📦 生成されたファイル:${NC}"
    ls -lh "$DMG_FILE"
    ls -lh "$SIG_FILE"

    # DMGファイルの検証
    echo -e "${BLUE}🔍 DMGファイルを検証中...${NC}"
    if hdiutil verify "$DMG_FILE" 2>&1 | grep -q "VALID"; then
        echo -e "${GREEN}✅ DMGファイルは正常です${NC}"
    else
        echo -e "${RED}⚠️  警告: DMGファイルの検証に問題があります${NC}"
    fi

    # Apple署名の確認
    echo -e "${BLUE}🔍 Apple署名を確認中...${NC}"
    if codesign -dv "$DMG_FILE" 2>&1 | grep -q "Signature"; then
        echo -e "${GREEN}✅ Apple署名が見つかりました${NC}"
        codesign -dv "$DMG_FILE" 2>&1 | head -5
    else
        echo -e "${YELLOW}⚠️  Apple Developer署名がありません${NC}"
        echo -e "${YELLOW}   本番環境では、Apple Developer証明書での署名が必要です${NC}"
        echo -e "${YELLOW}   テスト環境では、以下のコマンドで検疫属性を削除できます:${NC}"
        echo -e "${YELLOW}   xattr -d com.apple.quarantine <ダウンロードしたDMGファイル>${NC}"
    fi

    return 0
}

build_windows() {
    echo -e "${YELLOW}🪟 Windowsアプリケーションをビルド中...${NC}"

    cd packages/desktop

    # 環境変数を設定してビルド
    ENVIRONMENT=production pnpm tauri build --bundles msi

    cd ../..

    # ビルド結果の確認
    MSI_FILE=$(find packages/desktop/src-tauri/target/release/bundle/msi -name "*.msi" | head -n 1)

    if [ -z "$MSI_FILE" ]; then
        echo -e "${RED}エラー: MSIファイルが見つかりません${NC}"
        return 1
    fi

    echo -e "${GREEN}✅ Windowsビルド完了: $MSI_FILE${NC}"

    # ファイル情報を表示
    echo -e "${BLUE}📦 生成されたファイル:${NC}"
    ls -lh "$MSI_FILE"

    return 0
}

# プラットフォーム別実行
case "$PLATFORM" in
    "macos")
        build_macos
        ;;
    # "windows")
        # build_windows
        # ;;
    "all")
        echo -e "${BLUE}🔄 全プラットフォームをビルド中...${NC}"
        build_macos
        echo ""
        # build_windows
        ;;
    *)
        echo -e "${RED}エラー: 不明なプラットフォーム: $PLATFORM${NC}"
        echo "サポートされているプラットフォーム: macos, windows, all"
        exit 1
        ;;
esac

echo ""

# マニフェストファイルの生成
echo -e "${YELLOW}📄 マニフェストファイルを生成中...${NC}"

export VERSION="${VERSION#v}"  # v プレフィックスを削除
export RELEASE_TAG="$VERSION"
export RELEASE_NOTES="ローカルテストビルド $VERSION"
export GITHUB_REPOSITORY="tsucchinoko/orano-keihi"

node script/generate-update-manifest.cjs

if [ -d "update-manifests" ]; then
    echo -e "${GREEN}✅ マニフェストファイル生成完了${NC}"
    echo -e "${BLUE}📁 生成されたマニフェストファイル:${NC}"
    ls -la update-manifests/
else
    echo -e "${RED}エラー: マニフェストファイルの生成に失敗しました${NC}"
fi

echo ""
echo -e "${GREEN}===========================================================${NC}"
echo -e "${GREEN}ローカルビルド完了！${NC}"
echo -e "${GREEN}===========================================================${NC}"
echo ""
echo -e "${BLUE}次のステップ:${NC}"
echo "  1. 生成されたファイルを確認"
echo "  2. 手動でGitHubリリースを作成"
echo "  3. ファイルをアップロード"
echo "  4. script/upload-to-github.sh でアップロード（推奨）"
echo ""

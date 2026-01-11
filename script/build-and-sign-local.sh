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

# Apple Developer証明書の環境変数をチェック（macOSビルド用）
if [[ "$PLATFORM" == "macos" || "$PLATFORM" == "all" ]]; then
    if [ -z "$APPLE_CERTIFICATE" ]; then
        echo -e "${YELLOW}⚠️  警告: APPLE_CERTIFICATE環境変数が設定されていません${NC}"
        echo -e "${YELLOW}   Apple Developer証明書による署名をスキップします${NC}"
        echo ""
    else
        echo -e "${BLUE}🍎 Apple Developer証明書: 検出されました${NC}"

        # 証明書パスワードのチェック
        if [ -z "$APPLE_CERTIFICATE_PASSWORD" ]; then
            echo -e "${RED}エラー: APPLE_CERTIFICATE_PASSWORD環境変数が設定されていません${NC}"
            exit 1
        fi

        # キーチェーンパスワードのデフォルト値を設定
        KEYCHAIN_PASSWORD="${KEYCHAIN_PASSWORD:-build-keychain-password}"
    fi
fi

echo ""

# 依存関係のインストール
echo -e "${YELLOW}📦 依存関係をインストール中...${NC}"
pnpm install

# フロントエンドビルド
echo -e "${YELLOW}🏗️  フロントエンドをビルド中...${NC}"
pnpm build

# Apple Developer証明書のセットアップ（macOS用）
setup_macos_signing() {
    echo -e "${YELLOW}🔐 Apple Developer証明書をセットアップ中...${NC}"

    # 新しいキーチェーンを作成
    security create-keychain -p "$KEYCHAIN_PASSWORD" build.keychain 2>/dev/null || true
    security default-keychain -s build.keychain
    security unlock-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
    security set-keychain-settings -t 3600 -u build.keychain

    # loginキーチェーンもキーチェーン検索パスに追加（証明書チェーン用）
    # 注: loginキーチェーンは通常既にアンロックされています
    security list-keychains -d user -s build.keychain login.keychain

    echo -e "${BLUE}💳 login.keychainから証明書を使用します${NC}"

    # Apple中間証明書とルート証明書をダウンロードしてインポート
    echo -e "${BLUE}📥 Apple中間証明書をダウンロード中...${NC}"
    curl -s https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer -o DeveloperIDG2CA.cer
    curl -s https://www.apple.com/certificateauthority/AppleWWDRCAG3.cer -o AppleWWDRCAG3.cer

    security import DeveloperIDG2CA.cer -k build.keychain -T /usr/bin/codesign -A
    security import AppleWWDRCAG3.cer -k build.keychain -T /usr/bin/codesign -A

    # 一時ファイルを削除
    rm DeveloperIDG2CA.cer AppleWWDRCAG3.cer 2>/dev/null || true

    # build.keychainの中間証明書にアクセス許可を設定
    security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KEYCHAIN_PASSWORD" build.keychain 2>/dev/null || true

    # 利用可能な証明書を確認
    echo -e "${BLUE}📋 利用可能な証明書:${NC}"
    security find-identity -v -p codesigning

    echo -e "${GREEN}✅ Apple Developer証明書のセットアップが完了しました${NC}"
    echo ""

    # 証明書情報を取得（全キーチェーンから）
    echo -e "${YELLOW}🔍 証明書情報を確認中...${NC}"
    CERT_INFO=$(security find-identity -v -p codesigning | grep "Developer ID Application")

    if [ -z "$CERT_INFO" ]; then
        echo -e "${RED}❌ 有効な証明書が見つかりません${NC}"
        return 1
    fi

    # 証明書IDを抽出
    export CERT_ID=$(echo "$CERT_INFO" | head -n 1 | awk -F'"' '{print $2}')
    export APPLE_SIGNING_IDENTITY="$CERT_ID"
    echo -e "${GREEN}✅ 証明書が確認されました: $CERT_ID${NC}"
    echo ""
}

# Apple Developer証明書のクリーンアップ
cleanup_macos_signing() {
    if [ -n "$APPLE_CERTIFICATE" ]; then
        echo -e "${YELLOW}🧹 署名環境をクリーンアップ中...${NC}"

        # build.keychainを削除
        security delete-keychain build.keychain 2>/dev/null || true

        # デフォルトキーチェーンを復元
        security default-keychain -s login.keychain 2>/dev/null || true

        echo -e "${GREEN}✅ クリーンアップ完了${NC}"
    fi
}

# プラットフォーム別ビルド
build_macos() {
    echo -e "${YELLOW}🍎 macOSアプリケーションをビルド中...${NC}"

    # 証明書のセットアップ
    setup_macos_signing

    cd packages/desktop

    # 環境変数を設定してビルド
    if [ -n "$CERT_ID" ]; then
        echo -e "${BLUE}🔏 Apple Developer証明書で署名してビルド: $CERT_ID${NC}"
        echo -e "${BLUE}   APPLE_SIGNING_IDENTITY=$APPLE_SIGNING_IDENTITY${NC}"
        ENVIRONMENT=production \
        APPLE_SIGNING_IDENTITY="$APPLE_SIGNING_IDENTITY" \
        pnpm tauri:build:dmg:signed
    else
        echo -e "${YELLOW}⚠️  Apple Developer証明書なしでビルド${NC}"
        ENVIRONMENT=production pnpm tauri:build:dmg:signed
    fi

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

# エラー時のクリーンアップを確実に実行するためのtrap設定
trap cleanup_macos_signing EXIT

# プラットフォーム別実行
case "$PLATFORM" in
    "macos")
        build_macos
        BUILD_RESULT=$?
        ;;
    # "windows")
        # build_windows
        # BUILD_RESULT=$?
        # ;;
    "all")
        echo -e "${BLUE}🔄 全プラットフォームをビルド中...${NC}"
        build_macos
        BUILD_RESULT=$?
        echo ""
        # build_windows
        ;;
    *)
        echo -e "${RED}エラー: 不明なプラットフォーム: $PLATFORM${NC}"
        echo "サポートされているプラットフォーム: macos, windows, all"
        cleanup_macos_signing
        exit 1
        ;;
esac

# ビルドが失敗した場合は終了
if [ $BUILD_RESULT -ne 0 ]; then
    echo -e "${RED}❌ ビルドが失敗しました${NC}"
    exit $BUILD_RESULT
fi

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

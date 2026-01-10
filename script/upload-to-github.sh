#!/bin/bash

# ローカルでビルドしたファイルをGitHubリリースにアップロードするスクリプト

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${GREEN}===========================================================${NC}"
echo -e "${GREEN}GitHubリリース手動アップロードスクリプト${NC}"
echo -e "${GREEN}===========================================================${NC}"
echo ""

# 引数チェック
if [ $# -lt 1 ]; then
    echo -e "${RED}使用方法: $0 <バージョン> [説明]${NC}"
    echo ""
    echo "例:"
    echo "  $0 v0.1.1-test \"テスト用リリース\""
    echo ""
    exit 1
fi

VERSION="$1"
DESCRIPTION="${2:-ローカルテストビルド}"

# 必要なコマンドの確認
if ! command -v gh &> /dev/null; then
    echo -e "${RED}エラー: GitHub CLI (gh) がインストールされていません${NC}"
    echo ""
    echo -e "${YELLOW}インストール方法:${NC}"
    echo "  brew install gh"
    echo "  または https://cli.github.com/ からダウンロード"
    echo ""
    exit 1
fi

# GitHub CLIの認証確認
echo -e "${BLUE}🔐 GitHub CLI認証を確認中...${NC}"

if ! gh auth status &> /dev/null; then
    echo -e "${RED}エラー: GitHub CLIの認証が必要です${NC}"
    echo ""
    echo -e "${YELLOW}以下のコマンドで認証してください:${NC}"
    echo "  gh auth login"
    echo ""
    exit 1
fi

echo -e "${GREEN}✅ GitHub CLI認証確認完了${NC}"
echo ""

# リポジトリ情報
REPO="tsucchinoko/orano-keihi"

echo -e "${BLUE}📋 アップロード設定:${NC}"
echo "  バージョン: $VERSION"
echo "  リポジトリ: $REPO"
echo "  説明: $DESCRIPTION"
echo ""

# ビルドファイルの検索
echo -e "${YELLOW}📦 ビルドファイルを検索中...${NC}"

# macOSファイル
DMG_FILES=($(find packages/desktop/src-tauri/target/release/bundle/dmg -name "*.dmg" 2>/dev/null || true))
SIG_FILES=($(find packages/desktop/src-tauri/target/release/bundle/dmg -name "*.dmg.sig" 2>/dev/null || true))

# Windowsファイル
MSI_FILES=($(find packages/desktop/src-tauri/target/release/bundle/msi -name "*.msi" 2>/dev/null || true))

# マニフェストファイル（"copy"という名前のファイルを除外）
MANIFEST_FILES=($(find update-manifests -type f -name "*.json" ! -name "*copy*" 2>/dev/null || true))

# ファイル存在確認
UPLOAD_FILES=()

if [ ${#DMG_FILES[@]} -gt 0 ]; then
    echo -e "${GREEN}  ✅ macOS DMG: ${DMG_FILES[0]}${NC}"
    UPLOAD_FILES+=("${DMG_FILES[0]}")
else
    echo -e "${YELLOW}  ⚠️  macOS DMGファイルが見つかりません${NC}"
fi

if [ ${#SIG_FILES[@]} -gt 0 ]; then
    echo -e "${GREEN}  ✅ macOS署名: ${SIG_FILES[0]}${NC}"
    UPLOAD_FILES+=("${SIG_FILES[0]}")
else
    echo -e "${YELLOW}  ⚠️  macOS署名ファイルが見つかりません${NC}"
fi

if [ ${#MSI_FILES[@]} -gt 0 ]; then
    echo -e "${GREEN}  ✅ Windows MSI: ${MSI_FILES[0]}${NC}"
    UPLOAD_FILES+=("${MSI_FILES[0]}")
else
    echo -e "${YELLOW}  ⚠️  Windows MSIファイルが見つかりません${NC}"
fi

if [ ${#MANIFEST_FILES[@]} -gt 0 ]; then
    echo -e "${GREEN}  ✅ マニフェスト: ${#MANIFEST_FILES[@]}個のファイル${NC}"
    UPLOAD_FILES+=("${MANIFEST_FILES[@]}")
else
    echo -e "${YELLOW}  ⚠️  マニフェストファイルが見つかりません${NC}"
fi

if [ ${#UPLOAD_FILES[@]} -eq 0 ]; then
    echo -e "${RED}エラー: アップロードするファイルが見つかりません${NC}"
    echo ""
    echo -e "${YELLOW}まず以下のコマンドでビルドを実行してください:${NC}"
    echo "  ./script/build-and-sign-local.sh $VERSION"
    echo ""
    exit 1
fi

echo ""
echo -e "${BLUE}📤 アップロード予定ファイル (${#UPLOAD_FILES[@]}個):${NC}"
for file in "${UPLOAD_FILES[@]}"; do
    size=$(ls -lh "$file" | awk '{print $5}')
    filename=$(basename "$file")
    echo "  - $filename ($size)"

    # DMGファイルの場合はチェックサムも表示
    if [[ "$filename" == *.dmg ]]; then
        checksum=$(shasum -a 256 "$file" | awk '{print $1}')
        echo "    SHA256: $checksum"
    fi
done

echo ""
read -p "アップロードを実行しますか？ (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}アップロードをキャンセルしました${NC}"
    exit 0
fi

# リリースの作成または確認
echo -e "${YELLOW}📝 GitHubリリースを確認中...${NC}"

# 既存のリリースを確認
if gh release view "$VERSION" --repo "$REPO" &> /dev/null; then
    echo -e "${BLUE}既存のリリースが見つかりました: $VERSION${NC}"
    read -p "既存のリリースにファイルを追加しますか？ (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}アップロードをキャンセルしました${NC}"
        exit 0
    fi
else
    echo -e "${YELLOW}新しいリリースを作成中...${NC}"

    # リリースノートの生成
    RELEASE_NOTES="## 🧪 テスト用リリース: $VERSION

### 📦 ダウンロード

このリリースはローカル環境でビルドされたテスト用です。

### 📋 含まれるファイル"

    if [ ${#DMG_FILES[@]} -gt 0 ]; then
        RELEASE_NOTES="$RELEASE_NOTES
- **macOS**: $(basename "${DMG_FILES[0]}")"
    fi

    if [ ${#MSI_FILES[@]} -gt 0 ]; then
        RELEASE_NOTES="$RELEASE_NOTES
- **Windows**: $(basename "${MSI_FILES[0]}")"
    fi

    if [ ${#MANIFEST_FILES[@]} -gt 0 ]; then
        RELEASE_NOTES="$RELEASE_NOTES
- **マニフェスト**: ${#MANIFEST_FILES[@]}個のファイル"
    fi

    RELEASE_NOTES="$RELEASE_NOTES

### ⚠️ 注意事項

- これはテスト用のリリースです
- 本番環境での使用は推奨されません
- 自動アップデート機能をテストする場合は、マニフェストファイルをAPI Serverにデプロイしてください

### 🔧 説明

$DESCRIPTION"

    # リリースを作成
    gh release create "$VERSION" \
        --repo "$REPO" \
        --title "テスト用リリース $VERSION" \
        --notes "$RELEASE_NOTES" \
        --prerelease

    echo -e "${GREEN}✅ リリースを作成しました${NC}"
fi

# マニフェストファイルの署名を更新
if [ ${#MANIFEST_FILES[@]} -gt 0 ] && [ ${#SIG_FILES[@]} -gt 0 ]; then
    echo -e "${YELLOW}🔐 マニフェストファイルの署名を更新中...${NC}"

    # 署名ファイルから署名を読み取る
    SIGNATURE=$(cat "${SIG_FILES[0]}")

    if [ -n "$SIGNATURE" ]; then
        echo -e "${BLUE}  署名を取得しました (${#SIGNATURE}文字)${NC}"

        # 各マニフェストファイルのPLACEHOLDERを実際の署名に置き換え
        for manifest in "${MANIFEST_FILES[@]}"; do
            # ファイルが存在するか確認
            if [ ! -f "$manifest" ]; then
                echo -e "${YELLOW}  ⚠️  ファイルが見つかりません: $manifest${NC}"
                continue
            fi

            manifest_basename=$(basename "$manifest")
            echo -e "${BLUE}  更新中: $manifest_basename${NC}"

            # jqを使ってJSON内のsignatureフィールドを更新
            if command -v jq &> /dev/null; then
                # 各プラットフォームのsignatureを更新
                tmp_file=$(mktemp)
                if jq --arg sig "$SIGNATURE" '
                    .platforms |= with_entries(
                        .value.signature = $sig
                    )
                ' "$manifest" > "$tmp_file" 2>/dev/null; then
                    mv "$tmp_file" "$manifest"
                    echo -e "${GREEN}    ✅ 署名を更新しました${NC}"
                else
                    rm -f "$tmp_file"
                    echo -e "${RED}    ⚠️  署名の更新に失敗しました${NC}"
                fi
            else
                echo -e "${RED}    ⚠️  jqがインストールされていません。署名の更新をスキップします${NC}"
                echo -e "${YELLOW}    jqのインストール: brew install jq${NC}"
            fi
        done

        echo -e "${GREEN}✅ マニフェストファイルの署名更新完了${NC}"
        echo ""
    else
        echo -e "${YELLOW}  ⚠️  署名ファイルが空です。署名の更新をスキップします${NC}"
        echo ""
    fi
fi

# ファイルのアップロード
echo -e "${YELLOW}📤 ファイルをアップロード中...${NC}"

for file in "${UPLOAD_FILES[@]}"; do
    filename=$(basename "$file")
    echo -e "${BLUE}  アップロード中: $filename${NC}"

    # 既存のファイルを削除（存在する場合）
    gh release delete-asset "$VERSION" "$filename" --repo "$REPO" --yes 2>/dev/null || true

    # ファイルをアップロード
    gh release upload "$VERSION" "$file" --repo "$REPO"

    echo -e "${GREEN}  ✅ 完了: $filename${NC}"
done

echo ""
echo -e "${GREEN}===========================================================${NC}"
echo -e "${GREEN}アップロード完了！${NC}"
echo -e "${GREEN}===========================================================${NC}"
echo ""

# リリースURLを表示
RELEASE_URL="https://github.com/$REPO/releases/tag/$VERSION"
echo -e "${BLUE}🔗 リリースURL:${NC}"
echo "  $RELEASE_URL"
echo ""

echo -e "${BLUE}次のステップ:${NC}"
echo "  1. リリースページでファイルを確認"
echo "  2. マニフェストファイルをAPI Serverにデプロイ"
echo "  3. アプリケーションで自動アップデートをテスト"
echo ""

# マニフェストファイルのデプロイ案内
if [ ${#MANIFEST_FILES[@]} -gt 0 ]; then
    echo -e "${YELLOW}📄 マニフェストファイルのデプロイ:${NC}"
    echo "  ./script/upload-manifests.sh"
    echo ""
fi

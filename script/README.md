# Tauri自動アップデート用スクリプト

このディレクトリには、Tauri自動アップデート機能で使用する静的JSONファイル生成スクリプトが含まれています。

## generate-update-manifest.cjs

### 概要

GitHub Actionsのリリースワークフロー内で実行され、各プラットフォーム・アーキテクチャ用の静的JSONファイルを生成するスクリプトです。

### 生成されるファイル

- `darwin-x86_64.json` - macOS Intel用
- `darwin-aarch64.json` - macOS Apple Silicon用  
- `windows-x86_64.json` - Windows 64bit用

### 使用方法

#### 基本実行

```bash
node script/generate-update-manifest.cjs
```

#### 環境変数を指定した実行

```bash
VERSION=1.2.3 \
RELEASE_TAG=v1.2.3-20250107-120000 \
RELEASE_NOTES="新機能追加とバグ修正" \
GITHUB_REPOSITORY=tsucchinoko/orano-keihi \
node script/generate-update-manifest.cjs
```

### 環境変数

| 変数名 | 説明 | デフォルト値 |
|--------|------|-------------|
| `VERSION` | アプリケーションのバージョン | package.jsonから取得 |
| `RELEASE_TAG` | GitHubリリースのタグ名 | `v${VERSION}` |
| `RELEASE_NOTES` | リリースノート | `バージョン ${VERSION} のリリース` |
| `GITHUB_REPOSITORY` | GitHubリポジトリ名 | `tsucchinoko/orano-keihi` |

### 出力

スクリプトは `update-manifests/` ディレクトリに以下のファイルを生成します：

```
update-manifests/
├── darwin-x86_64.json
├── darwin-aarch64.json
└── windows-x86_64.json
```

### JSONファイル形式

生成されるJSONファイルは、Tauri updater仕様に準拠した以下の形式です：

```json
{
  "version": "1.2.3",
  "notes": "新機能追加とバグ修正",
  "pub_date": "2026-01-07T08:10:17.681Z",
  "platforms": {
    "darwin-x86_64": {
      "signature": "SIGNATURE_PLACEHOLDER",
      "url": "https://github.com/tsucchinoko/orano-keihi/releases/download/v1.2.3-20250107-120000/orano-keihi_1.2.3_x86_64.dmg"
    }
  }
}
```

### GitHub Actionsでの使用

GitHub Actionsワークフローでは、以下のようにスクリプトを実行します：

```yaml
- name: 静的JSONファイルの生成
  run: node script/generate-update-manifest.cjs
  env:
    VERSION: ${{ needs.version-tag.outputs.VERSION }}
    RELEASE_TAG: ${{ needs.version-tag.outputs.RELEASE_TAG }}
    RELEASE_NOTES: "リリースノート内容"
    GITHUB_REPOSITORY: ${{ github.repository }}

- name: JSONファイルのリリースへのアップロード
  uses: actions/upload-release-asset@v1
  with:
    upload_url: ${{ steps.create_release.outputs.upload_url }}
    asset_path: ./update-manifests/
    asset_name: update-manifests
    asset_content_type: application/json
```

### 署名について

実際のリリース時には、Tauriが自動的に署名ファイル（`.sig`）を生成します。スクリプトは、対応する署名ファイルが存在する場合はそれを読み込み、存在しない場合はプレースホルダーを使用します。

### トラブルシューティング

#### 署名ファイルが見つからない

```
署名ファイルの読み込みに失敗: /path/to/file.sig
```

これは正常な動作です。実際のビルド時にTauriが署名ファイルを生成するため、開発時にはプレースホルダーが使用されます。

#### 必須フィールドが不足

```
必須フィールドが不足: version
```

環境変数またはpackage.jsonの設定を確認してください。

#### 無効なURL形式

```
無効なURL形式: http://example.com/file.dmg
```

URLはHTTPS形式である必要があります。GITHUB_REPOSITORYの設定を確認してください。


## sign-local-update.sh ⭐ NEW

### 概要

ローカル環境でビルドしたDMGファイルに署名を追加し、マニフェストファイルを更新するスクリプトです。

### 使用方法

```bash
./script/sign-local-update.sh <DMGファイルのパス> [バージョン]
```

### 例

```bash
# aarch64版に署名
./script/sign-local-update.sh \
  packages/desktop/src-tauri/target/release/bundle/dmg/orano-keihi_0.1.1_aarch64.dmg \
  v0.1.1

# x64版に署名
./script/sign-local-update.sh \
  packages/desktop/src-tauri/target/release/bundle/dmg/orano-keihi_0.1.1_x64.dmg \
  v0.1.1
```

### 機能

1. DMGファイルにminisign署名を生成
2. `.sig` ファイルを作成
3. マニフェストファイルを自動更新（jqがインストールされている場合）

### 前提条件

- **Tauri CLI**: `cargo install tauri-cli`
- **署名鍵**: `tauri signer generate -w ~/.tauri/orano-keihi.key`
- **環境変数**: `TAURI_SIGNING_PRIVATE_KEY` または デフォルトパス `~/.tauri/orano-keihi.key`
- **jq** (オプション): マニフェスト自動更新用

### 署名鍵の生成

```bash
# 新しい署名鍵を生成
tauri signer generate -w ~/.tauri/orano-keihi.key

# 公開鍵を表示（tauri.conf.jsonに設定）
tauri signer generate -w ~/.tauri/orano-keihi.key --ci
```

公開鍵を `packages/desktop/src-tauri/tauri.conf.json` の `plugins.updater.pubkey` に設定します。

### 環境変数

| 変数名 | 説明 | デフォルト値 |
|--------|------|-------------|
| `TAURI_SIGNING_PRIVATE_KEY` | 署名鍵のパス | `~/.tauri/orano-keihi.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 署名鍵のパスワード | なし |

## ローカル環境での署名付きアップデートテスト手順

### 1. 署名鍵の準備

```bash
# 署名鍵を生成（初回のみ）
tauri signer generate -w ~/.tauri/orano-keihi.key

# 公開鍵を取得
tauri signer generate -w ~/.tauri/orano-keihi.key --ci
```

公開鍵を `packages/desktop/src-tauri/tauri.conf.json` の `plugins.updater.pubkey` に設定します。

### 2. アプリケーションのビルド

```bash
cd packages/desktop
pnpm tauri build
```

ビルドされたDMGファイルは以下のパスに生成されます：
- `packages/desktop/src-tauri/target/release/bundle/dmg/orano-keihi_0.1.0_aarch64.dmg` (Apple Silicon)
- `packages/desktop/src-tauri/target/release/bundle/dmg/orano-keihi_0.1.0_x64.dmg` (Intel)

### 3. DMGファイルに署名

```bash
# 現在のアーキテクチャに応じて実行
./script/sign-local-update.sh \
  packages/desktop/src-tauri/target/release/bundle/dmg/orano-keihi_0.1.1_aarch64.dmg \
  v0.1.1
```

### 4. マニフェストファイルの確認

`update-manifests/darwin-aarch64.json` が更新されていることを確認します：

```json
{
  "version": "0.1.1",
  "platforms": {
    "darwin-aarch64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6...",
      "url": "https://..."
    }
  }
}
```

### 5. GitHub Releasesにアップロード

1. GitHubでリリースを作成（例: `v0.1.1`）
2. DMGファイルとマニフェストファイルをアップロード

### 6. API Serverをデプロイ

```bash
cd packages/api-server
pnpm deploy
```

### 7. アプリからテスト

1. アプリを起動
2. メニューバーの「ヘルプ」→「アップデートを確認」をクリック
3. 署名検証が成功し、アップデートがインストールされることを確認

## トラブルシューティング

### 署名エラー: "Invalid encoding in minisign data"

**原因**: マニフェストファイルの `signature` フィールドが空または無効

**解決方法**: 
```bash
./script/sign-local-update.sh <DMGファイル> <バージョン>
```

### 404 Not Found エラー

**原因**: 
- GitHub Releasesにファイルが存在しない
- URLが正しくない

**解決方法**: マニフェストファイルのURLを確認

### 署名鍵が見つからない

**原因**: 署名鍵が生成されていない

**解決方法**:
```bash
# 署名鍵を生成
tauri signer generate -w ~/.tauri/orano-keihi.key

# 環境変数を設定
export TAURI_SIGNING_PRIVATE_KEY=~/.tauri/orano-keihi.key
```

### jqがインストールされていない

**原因**: マニフェスト自動更新にjqが必要

**解決方法**:
```bash
# macOS
brew install jq

# Ubuntu/Debian
sudo apt-get install jq
```

## 参考資料

- [Tauri Updater Documentation](https://v2.tauri.app/plugin/updater/)
- [Minisign](https://jedisct1.github.io/minisign/)
- [GitHub Releases](https://docs.github.com/en/repositories/releasing-projects-on-github)

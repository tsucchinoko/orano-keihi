# ビルド・リリース・署名ガイド

## 概要

このドキュメントでは、オラの経費だゾアプリケーションのビルド、署名、リリースプロセスについて説明します。

## 目次

1. [リリース方法の種類](#リリース方法の種類)
2. [GitHub Actionsによる自動リリース](#github-actionsによる自動リリース)
3. [ローカルビルド・手動リリース](#ローカルビルド手動リリース)
4. [署名について](#署名について)
5. [トラブルシューティング](#トラブルシューティング)

## リリース方法の種類

### 1. GitHub Actionsによる自動リリース（推奨）
- `release`ブランチへのプッシュで自動実行
- Apple Developer証明書とTauri署名を自動適用
- クロスプラットフォーム対応（macOS + Windows）
- マニフェストファイル自動生成・アップロード

### 2. ローカルビルド・手動リリース
- 開発・テスト用
- ローカル環境でビルド・署名
- 手動でGitHubリリースにアップロード
- 動作確認やデバッグに最適

## GitHub Actionsによる自動リリース

### 前提条件

#### 必要なGitHub Secrets

```bash
# Apple Developer証明書関連
MACOS_CERTIFICATE          # Apple Developer証明書（Base64エンコード）
MACOS_CERTIFICATE_PWD      # 証明書のパスワード
KEYCHAIN_PASSWORD          # キーチェーンのパスワード
APPLE_ID                   # Apple IDのメールアドレス
APPLE_ID_PASSWORD          # App用パスワード
APPLE_TEAM_ID              # Apple Developer TeamのID

# Tauri署名関連
TAURI_SIGNING_PRIVATE_KEY          # Tauri署名用の秘密鍵
TAURI_SIGNING_PRIVATE_KEY_PASSWORD # 秘密鍵のパスワード（オプション）

# Cloudflare R2関連
R2_ACCOUNT_ID              # Cloudflare R2アカウントID
R2_ACCESS_KEY_ID           # R2アクセスキーID
R2_SECRET_ACCESS_KEY       # R2シークレットアクセスキー
R2_BUCKET_NAME             # R2バケット名
```

詳細な設定方法は[GitHub Secrets設定ガイド](./GITHUB_SECRETS_SETUP.md)を参照してください。

### リリース手順

1. **コードの準備**
   ```bash
   # 最新のコードをreleaseブランチにマージ
   git checkout release
   git merge main
   ```

2. **バージョンの更新**
   ```bash
   # package.jsonのバージョンを更新
   vim package.json
   vim packages/desktop/package.json
   vim packages/desktop/src-tauri/tauri.conf.json
   ```

3. **リリースの実行**
   ```bash
   # releaseブランチにプッシュ
   git add .
   git commit -m "Release v0.1.1"
   git push origin release
   ```

4. **GitHub Actionsの確認**
   - GitHub Actionsページでワークフローの実行状況を確認
   - ビルドが成功すると自動的にリリースが作成される

### 生成されるファイル

#### macOS
- `orano-keihi_0.1.1_aarch64.dmg` - Apple Silicon用アプリケーション
- `orano-keihi_0.1.1_aarch64.dmg.sig` - Tauri署名ファイル
- `orano-keihi_0.1.1_x64.dmg` - Intel用アプリケーション
- `orano-keihi_0.1.1_x64.dmg.sig` - Tauri署名ファイル

#### Windows
- `orano-keihi_0.1.1_x64_ja-JP.msi` - Windows用インストーラー

#### マニフェストファイル
- `darwin-aarch64.json` - macOS Apple Silicon用
- `darwin-x86_64.json` - macOS Intel用
- `windows-x86_64.json` - Windows 64bit用

## ローカルビルド・手動リリース

### 前提条件

#### 必要なツール

```bash
# GitHub CLI
brew install gh

# jq（マニフェスト生成用）
brew install jq

# Tauri CLI
cargo install tauri-cli
```

#### Tauri署名鍵の準備

```bash
# 署名鍵を生成（初回のみ）
pnpm tauri signer generate -w ~/.tauri/orano-keihi.key

# パスワード付きで生成する場合
pnpm tauri signer generate -w ~/.tauri/orano-keihi.key -p "your-password"

# 公開鍵を確認
pnpm tauri signer generate -w ~/.tauri/orano-keihi.key --force
```

公開鍵を`packages/desktop/src-tauri/tauri.conf.json`の`plugins.updater.pubkey`に設定してください。

### 手順

#### ステップ1: ローカルビルド・署名

```bash
# 全プラットフォームをビルド
./script/build-and-sign-local.sh v0.1.1-test

# macOSのみビルド
./script/build-and-sign-local.sh v0.1.1-test macos

# Windowsのみビルド
./script/build-and-sign-local.sh v0.1.1-test windows
```

このスクリプトは以下を実行します：
- 依存関係のインストール
- フロントエンドビルド
- Tauriアプリケーションビルド
- Tauri署名ファイル（.sig）の生成
- マニフェストファイルの生成

#### ステップ2: GitHub CLI認証

```bash
# GitHub CLIで認証（初回のみ）
gh auth login
```

#### ステップ3: GitHubリリースへアップロード

```bash
# GitHubリリースにアップロード
./script/upload-to-github.sh v0.1.1-test "テスト用リリース"
```

このスクリプトは以下を実行します：
- GitHub CLI認証確認
- リリースの作成（存在しない場合）
- DMG、署名ファイル、MSI、マニフェストファイルのアップロード

#### ステップ4: マニフェストファイルの個別更新（オプション）

マニフェストファイルは`upload-to-github.sh`で自動的にアップロードされますが、
後から修正・更新する場合は以下のコマンドを使用します：

```bash
# 既存リリースにマニフェストファイルを追加・更新
./script/upload-manifests.sh v0.1.1-test
```

#### ステップ5: 動作確認

1. GitHubリリースページでファイルを確認
2. DMGファイルをダウンロードしてインストール
3. アプリケーションで「アップデートを確認」をテスト

## 署名について

### 署名の種類

#### 1. Apple Developer署名（macOS）
- **目的**: macOSでの実行許可とセキュリティ警告の回避
- **適用対象**: DMGファイル全体
- **証明書**: Apple Developer Program証明書
- **実行タイミング**: Tauriビルド時

#### 2. Tauri署名（クロスプラットフォーム）
- **目的**: 自動アップデート時の整合性検証
- **適用対象**: アプリケーションファイルのハッシュ
- **証明書**: minisign形式の秘密鍵
- **実行タイミング**: ビルド後

### 署名プロセスの流れ

#### GitHub Actions（自動）
1. Apple Developer証明書でmacOS署名
2. Tauri署名（minisign）を生成
3. DMGファイルと署名ファイル（.sig）を両方アップロード

#### ローカル（手動）
1. Tauriビルド（Apple署名なし）
2. Tauri署名（minisign）を生成
3. 手動でGitHubリリースにアップロード

### 署名の検証

#### macOS署名の確認
```bash
# 署名の確認
codesign -dv --verbose=4 /path/to/app.dmg

# 公証の確認（GitHub Actionsの場合）
spctl -a -vv /path/to/app.dmg
```

#### Tauri署名の確認
```bash
# 署名ファイルの内容確認
cat /path/to/app.dmg.sig

# 署名の検証
pnpm tauri signer verify /path/to/app.dmg --public-key "公開鍵"
```

## 利用可能なスクリプト

### `script/build-and-sign-local.sh`
ローカル環境でビルド・署名・マニフェスト生成を一括実行

```bash
./script/build-and-sign-local.sh <バージョン> [プラットフォーム]
```

### `script/upload-to-github.sh`
ローカルビルドファイルをGitHubリリースにアップロード

```bash
./script/upload-to-github.sh <バージョン> [説明]
```

### `script/upload-manifests.sh`
既存リリースにマニフェストファイルを個別アップロード

```bash
./script/upload-manifests.sh <バージョン>
```

### `script/generate-update-manifest.cjs`
マニフェストファイルの生成（GitHub Actions内で自動実行）

```bash
node script/generate-update-manifest.cjs
```

### `script/verify_signatures.sh`
署名ファイルの検証

```bash
./script/verify_signatures.sh
```

## トラブルシューティング

### DMGファイルが破損している

**原因**: 二重署名による競合
- Apple署名後にTauri署名を追加すると、ファイルの整合性が破損する

**解決方法**: 
- GitHub Actionsを使用（推奨）
- ローカルでは手動署名スクリプトを使用

### 署名エラー: "Invalid encoding in minisign data"

**原因**: マニフェストファイルの`signature`フィールドが空または無効

**解決方法**: 
```bash
./script/build-and-sign-local.sh v0.1.1-test
```

### GitHub CLI認証エラー

**原因**: GitHub CLIの認証が完了していない

**解決方法**:
```bash
# GitHub CLIで認証
gh auth login

# 認証状態を確認
gh auth status
```

### 署名鍵が見つからない

**原因**: Tauri署名鍵が生成されていない

**解決方法**:
```bash
# 署名鍵を生成
pnpm tauri signer generate -w ~/.tauri/orano-keihi.key

# 環境変数を設定（必要に応じて）
export TAURI_SIGNING_PRIVATE_KEY=~/.tauri/orano-keihi.key
```

### 自動アップデートが動作しない

**原因**: 
- マニフェストファイルがGitHubリリースにアップロードされていない
- 公開鍵が正しく設定されていない
- 署名ファイルが見つからない

**解決方法**:
1. マニフェストファイルをアップロード: `./script/upload-manifests.sh v0.1.1-test`
2. 公開鍵を`tauri.conf.json`に設定
3. 署名ファイル（.sig）がリリースにアップロードされていることを確認

### ビルドエラー

**原因**: 依存関係の問題

**解決方法**:
```bash
# 依存関係を再インストール
pnpm install

# キャッシュをクリア
pnpm store prune
rm -rf node_modules
pnpm install
```

## セキュリティ上の注意事項

1. **署名鍵の管理**
   - 秘密鍵は絶対に公開しない
   - GitHub Secretsは暗号化されて保存される
   - 定期的に鍵をローテーションする

2. **証明書の管理**
   - Apple Developer証明書は安全に保管
   - パスワードは強力なものを使用
   - 不要になった証明書は削除

3. **アクセス制御**
   - リポジトリへのアクセス権限を適切に管理
   - 署名権限を持つユーザーを制限
   - GitHub Actionsのログを定期的に確認

## 参考リンク

- [Tauri Documentation](https://tauri.app/)
- [Tauri Updater Plugin](https://v2.tauri.app/plugin/updater/)
- [Apple Code Signing](https://developer.apple.com/documentation/security/code_signing_services)
- [GitHub CLI](https://cli.github.com/)
- [Minisign](https://jedisct1.github.io/minisign/)
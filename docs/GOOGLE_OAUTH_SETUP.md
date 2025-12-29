# Google OAuth2認証セットアップガイド（デスクトップアプリケーション用）

このドキュメントでは、TauriデスクトップアプリケーションでGoogle OAuth2認証を使用するために必要な設定手順を説明します。

## 概要

**重要**: デスクトップアプリケーションでは、セキュリティ上の理由から`client_secret`を使用せず、PKCE（Proof Key for Code Exchange）方式を使用します。

必要な環境変数：
- `GOOGLE_CLIENT_ID`: GoogleクライアントID（デスクトップアプリケーション用）
- `GOOGLE_REDIRECT_URI`: OAuth2リダイレクトURI（ループバック方式）
- `SESSION_ENCRYPTION_KEY`: セッション暗号化キー

## セットアップ手順

### 1. Google Cloud Consoleでプロジェクトを作成

1. [Google Cloud Console](https://console.cloud.google.com/)にアクセス
2. 新しいプロジェクトを作成するか、既存のプロジェクトを選択
3. プロジェクト名を入力して「作成」をクリック

### 2. 必要なAPIを有効化

1. Google Cloud Consoleの左側メニューから「APIとサービス」→「ライブラリ」を選択
2. 「Google+ API」を検索して選択
3. 「有効にする」をクリック

### 3. OAuth同意画面を設定

1. 左側メニューから「APIとサービス」→「OAuth同意画面」を選択
2. ユーザータイプを選択：
   - **外部**: 一般ユーザーが使用可能（推奨）
   - **内部**: Google Workspaceユーザーのみ
3. 「作成」をクリック
4. 必要な情報を入力：
   - **アプリ名**: アプリケーションの名前（例：「オラの経費だゾ」）
   - **ユーザーサポートメール**: サポート用メールアドレス
   - **デベロッパーの連絡先情報**: 開発者のメールアドレス
5. 「保存して次へ」をクリック
6. スコープの設定（基本的なプロフィール情報のみで十分）
7. テストユーザーの追加（必要に応じて）
8. 設定を確認して完了

### 4. OAuth2クライアントIDを作成（デスクトップアプリケーション用）

**重要**: 必ず「デスクトップアプリケーション」を選択してください。

1. 左側メニューから「APIとサービス」→「認証情報」を選択
2. 「認証情報を作成」→「OAuth クライアント ID」をクリック
3. **アプリケーションの種類**で「**デスクトップアプリケーション**」を選択
4. 名前を入力（例：「オラの経費だゾ - デスクトップアプリ」）
5. 「作成」をクリック

**注意**: デスクトップアプリケーションタイプでは、リダイレクトURIの設定項目は表示されません。これは正常な動作です。

### 5. クライアントIDを取得

1. 作成されたOAuth2クライアントの詳細画面で以下の情報を確認：
   - **クライアントID**: `GOOGLE_CLIENT_ID`として使用
   - **クライアントシークレット**: デスクトップアプリでは使用しません

## 環境変数の設定

### 開発環境での設定

`src-tauri/.env`ファイルを作成し、以下の内容を記述：

```bash
# Google OAuth 2.0設定（デスクトップアプリケーション用）
GOOGLE_CLIENT_ID=your_desktop_client_id_here
GOOGLE_REDIRECT_URI=http://127.0.0.1/callback
SESSION_ENCRYPTION_KEY=your_32_byte_encryption_key_here
```

### 本番環境での設定

本番環境では、セキュリティのため環境変数を直接設定：

```bash
export GOOGLE_CLIENT_ID="your_desktop_client_id_here"
export GOOGLE_REDIRECT_URI="http://127.0.0.1/callback"
export SESSION_ENCRYPTION_KEY="your_32_byte_encryption_key_here"
```

## 設定値の説明

### GOOGLE_CLIENT_ID
- Google Cloud Consoleで生成されたデスクトップアプリケーション用クライアントID
- 形式：`123456789-abcdefghijklmnop.apps.googleusercontent.com`
- **重要**: 「デスクトップアプリケーション」タイプで作成されたものを使用

### GOOGLE_REDIRECT_URI
- OAuth認証後にリダイレクトされるURI
- **ループバック方式（推奨）**: `http://127.0.0.1/callback`
- ポート番号は動的に割り当てられるため、指定しません
- Googleが`http://127.0.0.1`の任意のポートを自動的に許可します

### SESSION_ENCRYPTION_KEY
- セッション暗号化用の32バイトキー
- Base64エンコードされた文字列
- 生成方法：`openssl rand -base64 32`

## デスクトップアプリケーションの特徴

### PKCE（Proof Key for Code Exchange）方式
- クライアントシークレットを使用しない
- より安全な認証フロー
- 動的に生成されるコードチャレンジとベリファイアを使用

### ループバック方式
- 動的ポート割り当て（`127.0.0.1:0`）
- ポート競合の回避
- セキュリティの向上

## トラブルシューティング

### よくあるエラー

1. **client_secret is missing**
   - 原因：「Webアプリケーション」タイプのクライアントIDを使用している
   - 解決方法：「デスクトップアプリケーション」タイプのクライアントIDを新規作成

2. **redirect_uri_mismatch**
   - 原因：リダイレクトURIの設定が間違っている
   - 解決方法：`http://127.0.0.1/callback`を使用（ポート番号なし）

3. **invalid_client**
   - 原因：クライアントIDが間違っている
   - 解決方法：Google Cloud Consoleで正しいデスクトップアプリ用クライアントIDを確認

### デバッグ方法

1. ログを確認：`LOG_LEVEL=debug`を設定
2. クライアントIDのタイプを確認（デスクトップアプリケーション用か）
3. 環境変数が正しく読み込まれているか確認
4. ネットワーク接続を確認

## 参考リンク

- [Google OAuth 2.0 for Mobile & Desktop Apps](https://developers.google.com/identity/protocols/oauth2/native-app)
- [RFC 7636: Proof Key for Code Exchange](https://tools.ietf.org/html/rfc7636)
- [Google Cloud Console](https://console.cloud.google.com/)

### 方法B: gcloudコマンド（CLI）を使用

#### 前提条件

1. Google Cloud SDKがインストールされていること
2. gcloudコマンドが使用可能であること

Google Cloud SDKのインストール方法：
```bash
# macOSの場合（Homebrewを使用）
brew install google-cloud-sdk

# または公式インストーラーを使用
# https://cloud.google.com/sdk/docs/install
```

#### 1. gcloudの初期設定

```bash
# Google Cloudにログイン
gcloud auth login

# プロジェクトを作成（プロジェクトIDは一意である必要があります）
export PROJECT_ID="your-project-id-$(date +%s)"
gcloud projects create $PROJECT_ID --name="My OAuth App"

# プロジェクトを設定
gcloud config set project $PROJECT_ID

# 課金アカウントを設定（必要に応じて）
# gcloud billing projects link $PROJECT_ID --billing-account=BILLING_ACCOUNT_ID
```

#### 2. 必要なAPIを有効化

```bash
# Google+ APIを有効化
gcloud services enable plus.googleapis.com

# OAuth2に必要なAPIを有効化
gcloud services enable iamcredentials.googleapis.com
```

#### 3. OAuth同意画面の設定

OAuth同意画面の設定は現在gcloudコマンドでは完全にサポートされていないため、最小限の設定のみ行います：

```bash
# OAuth同意画面の基本設定（手動での追加設定が必要）
echo "OAuth同意画面の設定は Google Cloud Console で手動で行う必要があります"
echo "https://console.cloud.google.com/apis/credentials/consent?project=$PROJECT_ID"
```

#### 4. OAuth2クライアントIDを作成

```bash
# デスクトップアプリケーション用のOAuth2クライアントを作成
gcloud auth application-default set-quota-project $PROJECT_ID

# OAuth2クライアント認証情報を作成
CLIENT_NAME="my-oauth-client"
REDIRECT_URIS="http://localhost:1420/auth/callback,http://localhost:5173/auth/callback"

# 認証情報ファイルを作成
cat > oauth_client.json << EOF
{
  "installed": {
    "client_id": "",
    "client_secret": "",
    "auth_uri": "https://accounts.google.com/o/oauth2/auth",
    "token_uri": "https://oauth2.googleapis.com/token",
    "redirect_uris": ["$REDIRECT_URIS"]
  }
}
EOF

echo "OAuth2クライアントの作成は Google Cloud Console で行う必要があります"
echo "以下のコマンドでコンソールを開いてください："
echo "open https://console.cloud.google.com/apis/credentials?project=$PROJECT_ID"
```

#### 5. 環境変数の設定用スクリプト

作成したクライアントIDとシークレットを使用して環境変数を設定するスクリプトを作成：

```bash
# 環境変数設定スクリプトを作成
cat > setup_oauth_env.sh << 'EOF'
#!/bin/bash

echo "Google OAuth2 環境変数設定スクリプト"
echo "======================================="

read -p "Google Client ID を入力してください: " CLIENT_ID
read -s -p "Google Client Secret を入力してください: " CLIENT_SECRET
echo

# .envファイルに追記
cat >> src-tauri/.env << EOL

# Google OAuth2設定
GOOGLE_CLIENT_ID=$CLIENT_ID
GOOGLE_CLIENT_SECRET=$CLIENT_SECRET
GOOGLE_REDIRECT_URI=http://localhost:1420/auth/callback
EOL

echo "環境変数が src-tauri/.env に設定されました"
EOF

chmod +x setup_oauth_env.sh
```

#### 6. 完全自動化スクリプト（実験的）

```bash
# 完全自動化スクリプト（OAuth同意画面の設定は手動が必要）
cat > create_oauth_setup.sh << 'EOF'
#!/bin/bash

set -e

echo "Google OAuth2 自動セットアップスクリプト"
echo "======================================="

# プロジェクトIDの生成
PROJECT_ID="oauth-app-$(date +%s)"
echo "プロジェクトID: $PROJECT_ID"

# プロジェクト作成
echo "プロジェクトを作成中..."
gcloud projects create $PROJECT_ID --name="OAuth App"

# プロジェクト設定
gcloud config set project $PROJECT_ID

# API有効化
echo "必要なAPIを有効化中..."
gcloud services enable plus.googleapis.com
gcloud services enable iamcredentials.googleapis.com

echo "セットアップが完了しました！"
echo ""
echo "次の手順："
echo "1. OAuth同意画面を設定: https://console.cloud.google.com/apis/credentials/consent?project=$PROJECT_ID"
echo "2. OAuth2クライアントを作成: https://console.cloud.google.com/apis/credentials?project=$PROJECT_ID"
echo "3. 作成したクライアントIDとシークレットを ./setup_oauth_env.sh で設定"

# ブラウザでコンソールを開く（macOSの場合）
if command -v open &> /dev/null; then
    echo ""
    read -p "Google Cloud Console を開きますか？ (y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        open "https://console.cloud.google.com/apis/credentials/consent?project=$PROJECT_ID"
    fi
fi
EOF

chmod +x create_oauth_setup.sh
```

#### 使用方法

1. **自動セットアップスクリプトを実行**：
   ```bash
   ./create_oauth_setup.sh
   ```

2. **OAuth同意画面を手動で設定**（ブラウザで開かれるページで）

3. **OAuth2クライアントを手動で作成**

4. **環境変数を設定**：
   ```bash
   ./setup_oauth_env.sh
   ```

## 環境変数の設定

### 開発環境での設定

`src-tauri/.env`ファイルを作成し、以下の内容を記述：

```bash
# Google OAuth2設定
GOOGLE_CLIENT_ID=your_google_client_id_here
GOOGLE_CLIENT_SECRET=your_google_client_secret_here
GOOGLE_REDIRECT_URI=http://localhost:1420/auth/callback
```

### 本番環境での設定

本番環境では、セキュリティのため環境変数を直接設定：

```bash
export GOOGLE_CLIENT_ID="your_google_client_id_here"
export GOOGLE_CLIENT_SECRET="your_google_client_secret_here"
export GOOGLE_REDIRECT_URI="https://yourdomain.com/auth/callback"
```

## 設定値の説明

### GOOGLE_CLIENT_ID
- Google Cloud Consoleで生成されたクライアントID
- 公開情報として扱われるため、フロントエンドでも使用可能
- 形式：`123456789-abcdefghijklmnop.apps.googleusercontent.com`

### GOOGLE_CLIENT_SECRET
- Google Cloud Consoleで生成されたクライアントシークレット
- **機密情報**のため、バックエンドでのみ使用
- 絶対にフロントエンドやパブリックリポジトリに含めない

### GOOGLE_REDIRECT_URI
- OAuth認証後にリダイレクトされるURI
- Google Cloud Consoleで設定した承認済みリダイレクトURIと一致する必要がある
- **ループバック方式（推奨）**: `http://127.0.0.1/callback`
- Tauriアプリケーション（従来方式）：`http://localhost:1420/auth/callback`
- SvelteKit開発環境：`http://localhost:5173/auth/callback`
- 本番環境：`https://yourdomain.com/auth/callback`

**ループバック方式の利点**:
- 動的ポート割り当てによるセキュリティ向上
- ポート競合の回避
- Googleの推奨方式に準拠

## セキュリティ注意事項

1. **クライアントシークレットの管理**
   - `.env`ファイルを`.gitignore`に追加
   - 本番環境では環境変数として設定
   - 定期的にローテーション

2. **リダイレクトURIの制限**
   - 必要最小限のURIのみを承認済みリストに追加
   - HTTPSを使用（本番環境）

3. **スコープの最小化**
   - 必要最小限の権限のみを要求
   - ユーザーのプライバシーを尊重

## トラブルシューティング

### よくあるエラー

1. **redirect_uri_mismatch**
   - 設定したリダイレクトURIがGoogle Cloud Consoleの設定と一致しない
   - 解決方法：URIを正確に一致させる

2. **invalid_client**
   - クライアントIDまたはシークレットが間違っている
   - 解決方法：Google Cloud Consoleで正しい値を確認

3. **access_denied**
   - ユーザーが認証を拒否した
   - 解決方法：ユーザーに再度認証を求める

### デバッグ方法

1. 環境変数が正しく読み込まれているか確認
2. Google Cloud Consoleの設定を再確認
3. ネットワーク接続を確認
4. ブラウザのコンソールでエラーメッセージを確認

## 参考リンク

- [Google OAuth 2.0 ドキュメント](https://developers.google.com/identity/protocols/oauth2)
- [Google Cloud Console](https://console.cloud.google.com/)
- [OAuth 2.0 Scopes for Google APIs](https://developers.google.com/identity/protocols/oauth2/scopes)
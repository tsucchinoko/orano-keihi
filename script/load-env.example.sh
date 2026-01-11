#!/usr/bin/env bash
# 環境変数設定スクリプト（サンプル）
# 実際の値は .env.local に設定してください（このファイルはGitにコミットされません）

# 使い方:
# 1. このファイルを .env.local にコピー
#    cp script/.env.local.example script/.env.local
#
# 2. .env.local を編集して実際の値を設定
#
# 3. ビルドスクリプトから読み込む
#    source script/.env.local && pnpm tauri build

# 環境設定
export ENVIRONMENT=production

# APIサーバー設定（本番環境）
export API_SERVER_URL=https://orano-keihi.tsucchinoko.workers.dev
export API_TIMEOUT_SECONDS=30
export API_MAX_RETRIES=3

# Google OAuth 2.0設定
# Google Cloud Consoleで取得したクライアント情報を設定してください
export GOOGLE_CLIENT_ID=your_google_client_id_here
export GOOGLE_CLIENT_SECRET=your_google_client_secret_here
export GOOGLE_REDIRECT_URI=http://127.0.0.1/callback

# セッション暗号化キー（32バイトのランダムな文字列）
export SESSION_ENCRYPTION_KEY=your_32_byte_random_encryption_key_here

# ログレベル設定
export LOG_LEVEL=info

# Apple署名設定（macOS用）
# 証明書（base64エンコードされた.p12ファイル）
# 生成方法: base64 -i /path/to/certificate.p12 -o certificate.p12.base64
# export APPLE_CERTIFICATE=$(cat /path/to/certificate.p12.base64)

# 証明書のパスワード
# export APPLE_CERTIFICATE_PASSWORD=your_certificate_password_here

# キーチェーンのパスワード（省略可、デフォルト: build-keychain-password）
# export KEYCHAIN_PASSWORD=your_keychain_password_here

# Apple ID（公証用、オプション）
# export APPLE_ID=your_apple_id@example.com
# export APPLE_ID_PASSWORD=your_app_specific_password_here
# export APPLE_TEAM_ID=YOUR_TEAM_ID

# Windows署名設定（Windows用）
export WINDOWS_TIMESTAMP_URL=http://timestamp.digicert.com

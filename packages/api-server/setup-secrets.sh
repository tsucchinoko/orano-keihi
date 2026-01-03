#!/bin/bash

# Cloudflare Workers秘匿情報設定スクリプト
# 使用方法: ./setup-secrets.sh [development|production]

ENVIRONMENT=${1:-development}

echo "🔐 Cloudflare Workers秘匿情報を設定します (環境: $ENVIRONMENT)"
echo "注意: 各秘匿情報の値を入力してください"

# JWT秘密鍵の設定
echo ""
echo "📝 JWT_SECRET を設定します"
echo "推奨: 32バイト以上のランダムな文字列"
echo "例: $(openssl rand -base64 32)"
wrangler secret put JWT_SECRET --env $ENVIRONMENT

# R2アクセスキーIDの設定
echo ""
echo "📝 R2_ACCESS_KEY_ID を設定します"
echo "Cloudflare R2のアクセスキーIDを入力してください"
wrangler secret put R2_ACCESS_KEY_ID --env $ENVIRONMENT

# R2シークレットアクセスキーの設定
echo ""
echo "📝 R2_SECRET_ACCESS_KEY を設定します"
echo "Cloudflare R2のシークレットアクセスキーを入力してください"
wrangler secret put R2_SECRET_ACCESS_KEY --env $ENVIRONMENT

# Google OAuth クライアントシークレットの設定
echo ""
echo "📝 GOOGLE_CLIENT_SECRET を設定します"
echo "Google Cloud ConsoleのOAuth 2.0クライアントシークレットを入力してください"
wrangler secret put GOOGLE_CLIENT_SECRET --env $ENVIRONMENT

# セッション暗号化キーの設定
echo ""
echo "📝 SESSION_ENCRYPTION_KEY を設定します"
echo "推奨: 32バイトのランダムな文字列"
echo "例: $(openssl rand -base64 32)"
wrangler secret put SESSION_ENCRYPTION_KEY --env $ENVIRONMENT

echo ""
echo "✅ 秘匿情報の設定が完了しました"
echo ""
echo "📋 設定された秘匿情報を確認するには:"
echo "wrangler secret list --env $ENVIRONMENT"
echo ""
echo "🚀 デプロイするには:"
echo "wrangler deploy --env $ENVIRONMENT"
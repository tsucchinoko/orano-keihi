# 秘匿情報設定ガイド

## 概要

このプロジェクトでは、開発環境とCloudflare Workers本番環境で異なる方法で秘匿情報を管理します。

## 開発環境での設定

### 1. 環境変数ファイルの作成

```bash
# .env.exampleをコピーして.envファイルを作成
cp .env.example .env
```

### 2. .envファイルの編集

`.env`ファイルを開いて、実際の値を設定してください：

```bash
# 必須項目
JWT_SECRET="$(openssl rand -base64 32)"
SESSION_ENCRYPTION_KEY="$(openssl rand -base64 32)"
R2_ACCESS_KEY_ID="your_r2_access_key"
R2_SECRET_ACCESS_KEY="your_r2_secret_key"
GOOGLE_CLIENT_SECRET="your_google_client_secret"
```

## Cloudflare Workers本番環境での設定

### 1. 自動設定スクリプトの使用

```bash
# 開発環境用
./setup-secrets.sh development

# 本番環境用
./setup-secrets.sh production
```

### 2. 手動設定（個別に設定する場合）

```bash
# JWT秘密鍵の設定
wrangler secret put JWT_SECRET --env development
# 入力プロンプトで32バイト以上のランダム文字列を入力

# R2認証情報の設定
wrangler secret put R2_ACCESS_KEY_ID --env development
wrangler secret put R2_SECRET_ACCESS_KEY --env development

# Google OAuth設定
wrangler secret put GOOGLE_CLIENT_SECRET --env development

# セッション暗号化キー
wrangler secret put SESSION_ENCRYPTION_KEY --env development
```

### 3. 設定確認

```bash
# 設定された秘匿情報の一覧表示
wrangler secret list --env development
wrangler secret list --env production
```

## 秘匿情報の種類と説明

| 変数名                   | 説明                                  | 生成方法                       |
| ------------------------ | ------------------------------------- | ------------------------------ |
| `JWT_SECRET`             | JWT トークンの署名用秘密鍵            | `openssl rand -base64 32`      |
| `SESSION_ENCRYPTION_KEY` | セッション暗号化キー                  | `openssl rand -base64 32`      |
| `R2_ACCESS_KEY_ID`       | Cloudflare R2 アクセスキーID          | Cloudflareダッシュボードで生成 |
| `R2_SECRET_ACCESS_KEY`   | Cloudflare R2 シークレットキー        | Cloudflareダッシュボードで生成 |
| `GOOGLE_CLIENT_SECRET`   | Google OAuth クライアントシークレット | Google Cloud Consoleで取得     |

## セキュリティ注意事項

### ✅ 推奨事項

- 本番環境では必ず強力なランダム値を使用
- 秘匿情報は絶対にGitにコミットしない
- 定期的に秘匿情報をローテーション
- 開発環境と本番環境で異なる値を使用

### ❌ 禁止事項

- `.env`ファイルをGitにコミット
- `wrangler.toml`に秘匿情報を直接記載
- 弱い秘密鍵の使用（短い文字列、辞書にある単語など）

## トラブルシューティング

### 秘匿情報が反映されない場合

```bash
# Workerを再デプロイ
wrangler deploy --env development

# 秘匿情報を再設定
wrangler secret delete JWT_SECRET --env development
wrangler secret put JWT_SECRET --env development
```

### 開発環境で認証エラーが発生する場合

1. `.env`ファイルの値が正しいか確認
2. R2の認証情報がCloudflareダッシュボードの値と一致するか確認
3. Google OAuth設定がGoogle Cloud Consoleの値と一致するか確認

## 参考リンク

- [Cloudflare Workers Secrets](https://developers.cloudflare.com/workers/configuration/secrets/)
- [Cloudflare R2 API Tokens](https://developers.cloudflare.com/r2/api/s3/tokens/)
- [Google OAuth 2.0 Setup](https://developers.google.com/identity/protocols/oauth2)

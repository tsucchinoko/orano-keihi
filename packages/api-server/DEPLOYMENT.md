# Cloudflare Workers デプロイ設定

## 環境変数の設定方法

### 1. Cloudflareダッシュボードでの設定（推奨）

#### 手順

1. **Cloudflareダッシュボード**にログイン
2. **Workers & Pages** → 対象のWorkerを選択
3. **Settings** → **Variables**に移動
4. **Environment Variables**セクションで設定

#### 本番環境用の環境変数

```
NODE_ENV = production
CORS_ORIGIN = https://yourdomain.com
JWT_SECRET = [強力なランダム文字列 - 32文字以上推奨]
SESSION_ENCRYPTION_KEY = [32バイトのランダム文字列]
SESSION_EXPIRATION_DAYS = 30
MAX_FILE_SIZE = 10485760
ALLOWED_FILE_TYPES = image/jpeg,image/png,image/gif,application/pdf
MAX_FILES_PER_REQUEST = 10
RATE_LIMIT_WINDOW_MS = 900000
RATE_LIMIT_MAX_REQUESTS = 100
LOG_LEVEL = info
```

#### 開発環境用の環境変数

```
NODE_ENV = development
CORS_ORIGIN = http://localhost:1420,tauri://localhost
JWT_SECRET = [開発用シークレット]
SESSION_ENCRYPTION_KEY = [開発用32バイト文字列]
LOG_LEVEL = debug
```

### 2. wrangler secretコマンドでの設定

機密情報はコマンドラインから設定：

```bash
# 本番環境の機密情報を設定
wrangler secret put JWT_SECRET --env production
wrangler secret put SESSION_ENCRYPTION_KEY --env production

# 開発環境の機密情報を設定
wrangler secret put JWT_SECRET --env development
wrangler secret put SESSION_ENCRYPTION_KEY --env development
```

### 3. 環境変数の優先順位

1. **wrangler secret**（最高優先度）
2. **ダッシュボードのEnvironment Variables**
3. **wrangler.tomlのvars**（最低優先度）

## wrangler.toml の設定

デプロイ前に以下の値を実際の値に置き換えてください：

### R2バケット名

- `REPLACE_WITH_PRODUCTION_BUCKET_NAME` → 本番環境のR2バケット名
- `REPLACE_WITH_DEVELOPMENT_BUCKET_NAME` → 開発環境のR2バケット名

### KV ネームスペースID

- `REPLACE_WITH_PRODUCTION_KV_ID` → 本番環境のKV ID
- `REPLACE_WITH_DEVELOPMENT_KV_ID` → 開発環境のKV ID

## 必要なリソースの事前作成

### R2バケットの作成

1. **R2 Object Storage**に移動
2. **Create bucket**をクリック
3. バケット名を設定（例：`your-production-bucket-name`）

### KVネームスペースの作成

1. **Workers & Pages** → **KV**に移動
2. **Create namespace**をクリック
3. ネームスペース名を設定（例：`sessions-production`）

## デプロイ手順

1. Cloudflareダッシュボードでリソースを作成
2. `wrangler.toml`のプレースホルダーを実際の値に置き換え
3. 環境変数をダッシュボードまたはwrangler secretで設定
4. デプロイコマンドを実行：

```bash
# 開発環境へのデプロイ
pnpm deploy:staging

# 本番環境へのデプロイ
pnpm deploy:production
```

## 環境変数の生成方法

### JWT_SECRETの生成

```bash
# Node.jsで生成
node -e "console.log(require('crypto').randomBytes(32).toString('hex'))"

# OpenSSLで生成
openssl rand -hex 32
```

### SESSION_ENCRYPTION_KEYの生成

```bash
# 32バイトのランダム文字列を生成
node -e "console.log(require('crypto').randomBytes(32).toString('base64'))"
```

## 注意事項

- `wrangler.toml`はGitにコミットされます
- 機密情報は環境変数として設定し、ファイルには含めません
- 実際のリソース名/IDは各環境で個別に設定してください
- 本番環境では必ず強力なランダム文字列を使用してください

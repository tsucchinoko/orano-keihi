# Cloudflare R2 設定ガイド

このガイドでは、経費管理アプリケーションでCloudflare R2を使用するための設定手順を説明します。

## 前提条件

- Cloudflareアカウント（無料プランでも利用可能）
- R2サービスの有効化

## 1. Cloudflare R2の設定

### 1.1 R2バケットの作成

1. [Cloudflare Dashboard](https://dash.cloudflare.com/)にログイン
2. 左サイドバーから「R2 Object Storage」を選択
3. 「Create bucket」をクリック
4. バケット名を入力（例：`expense-receipts-dev`、`expense-receipts-prod`）
5. 「Create bucket」をクリックして作成完了

### 1.2 API トークンの作成

1. Cloudflare Dashboardで「My Profile」→「API Tokens」に移動
2. 「Create Token」をクリック
3. 「Custom token」を選択
4. 以下の設定を行う：
   - **Token name**: `expense-app-r2-access`
   - **Permissions**: 
     - `Account` - `Cloudflare R2:Edit`
   - **Account Resources**: 
     - `Include` - `All accounts` または特定のアカウント
   - **Zone Resources**: 
     - `Include` - `All zones`（必要に応じて）

5. 「Continue to summary」→「Create Token」をクリック
6. 生成されたトークンを安全な場所に保存

### 1.3 アカウントIDの取得

1. Cloudflare Dashboardの右サイドバーでアカウントIDを確認
2. または「R2 Object Storage」ページでアカウントIDを確認

## 2. CORS設定（オプション）

ブラウザから直接R2にアクセスする場合は、CORS設定が必要です。

### 2.1 Wrangler CLIを使用した設定

```bash
# Wrangler CLIのインストール
npm install -g wrangler

# ログイン
wrangler login

# CORS設定ファイルの作成
cat > cors.json << EOF
[
  {
    "AllowedOrigins": ["http://localhost:1420", "tauri://localhost"],
    "AllowedMethods": ["GET", "PUT", "POST", "DELETE"],
    "AllowedHeaders": ["*"],
    "ExposeHeaders": ["ETag"],
    "MaxAgeSeconds": 3000
  }
]
EOF

# CORS設定の適用
wrangler r2 bucket cors put expense-receipts-dev --file cors.json
```

### 2.2 Cloudflare Dashboard経由での設定

1. R2バケットの詳細ページに移動
2. 「Settings」タブを選択
3. 「CORS policy」セクションで設定を追加

## 3. セキュリティ設定

### 3.1 バケットアクセス制限

- バケットは**プライベート**に設定することを推奨
- Presigned URLを使用してセキュアなアクセスを実現

### 3.2 APIトークンの権限制限

- 必要最小限の権限のみを付与
- 定期的なトークンのローテーション
- 開発環境と本番環境で異なるトークンを使用

## 4. 環境別設定

### 4.1 開発環境

```bash
# 開発用バケット
R2_BUCKET_NAME=expense-receipts-dev
```

### 4.2 本番環境

```bash
# 本番用バケット
R2_BUCKET_NAME=expense-receipts-prod
```

## 5. 料金について

### 5.1 R2の料金体系

- **ストレージ**: 10GB/月まで無料
- **Class A操作** (PUT, COPY, POST, LIST): 1,000,000回/月まで無料
- **Class B操作** (GET, HEAD): 10,000,000回/月まで無料
- **データ転送**: Cloudflareネットワーク内は無料

### 5.2 コスト最適化のヒント

- 不要なファイルの定期削除
- キャッシュ機能の活用
- 適切なファイルサイズ制限の設定

## 6. 次のステップ

R2の設定が完了したら、[環境変数設定ガイド](./ENVIRONMENT_SETUP.md)に従ってアプリケーションの設定を行ってください。

## 参考リンク

- [Cloudflare R2 公式ドキュメント](https://developers.cloudflare.com/r2/)
- [R2 API リファレンス](https://developers.cloudflare.com/r2/api/)
- [Wrangler CLI ドキュメント](https://developers.cloudflare.com/workers/wrangler/)
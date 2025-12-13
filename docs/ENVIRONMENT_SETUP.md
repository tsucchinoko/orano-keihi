# 環境変数設定ガイド

このガイドでは、Cloudflare R2機能を使用するために必要な環境変数の設定方法を説明します。

## 必要な環境変数

アプリケーションでR2機能を使用するには、以下の環境変数を設定する必要があります：

| 変数名 | 説明 | 必須 | 例 |
|--------|------|------|-----|
| `R2_ACCOUNT_ID` | CloudflareアカウントID | ✅ | `abc123def456ghi789` |
| `R2_ACCESS_KEY` | R2 APIアクセスキー | ✅ | `your_access_key` |
| `R2_SECRET_KEY` | R2 APIシークレットキー | ✅ | `your_secret_key` |
| `R2_BUCKET_NAME` | 使用するR2バケット名 | ✅ | `expense-receipts-dev` |
| `R2_REGION` | R2リージョン | ❌ | `auto`（デフォルト） |

## 設定方法

### 1. 環境変数ファイルの作成

プロジェクトルートの`src-tauri`ディレクトリに`.env`ファイルを作成します：

```bash
cd src-tauri
cp .env.example .env
```

### 2. 環境変数の設定

`.env`ファイルを編集して、R2の認証情報を設定します：

```bash
# Cloudflare R2 設定
R2_ACCOUNT_ID=your_account_id_here
R2_ACCESS_KEY=your_access_key_here
R2_SECRET_KEY=your_secret_key_here
R2_BUCKET_NAME=expense-receipts-dev
R2_REGION=auto
```

### 3. 値の取得方法

#### R2_ACCOUNT_ID
1. [Cloudflare Dashboard](https://dash.cloudflare.com/)にログイン
2. 右サイドバーでアカウントIDを確認
3. または「R2 Object Storage」ページで確認

#### R2_ACCESS_KEY と R2_SECRET_KEY
1. Cloudflare Dashboardで「My Profile」→「API Tokens」に移動
2. 「Create Token」をクリック
3. 「R2 Token」テンプレートを選択、または以下の権限でカスタムトークンを作成：
   - `Account` - `Cloudflare R2:Edit`
4. 生成されたトークンをアクセスキーとして使用
5. シークレットキーは同じトークンを使用（R2はトークンベース認証）

#### R2_BUCKET_NAME
- [R2設定ガイド](./R2_SETUP.md)で作成したバケット名を使用

## 環境別設定

### 開発環境

```bash
# 開発環境用設定
R2_ACCOUNT_ID=your_account_id
R2_ACCESS_KEY=your_dev_access_key
R2_SECRET_KEY=your_dev_secret_key
R2_BUCKET_NAME=expense-receipts-dev
R2_REGION=auto
```

### 本番環境

```bash
# 本番環境用設定
R2_ACCOUNT_ID=your_account_id
R2_ACCESS_KEY=your_prod_access_key
R2_SECRET_KEY=your_prod_secret_key
R2_BUCKET_NAME=expense-receipts-prod
R2_REGION=auto
```

## セキュリティのベストプラクティス

### 1. 環境変数ファイルの保護

```bash
# .envファイルを.gitignoreに追加（既に追加済み）
echo ".env" >> .gitignore

# ファイル権限を制限
chmod 600 src-tauri/.env
```

### 2. 本番環境での設定

本番環境では、以下の方法で環境変数を設定することを推奨：

- **CI/CD環境**: GitHub Secrets、GitLab Variables等
- **サーバー環境**: システム環境変数、Docker secrets等
- **デスクトップアプリ**: 設定ファイルの暗号化

### 3. 認証情報のローテーション

- 定期的にAPIトークンを更新
- 古いトークンは無効化
- アクセスログの監視

## 設定の検証

### 1. 接続テスト

アプリケーション内でR2接続テストを実行：

```bash
# 開発サーバーを起動
pnpm tauri dev

# アプリケーション内の「設定」→「R2接続テスト」を実行
```

### 2. 手動検証

Wrangler CLIを使用した手動検証：

```bash
# Wrangler CLIでの接続確認
wrangler r2 bucket list

# 特定のバケットの確認
wrangler r2 object list expense-receipts-dev
```

## トラブルシューティング

### よくあるエラー

#### 1. 認証エラー
```
Error: Invalid credentials
```
**解決方法**: アクセスキーとシークレットキーを再確認

#### 2. バケットが見つからない
```
Error: Bucket not found
```
**解決方法**: バケット名とアカウントIDを確認

#### 3. 権限エラー
```
Error: Access denied
```
**解決方法**: APIトークンの権限設定を確認

詳細なトラブルシューティングについては、[トラブルシューティングガイド](./TROUBLESHOOTING.md)を参照してください。

## 次のステップ

環境変数の設定が完了したら、アプリケーションを起動してR2機能をテストしてください：

```bash
# 開発サーバーの起動
pnpm tauri dev
```

## 参考リンク

- [Cloudflare R2 認証ドキュメント](https://developers.cloudflare.com/r2/api/s3/tokens/)
- [環境変数のセキュリティ](https://12factor.net/config)
- [Tauri 環境変数ガイド](https://tauri.app/v1/guides/building/environment-variables/)
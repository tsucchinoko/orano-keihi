# D1データベースセットアップガイド

このドキュメントでは、Cloudflare D1データベースのセットアップ手順を説明します。

## 前提条件

- Cloudflareアカウントを持っていること
- wrangler CLIがインストールされていること
- wranglerでCloudflareアカウントにログインしていること

## セットアップ手順

### 1. D1データベースの作成

#### 開発環境用データベースの作成

```bash
cd packages/api-server
wrangler d1 create orano-keihi-dev-db
```

コマンド実行後、以下のような出力が表示されます：

```
✅ Successfully created DB 'orano-keihi-dev-db'!

[[d1_databases]]
binding = "DB"
database_name = "orano-keihi-dev-db"
database_id = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
```

**重要**: 出力された`database_id`をコピーして、`wrangler.toml`の開発環境設定の`YOUR_DEVELOPMENT_DATABASE_ID`を置き換えてください。

#### 本番環境用データベースの作成

```bash
wrangler d1 create orano-keihi-prod-db
```

コマンド実行後、以下のような出力が表示されます：

```
✅ Successfully created DB 'orano-keihi-prod-db'!

[[d1_databases]]
binding = "DB"
database_name = "orano-keihi-prod-db"
database_id = "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy"
```

**重要**: 出力された`database_id`をコピーして、`wrangler.toml`の本番環境設定の`YOUR_PRODUCTION_DATABASE_ID`を置き換えてください。

### 2. wrangler.tomlの更新

`packages/api-server/wrangler.toml`を開き、以下の箇所を更新します：

```toml
# D1データベースのバインディング
[[env.production.d1_databases]]
binding = "DB"
database_name = "orano-keihi-prod-db"
database_id = "yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy"  # ← 本番環境のdatabase_idに置き換え

[[env.development.d1_databases]]
binding = "DB"
database_name = "orano-keihi-dev-db"
database_id = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"  # ← 開発環境のdatabase_idに置き換え
```

### 3. データベーススキーマの適用

#### 開発環境へのマイグレーション

```bash
wrangler d1 execute orano-keihi-dev-db --file=./schema.sql --env development
```

実行結果の確認：

```bash
wrangler d1 execute orano-keihi-dev-db --command="SELECT name FROM sqlite_master WHERE type='table';" --env development
```

#### 本番環境へのマイグレーション

```bash
wrangler d1 execute orano-keihi-prod-db --file=./schema.sql --env production
```

実行結果の確認：

```bash
wrangler d1 execute orano-keihi-prod-db --command="SELECT name FROM sqlite_master WHERE type='table';" --env production
```

### 4. データベース接続の確認

テーブルが正しく作成されたことを確認します：

```bash
# 開発環境
wrangler d1 execute orano-keihi-dev-db --command="SELECT name FROM sqlite_master WHERE type='table';" --env development

# 本番環境
wrangler d1 execute orano-keihi-prod-db --command="SELECT name FROM sqlite_master WHERE type='table';" --env production
```

以下のテーブルが表示されることを確認してください：

- users
- expenses
- subscriptions

### 5. インデックスの確認

インデックスが正しく作成されたことを確認します：

```bash
# 開発環境
wrangler d1 execute orano-keihi-dev-db --command="SELECT name FROM sqlite_master WHERE type='index';" --env development

# 本番環境
wrangler d1 execute orano-keihi-prod-db --command="SELECT name FROM sqlite_master WHERE type='index';" --env production
```

## トラブルシューティング

### データベースが見つからない場合

```bash
# データベース一覧を確認
wrangler d1 list
```

### スキーマの再適用が必要な場合

```bash
# テーブルを削除してから再作成
wrangler d1 execute orano-keihi-dev-db --command="DROP TABLE IF EXISTS subscriptions; DROP TABLE IF EXISTS expenses; DROP TABLE IF EXISTS users;" --env development
wrangler d1 execute orano-keihi-dev-db --file=./schema.sql --env development
```

### データベースの削除（注意: データが失われます）

```bash
# 開発環境
wrangler d1 delete orano-keihi-dev-db

# 本番環境
wrangler d1 delete orano-keihi-prod-db
```

## 次のステップ

D1データベースのセットアップが完了したら、次のタスクに進んでください：

- TypeScript型定義とDTOの作成
- リポジトリ層の実装
- APIエンドポイントの実装

## 参考リンク

- [Cloudflare D1 Documentation](https://developers.cloudflare.com/d1/)
- [Wrangler CLI Documentation](https://developers.cloudflare.com/workers/wrangler/)

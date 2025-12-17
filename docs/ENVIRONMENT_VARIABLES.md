# 環境変数設定ガイド

## 概要

このアプリケーションでは、`ENVIRONMENT`環境変数に基づいて自動的に適切な環境変数ファイルを読み込みます。

## 環境変数ファイル

### 開発環境
- ファイル: `src-tauri/.env`
- 用途: 開発時（`pnpm tauri dev`）に使用
- 条件: `ENVIRONMENT=development`または未設定時

### 本番環境
- ファイル: `src-tauri/.env.production`
- 用途: 本番ビルド時（`pnpm tauri:build:production`）に使用
- 条件: `ENVIRONMENT=production`時

## 環境の自動判定

アプリケーション起動時に以下の順序で環境変数ファイルを読み込みます：

1. `ENVIRONMENT`環境変数をチェック
2. `production`の場合 → `.env.production`を読み込み
3. `development`または未設定の場合 → `.env`を読み込み
4. 指定ファイルが見つからない場合 → デフォルトの`.env`を試行

## 設定項目

```bash
# CloudflareアカウントID
R2_ACCOUNT_ID=your_account_id_here

# R2 APIトークンのアクセスキー
R2_ACCESS_KEY=your_access_key_here

# R2 APIトークンのシークレットキー
R2_SECRET_KEY=your_secret_key_here

# R2バケット名
R2_BUCKET_NAME=your_bucket_name_here

# R2リージョン（通常は"auto"）
R2_REGION=auto
```

## ビルドコマンド

### 開発環境でのビルド
```bash
# 開発環境の設定を使用
pnpm tauri:build
```

### 本番環境でのビルド
```bash
# 本番環境の設定を使用
pnpm tauri:build:production

# DMGファイルも作成する場合
pnpm tauri:build:dmg:production
```

## 環境変数の確認方法

### 開発環境での確認
```bash
# 開発環境で起動（.envを読み込み）
pnpm tauri dev
```

### 本番環境設定での確認
```bash
# 本番環境設定で起動（.env.productionを読み込み）
ENVIRONMENT=production pnpm tauri dev
```

アプリケーションのログで以下のような出力が確認できます：
```
[INFO] 環境: production, 読み込み対象: .env.production
[INFO] .env.productionファイルを読み込みました
[DEBUG] 環境変数 R2_ACCOUNT_ID = d6392b1230a419b37b30f45fc13de9cf
[DEBUG] 環境変数 R2_BUCKET_NAME = orano-keihi-prod
```

## フロントエンドでの環境変数使用

TypeScriptコード内で環境変数を使用する場合：

```typescript
// 環境変数にアクセス
const accountId = import.meta.env.R2_ACCOUNT_ID;
const bucketName = import.meta.env.R2_BUCKET_NAME;
```

## 注意事項

1. **セキュリティ**: 本番環境の環境変数ファイルには実際の認証情報を含めないでください
2. **Git管理**: `.env.production`ファイルは`.gitignore`により除外されています
3. **型安全性**: `src/app.d.ts`で環境変数の型定義を行っています
4. **ビルド時読み込み**: 環境変数はビルド時に読み込まれ、実行時には変更できません
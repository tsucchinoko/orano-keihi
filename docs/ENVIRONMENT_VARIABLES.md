# 環境変数設定ガイド

## 概要

このアプリケーションでは、開発環境と本番環境で異なる環境変数を使用できます。

## 環境変数ファイル

### 開発環境
- ファイル: `src-tauri/.env`
- 用途: 開発時（`pnpm tauri dev`）に使用

### 本番環境
- ファイル: `src-tauri/.env.production`
- 用途: 本番ビルド時（`pnpm tauri:build:production`）に使用

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
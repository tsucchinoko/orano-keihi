# APIサーバー

TypeScriptとHonoを使用したファイルアップロードAPIサーバーです。デスクトップアプリケーションからのファイルアップロード処理を集約化し、Cloudflare R2ストレージとの連携を行います。

## 機能

- ファイルアップロードAPI（単一・複数ファイル対応）
- 認証・認可システム
- レート制限
- 構造化ログ
- エラーハンドリング
- ヘルスチェック

## 技術スタック

- **フレームワーク**: Hono
- **言語**: TypeScript
- **ストレージ**: Cloudflare R2
- **ログ**: Winston
- **バリデーション**: Zod
- **テスト**: Vitest + fast-check

## セットアップ

### 1. 依存関係のインストール

```bash
pnpm install
```

### 2. 環境変数の設定

`.env.example`をコピーして`.env`ファイルを作成し、必要な値を設定してください。

```bash
cp .env.example .env
```

### 3. 開発サーバーの起動

```bash
pnpm dev
```

サーバーは `http://localhost:3000` で起動します。

## スクリプト

- `pnpm dev` - 開発サーバーの起動（ホットリロード）
- `pnpm build` - プロダクションビルド
- `pnpm start` - プロダクションサーバーの起動
- `pnpm test` - テストの実行
- `pnpm test:watch` - テストの監視実行
- `pnpm check` - TypeScript型チェック
- `pnpm lint` - コードリンティング
- `pnpm fmt` - コードフォーマット

## API エンドポイント

### ヘルスチェック

```
GET /api/v1/health
```

サーバーの状態を確認します。

## 開発

### プロジェクト構造

```
src/
├── app.ts              # Honoアプリケーション設定
├── index.ts            # エントリーポイント
├── config/
│   └── environment.ts  # 環境変数設定
├── types/
│   └── config.ts       # 型定義
└── utils/
    └── logger.ts       # ログシステム

tests/
├── setup.ts            # テストセットアップ
└── **/*.test.ts        # テストファイル
```

### テスト

ユニットテストとプロパティベーステストの両方を使用しています：

- **ユニットテスト**: 具体的な例とエッジケースのテスト
- **プロパティベーステスト**: 汎用的な正確性プロパティのテスト（fast-check使用）

```bash
# テスト実行
pnpm test

# テスト監視
pnpm test:watch
```

## 環境変数

主要な環境変数：

- `PORT`: サーバーポート（デフォルト: 3000）
- `R2_ENDPOINT`: Cloudflare R2エンドポイント
- `R2_ACCESS_KEY_ID`: R2アクセスキーID
- `R2_SECRET_ACCESS_KEY`: R2シークレットアクセスキー
- `R2_BUCKET_NAME`: R2バケット名
- `JWT_SECRET`: JWT署名用シークレット

詳細は `.env.example` を参照してください。

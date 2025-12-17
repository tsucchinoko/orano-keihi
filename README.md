# Subscription Memo

Tauri + SvelteKit + TypeScript で構築されたサブスクリプション管理アプリケーションです。

## 開発環境のセットアップ

### 必要な環境

- [Node.js](https://nodejs.org/) (v18以上)
- [pnpm](https://pnpm.io/) (パッケージマネージャー)
- [Rust](https://rustup.rs/) (Tauriビルド用)

### インストール手順

1. リポジトリをクローン
```bash
git clone <repository-url>
cd subscription-memo
```

2. フロントエンド依存関係をインストール
```bash
pnpm install
```

3. 開発サーバーを起動
```bash
pnpm dev
```

### 利用可能なコマンド

#### フロントエンド開発
- `pnpm dev` - 開発サーバーの起動
- `pnpm build` - プロダクションビルド
- `pnpm preview` - ビルド結果のプレビュー
- `pnpm check` - TypeScript型チェック
- `pnpm check:watch` - TypeScript型チェック（ウォッチモード）

#### コード品質
- `pnpm fmt` - コードフォーマット（自動修正）
- `pnpm fmt:check` - コードフォーマットチェック
- `pnpm lint` - コードリンティング（自動修正）
- `pnpm lint:check` - コードリンティングチェック

#### Tauriアプリケーション
- `pnpm tauri dev` - Tauriアプリケーションの開発モード
- `pnpm tauri build` - Tauriアプリケーションのビルド

## 推奨IDE設定

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## プロジェクト構成

```
├── src/                    # フロントエンドソースコード
│   ├── features/          # 機能別コンポーネント
│   ├── lib/               # 共通ライブラリ
│   └── routes/            # SvelteKitルート
├── src-tauri/             # Tauriバックエンド（Rust）
├── static/                # 静的ファイル
├── package.json           # Node.js依存関係とスクリプト
├── pnpm-lock.yaml         # pnpmロックファイル
└── .pnpmrc               # pnpm設定
```

## 技術スタック

- **フロントエンド**: SvelteKit + TypeScript + Vite
- **バックエンド**: Tauri (Rust)
- **ストレージ**: Cloudflare R2 (領収書保存)
- **データベース**: SQLite
- **パッケージマネージャー**: pnpm
- **コード品質**: Biome (フォーマット・リンティング)
- **CI/CD**: GitHub Actions

## Cloudflare R2 設定

このアプリケーションは領収書の保存にCloudflare R2を使用します。R2機能を使用するには、以下のドキュメントを参照して設定を行ってください：

1. **[R2設定ガイド](./docs/R2_SETUP.md)** - Cloudflare R2の初期設定
2. **[環境変数設定ガイド](./docs/ENVIRONMENT_SETUP.md)** - 必要な環境変数の設定方法
3. **[トラブルシューティングガイド](./docs/TROUBLESHOOTING.md)** - よくある問題と解決方法

### クイックスタート

R2機能を使用するための最小限の設定：

```bash
# 1. src-tauri/.envファイルを作成
cp src-tauri/.env.example src-tauri/.env

# 2. .envファイルにR2認証情報を設定
# R2_ACCOUNT_ID=your_account_id
# R2_ACCESS_KEY=your_access_key  
# R2_SECRET_KEY=your_secret_key
# R2_BUCKET_NAME=expense-receipts-dev

# 3. アプリケーションを起動
pnpm tauri dev
```

詳細な設定手順については、上記のドキュメントを参照してください。

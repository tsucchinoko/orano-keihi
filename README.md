# Subscription Memo

Tauri + SvelteKit + TypeScript で構築されたサブスクリプション管理アプリケーションです。

## プロジェクト構造

このプロジェクトはpnpmワークスペースを使用したモノレポ構成になっています：

```
├── packages/
│   └── desktop/           # デスクトップアプリケーション（Tauri + SvelteKit）
├── package.json           # ワークスペース管理用
├── pnpm-workspace.yaml    # pnpmワークスペース設定
└── README.md             # このファイル
```

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

2. 依存関係をインストール（ワークスペース全体）
```bash
pnpm install
```

3. 開発サーバーを起動
```bash
pnpm dev
```

### 利用可能なコマンド

すべてのコマンドはワークスペースルートから実行できます：

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
- `pnpm tauri:dev` - Tauriアプリケーションの開発モード
- `pnpm tauri:build` - Tauriアプリケーションのビルド
- `pnpm tauri:build:dmg` - macOS DMGファイルのビルド

#### 個別パッケージでの作業
デスクトップアプリケーションディレクトリで直接作業する場合：
```bash
cd packages/desktop
pnpm dev          # 開発サーバー起動
pnpm tauri dev    # Tauriアプリケーション起動
```

## 推奨IDE設定

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## プロジェクト構成

```
├── packages/
│   └── desktop/           # デスクトップアプリケーション
│       ├── src/           # フロントエンドソースコード
│       │   ├── features/  # 機能別コンポーネント
│       │   ├── lib/       # 共通ライブラリ
│       │   └── routes/    # SvelteKitルート
│       ├── src-tauri/     # Tauriバックエンド（Rust）
│       ├── static/        # 静的ファイル
│       └── package.json   # デスクトップアプリの依存関係
├── package.json           # ワークスペース管理用
├── pnpm-workspace.yaml    # ワークスペース設定
└── pnpm-lock.yaml         # 依存関係ロックファイル
```

## 技術スタック

- **フロントエンド**: SvelteKit + TypeScript + Vite
- **バックエンド**: Tauri (Rust)
- **ストレージ**: Cloudflare R2 (領収書保存)
- **データベース**: SQLite
- **パッケージマネージャー**: pnpm
- **コード品質**: Biome (フォーマット・リンティング)
- **CI/CD**: GitHub Actions (クロスプラットフォーム自動ビルド・リリース)
- **コード署名**: MacOS (Developer ID) + Windows (Code Signing Certificate)

## Cloudflare R2 設定

このアプリケーションは領収書の保存にCloudflare R2を使用します。R2機能を使用するには、以下のドキュメントを参照して設定を行ってください：

1. **[R2設定ガイド](./docs/R2_SETUP.md)** - Cloudflare R2の初期設定
2. **[環境変数設定ガイド](./docs/ENVIRONMENT_SETUP.md)** - 必要な環境変数の設定方法
3. **[トラブルシューティングガイド](./docs/TROUBLESHOOTING.md)** - よくある問題と解決方法

### クイックスタート

R2機能を使用するための最小限の設定：

```bash
# 1. packages/desktop/src-tauri/.envファイルを作成
cp packages/desktop/src-tauri/.env.example packages/desktop/src-tauri/.env

# 2. .envファイルにR2認証情報を設定
# R2_ACCOUNT_ID=your_account_id
# R2_ACCESS_KEY=your_access_key  
# R2_SECRET_KEY=your_secret_key
# R2_BUCKET_NAME=expense-receipts-dev

# 3. アプリケーションを起動
pnpm tauri:dev
```

詳細な設定手順については、上記のドキュメントを参照してください。

## コード署名設定

このプロジェクトは、MacOSとWindows向けのアプリケーションにコード署名を適用してセキュリティを向上させています。

### 署名済みアプリケーションの利点

- **セキュリティ警告の軽減**: ユーザーがアプリケーションをダウンロード・実行する際の警告が軽減されます
- **信頼性の向上**: 署名により、アプリケーションの発行者が検証可能になります
- **改ざん検出**: ファイルが改ざんされていないことを確認できます

### 開発者向け署名設定

コード署名を設定する場合は、以下のドキュメントを参照してください：

- **[コード署名設定ガイド](./docs/CODE_SIGNING_SETUP.md)** - MacOSとWindows向けの署名設定方法

### 署名検証

ダウンロードしたアプリケーションの署名を検証するには：

#### MacOS
```bash
# アプリケーションの署名確認
codesign -dv --verbose=4 /Applications/YourApp.app

# Gatekeeperの検証
spctl -a -vv /Applications/YourApp.app

# 検証スクリプトの使用
./script/verify_signatures.sh path/to/app.dmg
```

#### Windows
```powershell
# PowerShellでの署名確認
Get-AuthenticodeSignature "C:\path\to\app.exe"

# 検証スクリプトの使用
.\script\verify_signatures.ps1 -MsiFile "C:\path\to\app.msi"
```

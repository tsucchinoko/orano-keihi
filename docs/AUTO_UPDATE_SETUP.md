# 自動アップデート機能セットアップガイド

このドキュメントでは、Tauriアプリケーションの自動アップデート機能の設定と使用方法について説明します。

## 概要

自動アップデート機能により、ユーザーは新しいバージョンがリリースされた際に自動的に通知を受け取り、ワンクリックでアップデートを適用できます。

## アーキテクチャ

### コンポーネント構成

1. **Tauriバックエンド（Rust）**
   - `tauri-plugin-updater`: Tauriの公式アップデータープラグイン
   - アップデートチェック、ダウンロード、インストール機能
   - バックグラウンドでの定期チェック

2. **フロントエンド（SvelteKit + TypeScript）**
   - アップデート通知UI
   - ユーザーインタラクション処理
   - 進捗表示

3. **アップデート配信サーバー（Cloudflare Workers）**
   - アップデート情報の配信
   - プラットフォーム別のバイナリ配信
   - バージョン管理

## セットアップ手順

### 1. Tauriプラグインの設定

#### Cargo.toml
```toml
[dependencies]
tauri-plugin-updater = "2"
```

#### tauri.conf.json
```json
{
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://orano-keihi.tsucchinoko.workers.dev/api/updater/{{target}}/{{arch}}/{{current_version}}"
      ],
      "dialog": true,
      "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDhEQzY4M0Y5RkY4NzU5MkUKUldSVE1qVXhNVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeA=="
    }
  }
}
```

### 2. Rustコードの実装

#### アップデートサービス（`src/features/updater/service.rs`）
- アップデートチェック機能
- ダウンロード・インストール機能
- 自動チェック機能

#### Tauriコマンド（`src/features/updater/commands.rs`）
- `check_for_updates`: アップデートチェック
- `download_and_install_update`: アップデートインストール
- `get_app_version`: 現在のバージョン取得
- `start_auto_update_check`: 自動チェック開始

### 3. フロントエンドの実装

#### TypeScript型定義（`src/lib/types/updater.ts`）
```typescript
export interface UpdateInfo {
  available: boolean;
  current_version: string;
  latest_version?: string;
  release_notes?: string;
  content_length?: number;
  last_checked: number;
}
```

#### アップデートサービス（`src/lib/services/updater.ts`）
- Tauriコマンドのラッパー
- イベントリスナー管理
- ユーティリティ関数

#### UI コンポーネント（`src/lib/components/UpdateNotification.svelte`）
- アップデート通知モーダル
- ダウンロード進捗表示
- ユーザーアクション処理

### 4. アップデート配信サーバー

#### Cloudflare Workers（`packages/api-server/src/routes/updater.ts`）
- アップデート情報API
- プラットフォーム別配信
- バージョン比較ロジック

#### エンドポイント
- `GET /api/updater/{target}/{arch}/{current_version}`: アップデートチェック
- `GET /api/updater/latest`: 最新バージョン情報
- `GET /api/updater/health`: ヘルスチェック

## 使用方法

### 開発環境でのテスト

1. **デバッグページでのテスト**
   ```
   http://localhost:1420/debug
   ```
   - アップデートチェック機能
   - 現在のバージョン表示
   - 手動アップデート実行

2. **自動通知のテスト**
   - アプリケーション起動時に自動チェック
   - 24時間間隔での定期チェック
   - 新しいバージョンが利用可能な場合の通知表示

### 本番環境での運用

1. **リリースプロセス**
   - 新しいバージョンのビルド
   - 署名付きバイナリの生成
   - アップデート配信サーバーの更新

2. **ユーザーエクスペリエンス**
   - 自動的なアップデート通知
   - ワンクリックでのアップデート適用
   - アプリケーションの自動再起動

## 設定項目

### アップデートチェック間隔
```typescript
// デフォルト: 24時間
await UpdaterService.startAutoUpdateCheck(24);
```

### 通知設定
- `dialog: true`: Tauriの標準ダイアログを使用
- カスタムUI: Svelteコンポーネントでの通知

### セキュリティ
- 公開鍵による署名検証
- HTTPS通信の強制
- バイナリの整合性チェック

## トラブルシューティング

### よくある問題

1. **アップデートチェックが失敗する**
   - ネットワーク接続の確認
   - アップデート配信サーバーの状態確認
   - 設定ファイルの確認

2. **署名検証エラー**
   - 公開鍵の設定確認
   - バイナリの署名状態確認

3. **ダウンロードが進まない**
   - ファイアウォール設定の確認
   - プロキシ設定の確認

### ログの確認

```rust
// Rustログ
log::info!("アップデートチェック中...");

// フロントエンドログ
console.log('アップデート情報:', updateInfo);
```

### デバッグモード

開発環境では詳細なログが出力されます：
- アップデートチェックの詳細
- ダウンロード進捗
- エラーの詳細情報

## セキュリティ考慮事項

1. **署名検証**
   - すべてのアップデートファイルは署名が必要
   - 公開鍵による検証を実施

2. **HTTPS通信**
   - アップデート情報の取得はHTTPS必須
   - バイナリダウンロードもHTTPS必須

3. **権限管理**
   - インストール時の適切な権限要求
   - ユーザーの明示的な同意

## パフォーマンス最適化

1. **キャッシュ機能**
   - アップデート情報のキャッシュ
   - 不要なチェックの削減

2. **バックグラウンド処理**
   - UIをブロックしない非同期処理
   - 進捗表示による UX 向上

3. **帯域幅の最適化**
   - 差分アップデートの検討
   - 圧縮ファイルの使用

## 今後の拡張予定

1. **差分アップデート**
   - 全体ダウンロードではなく差分のみ
   - ダウンロード時間の短縮

2. **ロールバック機能**
   - 問題のあるアップデートの自動ロールバック
   - 以前のバージョンへの復元

3. **A/Bテスト**
   - 段階的なロールアウト
   - ユーザーグループ別の配信制御

## 関連ファイル

### Rust
- `packages/desktop/src-tauri/src/features/updater/`
- `packages/desktop/src-tauri/Cargo.toml`
- `packages/desktop/src-tauri/tauri.conf.json`

### TypeScript/Svelte
- `packages/desktop/src/lib/types/updater.ts`
- `packages/desktop/src/lib/services/updater.ts`
- `packages/desktop/src/lib/components/UpdateNotification.svelte`

### API サーバー
- `packages/api-server/src/routes/updater.ts`
- `packages/api-server/src/app.ts`

### ドキュメント
- `docs/AUTO_UPDATE_SETUP.md`（このファイル）
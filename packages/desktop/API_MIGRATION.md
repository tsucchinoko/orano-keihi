# API Server移行ガイド

## 概要

デスクトップアプリケーションの経費とサブスクリプション機能が、ローカルSQLiteからAPI Server（Cloudflare D1）経由でのデータ管理に移行されました。

## 変更内容

### アーキテクチャの変更

**以前:**

```
Tauri App (Rust) → ローカルSQLite
```

**現在:**

```
Tauri App (Rust) → API Server (Cloudflare Workers) → D1 Database
```

### 主な変更点

1. **経費機能**
   - `packages/desktop/src-tauri/src/features/expenses/api_commands.rs` - API Server経由のコマンド実装
   - すべての経費操作（作成、取得、更新、削除）がAPI経由で実行されます

2. **サブスクリプション機能**
   - `packages/desktop/src-tauri/src/features/subscriptions/api_commands.rs` - API Server経由のコマンド実装
   - すべてのサブスクリプション操作がAPI経由で実行されます

3. **API クライアント**
   - `packages/desktop/src-tauri/src/shared/api_client.rs` - 汎用APIクライアント
   - リトライ機能、エラーハンドリング、認証トークン管理を含む

## 環境設定

### API ServerのURL設定

デフォルトでは`http://localhost:8787`（Cloudflare Workersのデフォルトポート）に接続します。

環境変数で変更可能：

```bash
# API ServerのURL
export API_SERVER_URL="http://localhost:8787"

# タイムアウト（秒）
export API_TIMEOUT_SECONDS=30

# 最大リトライ回数
export API_MAX_RETRIES=3
```

## 開発環境でのテスト

### 1. API Serverを起動

```bash
cd packages/api-server
pnpm dev
```

API Serverは`http://localhost:8787`で起動します。

### 2. デスクトップアプリを起動

```bash
cd packages/desktop
pnpm tauri dev
```

## 動作確認

### 経費機能

1. ログイン後、経費一覧ページに移動
2. 新しい経費を作成
3. 経費の編集・削除を試行
4. 月別フィルターとカテゴリフィルターを試行

### サブスクリプション機能

1. サブスクリプション一覧ページに移動
2. 新しいサブスクリプションを作成
3. サブスクリプションの編集・削除を試行
4. アクティブ/非アクティブの切り替えを試行
5. 月額合計が正しく計算されることを確認

## トラブルシューティング

### API Serverに接続できない

**症状:** 経費やサブスクリプションの操作時に「APIサーバーへの接続に失敗しました」エラーが表示される

**解決方法:**

1. API Serverが起動していることを確認
   ```bash
   curl http://localhost:8787/api/v1/health
   ```
2. 環境変数`API_SERVER_URL`が正しく設定されていることを確認
3. ファイアウォールやセキュリティソフトがポート8787をブロックしていないか確認

### 認証エラー

**症状:** 「認証エラー」が表示される

**解決方法:**

1. 再度ログインを試行
2. セッショントークンが有効であることを確認
3. API Serverのログを確認して詳細なエラーメッセージを確認

### データが表示されない

**症状:** 経費やサブスクリプションの一覧が空

**解決方法:**

1. ブラウザの開発者ツールでネットワークタブを確認
2. API Serverのレスポンスを確認
3. D1データベースにデータが存在することを確認
   ```bash
   cd packages/api-server
   wrangler d1 execute orano-keihi-dev-db --command="SELECT * FROM expenses LIMIT 10;" --env development
   ```

## API エンドポイント

### 経費API

- `POST /api/v1/expenses` - 経費作成
- `GET /api/v1/expenses` - 経費一覧取得
- `GET /api/v1/expenses/:id` - 経費取得
- `PUT /api/v1/expenses/:id` - 経費更新
- `DELETE /api/v1/expenses/:id` - 経費削除

### サブスクリプションAPI

- `POST /api/v1/subscriptions` - サブスクリプション作成
- `GET /api/v1/subscriptions` - サブスクリプション一覧取得
- `GET /api/v1/subscriptions/:id` - サブスクリプション取得
- `PUT /api/v1/subscriptions/:id` - サブスクリプション更新
- `PATCH /api/v1/subscriptions/:id/toggle` - ステータス切り替え
- `DELETE /api/v1/subscriptions/:id` - サブスクリプション削除
- `GET /api/v1/subscriptions/monthly-total` - 月額合計取得

詳細は`packages/api-server/EXPENSE_ENDPOINTS.md`を参照してください。

## 今後の改善点

1. **オフライン対応**
   - ローカルキャッシュの実装
   - オフライン時の操作をキューに保存し、オンライン復帰時に同期

2. **パフォーマンス最適化**
   - データのページネーション
   - レスポンスキャッシュ
   - 楽観的UI更新

3. **エラーハンドリングの改善**
   - より詳細なエラーメッセージ
   - リトライ戦略の最適化
   - ユーザーフレンドリーなエラー表示

## 参考資料

- [D1データベース移行仕様](.kiro/specs/d1-database-migration/)
- [API Server実装](packages/api-server/)
- [Cloudflare D1ドキュメント](https://developers.cloudflare.com/d1/)

# トラブルシューティングガイド

このガイドでは、Cloudflare R2機能使用時によく発生する問題とその解決方法を説明します。

## 目次

1. [接続関連の問題](#接続関連の問題)
2. [認証関連の問題](#認証関連の問題)
3. [アップロード関連の問題](#アップロード関連の問題)
4. [ダウンロード関連の問題](#ダウンロード関連の問題)
5. [パフォーマンス関連の問題](#パフォーマンス関連の問題)
6. [設定関連の問題](#設定関連の問題)
7. [デバッグ方法](#デバッグ方法)

## 接続関連の問題

### 問題: R2接続テストが失敗する

**症状**:
```
Error: R2 connection failed: Connection timeout
```

**原因と解決方法**:

1. **ネットワーク接続の確認**
   ```bash
   # インターネット接続の確認
   ping cloudflare.com
   
   # R2エンドポイントの確認
   curl -I https://your-account-id.r2.cloudflarestorage.com
   ```

2. **ファイアウォール設定の確認**
   - 企業ネットワークの場合、HTTPS (443ポート) の通信が許可されているか確認
   - プロキシ設定が必要な場合は、システム環境変数を設定

3. **エンドポイントURLの確認**
   ```rust
   // 正しいエンドポイント形式
   https://{account_id}.r2.cloudflarestorage.com
   ```

### 問題: 断続的な接続エラー

**症状**:
```
Error: Network error: Connection reset by peer
```

**解決方法**:
1. **リトライ機能の確認**
   - アプリケーションのリトライ設定を確認
   - 最大3回のリトライが設定されているか確認

2. **タイムアウト設定の調整**
   ```rust
   // タイムアウト設定の例
   let timeout = Duration::from_secs(30);
   ```

## 認証関連の問題

### 問題: 認証情報が無効

**症状**:
```
Error: Invalid credentials
Error: Access denied
```

**解決手順**:

1. **環境変数の確認**
   ```bash
   # 環境変数が設定されているか確認
   echo $R2_ACCOUNT_ID
   echo $R2_ACCESS_KEY
   echo $R2_BUCKET_NAME
   
   # 注意: SECRET_KEYは表示しない
   ```

2. **APIトークンの確認**
   - Cloudflare Dashboardでトークンが有効か確認
   - トークンの権限設定を確認（`Cloudflare R2:Edit`が必要）
   - トークンの有効期限を確認

3. **アカウントIDの確認**
   - Cloudflare DashboardでアカウントIDを再確認
   - 32文字の英数字文字列であることを確認

### 問題: 権限不足エラー

**症状**:
```
Error: You don't have permission to access this resource
```

**解決方法**:
1. **APIトークンの権限を確認**
   - `Account` - `Cloudflare R2:Edit` 権限が必要
   - 必要に応じて `Cloudflare R2:Read` も追加

2. **バケットレベルの権限確認**
   - 特定のバケットに対する権限が設定されているか確認

## アップロード関連の問題

### 問題: ファイルアップロードが失敗する

**症状**:
```
Error: Upload failed: File too large
Error: Upload failed: Invalid file format
```

**解決手順**:

1. **ファイルサイズの確認**
   ```bash
   # ファイルサイズを確認（10MB制限）
   ls -lh /path/to/file
   ```

2. **ファイル形式の確認**
   - 対応形式: PNG, JPG, JPEG, PDF
   - ファイル拡張子とMIMEタイプが一致しているか確認

3. **メモリ使用量の確認**
   ```bash
   # システムメモリの確認
   free -h  # Linux/macOS
   ```

### 問題: アップロードが途中で停止する

**症状**:
- プログレスバーが途中で止まる
- タイムアウトエラーが発生

**解決方法**:
1. **ネットワーク安定性の確認**
   - Wi-Fi接続の場合、有線接続を試す
   - 他のネットワーク集約的なアプリケーションを停止

2. **チャンクサイズの調整**
   ```rust
   // より小さなチャンクサイズを使用
   const CHUNK_SIZE: usize = 1024 * 1024; // 1MB
   ```

## ダウンロード関連の問題

### 問題: 領収書が表示されない

**症状**:
```
Error: Failed to load receipt
Error: File not found
```

**解決手順**:

1. **URLの確認**
   ```sql
   -- データベースでURLを確認
   SELECT receipt_url FROM expenses WHERE id = ?;
   ```

2. **Presigned URLの有効期限確認**
   - URLが期限切れでないか確認
   - 必要に応じて新しいPresigned URLを生成

3. **キャッシュの確認**
   ```bash
   # キャッシュディレクトリの確認
   ls -la ~/.cache/expense-app/receipts/
   ```

### 問題: 画像が破損して表示される

**症状**:
- 画像が部分的にしか表示されない
- PDFが開けない

**解決方法**:
1. **ファイル整合性の確認**
   ```bash
   # ファイルのハッシュ値を確認
   sha256sum original_file downloaded_file
   ```

2. **キャッシュのクリア**
   - アプリケーション内でキャッシュクリアを実行
   - または手動でキャッシュディレクトリを削除

## パフォーマンス関連の問題

### 問題: アップロードが遅い

**症状**:
- 10MB以下のファイルが30秒以上かかる
- プログレスバーの進行が遅い

**解決方法**:

1. **ネットワーク速度の確認**
   ```bash
   # アップロード速度のテスト
   curl -o /dev/null -s -w "%{speed_upload}\n" -T testfile.jpg https://httpbin.org/put
   ```

2. **並列アップロードの無効化**
   - 複数ファイルを同時にアップロードしている場合、順次実行に変更

3. **地理的な距離**
   - Cloudflareのエッジロケーションまでの距離が影響する可能性

### 問題: メモリ使用量が多い

**症状**:
- アプリケーションのメモリ使用量が急増
- システムが重くなる

**解決方法**:
1. **ファイルストリーミングの確認**
   ```rust
   // ファイル全体をメモリに読み込まず、ストリーミング処理を使用
   ```

2. **キャッシュサイズの制限**
   - キャッシュの最大サイズを制限
   - 古いキャッシュファイルの自動削除

## 設定関連の問題

### 問題: 環境変数が読み込まれない

**症状**:
```
Error: Environment variable R2_ACCOUNT_ID not found
```

**解決手順**:

1. **.envファイルの場所確認**
   ```bash
   # .envファイルが正しい場所にあるか確認
   ls -la src-tauri/.env
   ```

2. **.envファイルの形式確認**
   ```bash
   # 正しい形式で記述されているか確認
   cat src-tauri/.env
   
   # 正しい形式の例:
   # R2_ACCOUNT_ID=abc123
   # R2_ACCESS_KEY=def456
   ```

3. **権限の確認**
   ```bash
   # ファイルの読み取り権限を確認
   ls -la src-tauri/.env
   ```

### 問題: 開発環境と本番環境の設定が混在

**症状**:
- 開発中に本番データにアクセスしてしまう
- 本番環境で開発用バケットを使用してしまう

**解決方法**:
1. **環境別設定ファイルの使用**
   ```bash
   # 環境別の設定ファイルを作成
   src-tauri/.env.development
   src-tauri/.env.production
   ```

2. **バケット名の命名規則**
   ```bash
   # 明確な命名規則を使用
   R2_BUCKET_NAME=expense-receipts-dev    # 開発環境
   R2_BUCKET_NAME=expense-receipts-prod   # 本番環境
   ```

## デバッグ方法

### 1. ログレベルの設定

```bash
# 詳細なログを有効にする
export RUST_LOG=debug
pnpm tauri dev
```

### 2. R2接続の手動テスト

```bash
# Wrangler CLIでの接続テスト
wrangler r2 bucket list

# 特定のバケットへのアクセステスト
wrangler r2 object put expense-receipts-dev/test.txt --file test.txt
wrangler r2 object get expense-receipts-dev/test.txt
```

### 3. ネットワークトラフィックの監視

```bash
# macOSでのネットワーク監視
sudo tcpdump -i any host your-account-id.r2.cloudflarestorage.com

# Linuxでのネットワーク監視
sudo netstat -tuln | grep :443
```

### 4. アプリケーション内デバッグ

アプリケーション内の「設定」→「デバッグ情報」で以下を確認：
- R2接続状態
- 環境変数の読み込み状態
- キャッシュ使用量
- 最近のエラーログ

## よくある質問 (FAQ)

### Q: オフライン時に領収書が表示されないのはなぜですか？

A: 以下を確認してください：
1. 該当の領収書が以前に表示されてキャッシュされているか
2. キャッシュが有効期限内か（7日間）
3. キャッシュファイルが破損していないか

### Q: 複数のデバイスで同じR2バケットを使用できますか？

A: はい、可能です。ただし以下に注意：
1. 同じ認証情報を使用
2. ファイル名の競合を避けるため、デバイス固有のプレフィックスを使用することを推奨
3. 同期の遅延が発生する可能性

### Q: R2の使用量を監視する方法は？

A: 以下の方法があります：
1. Cloudflare Dashboardの「R2」→「Metrics」
2. アプリケーション内の使用量監視機能
3. Wrangler CLI: `wrangler r2 bucket usage expense-receipts-dev`

## サポートとヘルプ

### 1. ログの収集

問題報告時は以下の情報を含めてください：
- エラーメッセージの全文
- 発生時刻
- 操作手順
- 環境情報（OS、アプリバージョン）

### 2. 設定情報の確認

```bash
# 設定情報の匿名化された出力
echo "Account ID: ${R2_ACCOUNT_ID:0:8}..."
echo "Bucket: $R2_BUCKET_NAME"
echo "Region: $R2_REGION"
```

### 3. 参考リンク

- [Cloudflare R2 公式ドキュメント](https://developers.cloudflare.com/r2/)
- [Cloudflare コミュニティフォーラム](https://community.cloudflare.com/)
- [GitHub Issues](https://github.com/your-repo/issues)

---

**注意**: 認証情報（アクセスキー、シークレットキー）は絶対に他人と共有しないでください。問題報告時も認証情報は含めないよう注意してください。
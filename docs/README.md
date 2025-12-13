# ドキュメント

このディレクトリには、経費管理アプリケーションのCloudflare R2機能に関するドキュメントが含まれています。

## ドキュメント一覧

### 設定ガイド

1. **[R2設定ガイド](./R2_SETUP.md)**
   - Cloudflare R2の初期設定
   - バケットの作成
   - APIトークンの設定
   - CORS設定
   - セキュリティ設定

2. **[環境変数設定ガイド](./ENVIRONMENT_SETUP.md)**
   - 必要な環境変数の説明
   - .envファイルの設定方法
   - 環境別設定（開発/本番）
   - セキュリティのベストプラクティス

### トラブルシューティング

3. **[トラブルシューティングガイド](./TROUBLESHOOTING.md)**
   - よくある問題と解決方法
   - エラーメッセージの対処法
   - デバッグ方法
   - パフォーマンス最適化

## セットアップの流れ

R2機能を使用するには、以下の順序でドキュメントを参照してください：

```
1. R2設定ガイド
   ↓
2. 環境変数設定ガイド  
   ↓
3. アプリケーションの起動・テスト
   ↓
4. 問題が発生した場合：トラブルシューティングガイド
```

## 追加リソース

### 公式ドキュメント
- [Cloudflare R2 公式ドキュメント](https://developers.cloudflare.com/r2/)
- [R2 API リファレンス](https://developers.cloudflare.com/r2/api/)
- [Wrangler CLI ドキュメント](https://developers.cloudflare.com/workers/wrangler/)

### コミュニティ
- [Cloudflare コミュニティフォーラム](https://community.cloudflare.com/)
- [Cloudflare Discord](https://discord.gg/cloudflaredev)

## 貢献

ドキュメントの改善提案や追加情報がある場合は、GitHubのIssueまたはPull Requestでお知らせください。

## ライセンス

このドキュメントは、プロジェクトのライセンスと同じライセンスの下で提供されています。
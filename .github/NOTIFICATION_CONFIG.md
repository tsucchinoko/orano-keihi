# 通知設定ガイド

## 概要

このドキュメントでは、CI/CDパイプラインでの通知機能の設定方法について説明します。

## 必要なシークレット

GitHub Actionsで通知機能を有効にするには、以下のシークレットをリポジトリ設定で追加してください：

### Slack通知用
- `SLACK_WEBHOOK_URL`: SlackのIncoming Webhook URL

### メール通知用
- `MAIL_USERNAME`: SMTPサーバーのユーザー名
- `MAIL_PASSWORD`: SMTPサーバーのパスワード
- `NOTIFICATION_EMAIL`: 通知を受信するメールアドレス

## シークレットの設定方法

1. GitHubリポジトリの「Settings」タブに移動
2. 左サイドバーの「Secrets and variables」→「Actions」を選択
3. 「New repository secret」をクリック
4. 上記のシークレット名と値を入力

## Slack Webhook URLの取得方法

1. Slackワークスペースで「Apps」を開く
2. 「Incoming Webhooks」アプリを検索してインストール
3. 通知を送信したいチャンネルを選択
4. 生成されたWebhook URLをコピー
5. GitHubのシークレットに`SLACK_WEBHOOK_URL`として追加

## メール設定の例

### Gmail使用時
- `MAIL_USERNAME`: your-email@gmail.com
- `MAIL_PASSWORD`: アプリパスワード（2段階認証が必要）
- `NOTIFICATION_EMAIL`: notification-recipient@example.com

### その他のSMTPサーバー
ワークフローファイルの`server_address`と`server_port`を適切な値に変更してください。

## 通知の無効化

通知機能を無効にしたい場合は、以下のステップをワークフローファイルから削除またはコメントアウトしてください：

- `Slack通知の送信`
- `メール通知の送信`

## カスタマイズ

### 通知条件の変更
現在の設定では以下の条件で通知が送信されます：

- **Slack通知**: ビルドの成功・失敗に関わらず常に送信
- **メール通知**: mainブランチでのビルド失敗時のみ送信

これらの条件は各ステップの`if`条件を変更することでカスタマイズできます。

### 通知内容の変更
通知メッセージの内容は各ステップの`text`または`body`セクションで変更できます。

## トラブルシューティング

### 通知が送信されない場合
1. シークレットが正しく設定されているか確認
2. Slack Webhook URLが有効か確認
3. メール認証情報が正しいか確認
4. ワークフローログでエラーメッセージを確認

### 通知は送信されるがビルドが失敗扱いになる場合
通知ステップに`continue-on-error: true`が設定されているか確認してください。これにより、通知の失敗がビルド全体の失敗として扱われることを防げます。
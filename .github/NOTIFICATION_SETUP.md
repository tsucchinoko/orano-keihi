# 通知機能セットアップガイド

このドキュメントでは、CI/CDパイプラインの通知機能を設定する方法について説明します。

## 📢 実装されている通知機能

### 1. 基本通知機能（設定不要）

以下の通知機能は追加設定なしで動作します：

- **GitHub Actions通知**: ワークフローログでの詳細な状況報告
- **GitHub Issues通知**: リリース完了時に自動でIssueを作成
- **GitHub Discussion通知**: リリース告知用のDiscussionを自動作成

### 2. 拡張通知機能（設定が必要）

以下の通知機能は追加設定により利用できます：

- **Slack通知**: Slackチャンネルへの通知
- **メール通知**: 指定したメールアドレスへの通知
- **関係者メンション**: 特定のユーザーへのメンション通知

## ⚙️ 設定方法

### リポジトリ変数の設定

GitHub リポジトリの Settings > Secrets and variables > Actions > Variables で以下を設定：

#### MAINTAINERS（推奨）
```
@username1 @username2 @username3
```
- リリース時にメンションされる関係者のGitHubユーザー名
- スペース区切りで複数指定可能
- 例: `@alice @bob @charlie`

### リポジトリ秘密の設定（オプション）

GitHub リポジトリの Settings > Secrets and variables > Actions > Secrets で以下を設定：

#### Slack通知用
- **SLACK_WEBHOOK_URL**: Slack Incoming Webhook URL
  ```
  https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXXXXXX
  ```

#### メール通知用
- **NOTIFICATION_EMAIL**: 通知先メールアドレス
  ```
  team@example.com
  ```
- **MAIL_USERNAME**: SMTP認証用ユーザー名
  ```
  your-email@gmail.com
  ```
- **MAIL_PASSWORD**: SMTP認証用パスワード（Gmailの場合はアプリパスワード）
  ```
  your-app-password
  ```

## 🔧 Slack通知の設定手順

### 1. Slack Appの作成

1. [Slack API](https://api.slack.com/apps)にアクセス
2. "Create New App" をクリック
3. "From scratch" を選択
4. アプリ名とワークスペースを設定

### 2. Incoming Webhookの有効化

1. 作成したアプリの設定画面で "Incoming Webhooks" を選択
2. "Activate Incoming Webhooks" をオンにする
3. "Add New Webhook to Workspace" をクリック
4. 通知先チャンネルを選択
5. 生成されたWebhook URLをコピー

### 3. GitHubでの設定

1. GitHubリポジトリの Settings > Secrets and variables > Actions > Secrets
2. "New repository secret" をクリック
3. Name: `SLACK_WEBHOOK_URL`
4. Secret: コピーしたWebhook URL

## 📧 メール通知の設定手順（Gmail使用例）

### 1. Gmailアプリパスワードの生成

1. Googleアカウントの2段階認証を有効にする
2. [Googleアカウント設定](https://myaccount.google.com/)にアクセス
3. "セキュリティ" > "2段階認証プロセス" > "アプリパスワード"
4. アプリパスワードを生成

### 2. GitHubでの設定

以下の秘密を設定：

- `NOTIFICATION_EMAIL`: 通知先メールアドレス
- `MAIL_USERNAME`: Gmailアドレス
- `MAIL_PASSWORD`: 生成したアプリパスワード

## 📋 通知内容

### リリース成功時

- **GitHub Issue**: リリース完了通知Issue（チェックリスト付き）
- **GitHub Discussion**: ユーザー向けリリース告知
- **Slack**: ビルド成功とリリース情報
- **メール**: 詳細なリリース情報とダウンロードリンク

### ビルド失敗時

- **GitHub Actions**: エラーログと詳細情報
- **Slack**: ビルド失敗通知
- **メール**: 緊急通知（mainブランチの場合）

## 🔍 通知の確認方法

### 1. GitHub Actions ログ

ワークフロー実行ログで通知状況を確認できます：
- "通知状況の確認と統合レポート" ステップ
- "最終通知確認とフォールバック" ステップ

### 2. 作成されるIssue/Discussion

リリース成功時に以下が自動作成されます：
- Issues: `🎉 リリース完了通知: vX.X.X` ラベル付きIssue
- Discussions: `📢 vX.X.X リリースのお知らせ` Discussion

## 🚨 トラブルシューティング

### 通知が届かない場合

1. **設定の確認**
   - リポジトリ変数・秘密が正しく設定されているか確認
   - 変数名・秘密名のスペルミスがないか確認

2. **権限の確認**
   - GitHub Actionsに必要な権限が付与されているか確認
   - Slack Webhook URLが有効か確認

3. **ログの確認**
   - GitHub Actionsのログで詳細なエラー情報を確認
   - "通知状況の確認と統合レポート" ステップの出力を確認

### よくある問題

#### Slack通知が失敗する
- Webhook URLが正しいか確認
- Slackアプリの権限設定を確認
- チャンネルが存在するか確認

#### メール通知が失敗する
- Gmailの2段階認証が有効か確認
- アプリパスワードが正しく生成されているか確認
- SMTP設定が正しいか確認

#### Discussion作成が失敗する
- リポジトリでDiscussion機能が有効か確認
- 適切なDiscussionカテゴリが存在するか確認

## 📝 カスタマイズ

通知内容や動作をカスタマイズしたい場合は、`.github/workflows/release.yml` ファイルの以下のセクションを編集してください：

- GitHub Issues通知: "GitHub Issues通知の作成" ステップ
- GitHub Discussion通知: "GitHub Discussion通知の作成" ステップ
- Slack通知: "Slack通知の送信" ステップ
- メール通知: "メール通知の送信" ステップ

## 🔗 関連リンク

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Slack API Documentation](https://api.slack.com/)
- [GitHub GraphQL API](https://docs.github.com/en/graphql)
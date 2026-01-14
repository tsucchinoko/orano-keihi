# 自動アップデート：手動再起動機能

## 概要

自動アップデート機能において、ダウンロード完了後にアプリケーションを自動的に再起動するのではなく、ユーザーが手動で再起動するように変更しました。

## 変更内容

### 1. バックエンド（Rust）の変更

#### `packages/desktop/src-tauri/src/features/updater/service.rs`

- `download_and_install`メソッドを変更
  - `update.download_and_install()`から`update.download()`に変更
  - ダウンロード完了後、自動的にインストールせず、`download-complete`イベントを発行
  - ユーザーに手動での再起動を促すメッセージをログに記録

#### `packages/desktop/src-tauri/src/features/updater/commands.rs`

- 新しいコマンド`restart_application`を追加
  - アプリケーションを再起動してアップデートをインストール
  - `app_handle.restart()`を呼び出してプロセスを再起動

#### `packages/desktop/src-tauri/src/lib.rs`

- `restart_application`コマンドを登録

### 2. フロントエンド（TypeScript/Svelte）の変更

#### `packages/desktop/src/lib/services/updater.ts`

- 新しいメソッド`restartApplication()`を追加
  - `restart_application`コマンドを呼び出してアプリケーションを再起動
- 新しいイベントリスナーを追加
  - `listenForDownloadComplete()`: ダウンロード完了イベントをリッスン
  - `listenForDownloadProgress()`: ダウンロード進捗イベントをリッスン

#### `packages/desktop/src/lib/types/updater.ts`

- `UpdateNotificationState`型に`downloadComplete`フィールドを追加
  - ダウンロードが完了したかどうかを追跡

#### `packages/desktop/src/lib/components/UpdateNotification.svelte`

- UIの状態管理を更新
  - `downloadComplete`状態を追加
  - ダウンロード完了後に「今すぐ再起動」ボタンを表示
  - 「後で再起動」ボタンで通知を閉じることができる
- イベントリスナーを更新
  - `download-complete`イベントをリッスンして`downloadComplete`を`true`に設定
  - `download-progress`イベントをリッスンして進捗バーを更新
- 新しい関数`handleRestart()`を追加
  - 「今すぐ再起動」ボタンがクリックされたときに`UpdaterService.restartApplication()`を呼び出す

## ユーザーエクスペリエンス

### アップデートフロー

1. **アップデート通知**
   - 新しいバージョンが利用可能になると通知が表示される
   - 「今すぐアップデート」「後で」「スキップ」ボタンが表示される

2. **ダウンロード中**
   - 「今すぐアップデート」をクリックするとダウンロードが開始される
   - 進捗バーでダウンロードの進行状況が表示される
   - ダウンロード中は「ダウンロード中...」ボタンが無効化される

3. **ダウンロード完了**
   - ダウンロードが完了すると、進捗バーが100%になる
   - 「今すぐ再起動」ボタンが表示される
   - 「後で再起動」ボタンで通知を閉じることができる
   - メッセージ：「アプリケーションを再起動してインストールを完了してください」

4. **再起動**
   - 「今すぐ再起動」をクリックするとアプリケーションが再起動される
   - 再起動後、アップデートが自動的にインストールされる

## 技術的な詳細

### イベントフロー

```
1. check_for_updates() → update-available イベント
2. download_and_install() → download-progress イベント（複数回）
3. ダウンロード完了 → download-complete イベント
4. restart_application() → アプリケーション再起動
5. 再起動後 → アップデート自動インストール
```

### セキュリティ

- ダウンロードしたファイルは署名検証される
- HTTPS通信が強制される
- 信頼できるエンドポイント（GitHub Releases）からのみダウンロード

## テスト方法

1. アプリケーションを起動
2. 新しいバージョンをGitHub Releasesに公開
3. アップデート通知が表示されることを確認
4. 「今すぐアップデート」をクリック
5. ダウンロード進捗が表示されることを確認
6. ダウンロード完了後、「今すぐ再起動」ボタンが表示されることを確認
7. 「今すぐ再起動」をクリックしてアプリケーションが再起動されることを確認
8. 再起動後、新しいバージョンがインストールされていることを確認

## 注意事項

- ダウンロード完了後、ユーザーが再起動するまでアップデートはインストールされません
- 「後で再起動」を選択した場合、次回アプリケーション起動時にアップデートがインストールされます
- ダウンロード中にアプリケーションを終了すると、ダウンロードは中断されます

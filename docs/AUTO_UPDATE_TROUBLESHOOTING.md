# 自動アップデート機能のトラブルシューティング

## 現在の状況

### 実装済みの機能

1. **Tauri設定**: GitHub Releasesの静的JSONファイルベースのアップデートエンドポイントを設定済み
2. **静的JSONファイル生成スクリプト**: プラットフォーム別のマニフェストファイルを生成するスクリプトを実装済み
3. **GitHub Actionsワークフロー**: リリース時に自動的にマニフェストファイルを生成・アップロードするワークフローを実装済み
4. **Rustバックエンド**: アップデートチェック、ダウンロード、インストール機能を実装済み
5. **TypeScriptフロントエンド**: アップデート通知UIを実装済み

### 現在のエラーについて

開発環境で以下のエラーが表示されています：

```
[2026-01-07T09:41:04Z ERROR] update endpoint did not respond with a successful status code
[2026-01-07T09:41:04Z ERROR] ネットワークエラーが発生: アップデートチェックに失敗しました: Could not fetch a valid release JSON from the remote
```

**これは正常な動作です。** 理由は以下の通りです：

1. **リリースがまだ作成されていない**: GitHub Releasesにマニフェストファイルがアップロードされていないため、エンドポイントが404エラーを返します
2. **開発環境での動作**: 開発中は、アップデートチェックが失敗しても問題ありません
3. **エラーハンドリング改善済み**: 最新のコードでは、このエラーを警告として扱い、アプリケーションの動作に影響を与えません

## よくあるエラーと解決策

### エラー1: "Failed to move the new app into place"

#### 原因
- **macOS**: アプリケーションが`/Applications`フォルダにインストールされている場合、管理者権限が必要
- **Windows**: アプリケーションが`Program Files`にインストールされている場合、管理者権限が必要
- **共通**: アプリケーションが実行中で、自分自身を置き換えようとしている

#### 解決策

1. **インストールモードの確認**
   - `tauri.conf.json`で`installMode: "currentUser"`が設定されていることを確認
   - これにより、ユーザーディレクトリにインストールされ、管理者権限が不要になります

2. **再起動プロセスの改善**
   - アップデートのダウンロード完了後、アプリケーションを再起動してインストールを完了
   - Tauriのアップデーターは、再起動時に新しいバージョンを適用します

3. **手動での対処**
   - アプリケーションを完全に終了
   - 新しいバージョンのインストーラーを手動でダウンロード
   - インストーラーを実行して上書きインストール

#### 設定の確認

`packages/desktop/src-tauri/tauri.conf.json`:
```json
{
  "bundle": {
    "windows": {
      "nsis": {
        "installMode": "currentUser",
        "perMachine": false
      }
    }
  },
  "plugins": {
    "updater": {
      "windows": {
        "installMode": "passive",
        "installerArgs": ["/S"]
      }
    }
  }
}
```

## 自動アップデート機能を有効にする手順

### 1. 最初のリリースを作成

自動アップデート機能を有効にするには、まず最初のリリースを作成する必要があります。

#### 手順

1. **releaseブランチにコードをプッシュ**:
   ```bash
   git checkout -b release
   git push origin release
   ```

2. **GitHub Actionsが自動的に実行される**:
   - MacOSとWindowsのビルドが並行して実行されます
   - ビルド成果物（dmg、msi）が生成されます
   - 静的JSONマニフェストファイルが生成されます
   - GitHub Releasesにすべてのファイルがアップロードされます

3. **リリースの確認**:
   - GitHub Releasesページで新しいリリースが作成されていることを確認
   - 以下のファイルがアップロードされていることを確認：
     - `orano-keihi_0.1.0_x64.dmg` (macOS Intel)
     - `orano-keihi_0.1.0_aarch64.dmg` (macOS Apple Silicon)
     - `orano-keihi_0.1.0_x64_ja-JP.msi` (Windows)
     - `darwin-x86_64.json` (macOS Intel用マニフェスト)
     - `darwin-aarch64.json` (macOS Apple Silicon用マニフェスト)
     - `windows-x86_64.json` (Windows用マニフェスト)

### 2. 2回目以降のリリース

2回目以降のリリースでは、自動アップデート機能が動作します。

#### 手順

1. **バージョン番号を更新**:
   ```bash
   # package.jsonのバージョンを更新（例: 0.1.0 → 0.2.0）
   # tauri.conf.jsonのバージョンも同期して更新
   ```

2. **releaseブランチにプッシュ**:
   ```bash
   git add package.json packages/desktop/src-tauri/tauri.conf.json
   git commit -m "chore: bump version to 0.2.0"
   git push origin release
   ```

3. **既存のアプリケーションが自動的にアップデートをチェック**:
   - アプリケーション起動時に自動チェック
   - バックグラウンドで定期的にチェック（デフォルト: 24時間ごと）
   - 新しいバージョンが利用可能な場合、通知が表示されます

## 開発環境での動作

### 現在の動作

開発環境（`pnpm tauri dev`）では：

1. アプリケーション起動時にアップデートチェックが実行されます
2. GitHub Releasesにマニフェストファイルが存在しない場合、警告ログが出力されます
3. **アプリケーションは正常に動作し続けます**（エラーで停止しません）

### ログの意味

```
[INFO] アップデーター設定を読み込みました
[INFO] セキュリティチェックを実行中...
[INFO] ✓ HTTPS通信が確認されました
[INFO] ✓ 署名検証が有効です
[INFO] ✓ エンドポイントが信頼できます
[INFO] すべてのセキュリティチェックに合格しました
[INFO] アップデートチェックを開始: 現在のバージョン 0.1.0
[INFO] アップデートをチェック中...
[WARN] アップデートチェックに失敗しました: Could not fetch a valid release JSON from the remote
[INFO] 開発環境またはリリース未作成のため、アップデートチェックをスキップします
```

これは正常な動作です。リリースが作成されるまで、このログが表示されます。

## トラブルシューティング

### Q1: アップデートチェックが常に失敗する

**A**: 以下を確認してください：

1. **GitHub Releasesにマニフェストファイルが存在するか**:
   - `https://github.com/tsucchinoko/orano-keihi/releases/latest`にアクセス
   - `darwin-x86_64.json`などのファイルがダウンロードできるか確認

2. **ネットワーク接続**:
   - インターネット接続が正常か確認
   - ファイアウォールやプロキシの設定を確認

3. **Tauri設定**:
   - `tauri.conf.json`の`plugins.updater.endpoints`が正しいか確認
   - 公開鍵（`pubkey`）が設定されているか確認

### Q2: アップデート通知が表示されない

**A**: 以下を確認してください：

1. **バージョン番号**:
   - 新しいリリースのバージョン番号が現在のバージョンより大きいか確認
   - セマンティックバージョニング（`major.minor.patch`）に従っているか確認

2. **自動チェック設定**:
   - アップデート設定で自動チェックが有効になっているか確認
   - 最後のチェック時刻から十分な時間が経過しているか確認

3. **スキップされたバージョン**:
   - 該当バージョンをスキップしていないか確認
   - アップデート設定でスキップリストを確認

### Q3: ダウンロードが失敗する

**A**: 以下を確認してください：

1. **ダウンロードURL**:
   - マニフェストファイルのURLが正しいか確認
   - GitHub Releasesにファイルが存在するか確認

2. **署名検証**:
   - 署名ファイル（`.sig`）が存在するか確認
   - 公開鍵が正しく設定されているか確認

3. **ディスク容量**:
   - 十分なディスク容量があるか確認

### Q4: インストールが完了しない

**A**: 以下を確認してください：

1. **再起動の実行**:
   - ダウンロード完了後、アプリケーションを再起動してください
   - Tauriのアップデーターは、再起動時に新しいバージョンを適用します

2. **権限の確認**:
   - アプリケーションがユーザーディレクトリにインストールされているか確認
   - 管理者権限が必要な場所にインストールされている場合は、手動でインストーラーを実行

3. **ログの確認**:
   - アプリケーションログで詳細なエラーメッセージを確認
   - `updater.log`ファイルを確認

## 設定のカスタマイズ

### 自動チェック頻度の変更

アップデート設定ファイル（`updater_config.json`）で変更できます：

```json
{
  "auto_check_enabled": true,
  "check_interval_hours": 24,  // 24時間ごとにチェック
  "beta_channel_enabled": false,
  "skipped_versions": [],
  "last_check_time": 0
}
```

### ベータ版の受信

ベータ版を受信する場合は、`beta_channel_enabled`を`true`に設定します。

## 参考資料

- [Tauri Updater Plugin Documentation](https://v2.tauri.app/plugin/updater/)
- [GitHub Releases Documentation](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
- [セマンティックバージョニング](https://semver.org/lang/ja/)

## サポート

問題が解決しない場合は、以下の情報を含めてIssueを作成してください：

1. エラーメッセージの全文
2. 使用しているOS（macOS/Windows）とバージョン
3. アプリケーションのバージョン
4. ログファイル（`updater.log`）の内容

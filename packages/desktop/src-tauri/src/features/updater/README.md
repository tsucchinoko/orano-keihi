# Tauri自動アップデート機能

## 概要

このモジュールは、Tauri v2のupdaterプラグインを使用した自動アップデート機能を提供します。
GitHub Releasesの静的JSONファイルベースのアップデート配信システムを実装しています。

## 主要コンポーネント

### UpdaterService

アップデート機能の中核となるサービスクラスです。

#### 主要メソッド

- `check_for_updates()`: アップデートをチェック
- `download_and_install()`: アップデートをダウンロードしてインストール
- `skip_version()`: 特定のバージョンをスキップ
- `get_config()`: 設定を取得
- `update_config()`: 設定を更新
- `start_auto_check()`: 自動アップデートチェックを開始
- `stop_auto_check()`: 自動アップデートチェックを停止

#### セキュリティ機能

- `perform_security_checks()`: セキュリティチェックを実行
  - HTTPS通信の強制
  - 署名検証の有効性確認
  - エンドポイントの信頼性確認
- `verify_file_hash()`: ファイルのハッシュ値を検証（将来的な拡張）

### UpdaterConfig

アップデーター設定を管理するクラスです。

#### 設定項目

- `auto_check_enabled`: 自動アップデートチェックの有効/無効
- `check_interval_hours`: アップデートチェックの頻度（時間単位）
- `include_prereleases`: ベータ版アップデートの受信可否
- `skipped_versions`: スキップされたバージョンのリスト
- `last_check_time`: 最後にチェックした時刻

### UpdateLogger

アップデート処理のログを記録するクラスです。

#### ログ機能

- チェック開始/結果のログ
- ダウンロード開始/進捗/完了のログ
- インストール開始/完了のログ
- エラー/警告のログ

### UpdateError

アップデート処理のエラーを表現する列挙型です。

#### エラーの種類

- `Network`: ネットワークエラー
- `SignatureVerification`: 署名検証エラー
- `Download`: ダウンロードエラー
- `Installation`: インストールエラー
- `Configuration`: 設定エラー
- `FileSystem`: ファイルシステムエラー
- `Permission`: 権限エラー
- `Timeout`: タイムアウトエラー
- `InvalidVersion`: 不正なバージョンエラー
- `InitializationError`: アップデーター初期化エラー
- `General`: 一般的なエラー

## 使用方法

### 基本的な使用例

```rust
use crate::features::updater::service::UpdaterService;

// アップデートサービスを作成
let mut service = UpdaterService::new(app_handle.clone());

// アップデートをチェック
match service.check_for_updates().await {
    Ok(update_info) => {
        if update_info.available {
            println!("アップデートが利用可能: {}", update_info.latest_version.unwrap());

            // アップデートをダウンロードしてインストール
            service.download_and_install().await?;
        } else {
            println!("アップデートはありません");
        }
    }
    Err(e) => {
        eprintln!("アップデートチェックエラー: {}", e);
    }
}
```

### 自動アップデートチェック

```rust
// 自動アップデートチェックを開始
service.start_auto_check();

// 自動アップデートチェックを停止
service.stop_auto_check();
```

### 設定の管理

```rust
// 設定を取得
let config = service.get_config().await;

// 設定を更新
let mut new_config = config.clone();
new_config.auto_check_enabled = false;
new_config.check_interval_hours = 12;
service.update_config(new_config).await?;
```

### バージョンのスキップ

```rust
// 特定のバージョンをスキップ
service.skip_version("1.0.0".to_string()).await?;
```

## セキュリティ

詳細なセキュリティ情報については、[SECURITY.md](./SECURITY.md)を参照してください。

### セキュリティチェック

アップデートチェックとダウンロード時に、以下のセキュリティチェックが自動的に実行されます：

1. **HTTPS通信の強制**: すべての通信がHTTPSで行われることを確認
2. **署名検証**: Tauri updaterプラグインによる自動署名検証
3. **エンドポイントの信頼性**: 信頼できるドメインからの配信を確認

### エラーハンドリング

セキュリティ関連のエラーが発生した場合、アップデートは即座に中止され、
エラー情報がログに記録されます。

## テスト

### 単体テスト

```bash
cargo test --manifest-path packages/desktop/src-tauri/Cargo.toml updater
```

### 統合テスト

実際のGitHub Releasesを使用したエンドツーエンドテストは、
リリースワークフローで自動的に実行されます。

## ログファイル

アップデート処理のログは、以下の場所に保存されます：

- macOS: `~/Library/Application Support/com.tsucchinoko.orano-keihi/logs/updater.log`
- Windows: `%APPDATA%\com.tsucchinoko.orano-keihi\logs\updater.log`

## トラブルシューティング

### アップデートチェックが失敗する

1. ネットワーク接続を確認
2. ログファイルでエラー詳細を確認
3. GitHub Releasesが正しく設定されているか確認

### 署名検証エラー

1. 公開鍵が正しく設定されているか確認（`tauri.conf.json`）
2. リリースファイルに署名が含まれているか確認
3. 開発者に連絡して問題を報告

### インストールエラー

1. ディスク容量を確認
2. 管理者権限で実行
3. アプリケーションを再起動

## 参考資料

- [Tauri Updater Plugin Documentation](https://v2.tauri.app/plugin/updater/)
- [GitHub Releases Documentation](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
- [minisign Documentation](https://jedisct1.github.io/minisign/)

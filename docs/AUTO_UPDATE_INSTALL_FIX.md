# 自動アップデート「Failed to move the new app into place」エラーの修正

## 問題の概要

自動アップデート機能で以下のエラーが発生していました：

```
Failed to move the new app into place
```

このエラーは、ダウンロードした新しいアプリケーションを既存のアプリケーションと置き換える際に発生する権限エラーです。

## 根本原因

1. **権限の問題**: アプリケーションが管理者権限が必要な場所（`/Applications`や`Program Files`）にインストールされている
2. **インストールプロセスの問題**: アプリケーションが実行中に自分自身を置き換えようとしている
3. **再起動の欠如**: Tauriのアップデーターは、ダウンロード完了後にアプリケーションを再起動してインストールを完了する必要がある

## 実施した修正

### 1. `tauri.conf.json`の設定改善

#### 1.1 NSISインストーラーの設定

Windowsインストーラーがユーザーディレクトリにインストールされるように設定：

```json
{
  "bundle": {
    "windows": {
      "nsis": {
        "installMode": "currentUser"
      }
    }
  }
}
```

これにより、管理者権限なしでインストール・アップデートが可能になります。

#### 1.2 アップデーターの設定

Windowsアップデーターにサイレントインストールオプションを追加：

```json
{
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

### 2. Rustバックエンドの修正

#### 2.1 `download_and_install`関数の改善

`packages/desktop/src-tauri/src/features/updater/service.rs`:

```rust
pub async fn download_and_install(&self) -> Result<(), UpdateError> {
    // ... ダウンロード処理 ...
    
    match update.download_and_install(...).await {
        Ok(_) => {
            info!("アップデートのダウンロードが完了しました");
            info!("アプリケーションを再起動してインストールを完了してください");
            
            // フロントエンドに再起動が必要であることを通知
            if let Err(e) = self.app_handle.emit("restart-required", ()) {
                warn!("再起動通知の送信に失敗: {e}");
            }
            
            Ok(())
        }
        Err(e) => {
            let error = UpdateError::installation(format!("アップデートのインストールに失敗しました: {e}"));
            self.logger.log_error(&error);
            Err(error)
        }
    }
}
```

**変更点**:
- ダウンロード完了後、フロントエンドに`restart-required`イベントを送信
- ユーザーに再起動が必要であることを明示的に通知

### 3. TypeScriptフロントエンドの修正

#### 3.1 `UpdaterService`に再起動通知リスナーを追加

`packages/desktop/src/lib/services/updater.ts`:

```typescript
/**
 * 再起動必要通知イベントをリッスン
 * @param callback 再起動が必要になったときのコールバック
 */
static async listenForRestartRequired(
  callback: () => void
): Promise<() => void> {
  try {
    const unlisten = await listen('restart-required', () => {
      console.info('アプリケーションの再起動が必要です');
      callback();
    });
    return unlisten;
  } catch (error) {
    console.error('再起動通知リスナー設定エラー:', error);
    throw new Error(`再起動通知の設定に失敗しました: ${String(error)}`);
  }
}
```

#### 3.2 `UpdateNotification.svelte`の改善

`packages/desktop/src/lib/components/UpdateNotification.svelte`:

```typescript
onMount(() => {
  let unlistenUpdate: (() => void) | undefined;
  let unlistenRestart: (() => void) | undefined;

  const initializeUpdater = async () => {
    try {
      // アップデート通知イベントをリッスン
      unlistenUpdate = await UpdaterService.listenForUpdates(showUpdateNotification);

      // 再起動通知イベントをリッスン
      unlistenRestart = await UpdaterService.listenForRestartRequired(() => {
        // ダウンロード完了、再起動が必要
        updateState = {
          ...updateState,
          downloading: false,
          progress: 100
        };
        
        // 3秒後に自動的に再起動
        setTimeout(() => {
          window.location.reload();
        }, 3000);
      });

      // ... 以下省略 ...
    } catch (error) {
      console.error('アップデート機能の初期化エラー:', error);
    }
  };

  initializeUpdater();

  return () => {
    if (unlistenUpdate) unlistenUpdate();
    if (unlistenRestart) unlistenRestart();
  };
});
```

**変更点**:
- `restart-required`イベントをリッスン
- ダウンロード完了後、進捗を100%に設定
- 3秒後に自動的にアプリケーションを再起動

#### 3.3 UIメッセージの改善

ダウンロード進捗表示のメッセージを改善：

```svelte
{#if updateState.downloading}
  <div class="mb-4">
    <div class="flex justify-between text-sm mb-2">
      <span class="text-gray-600">
        {#if updateState.progress >= 100}
          インストール準備中...
        {:else}
          ダウンロード中...
        {/if}
      </span>
      <span class="font-medium">{updateState.progress.toFixed(1)}%</span>
    </div>
    <div class="w-full bg-gray-200 rounded-full h-2.5 overflow-hidden">
      <div 
        class="bg-blue-600 h-2.5 rounded-full transition-all duration-300 ease-out"
        style="width: {updateState.progress}%"
      ></div>
    </div>
    <p class="text-xs text-gray-500 mt-1.5">
      {#if updateState.progress >= 100}
        アプリケーションを再起動してインストールを完了してください
      {:else}
        ダウンロード完了後、アプリケーションを再起動してインストールを完了します
      {/if}
    </p>
  </div>
{/if}
```

## アップデートの流れ

修正後のアップデートプロセス：

1. **アップデートチェック**: アプリケーション起動時またはバックグラウンドで定期的にチェック
2. **通知表示**: 新しいバージョンが利用可能な場合、通知を表示
3. **ダウンロード開始**: ユーザーが「今すぐアップデート」をクリック
4. **ダウンロード進捗**: 進捗バーでダウンロード状況を表示
5. **ダウンロード完了**: 進捗が100%になり、「インストール準備中...」と表示
6. **再起動通知**: `restart-required`イベントが発火
7. **自動再起動**: 3秒後にアプリケーションが自動的に再起動
8. **インストール完了**: 再起動時に新しいバージョンが適用される

## 期待される結果

修正後、以下のように動作します：

1. **権限エラーの解消**: ユーザーディレクトリにインストールされるため、管理者権限が不要
2. **スムーズなアップデート**: ダウンロード完了後、自動的に再起動してインストールが完了
3. **明確なフィードバック**: ユーザーに再起動が必要であることを明示的に通知

## テスト方法

### 1. ローカルでのテスト

```bash
# 開発環境で起動
pnpm tauri dev

# アップデートチェックを実行
# （開発環境ではリリースが存在しないため、エラーが表示されますが正常です）
```

### 2. 本番環境でのテスト

1. **最初のリリースを作成**:
   ```bash
   git checkout release
   git push origin release
   ```

2. **バージョンを更新して2回目のリリースを作成**:
   ```bash
   # package.jsonとtauri.conf.jsonのバージョンを更新
   # 例: 0.1.6 → 0.1.7
   git add package.json packages/desktop/src-tauri/tauri.conf.json
   git commit -m "chore: bump version to 0.1.7"
   git push origin release
   ```

3. **古いバージョンのアプリケーションを起動**:
   - アップデート通知が表示されることを確認
   - 「今すぐアップデート」をクリック
   - ダウンロード進捗が表示されることを確認
   - ダウンロード完了後、「インストール準備中...」と表示されることを確認
   - 3秒後にアプリケーションが再起動することを確認
   - 再起動後、新しいバージョンが適用されていることを確認

## トラブルシューティング

### エラーが継続する場合

1. **アプリケーションの完全終了**:
   - タスクマネージャー（Windows）またはアクティビティモニタ（macOS）でプロセスを確認
   - バックグラウンドで実行中のプロセスを終了

2. **手動インストール**:
   - GitHub Releasesから最新のインストーラーをダウンロード
   - 既存のアプリケーションをアンインストール
   - 新しいインストーラーを実行

3. **インストール場所の確認**:
   - **Windows**: `%LOCALAPPDATA%\Programs\orano-keihi`にインストールされているか確認
   - **macOS**: `~/Applications`または`/Applications`にインストールされているか確認

4. **ログの確認**:
   - アプリケーションログで詳細なエラーメッセージを確認
   - `updater.log`ファイルを確認

## 関連ドキュメント

- [AUTO_UPDATE_SETUP.md](./AUTO_UPDATE_SETUP.md) - 自動アップデート機能のセットアップ手順
- [AUTO_UPDATE_TROUBLESHOOTING.md](./AUTO_UPDATE_TROUBLESHOOTING.md) - トラブルシューティングガイド
- [BUILD_AND_RELEASE_GUIDE.md](./BUILD_AND_RELEASE_GUIDE.md) - ビルドとリリースの手順

## 参考資料

- [Tauri Updater Plugin Documentation](https://v2.tauri.app/plugin/updater/)
- [NSIS Installer Documentation](https://nsis.sourceforge.io/Docs/)

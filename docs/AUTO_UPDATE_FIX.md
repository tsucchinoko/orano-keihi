# 自動アップデート署名エラーの修正

## 問題の概要

Tauriアプリケーションの自動アップデート機能で、以下のエラーが発生していました：

```
アップデートのインストールに失敗しました: インストールエラー: アップデートのインストールに失敗しました: 
The signature dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIG1pbmlzaWduIHNlY3JldCBrZXkKUldTN0NHckpXaU9JR2RwZ0pIUVIwbTE2WGF0ei9CWVRvejdLTnRlclV0ZmlzdUluNmhpbDdTUHEK645f654f4d69dd207de6bdd8286388a1 
could not be decoded, please check if it is a valid base64 string.
```

## 根本原因

1. **macOS用の署名ファイル（`.app.tar.gz.sig`）がGitHub Releasesに存在しない**
   - GitHub Actionsのビルドプロセスで、単一のmacOSランナーしか使用していなかった
   - Intel（x86_64）とApple Silicon（aarch64）の両方のビルドが生成されていなかった

2. **`generate-update-manifest.cjs`が不正なダミー署名を生成していた**
   - 署名ファイルが見つからない場合、ハッシュ値を追加した不正なbase64文字列を生成
   - この署名はminisign形式として無効で、Tauriのアップデーターがデコードに失敗

## 実施した修正

### 1. `script/generate-update-manifest.cjs`の修正

署名ファイルが見つからない場合、エラーをスローするように変更：

```javascript
// 修正前: ダミー署名を生成
return `RWS7CGrJWiOIGdpgJHQR0m16Xatz/BYToz7KNterUtfisuIn6hil7SPq${hash.substring(0, 32)}`;

// 修正後: エラーをスロー
throw new Error(
    `署名ファイルが見つかりません: ${signatureFilePath}\n` +
    `Tauriビルドプロセスで署名ファイルが正しく生成されているか確認してください。`
);
```

### 2. `.github/workflows/release.yml`の修正

#### 2.1 macOSビルドのマトリックス化

Intel（x86_64）とApple Silicon（aarch64）の両方をビルドするように変更：

```yaml
build-macos:
  needs: version-tag
  strategy:
    matrix:
      include:
        - target: x86_64-apple-darwin
          runner: macos-13  # Intel Mac
          arch: x86_64
        - target: aarch64-apple-darwin
          runner: macos-14  # Apple Silicon
          arch: aarch64
  runs-on: ${{ matrix.runner }}
```

#### 2.2 成果物の統合

両アーキテクチャの成果物を統合するステップを追加：

```yaml
- name: MacOS成果物の統合
  run: |
    mkdir -p ./artifacts/macos-artifacts
    
    # x86_64とaarch64の成果物を統合
    if [ -d "./artifacts/macos-x86_64-artifacts" ]; then
      cp -r ./artifacts/macos-x86_64-artifacts/* ./artifacts/macos-artifacts/
    fi
    
    if [ -d "./artifacts/macos-aarch64-artifacts" ]; then
      cp -r ./artifacts/macos-aarch64-artifacts/* ./artifacts/macos-artifacts/
    fi
```

#### 2.3 ファイルコピーの改善

ワイルドカードを使ったファイルコピーを`find`コマンドに変更：

```bash
# 修正前
if [ -f packages/desktop/src-tauri/target/release/bundle/macos/*.app.tar.gz ]; then
  cp packages/desktop/src-tauri/target/release/bundle/macos/*.app.tar.gz ./macos-artifacts/
fi

# 修正後
find packages/desktop/src-tauri/target/release/bundle/macos -name "*.app.tar.gz" -type f ! -name "*.sig" -exec cp {} ./macos-artifacts/ \;
```

#### 2.4 署名ファイルの必須チェック

署名ファイルが存在しない場合、ビルドを失敗させるように変更：

```bash
if [ ! -f ./macos-artifacts/*.app.tar.gz.sig ]; then
  echo "❌ エラー: 署名ファイルが成果物に含まれていません"
  exit 1
fi
```

#### 2.5 複数ファイルのアップロード対応

全アーキテクチャのファイルをアップロードするように変更：

```javascript
// 修正前: 最初のファイルのみアップロード
await uploadFile(dmgPath, dmgFiles[0], 'MacOS DMGファイル');

// 修正後: 全ファイルをアップロード
for (const dmgFile of dmgFiles) {
  const dmgPath = path.join(macosArtifactsDir, dmgFile);
  await uploadFile(dmgPath, dmgFile, 'MacOS DMGファイル');
}
```

## 期待される結果

修正後、以下のファイルがGitHub Releasesに含まれるようになります：

### macOS（Intel - x86_64）
- `orano-keihi_X.X.X_x64.dmg` - ユーザー配布用
- `orano-keihi_X.X.X_x86_64.app.tar.gz` - アップデーター用
- `orano-keihi_X.X.X_x86_64.app.tar.gz.sig` - 署名ファイル

### macOS（Apple Silicon - aarch64）
- `orano-keihi_X.X.X_aarch64.dmg` - ユーザー配布用
- `orano-keihi_X.X.X_aarch64.app.tar.gz` - アップデーター用
- `orano-keihi_X.X.X_aarch64.app.tar.gz.sig` - 署名ファイル

### Windows（x86_64）
- `orano-keihi_X.X.X_x64_ja-JP.msi` - ユーザー配布用
- `orano-keihi_X.X.X_x64_ja-JP.msi.zip` - アップデーター用
- `orano-keihi_X.X.X_x64_ja-JP.msi.zip.sig` - 署名ファイル

### マニフェストファイル
- `darwin-x86_64.json` - macOS Intel用アップデート情報
- `darwin-aarch64.json` - macOS Apple Silicon用アップデート情報
- `windows-x86_64.json` - Windows用アップデート情報

## 検証方法

1. **releaseブランチにプッシュしてビルドを実行**
   ```bash
   git checkout release
   git merge main
   git push origin release
   ```

2. **GitHub Actionsのログを確認**
   - macOS x86_64ビルドが成功しているか
   - macOS aarch64ビルドが成功しているか
   - 両方の署名ファイルが生成されているか

3. **GitHub Releasesを確認**
   - 上記のすべてのファイルが含まれているか
   - 署名ファイル（`.sig`）が存在するか

4. **マニフェストファイルの内容を確認**
   ```bash
   gh release download <tag> --pattern "darwin-*.json"
   cat darwin-aarch64.json
   cat darwin-x86_64.json
   ```
   
   署名フィールドが正しいbase64形式になっているか確認

5. **アプリケーションでアップデートをテスト**
   - 古いバージョンのアプリを起動
   - アップデート通知が表示されるか
   - アップデートが正常にインストールされるか

## トラブルシューティング

### 署名ファイルが生成されない場合

1. GitHub Secretsを確認：
   - `TAURI_SIGNING_PRIVATE_KEY`が設定されているか
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`が設定されているか

2. ビルドログを確認：
   - 「Tauri署名ファイルの生成」ステップが成功しているか
   - 署名ファイルのパスが正しいか

### マニフェスト生成が失敗する場合

1. 成果物が正しくダウンロードされているか確認
2. `artifacts/macos-artifacts/`ディレクトリに`.sig`ファイルが存在するか確認
3. ファイル名のパターンが期待通りか確認

## 関連ドキュメント

- [AUTO_UPDATE_SETUP.md](./AUTO_UPDATE_SETUP.md) - 自動アップデート機能のセットアップ手順
- [AUTO_UPDATE_TROUBLESHOOTING.md](./AUTO_UPDATE_TROUBLESHOOTING.md) - トラブルシューティングガイド
- [BUILD_AND_RELEASE_GUIDE.md](./BUILD_AND_RELEASE_GUIDE.md) - ビルドとリリースの手順

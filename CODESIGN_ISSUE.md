# Code Signing Issue - macOS 26.1

## 問題の概要

Developer ID証明書でコード署名を実行すると、以下のエラーが発生します:

```
Warning: unable to build chain to self-signed root for signer errSecInternalComponent
```

## 調査結果

### ✅ 正常に動作している項目

1. **証明書のインストール**: Developer ID証明書と秘密鍵は正しくキーチェーンにインストールされている
2. **証明書チェーン**: Developer ID Certificate → Developer ID CA (G2) → Apple Root CA のチェーンは完全
3. **証明書の有効期限**: 2025年12月21日〜2030年12月22日（有効期間内）
4. **システム日時**: 正常
5. **Ad-hoc署名**: 正常に動作（`codesign -s -` は成功）
6. **OCSPサーバー接続**: 正常
7. **中間証明書**: Developer ID CA (G2)、Apple Root CA 正しくインストール済み
8. **キーチェーンACL**: パーティションリスト設定済み

### ❌ 問題の原因

**macOS 26.1 のシステムレベルのバグ**

- `errSecInternalComponent` (-2070) エラーは「内部コンポーネントエラー」
- セキュリティフレームワーク（securityd）のログに以下のエラー:
  ```
  CSSMERR_CSP_ACL_ENTRY_TAG_NOT_FOUND
  MacOS error: -2070
  ```
- すべての署名方法（名前指定、ハッシュ指定、タイムスタンプなし等）で同じエラー
- macOS 26.1 はベータ版の可能性が高い

## 試した解決策（すべて失敗）

1. ✗ キーチェーンのパーティションリスト設定
2. ✗ 証明書の削除と再インポート
3. ✗ DetachedSignaturesディレクトリの作成
4. ✗ キーチェーンデーモンの再起動
5. ✗ タイムスタンプサーバーの無効化
6. ✗ 異なる署名パラメータの試行

## 推奨される解決策

### オプション1: システムの完全再起動 ⭐️ 最優先

セキュリティデーモンの完全なリセットにより解決する可能性があります:

```bash
sudo reboot
```

再起動後、以下のコマンドでテスト:
```bash
echo "test" > /tmp/test_after_reboot
codesign -s "Developer ID Application: xxxxx" /tmp/test_after_reboot
```

### オプション2: Xcodeのインストール

Command Line Toolsだけでは不十分な場合があります:

1. App StoreからXcodeをインストール
2. Xcodeを起動して追加コンポーネントをインストール
3. 以下を実行:
   ```bash
   sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
   xcodebuild -runFirstLaunch
   ```

### オプション3: macOSのアップデート/ダウングレード

macOS 26.1がベータ版の場合:

- **ベータ版から安定版へ**: macOS 15.xの最新安定版にダウングレード
- **アップデート**: macOS 26.xの新しいビルドがある場合はアップデート

確認方法:
```bash
sw_vers
# ProductVersion: 26.1 ← これがベータの可能性
```

### オプション4: 一時的な回避策（開発環境のみ）

開発中は Ad-hoc 署名を使用:

```bash
# Ad-hoc署名（Gatekeeperは通過しない）
codesign -s - --force --deep your_app.app

# 実行時に以下で許可
xattr -dr com.apple.quarantine your_app.app
```

**注意**: この方法は配布用には使えません。開発・テスト専用です。

### オプション5: CI/CDで署名

ローカルで署名せず、GitHub Actionsなどで署名:

1. 証明書をP12形式でエクスポート:
   ```bash
   security export -k ~/Library/Keychains/login.keychain-db \
     -t identities -f pkcs12 \
     -P "your_password" \
     -o developer_id.p12 \
     "Developer ID Application: xxxxxx"
   ```

2. GitHub Secretsに保存:
   - `MACOS_CERTIFICATE`: Base64エンコードしたP12ファイル
   - `MACOS_CERTIFICATE_PWD`: P12のパスワード

3. GitHub Actionsで署名を実行

## 現在の環境情報

```
macOS Version: 26.1 (Build 25B78)
Command Line Tools: 26.1.0.0.1.1761104275
証明書: Developer ID Application: xxxxxx
証明書有効期限: 2025-12-21 ~ 2030-12-22
SHA-1: 08:01:30:78:69:C1:3E:FF:E5:1C:DE:E8:77:77:52:BA:00:90:A2:73
```

## 次のステップ

1. **システムを再起動** （最優先）
2. 再起動後もエラーが続く場合、Xcodeをインストール
3. それでも解決しない場合、macOSのバージョン変更を検討
4. 緊急の場合、CI/CDでの署名に切り替え

## 関連リンク

- [Apple Technical Note: Code Signing](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [errSecInternalComponent エラー](https://developer.apple.com/forums/tags/codesign)

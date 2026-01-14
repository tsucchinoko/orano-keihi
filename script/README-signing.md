# ローカルビルド署名ガイド

このガイドでは、`build-and-sign-local.sh`スクリプトを使用してローカル環境でmacOSアプリケーションに署名する方法を説明します。

## 前提条件

1. **Apple Developer証明書**: Apple Developer Programに登録し、Developer ID Application証明書を取得していること
2. **Tauri署名鍵**: アプリケーションの自動更新用の署名鍵

## セットアップ手順

### 1. Apple Developer証明書の準備

#### 証明書のエクスポート

1. キーチェーンアクセスを開く
2. 「証明書」カテゴリで「Developer ID Application」証明書を探す
3. 証明書を右クリック → 「書き出す」
4. ファイル形式: `.p12` (個人情報交換)
5. パスワードを設定して保存（このパスワードを覚えておく）

#### 証明書のbase64エンコード

```bash
# 証明書をbase64エンコード
base64 -i /path/to/your-certificate.p12 -o certificate.p12.base64
```

### 2. 環境変数の設定

`script/load-env.sh`ファイルを作成（または編集）して、以下の環境変数を設定します：

```bash
# Apple Developer証明書（base64エンコード済み）
export APPLE_CERTIFICATE=$(cat /path/to/certificate.p12.base64)

# 証明書のパスワード
export APPLE_CERTIFICATE_PASSWORD="your_certificate_password"

# キーチェーンのパスワード（省略可）
export KEYCHAIN_PASSWORD="build-keychain-password"

# Tauri署名鍵のパス
export TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/orano-keihi.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your_tauri_key_password"
```

**注意**: `script/load-env.sh`は`.gitignore`に含まれているため、秘密情報が誤ってコミットされることはありません。

### 3. Tauri署名鍵の生成（未作成の場合）

```bash
pnpm tauri signer generate -w ~/.tauri/orano-keihi.key
```

パスワードを求められたら、安全なパスワードを設定してください。

### 4. ビルドの実行

```bash
# macOSのみビルド
./script/build-and-sign-local.sh v0.1.2 macos

# 全プラットフォームビルド
./script/build-and-sign-local.sh v0.1.2 all
```

## スクリプトの動作

### 証明書のインポートプロセス

1. **一時キーチェーンの作成**: `build.keychain`という名前の専用キーチェーンを作成
2. **証明書のインポート**: base64デコードした証明書をキーチェーンにインポート
3. **アクセス権の設定**: `codesign`ツールが証明書にアクセスできるように設定
4. **証明書IDの抽出**: インポートされた証明書から署名IDを自動検出

### ビルドプロセス

1. **フロントエンドビルド**: `pnpm build`でReactアプリケーションをビルド
2. **macOSアプリケーションビルド**: Tauriを使用してネイティブアプリケーションを生成
3. **Apple Developer署名**: 検出された証明書IDを使用してアプリケーションに署名
4. **Tauri署名**: DMGファイルに対してTauriの署名を生成（自動更新用）

### クリーンアップ

スクリプト終了時に自動的に以下の処理が実行されます：

- `build.keychain`の削除
- デフォルトキーチェーンの復元
- 一時ファイルの削除

## トラブルシューティング

### `errSecInternalComponent`エラー

**原因**: 証明書チェーンが正しく構築されていない、または証明書がキーチェーンに正しくインポートされていない。

**解決方法**:
1. `APPLE_CERTIFICATE`環境変数が正しく設定されているか確認
2. 証明書のパスワードが正しいか確認
3. 証明書が有効期限内か確認

### 証明書が見つからない

**原因**: base64エンコードが正しくない、またはファイルパスが間違っている。

**解決方法**:
```bash
# base64エンコードを再実行
base64 -i /path/to/certificate.p12 -o certificate.p12.base64

# ファイルの内容を確認
head -c 100 certificate.p12.base64
```

### 証明書なしでビルド

Apple Developer証明書がない場合、スクリプトは警告を表示しますが、ビルドは続行されます。ただし、以下の制限があります：

- macOSのGatekeeperが未署名アプリをブロックする可能性がある
- ユーザーは以下のコマンドで検疫属性を削除する必要がある：
  ```bash
  xattr -d com.apple.quarantine /path/to/downloaded.dmg
  ```

## セキュリティのベストプラクティス

1. **証明書の保管**: `.p12`ファイルは安全な場所に保管し、パスワードで保護する
2. **環境変数**: `load-env.sh`ファイルをGitにコミットしない
3. **パスワード管理**: パスワードマネージャーを使用してパスワードを管理する
4. **キーチェーン**: ビルド完了後、専用キーチェーンは自動的に削除される

## 参考リンク

- [Apple Developer証明書について](https://developer.apple.com/support/certificates/)
- [Tauriコード署名](https://tauri.app/v1/guides/distribution/sign-macos/)
- [macOSアプリの配布](https://developer.apple.com/documentation/xcode/distributing-your-app-for-beta-testing-and-releases)

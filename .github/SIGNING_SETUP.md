# macOS署名設定ガイド

## 概要

このドキュメントでは、macOSアプリケーションの署名設定について説明します。署名により、エンドユーザーが安全にアプリケーションを実行できるようになります。

## 署名の種類

### 1. 開発用署名（Development Signing）
- 開発者のローカル環境でのテスト用
- 他のマシンでは実行できない
- 証明書不要

### 2. 配布用署名（Distribution Signing）
- App Store外での配布用
- `Developer ID Application`証明書が必要
- 公証（Notarization）が推奨

## 必要な証明書

### Developer ID Application証明書
1. Apple Developer Programに登録
2. Apple Developer Portalで証明書を作成
3. Keychain Accessで証明書をダウンロード・インストール
4. .p12形式でエクスポート

## GitHub Actions設定

### 必要なSecrets

以下のSecretsをGitHubリポジトリに設定してください：

```
MACOS_CERTIFICATE: Apple Developer証明書（Base64エンコード済み）
MACOS_CERTIFICATE_PWD: 証明書のパスワード
KEYCHAIN_PASSWORD: 一時Keychainのパスワード（オプション）
```

### 証明書のBase64エンコード方法

```bash
# .p12ファイルをBase64エンコード
base64 -i YourCertificate.p12 -o certificate_base64.txt

# エンコード結果をGitHub Secretsに設定
cat certificate_base64.txt
```

## 署名設定ファイル

### tauri.conf.json
```json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "Developer ID Application",
      "entitlements": "entitlements.plist"
    }
  }
}
```

### entitlements.plist
アプリケーションが必要とする権限を定義：
- ファイルシステムアクセス
- ネットワークアクセス
- サンドボックス設定

## 署名プロセス

### 1. 証明書の設定
```bash
# 証明書をKeychainにインポート
security import certificate.p12 -k build.keychain -P "password" -T /usr/bin/codesign

# 証明書の信頼設定
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "keychain-password" build.keychain
```

### 2. アプリケーションの署名
```bash
# アプリケーションに署名
codesign --sign "Developer ID Application" --entitlements entitlements.plist --deep --force YourApp.app

# dmgファイルに署名
codesign --sign "Developer ID Application" --deep --force YourApp.dmg
```

### 3. 署名の検証
```bash
# 署名の確認
codesign -dv YourApp.app
codesign -dv YourApp.dmg

# 署名の検証
codesign --verify --deep --strict YourApp.app
codesign --verify --deep --strict YourApp.dmg
```

## トラブルシューティング

### よくある問題

#### 1. 証明書が見つからない
```
error: The specified item could not be found in the keychain.
```

**解決方法:**
- 証明書が正しくKeychainにインポートされているか確認
- 証明書の有効期限を確認
- 適切な署名IDを使用しているか確認

#### 2. 権限エラー
```
error: The operation couldn't be completed. Unable to locate a valid signing identity.
```

**解決方法:**
- Keychainのアクセス権限を確認
- `security set-key-partition-list`コマンドを実行
- 証明書の信頼設定を確認

#### 3. Entitlementsエラー
```
error: entitlements are not supported for this platform
```

**解決方法:**
- entitlements.plistファイルの形式を確認
- 不要な権限を削除
- macOS固有の権限のみを使用

### デバッグコマンド

```bash
# 利用可能な署名IDを確認
security find-identity -v -p codesigning

# Keychainの内容を確認
security list-keychains
security dump-keychain build.keychain

# 署名の詳細情報を確認
codesign -dvvv YourApp.app

# Entitlementsの確認
codesign -d --entitlements :- YourApp.app
```

## 開発用ビルド

証明書が設定されていない場合、自動的に開発用ビルドとして実行されます：

- 署名設定が無効化される
- ローカル環境でのみ実行可能
- 配布には適さない

## プロダクション環境への移行

1. Apple Developer Programに登録
2. Developer ID Application証明書を取得
3. GitHub Secretsに証明書情報を設定
4. 公証プロセスの設定（推奨）

## 参考リンク

- [Apple Developer Documentation - Code Signing](https://developer.apple.com/documentation/security/code_signing_services)
- [Tauri Documentation - macOS Signing](https://tauri.app/v1/guides/distribution/sign-macos)
- [Apple Developer Portal](https://developer.apple.com/)

## 注意事項

- 証明書の有効期限に注意
- 秘密鍵は安全に管理
- 本番環境では公証も検討
- 定期的な署名検証の実施
# コード署名設定ガイド

## 概要

このドキュメントでは、MacOSとWindows向けのTauriアプリケーションにコード署名を設定する方法について説明します。コード署名により、ユーザーがアプリケーションをダウンロードして実行する際のセキュリティ警告を軽減できます。

## MacOS コード署名設定

### 必要な証明書

1. **Apple Developer Program**への登録が必要です
2. **Developer ID Application Certificate**を取得してください
3. **Developer ID Installer Certificate**（オプション、インストーラー用）

### GitHub Secrets設定

以下のシークレットをGitHubリポジトリに設定してください：

#### 必須設定

- `APPLE_CERTIFICATE`: 開発者証明書（.p12形式をBase64エンコード）
- `APPLE_CERTIFICATE_PASSWORD`: 証明書のパスワード
- `APPLE_SIGNING_IDENTITY`: 署名アイデンティティ（例：`Developer ID Application: Your Name (TEAM_ID)`）
- `KEYCHAIN_PASSWORD`: ビルド用キーチェーンのパスワード（任意の強力なパスワード）

#### 公証用設定（推奨）

- `APPLE_ID`: Apple IDのメールアドレス
- `APPLE_ID_PASSWORD`: App用パスワード（Apple IDの2ファクタ認証用）
- `APPLE_TEAM_ID`: Apple Developer TeamのID

### 証明書の準備手順

1. **Keychain Accessで証明書をエクスポート**
   ```bash
   # Keychain Accessを開く
   open /Applications/Utilities/Keychain\ Access.app
   
   # 証明書を選択して右クリック → "書き出す"
   # .p12形式で保存し、パスワードを設定
   ```

2. **証明書をBase64エンコード**
   ```bash
   base64 -i certificate.p12 -o certificate_base64.txt
   ```

3. **署名アイデンティティの確認**
   ```bash
   security find-identity -v -p codesigning
   ```

### App用パスワードの作成

1. [Apple ID管理ページ](https://appleid.apple.com/)にアクセス
2. 「サインインとセキュリティ」→「App用パスワード」
3. 新しいApp用パスワードを生成
4. 生成されたパスワードを`APPLE_ID_PASSWORD`として設定

## Windows コード署名設定

### 必要な証明書

1. **Code Signing Certificate**を認証局（CA）から取得
   - DigiCert、Sectigo、GlobalSignなどから購入可能
   - EV（Extended Validation）証明書を推奨

### GitHub Secrets設定

以下のシークレットをGitHubリポジトリに設定してください：

#### 必須設定

- `WINDOWS_CERTIFICATE`: コード署名証明書（.pfx形式をBase64エンコード）
- `WINDOWS_CERTIFICATE_PASSWORD`: 証明書のパスワード

#### オプション設定

- `WINDOWS_TIMESTAMP_URL`: タイムスタンプサーバーURL（デフォルト：`http://timestamp.digicert.com`）

### 証明書の準備手順

1. **証明書をPFX形式でエクスポート**
   ```powershell
   # 証明書管理コンソール（certmgr.msc）を開く
   # 証明書を選択して右クリック → "すべてのタスク" → "エクスポート"
   # PFX形式を選択し、パスワードを設定
   ```

2. **証明書をBase64エンコード**
   ```powershell
   # PowerShellで実行
   $bytes = [System.IO.File]::ReadAllBytes("certificate.pfx")
   $base64 = [System.Convert]::ToBase64String($bytes)
   $base64 | Out-File -FilePath "certificate_base64.txt"
   ```

## GitHub Secretsの設定方法

1. GitHubリポジトリページで「Settings」タブをクリック
2. 左サイドバーで「Secrets and variables」→「Actions」を選択
3. 「New repository secret」をクリック
4. 各シークレットを以下の形式で追加：

### MacOS用シークレット

```
Name: APPLE_CERTIFICATE
Value: [Base64エンコードされた.p12ファイルの内容]

Name: APPLE_CERTIFICATE_PASSWORD
Value: [証明書のパスワード]

Name: APPLE_SIGNING_IDENTITY
Value: Developer ID Application: Your Name (TEAM_ID)

Name: KEYCHAIN_PASSWORD
Value: [強力なパスワード]

Name: APPLE_ID
Value: your-apple-id@example.com

Name: APPLE_ID_PASSWORD
Value: [App用パスワード]

Name: APPLE_TEAM_ID
Value: [10文字のTeam ID]
```

### Windows用シークレット

```
Name: WINDOWS_CERTIFICATE
Value: [Base64エンコードされた.pfxファイルの内容]

Name: WINDOWS_CERTIFICATE_PASSWORD
Value: [証明書のパスワード]

Name: WINDOWS_TIMESTAMP_URL
Value: http://timestamp.digicert.com
```

## 署名の確認方法

### MacOS

```bash
# 署名の確認
codesign -dv --verbose=4 /path/to/app.app

# 公証の確認
spctl -a -vv /path/to/app.app

# dmgファイルの署名確認
codesign -dv --verbose=4 /path/to/app.dmg
```

### Windows

```powershell
# 署名の確認（PowerShell）
Get-AuthenticodeSignature "C:\path\to\app.exe"

# signtoolを使用した確認
signtool verify /pa /v "C:\path\to\app.exe"
```

## トラブルシューティング

### MacOS

#### 問題: "Developer ID Application certificate not found"
**解決方法:**
1. 証明書が正しくキーチェーンにインポートされているか確認
2. `APPLE_SIGNING_IDENTITY`の値が正確か確認
3. 証明書の有効期限を確認

#### 問題: 公証に失敗する
**解決方法:**
1. `APPLE_ID`と`APPLE_ID_PASSWORD`が正しく設定されているか確認
2. App用パスワードが正しく生成されているか確認
3. `APPLE_TEAM_ID`が正しく設定されているか確認

### Windows

#### 問題: "Certificate not found"
**解決方法:**
1. 証明書ファイルが正しくBase64エンコードされているか確認
2. `WINDOWS_CERTIFICATE_PASSWORD`が正しく設定されているか確認
3. 証明書の有効期限を確認

#### 問題: タイムスタンプエラー
**解決方法:**
1. `WINDOWS_TIMESTAMP_URL`を別のサーバーに変更
   - `http://timestamp.sectigo.com`
   - `http://timestamp.globalsign.com`
2. ネットワーク接続を確認

## セキュリティ上の注意事項

1. **証明書の保護**
   - 証明書ファイルは安全に保管し、不要になったら削除
   - パスワードは強力なものを使用

2. **GitHub Secretsの管理**
   - 定期的にシークレットをローテーション
   - 不要になったシークレットは削除

3. **アクセス制御**
   - リポジトリへのアクセス権限を適切に管理
   - 署名権限を持つユーザーを制限

## 参考リンク

### MacOS
- [Apple Developer Documentation - Code Signing](https://developer.apple.com/documentation/security/code_signing_services)
- [Notarizing macOS Software Before Distribution](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)

### Windows
- [Microsoft Docs - Code Signing](https://docs.microsoft.com/en-us/windows/win32/seccrypto/cryptography-tools)
- [DigiCert Code Signing Guide](https://www.digicert.com/code-signing/)

### Tauri
- [Tauri Code Signing Documentation](https://tauri.app/v1/guides/distribution/sign-macos)
- [Tauri Windows Code Signing](https://tauri.app/v1/guides/distribution/sign-windows)
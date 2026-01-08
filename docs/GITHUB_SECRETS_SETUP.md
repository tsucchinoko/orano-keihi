# GitHub Secrets設定ガイド

## 概要

DMGファイルの署名問題を解決するため、GitHub ActionsでTauri署名を統合しました。
以下のシークレットを設定してください。

## 必要なシークレット

### 既存のシークレット（確認）

以下のシークレットが既に設定されていることを確認してください：

```
MACOS_CERTIFICATE          # Apple Developer証明書（Base64エンコード）
MACOS_CERTIFICATE_PWD      # 証明書のパスワード
KEYCHAIN_PASSWORD          # キーチェーンのパスワード
APPLE_ID                   # Apple IDのメールアドレス
APPLE_ID_PASSWORD          # App用パスワード
APPLE_TEAM_ID              # Apple Developer TeamのID
```

### 新規追加が必要なシークレット

```
TAURI_SIGNING_PRIVATE_KEY          # Tauri署名用の秘密鍵
TAURI_SIGNING_PRIVATE_KEY_PASSWORD # 秘密鍵のパスワード（オプション）
```

## Tauri署名鍵の生成と設定

### 1. 署名鍵の生成

```bash
# Tauri署名鍵を生成
pnpm tauri signer generate -w ~/.tauri/orano-keihi.key

# または、パスワード付きで生成
pnpm tauri signer generate -w ~/.tauri/orano-keihi.key -p "your-password"
```

### 2. 秘密鍵の内容を取得

```bash
# 秘密鍵の内容を表示
cat ~/.tauri/orano-keihi.key
```

### 3. GitHub Secretsに設定

1. GitHubリポジトリの「Settings」→「Secrets and variables」→「Actions」
2. 「New repository secret」をクリック
3. 以下を設定：

```
Name: TAURI_SIGNING_PRIVATE_KEY
Value: [秘密鍵ファイルの内容をそのまま貼り付け]

Name: TAURI_SIGNING_PRIVATE_KEY_PASSWORD
Value: [パスワードを設定した場合のみ]
```

## 公開鍵の確認

```bash
# 公開鍵を表示
pnpm tauri signer generate -w ~/.tauri/orano-keihi.key --force

# 公開鍵をtauri.conf.jsonに設定済みか確認
grep -A 1 "pubkey" packages/desktop/src-tauri/tauri.conf.json
```

## 署名プロセスの流れ

### 修正後のプロセス

1. **GitHub Actions**: Apple Developer証明書でmacOS署名
2. **GitHub Actions**: Tauri署名（minisign）を生成
3. **GitHub Actions**: DMGファイルと署名ファイル（.sig）を両方アップロード
4. **自動アップデート**: 署名ファイルを使用して検証

### 利点

- 署名の競合が解消される
- 手動での署名作業が不要になる
- 一貫した署名プロセスが保証される

## トラブルシューティング

### 問題: 秘密鍵が見つからない

```bash
# 秘密鍵の場所を確認
ls -la ~/.tauri/

# 新しい鍵を生成
pnpm tauri signer generate -w ~/.tauri/orano-keihi.key --force
```

### 問題: 公開鍵が一致しない

```bash
# 現在の公開鍵を確認
pnpm tauri signer generate -w ~/.tauri/orano-keihi.key --force | grep "公開鍵"

# tauri.conf.jsonの公開鍵を更新
```

### 問題: GitHub Actionsでの署名に失敗

1. シークレットが正しく設定されているか確認
2. 秘密鍵の形式が正しいか確認（改行文字も含めて）
3. パスワードが設定されている場合は、パスワードシークレットも設定

## セキュリティ上の注意

- 秘密鍵は絶対に公開しないでください
- GitHub Secretsは暗号化されて保存されます
- 定期的に鍵をローテーションすることを推奨します
- 不要になった鍵は削除してください

## 参考リンク

- [Tauri Signing Documentation](https://tauri.app/v1/guides/distribution/sign-macos)
- [GitHub Secrets Documentation](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
# アップデーターセキュリティ機能

## 概要

このドキュメントでは、Tauri自動アップデート機能のセキュリティ実装について説明します。

## セキュリティ機能

### 1. デジタル署名検証

Tauri updaterプラグインは、すべてのアップデートファイルに対してデジタル署名検証を自動的に実行します。

#### 実装詳細

- **公開鍵**: `tauri.conf.json`の`plugins.updater.pubkey`に設定
- **署名アルゴリズム**: minisign（Ed25519ベース）
- **検証タイミング**: ダウンロード完了後、インストール前
- **検証失敗時の動作**: アップデートを中止し、エラーメッセージを表示

#### 公開鍵の管理

```json
{
  "plugins": {
    "updater": {
      "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDE5ODgyMzVBQzk2QTA4QkIKUldTN0NHckpXaU9JR2RwZ0pIUVIwbTE2WGF0ei9CWVRvejdLTnRlclV0ZmlzdUluNmhpbDdTUHEK"
    }
  }
}
```

公開鍵はアプリケーションにハードコードされており、外部からの変更を防ぎます。

### 2. HTTPS通信の強制

すべてのアップデート情報とファイルのダウンロードは、HTTPS通信を使用して行われます。

#### 実装詳細

- **エンドポイント**: `https://github.com/tsucchinoko/orano-keihi/releases/latest/download/{{target}}-{{arch}}.json`
- **プロトコル**: HTTPS（TLS 1.2以上）
- **証明書検証**: 自動的に実行
- **検証失敗時の動作**: 接続を中止し、エラーメッセージを表示

#### セキュリティチェック

`UpdaterService::perform_security_checks()`メソッドで以下のチェックを実行：

1. **HTTPSエンドポイント検証**: すべてのエンドポイントがHTTPSで始まることを確認
2. **署名検証の有効性確認**: 公開鍵が設定されていることを確認
3. **エンドポイントの信頼性確認**: 信頼できるドメイン（github.com）からの配信を確認

### 3. ハッシュ値検証

ダウンロードしたファイルの整合性を検証するため、SHA256ハッシュ値を計算・比較します。

#### 実装詳細

- **ハッシュアルゴリズム**: SHA256
- **検証タイミング**: ダウンロード完了後
- **検証方法**: `UpdaterService::verify_file_hash()`メソッド

```rust
fn verify_file_hash(&self, file_data: &[u8], expected_hash: &str) -> Result<(), UpdateError> {
    // SHA256ハッシュを計算
    let mut hasher = Sha256::new();
    hasher.update(file_data);
    let calculated_hash = hasher.finalize();
    let calculated_hash_hex = format!("{calculated_hash:x}");

    // ハッシュ値を比較
    if calculated_hash_hex.to_lowercase() != expected_hash.to_lowercase() {
        return Err(UpdateError::signature_verification(
            format!("ファイルのハッシュ値が一致しません")
        ));
    }

    Ok(())
}
```

**注**: 現在のTauri updaterプラグインは、署名検証によってファイルの整合性を保証しているため、
追加のハッシュ値検証は将来的な拡張として実装されています。

### 4. エラーハンドリング

セキュリティ関連のエラーは、適切にログに記録され、ユーザーに通知されます。

#### エラーの種類

1. **署名検証エラー**: 署名が無効または検証に失敗
2. **ネットワークエラー**: HTTPS接続に失敗
3. **ハッシュ検証エラー**: ファイルのハッシュ値が一致しない

#### エラー時の動作

- アップデートを即座に中止
- エラー情報をログファイルに記録
- ユーザーにセキュリティ警告を表示
- 再試行オプションを提供（署名検証エラーを除く）

## セキュリティベストプラクティス

### 開発者向け

1. **秘密鍵の管理**
   - 秘密鍵は安全な場所に保管
   - GitHub Secretsを使用してCI/CDで管理
   - 秘密鍵をリポジトリにコミットしない

2. **リリースプロセス**
   - すべてのリリースに署名を付与
   - GitHub Actionsで自動的に署名を生成
   - 署名ファイルをリリースに含める

3. **エンドポイントの管理**
   - 信頼できるドメインのみを使用
   - HTTPSエンドポイントのみを設定
   - エンドポイントの変更は慎重に行う

### ユーザー向け

1. **アップデートの確認**
   - アップデート通知を確認
   - リリースノートを読む
   - 不審なアップデートは報告

2. **セキュリティ警告**
   - 署名検証エラーが表示された場合は、アップデートを中止
   - 開発者に連絡して問題を報告
   - 公式サイトから最新情報を確認

## 監査とコンプライアンス

### セキュリティ監査

定期的にセキュリティ監査を実施し、以下の項目を確認：

- [ ] 署名検証が正しく機能しているか
- [ ] HTTPS通信が強制されているか
- [ ] エラーハンドリングが適切か
- [ ] ログ記録が正しく行われているか

### コンプライアンス

- **GDPR**: ユーザーデータの収集は行わない
- **セキュリティ標準**: OWASP Top 10に準拠
- **暗号化**: TLS 1.2以上を使用

## 参考資料

- [Tauri Updater Plugin Documentation](https://v2.tauri.app/plugin/updater/)
- [minisign Documentation](https://jedisct1.github.io/minisign/)
- [OWASP Secure Coding Practices](https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/)

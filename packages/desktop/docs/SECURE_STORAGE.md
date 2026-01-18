# セキュアストレージ実装ガイド

## 概要

`packages/desktop`では、セッショントークンやその他の秘匿情報を安全に保存するために、Tauri Storeプラグインを使用したセキュアストレージを実装しています。

## アーキテクチャ

### バックエンド（Rust）

#### SecureStorage構造体

`packages/desktop/src-tauri/src/features/auth/secure_storage.rs`に実装されています。

```rust
pub struct SecureStorage {
    app_handle: Arc<AppHandle>,
    store_name: String,
}
```

#### 主な機能

1. **セッショントークンの保存・取得**
   - `save_session_token()` - セッショントークンを保存
   - `get_session_token()` - セッショントークンを取得

2. **ユーザーIDの保存・取得**
   - `save_user_id()` - ユーザーIDを保存
   - `get_user_id()` - ユーザーIDを取得

3. **最終ログイン日時の保存・取得**
   - `save_last_login()` - 最終ログイン日時を保存
   - `get_last_login()` - 最終ログイン日時を取得

4. **認証情報の一括操作**
   - `save_auth_info()` - 認証情報をまとめて保存
   - `get_auth_info()` - 認証情報をまとめて取得
   - `clear_auth_info()` - すべての認証情報を削除（ログアウト時）

#### ストレージキー

```rust
pub struct SecureStorageKeys;

impl SecureStorageKeys {
    pub const SESSION_TOKEN: &'static str = "session_token";
    pub const USER_ID: &'static str = "user_id";
    pub const LAST_LOGIN: &'static str = "last_login";
}
```

### フロントエンド（TypeScript/Svelte）

#### 認証ストアの統合

`packages/desktop/src/lib/stores/auth.svelte.ts`で、セキュアストレージを使用するように更新されています。

```typescript
// 初期化時にセキュアストレージから認証情報を取得
const storedAuthInfo = await invoke<StoredAuthInfo | null>(
  'get_stored_auth_info'
);

if (storedAuthInfo) {
  this.sessionToken = storedAuthInfo.session_token;
  await this.checkSession();
}
```

## セキュリティ上の利点

### 1. プラットフォームネイティブの暗号化

Tauri Storeプラグインは、各プラットフォームのネイティブな暗号化機能を使用します：

- **macOS**: Keychain
- **Windows**: Data Protection API (DPAPI)
- **Linux**: Secret Service API

### 2. ローカルストレージとの比較

| 項目         | ローカルストレージ             | セキュアストレージ                 |
| ------------ | ------------------------------ | ---------------------------------- |
| 暗号化       | なし                           | あり（プラットフォームネイティブ） |
| アクセス制御 | なし                           | あり（アプリケーションのみ）       |
| 永続性       | ブラウザキャッシュクリアで削除 | アプリケーション削除まで保持       |
| セキュリティ | 低                             | 高                                 |

### 3. XSS攻撃からの保護

セキュアストレージはバックエンド（Rust）で管理されるため、フロントエンドのJavaScriptからは直接アクセスできません。これにより、XSS攻撃によるトークン窃取のリスクが大幅に軽減されます。

## 使用方法

### バックエンド（Rust）

#### 認証情報の保存

```rust
use crate::features::auth::secure_storage::{SecureStorage, StoredAuthInfo};

let secure_storage = SecureStorage::new(app_handle);
let auth_info = StoredAuthInfo {
    session_token: session_token.clone(),
    user_id: user.id.clone(),
    last_login: Utc::now().to_rfc3339(),
};

secure_storage.save_auth_info(&auth_info)?;
```

#### 認証情報の取得

```rust
let secure_storage = SecureStorage::new(app_handle);
let auth_info = secure_storage.get_auth_info()?;

if let Some(info) = auth_info {
    println!("セッショントークン: {}", info.session_token);
    println!("ユーザーID: {}", info.user_id);
    println!("最終ログイン: {}", info.last_login);
}
```

#### 認証情報の削除（ログアウト時）

```rust
let secure_storage = SecureStorage::new(app_handle);
secure_storage.clear_auth_info()?;
```

### フロントエンド（TypeScript）

#### Tauriコマンド経由でアクセス

```typescript
import { invoke } from '@tauri-apps/api/core';

// 認証情報の取得
const authInfo = await invoke<StoredAuthInfo | null>('get_stored_auth_info');

if (authInfo) {
  console.log('セッショントークン:', authInfo.session_token);
  console.log('ユーザーID:', authInfo.user_id);
  console.log('最終ログイン:', authInfo.last_login);
}
```

## ストレージファイルの場所

Tauri Storeプラグインは、以下の場所にストレージファイルを保存します：

- **macOS**: `~/Library/Application Support/com.orano-keihi.app/secure.json`
- **Windows**: `%APPDATA%\com.orano-keihi.app\secure.json`
- **Linux**: `~/.config/com.orano-keihi.app/secure.json`

ファイルは暗号化されており、アプリケーション以外からは読み取れません。

## 権限設定

`packages/desktop/src-tauri/capabilities/default.json`に以下の権限が設定されています：

```json
{
  "permissions": [
    "store:default",
    "store:allow-get",
    "store:allow-set",
    "store:allow-save",
    "store:allow-load",
    "store:allow-delete",
    "store:allow-clear"
  ]
}
```

## トラブルシューティング

### ストレージファイルが見つからない

初回起動時は自動的に作成されます。手動で作成する必要はありません。

### 認証情報が保存されない

1. 権限設定を確認してください（`capabilities/default.json`）
2. アプリケーションログを確認してください（`src-tauri/logs/app.log`）
3. ストレージディレクトリの書き込み権限を確認してください

### 認証情報が取得できない

1. セキュアストレージに認証情報が保存されているか確認してください
2. セッショントークンの有効期限が切れていないか確認してください
3. アプリケーションを再起動してみてください

## ベストプラクティス

1. **セッショントークンは必ずセキュアストレージに保存する**
   - ローカルストレージやクッキーには保存しない

2. **ログアウト時は必ず認証情報を削除する**
   - `clear_auth_info()`を呼び出す

3. **定期的にセッションを検証する**
   - アプリケーション起動時
   - 一定時間ごと（例：5分ごと）

4. **エラーハンドリングを適切に行う**
   - セキュアストレージへのアクセスエラーを適切に処理する
   - ユーザーに分かりやすいエラーメッセージを表示する

## 参考資料

- [Tauri Store Plugin Documentation](https://v2.tauri.app/plugin/store/)
- [Tauri Security Best Practices](https://v2.tauri.app/security/)

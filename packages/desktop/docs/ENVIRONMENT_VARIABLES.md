# 環境変数設定ガイド

## 概要

`packages/desktop`では、環境変数を以下の優先順位で取得します：

1. **起動時の環境変数**（最優先）
2. **コンパイル時の環境変数**（フォールバック）
3. **どちらも見つからない場合はエラー**

この仕組みにより、開発環境では`.env`ファイルを使用し、本番環境ではビルド時に環境変数を埋め込むことができます。

## 環境変数の取得フロー

```
起動時の環境変数を確認
    ↓ 見つかった
    ✓ 使用

    ↓ 見つからない

コンパイル時の環境変数を確認
    ↓ 見つかった
    ✓ 使用

    ↓ 見つからない

    ✗ エラー（パニック）
```

## 必須の環境変数

### 1. API_SERVER_URL

APIサーバーのベースURL

- **開発環境**: `http://localhost:8787`
- **本番環境**: `https://your-api-server.com`

### 2. SESSION_ENCRYPTION_KEY

セッション暗号化キー（32バイト以上推奨）

- **開発環境**: `development_encryption_key_32bytes`
- **本番環境**: ランダムな32バイト以上の文字列

### 3. R2設定

Cloudflare R2ストレージの設定

- **R2_ENDPOINT**: R2エンドポイントURL
- **R2_BUCKET_NAME**: バケット名
- **R2_ACCESS_KEY_ID**: アクセスキーID
- **R2_SECRET_ACCESS_KEY**: シークレットアクセスキー
- **R2_REGION**: リージョン（デフォルト: `auto`）

## オプションの環境変数

### API_TIMEOUT_SECONDS

APIリクエストのタイムアウト（秒）

- **デフォルト**: `30`

### API_MAX_RETRIES

APIリクエストの最大リトライ回数

- **デフォルト**: `3`

### LOG_LEVEL

ログレベル

- **デフォルト**: 開発環境は`debug`、本番環境は`info`
- **選択肢**: `error`, `warn`, `info`, `debug`, `trace`

## 開発環境での設定

### .envファイルの作成

`packages/desktop/src-tauri/.env`ファイルを作成します：

```bash
# APIサーバー設定
API_SERVER_URL=http://localhost:8787
SESSION_ENCRYPTION_KEY=development_encryption_key_32bytes

# R2設定
R2_ENDPOINT=https://your-account-id.r2.cloudflarestorage.com
R2_BUCKET_NAME=your-bucket-name
R2_ACCESS_KEY_ID=your-access-key-id
R2_SECRET_ACCESS_KEY=your-secret-access-key
R2_REGION=auto

# オプション設定
API_TIMEOUT_SECONDS=30
API_MAX_RETRIES=3
LOG_LEVEL=debug
```

### 開発サーバーの起動

```bash
# .envファイルが自動的に読み込まれます
pnpm tauri dev
```

## 本番環境での設定

### ビルド時の環境変数設定

本番ビルド時に環境変数を設定します：

```bash
# 環境変数を設定してビルド
export API_SERVER_URL=https://your-api-server.com
export SESSION_ENCRYPTION_KEY=your-production-encryption-key
export R2_ENDPOINT=https://your-account-id.r2.cloudflarestorage.com
export R2_BUCKET_NAME=your-production-bucket
export R2_ACCESS_KEY_ID=your-production-access-key
export R2_SECRET_ACCESS_KEY=your-production-secret-key

# ビルド実行
pnpm tauri build
```

### GitHub Actionsでの設定

`.github/workflows/release.yml`で環境変数を設定します：

```yaml
- name: Build Tauri App
  env:
    API_SERVER_URL: ${{ secrets.API_SERVER_URL }}
    SESSION_ENCRYPTION_KEY: ${{ secrets.SESSION_ENCRYPTION_KEY }}
    R2_ENDPOINT: ${{ secrets.R2_ENDPOINT }}
    R2_BUCKET_NAME: ${{ secrets.R2_BUCKET_NAME }}
    R2_ACCESS_KEY_ID: ${{ secrets.R2_ACCESS_KEY_ID }}
    R2_SECRET_ACCESS_KEY: ${{ secrets.R2_SECRET_ACCESS_KEY }}
  run: pnpm tauri build
```

### 起動時の環境変数設定（オプション）

ビルド後のアプリケーションでも、起動時に環境変数を設定できます：

```bash
# macOS/Linux
API_SERVER_URL=https://custom-api-server.com ./your-app

# Windows
set API_SERVER_URL=https://custom-api-server.com
your-app.exe
```

起動時の環境変数は、コンパイル時の環境変数よりも優先されます。

## 実装詳細

### build.rs

コンパイル時に環境変数を埋め込みます：

```rust
// .envファイルを読み込む
dotenv::dotenv().ok();

// 環境変数をコンパイル時に埋め込む
if let Ok(api_url) = std::env::var("API_SERVER_URL") {
    println!("cargo:rustc-env=COMPILE_TIME_API_SERVER_URL={api_url}");
}
```

### environment.rs

環境変数を取得するヘルパー関数：

```rust
/// 環境変数を取得する（優先順位: 起動時 > コンパイル時 > エラー）
pub fn get_env_var(
    runtime_var_name: &str,
    compile_time_var_name: &str
) -> Result<String, EnvVarError> {
    // 1. 起動時の環境変数を確認
    if let Ok(value) = std::env::var(runtime_var_name) {
        return Ok(value);
    }

    // 2. コンパイル時の環境変数を確認
    if let Ok(value) = std::env::var(compile_time_var_name) {
        return Ok(value);
    }

    // 3. どちらも見つからない場合はエラー
    Err(EnvVarError { ... })
}
```

### 使用例

```rust
use shared::config::environment::get_env_var;

// 必須の環境変数を取得
let api_url = get_env_var("API_SERVER_URL", "COMPILE_TIME_API_SERVER_URL")
    .expect("API_SERVER_URLが設定されていません");

// オプションの環境変数を取得（デフォルト値あり）
let timeout = get_env_var_or_default(
    "API_TIMEOUT_SECONDS",
    "COMPILE_TIME_API_TIMEOUT_SECONDS",
    "30"
);
```

## セキュリティ上の注意

### 1. .envファイルの管理

- `.env`ファイルは`.gitignore`に追加してください
- 秘匿情報を含むため、リポジトリにコミットしないでください
- `.env.example`ファイルをテンプレートとして提供してください

### 2. コンパイル時の環境変数

- コンパイル時に埋め込まれた環境変数は、バイナリに含まれます
- 秘匿情報（APIキー、パスワードなど）は慎重に扱ってください
- 本番ビルドは信頼できる環境（CI/CD）で実行してください

### 3. 起動時の環境変数

- 起動時の環境変数は、コンパイル時の環境変数を上書きできます
- 緊急時の設定変更に使用できます
- ユーザーが簡単に変更できるため、セキュリティリスクがあります

## トラブルシューティング

### 環境変数が見つからないエラー

```
thread 'main' panicked at 'API_SERVER_URLが設定されていません'
```

**解決方法**:

1. `.env`ファイルが存在するか確認
2. `.env`ファイルに`API_SERVER_URL`が設定されているか確認
3. ビルド時に環境変数が設定されているか確認

### .envファイルが読み込まれない

**開発環境**:

- `packages/desktop/src-tauri/.env`にファイルが存在するか確認
- ファイルの権限を確認

**本番環境**:

- 本番ビルドでは`.env`ファイルは読み込まれません
- ビルド時に環境変数を設定してください

### コンパイル時の環境変数が埋め込まれない

**確認事項**:

1. ビルド時に環境変数が設定されているか確認
2. `build.rs`が正しく実行されているか確認
3. ビルドログを確認

```bash
# ビルドログを確認
cargo build --verbose
```

## ベストプラクティス

1. **開発環境では.envファイルを使用**
   - 簡単に設定を変更できる
   - チーム間で設定を共有しやすい

2. **本番環境ではビルド時に環境変数を設定**
   - CI/CDパイプラインで自動化
   - GitHub Secretsなどで秘匿情報を管理

3. **起動時の環境変数は緊急時のみ使用**
   - 設定の上書きが必要な場合
   - デバッグやトラブルシューティング時

4. **環境変数の検証を実装**
   - 起動時に必須の環境変数をチェック
   - 不正な値を早期に検出

5. **ドキュメントを最新に保つ**
   - `.env.example`を更新
   - 新しい環境変数を追加したらドキュメントを更新

## 参考資料

- [Rust環境変数ガイド](https://doc.rust-lang.org/cargo/reference/environment-variables.html)
- [dotenvクレート](https://docs.rs/dotenv/)
- [Tauriビルドガイド](https://tauri.app/v1/guides/building/)

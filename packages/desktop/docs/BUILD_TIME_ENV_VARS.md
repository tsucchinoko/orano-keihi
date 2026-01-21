# ビルド時環境変数の埋め込み

## 概要

このドキュメントでは、Tauriアプリケーションのビルド時に環境変数を埋め込む仕組みについて説明します。

## 環境変数の優先順位

環境変数は以下の優先順位で取得されます：

1. **起動時の環境変数**（最優先）
   - アプリケーション起動時に設定されている環境変数
   - `std::env::var()`で取得

2. **コンパイル時の環境変数**（フォールバック）
   - `build.rs`で埋め込まれた環境変数
   - `option_env!()`マクロで取得

3. **デフォルト値**（オプション変数のみ）
   - 環境変数が見つからない場合のデフォルト値

## ビルド時の環境変数埋め込み

### build.rsの役割

`packages/desktop/src-tauri/build.rs`では、以下の処理を行います：

1. `.env`ファイルを読み込む
2. 必須の環境変数をチェック
3. 環境変数を`cargo:rustc-env`で埋め込む

```rust
// 必須の環境変数をコンパイル時に埋め込む
embed_env_var("API_SERVER_URL", true);

// オプションの環境変数をコンパイル時に埋め込む
embed_env_var("API_TIMEOUT_SECONDS", false);
embed_env_var("API_MAX_RETRIES", false);
embed_env_var("LOG_LEVEL", false);
embed_env_var("ENVIRONMENT", false);
```

### 埋め込まれる環境変数

#### 必須の環境変数

- `API_SERVER_URL`: APIサーバーのベースURL

#### オプションの環境変数

- `API_TIMEOUT_SECONDS`: APIリクエストのタイムアウト（秒）
- `API_MAX_RETRIES`: APIリクエストの最大リトライ回数
- `LOG_LEVEL`: ログレベル（error, warn, info, debug, trace）
- `ENVIRONMENT`: 実行環境（development, production）

## 開発環境と本番環境の違い

### 開発環境（`pnpm tauri dev`）

1. `build.rs`が`.env`ファイルを読み込む
2. 環境変数をコンパイル時に埋め込む
3. 実行時に`.env`ファイルを再度読み込む（起動時の環境変数として）

### 本番環境（`pnpm tauri build`）

1. `build.rs`が`.env`ファイルを読み込む
2. 環境変数をコンパイル時に埋め込む
3. 実行時は`.env`ファイルを読み込まない（埋め込まれた値を使用）

## 使用方法

### 環境変数の取得

`get_env_var!`マクロを使用して環境変数を取得します：

```rust
use crate::get_env_var;

// 必須の環境変数を取得
let api_url = get_env_var!("API_SERVER_URL")?;

// オプションの環境変数を取得（デフォルト値付き）
let timeout = get_env_var_or_default!("API_TIMEOUT_SECONDS", "30");
```

### ビルド時の環境変数設定

#### 方法1: .envファイルを使用

```bash
# packages/desktop/src-tauri/.env
API_SERVER_URL=https://api.example.com
ENVIRONMENT=production
```

```bash
pnpm tauri build
```

#### 方法2: 環境変数を直接設定

```bash
API_SERVER_URL=https://api.example.com ENVIRONMENT=production pnpm tauri build
```

#### 方法3: GitHub Actionsで設定

```yaml
- name: Build Tauri App
  env:
    API_SERVER_URL: ${{ secrets.API_SERVER_URL }}
    ENVIRONMENT: production
  run: pnpm tauri build
```

## トラブルシューティング

### 必須の環境変数が見つからない

ビルド時に以下のエラーが表示される場合：

```
エラー: 必須の環境変数 API_SERVER_URL が設定されていません
```

**解決方法:**

1. `.env`ファイルに環境変数を設定
2. または、環境変数を直接設定してビルド

### 環境変数が空の値

コンパイル時に埋め込まれた環境変数が空の場合、実行時にエラーが発生します：

```
環境変数 API_SERVER_URL はコンパイル時に埋め込まれていますが、値が空です
```

**解決方法:**

1. `.env`ファイルの環境変数に値を設定
2. 再ビルド

### 環境変数が更新されない

ビルド後に環境変数を変更しても反映されない場合：

**原因:**

環境変数はコンパイル時に埋め込まれるため、変更後は再ビルドが必要です。

**解決方法:**

```bash
# クリーンビルド
pnpm tauri build --clean
```

## セキュリティ上の注意

### 秘匿情報の取り扱い

- **開発環境**: `.env`ファイルに秘匿情報を保存可能（`.gitignore`に追加）
- **本番環境**: 秘匿情報はコンパイル時に埋め込まれるため、バイナリに含まれます

### 推奨事項

1. 秘匿情報は環境変数として設定し、`.env`ファイルはGitにコミットしない
2. 本番ビルドでは、CI/CDパイプラインで環境変数を設定
3. 公開するバイナリには秘匿情報を埋め込まない

## 参考資料

- [Tauri Environment Variables](https://tauri.app/v1/guides/building/environment-variables)
- [Cargo Build Scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Rust option_env! Macro](https://doc.rust-lang.org/std/macro.option_env.html)

# 設計書

## 概要

既存の経費管理アプリケーションの領収書保存機能を、ローカルファイルシステムからCloudflare R2クラウドストレージに移行する機能拡張。S3互換APIを使用してセキュアな領収書管理を実現し、デバイス間でのアクセス、自動バックアップ、オフライン対応を提供する。

### 技術スタック

- **既存技術**: SvelteKit 5、Tauri 2、Rust、SQLite
- **新規追加**: 
  - `aws-sdk-s3` (Rust) - S3互換API操作
  - `tokio` - 非同期処理
  - `serde_json` - 設定管理
  - ローカルキャッシュシステム

## アーキテクチャ

### システム構成

```
┌─────────────────────────────────────────┐
│         SvelteKit Frontend              │
│  ┌─────────────────────────────────┐   │
│  │  UI Components                  │   │
│  │  - ExpenseForm (更新)           │   │
│  │  - ReceiptViewer (更新)         │   │
│  │  - UploadProgress (新規)        │   │
│  └─────────────────────────────────┘   │
│              ↕                          │
│  ┌─────────────────────────────────┐   │
│  │  Tauri Commands                 │   │
│  │  - upload_receipt_to_r2         │   │
│  │  - get_receipt_from_r2          │   │
│  │  - delete_receipt_from_r2       │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
              ↕
┌─────────────────────────────────────────┐
│         Rust Backend (Tauri)            │
│  ┌─────────────────────────────────┐   │
│  │  R2 Service Layer               │   │
│  │  - r2_client.rs                 │   │
│  │  - presigned_urls.rs            │   │
│  │  - cache_manager.rs             │   │
│  └─────────────────────────────────┘   │
│              ↕                          │
│  ┌─────────────────────────────────┐   │
│  │  Database Layer (更新)          │   │
│  │  - receipt_url migration        │   │
│  │  - cache metadata table         │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
              ↕
┌─────────────────────────────────────────┐
│         External Services               │
│  ┌─────────────────────────────────┐   │
│  │  Cloudflare R2                  │   │
│  │  - Object Storage               │   │
│  │  - Presigned URLs               │   │
│  └─────────────────────────────────┘   │
│  ┌─────────────────────────────────┐   │
│  │  Local Cache                    │   │
│  │  - Downloaded receipts         │   │
│  │  - Metadata                     │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

### ディレクトリ構造の変更

```
src-tauri/
├── src/
│   ├── services/              # 新規追加
│   │   ├── mod.rs
│   │   ├── r2_client.rs       # R2クライアント
│   │   ├── cache_manager.rs   # ローカルキャッシュ管理
│   │   └── config.rs          # R2設定管理
│   ├── commands/
│   │   ├── receipt_commands.rs # 更新: R2対応
│   │   └── ...
│   ├── db/
│   │   ├── migrations.rs      # 更新: receipt_url migration
│   │   └── ...
│   └── models/
│       ├── expense.rs         # 更新: receipt_url対応
│       └── cache.rs           # 新規: キャッシュモデル
└── Cargo.toml                 # 更新: 新依存関係追加
```

## コンポーネントとインターフェース

### R2サービス層

#### 1. R2Client
Cloudflare R2との通信を管理

```rust
// src-tauri/src/services/r2_client.rs

use aws_sdk_s3::{Client, Config, Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;

pub struct R2Client {
    client: Client,
    bucket_name: String,
}

impl R2Client {
    /// R2クライアントを初期化
    pub async fn new(
        account_id: &str,
        access_key: &str,
        secret_key: &str,
        bucket_name: &str,
    ) -> Result<Self, R2Error> {
        // S3互換エンドポイント設定
        let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);
        
        let credentials = Credentials::new(access_key, secret_key, None, None, "r2");
        let config = Config::builder()
            .endpoint_url(endpoint)
            .credentials_provider(credentials)
            .region(Region::new("auto"))
            .build();
            
        let client = Client::from_conf(config);
        
        Ok(Self {
            client,
            bucket_name: bucket_name.to_string(),
        })
    }

    /// ファイルをR2にアップロード
    pub async fn upload_file(
        &self,
        key: &str,
        file_data: Vec<u8>,
        content_type: &str,
    ) -> Result<String, R2Error> {
        // アップロード実装
    }

    /// Presigned URLを生成（ダウンロード用）
    pub async fn generate_presigned_url(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> Result<String, R2Error> {
        // Presigned URL生成実装
    }

    /// ファイルをR2から削除
    pub async fn delete_file(&self, key: &str) -> Result<(), R2Error> {
        // 削除実装
    }

    /// 接続テスト
    pub async fn test_connection(&self) -> Result<(), R2Error> {
        // 接続テスト実装
    }
}
```

#### 2. CacheManager
ローカルキャッシュの管理

```rust
// src-tauri/src/services/cache_manager.rs

use std::path::PathBuf;
use tokio::fs;

pub struct CacheManager {
    cache_dir: PathBuf,
    max_cache_size: u64,
    max_age: Duration,
}

impl CacheManager {
    /// キャッシュマネージャーを初期化
    pub fn new(cache_dir: PathBuf, max_size_mb: u64) -> Self {
        Self {
            cache_dir,
            max_cache_size: max_size_mb * 1024 * 1024,
            max_age: Duration::from_secs(7 * 24 * 3600), // 7日間
        }
    }

    /// ファイルをキャッシュに保存
    pub async fn cache_file(
        &self,
        key: &str,
        data: Vec<u8>,
    ) -> Result<PathBuf, CacheError> {
        // キャッシュ保存実装
    }

    /// キャッシュからファイルを取得
    pub async fn get_cached_file(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        // キャッシュ取得実装
    }

    /// 古いキャッシュを削除
    pub async fn cleanup_old_cache(&self) -> Result<(), CacheError> {
        // キャッシュクリーンアップ実装
    }

    /// キャッシュサイズを管理
    pub async fn manage_cache_size(&self) -> Result<(), CacheError> {
        // サイズ管理実装
    }
}
```

### 更新されるTauriコマンド

```rust
// src-tauri/src/commands/receipt_commands.rs

#[tauri::command]
pub async fn upload_receipt_to_r2(
    expense_id: i64,
    file_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // 1. ファイルを読み込み
    // 2. R2にアップロード
    // 3. URLをデータベースに保存
    // 4. 成功時にHTTPS URLを返却
}

#[tauri::command]
pub async fn get_receipt_from_r2(
    receipt_url: String,
    state: State<'_, AppState>,
) -> Result<Vec<u8>, String> {
    // 1. キャッシュを確認
    // 2. キャッシュにない場合はR2から取得
    // 3. キャッシュに保存
    // 4. ファイルデータを返却
}

#[tauri::command]
pub async fn delete_receipt_from_r2(
    expense_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 1. データベースからreceipt_urlを取得
    // 2. R2からファイルを削除
    // 3. ローカルキャッシュからも削除
    // 4. データベースのreceipt_urlをNULLに更新
}

#[tauri::command]
pub async fn test_r2_connection(
    state: State<'_, AppState>,
) -> Result<bool, String> {
    // R2接続テスト
}
```

## データモデル

### データベーススキーマの変更

#### マイグレーション: receipt_path → receipt_url

```sql
-- Migration: 20241210_receipt_url_migration.sql

-- 1. 新しいカラムを追加
ALTER TABLE expenses ADD COLUMN receipt_url TEXT;

-- 2. 制約を追加（HTTPS URLのみ許可）
-- SQLiteでは制約の追加が制限されるため、新テーブルを作成して移行

CREATE TABLE expenses_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    amount REAL NOT NULL,
    category TEXT NOT NULL,
    description TEXT,
    receipt_url TEXT CHECK(receipt_url IS NULL OR receipt_url LIKE 'https://%'),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 3. データを移行（receipt_pathは無視、新規データはreceipt_urlを使用）
INSERT INTO expenses_new (id, date, amount, category, description, created_at, updated_at)
SELECT id, date, amount, category, description, created_at, updated_at
FROM expenses;

-- 4. 古いテーブルを削除し、新しいテーブルをリネーム
DROP TABLE expenses;
ALTER TABLE expenses_new RENAME TO expenses;

-- 5. インデックスを再作成
CREATE INDEX idx_expenses_date ON expenses(date);
CREATE INDEX idx_expenses_category ON expenses(category);
CREATE INDEX idx_expenses_receipt_url ON expenses(receipt_url);
```

#### キャッシュメタデータテーブル

```sql
-- キャッシュ管理用テーブル
CREATE TABLE receipt_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    receipt_url TEXT NOT NULL UNIQUE,
    local_path TEXT NOT NULL,
    cached_at TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    last_accessed TEXT NOT NULL
);

CREATE INDEX idx_receipt_cache_url ON receipt_cache(receipt_url);
CREATE INDEX idx_receipt_cache_accessed ON receipt_cache(last_accessed);
```

### 更新されるRustモデル

```rust
// src-tauri/src/models/expense.rs

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Expense {
    pub id: i64,
    pub date: String,
    pub amount: f64,
    pub category: String,
    pub description: Option<String>,
    pub receipt_url: Option<String>, // 変更: receipt_path → receipt_url
    pub created_at: String,
    pub updated_at: String,
}

// src-tauri/src/models/cache.rs (新規)

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReceiptCache {
    pub id: i64,
    pub receipt_url: String,
    pub local_path: String,
    pub cached_at: String,
    pub file_size: i64,
    pub last_accessed: String,
}
```

### TypeScript型の更新

```typescript
// src/lib/types/index.ts

export interface Expense {
  id: number;
  date: string;
  amount: number;
  category: string;
  description?: string;
  receipt_url?: string; // 変更: receipt_path → receipt_url
  created_at: string;
  updated_at: string;
}

// 新規追加
export interface UploadProgress {
  loaded: number;
  total: number;
  percentage: number;
}

export interface R2Config {
  account_id: string;
  access_key: string;
  secret_key: string;
  bucket_name: string;
  endpoint?: string;
}
```

## 設定管理

### 環境変数

```bash
# .env (開発環境)
R2_ACCOUNT_ID=your_account_id
R2_ACCESS_KEY=your_access_key
R2_SECRET_KEY=your_secret_key
R2_BUCKET_NAME=expense-receipts-dev
R2_REGION=auto

# 本番環境では異なるバケット名を使用
R2_BUCKET_NAME=expense-receipts-prod
```

### 設定ファイル

```rust
// src-tauri/src/services/config.rs

#[derive(Debug, Deserialize)]
pub struct R2Config {
    pub account_id: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket_name: String,
    pub region: String,
}

impl R2Config {
    /// 環境変数から設定を読み込み
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            account_id: env::var("R2_ACCOUNT_ID")?,
            access_key: env::var("R2_ACCESS_KEY")?,
            secret_key: env::var("R2_SECRET_KEY")?,
            bucket_name: env::var("R2_BUCKET_NAME")?,
            region: env::var("R2_REGION").unwrap_or_else(|_| "auto".to_string()),
        })
    }

    /// 設定の検証
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.account_id.is_empty() {
            return Err(ConfigError::MissingAccountId);
        }
        // その他の検証...
        Ok(())
    }
}
```

## エラーハンドリング

### カスタムエラー型

```rust
// src-tauri/src/services/mod.rs

#[derive(Debug, thiserror::Error)]
pub enum R2Error {
    #[error("R2 connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Upload failed: {0}")]
    UploadFailed(String),
    
    #[error("Download failed: {0}")]
    DownloadFailed(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Network error: {0}")]
    NetworkError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Cache write failed: {0}")]
    WriteFailed(String),
    
    #[error("Cache read failed: {0}")]
    ReadFailed(String),
    
    #[error("Cache cleanup failed: {0}")]
    CleanupFailed(String),
}
```

### リトライ機能

```rust
// リトライ付きアップロード
pub async fn upload_with_retry(
    client: &R2Client,
    key: &str,
    data: Vec<u8>,
    content_type: &str,
    max_retries: u32,
) -> Result<String, R2Error> {
    let mut attempts = 0;
    
    loop {
        match client.upload_file(key, data.clone(), content_type).await {
            Ok(url) => return Ok(url),
            Err(e) if attempts < max_retries => {
                attempts += 1;
                let delay = Duration::from_secs(2_u64.pow(attempts)); // 指数バックオフ
                tokio::time::sleep(delay).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## セキュリティ考慮事項

### 認証情報の管理

1. **環境変数**: 認証情報は環境変数から読み込み
2. **暗号化**: ローカル設定ファイルは暗号化して保存
3. **Presigned URL**: 一時的なアクセス権限で安全性を確保
4. **HTTPS強制**: すべての通信はHTTPS経由

### アクセス制御

```rust
// Presigned URLの有効期限設定
const PRESIGNED_URL_EXPIRY: Duration = Duration::from_secs(3600); // 1時間

// ファイルキーの生成（予測困難にする）
pub fn generate_file_key(expense_id: i64, filename: &str) -> String {
    let timestamp = chrono::Utc::now().timestamp();
    let uuid = uuid::Uuid::new_v4();
    format!("receipts/{}/{}-{}-{}", expense_id, timestamp, uuid, filename)
}
```

## パフォーマンス最適化

### 並列アップロード

```rust
// 複数ファイルの並列アップロード
pub async fn upload_multiple_files(
    client: &R2Client,
    files: Vec<(String, Vec<u8>, String)>, // (key, data, content_type)
) -> Result<Vec<String>, R2Error> {
    let upload_tasks: Vec<_> = files
        .into_iter()
        .map(|(key, data, content_type)| {
            let client = client.clone();
            tokio::spawn(async move {
                client.upload_file(&key, data, &content_type).await
            })
        })
        .collect();

    let results = futures::future::join_all(upload_tasks).await;
    
    // 結果を処理...
}
```

### キャッシュ戦略

1. **LRU**: 最近使用されていないファイルから削除
2. **サイズ制限**: 最大キャッシュサイズを設定
3. **有効期限**: 7日間でキャッシュを無効化
4. **プリロード**: よく使用されるファイルを事前キャッシュ

## テスト戦略

### 単体テスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_r2_upload() {
        // R2アップロードのテスト
    }

    #[tokio::test]
    async fn test_cache_management() {
        // キャッシュ管理のテスト
    }

    #[test]
    fn test_file_key_generation() {
        // ファイルキー生成のテスト
    }
}
```

### 統合テスト

- R2との実際の通信テスト
- データベースマイグレーションテスト
- エラーハンドリングテスト
- パフォーマンステスト

## 正確性プロパティ

*プロパティは、システムのすべての有効な実行において真であるべき特性や動作です。これらは人間が読める仕様と機械で検証可能な正確性保証の橋渡しとなります。*

プロパティ1: 領収書アップロードの完全性
*任意の*有効な領収書ファイルについて、アップロードが成功した場合、R2にファイルが保存され、データベースにHTTPS形式のreceipt_urlが記録される
**検証: 要件 1.1, 1.2**

プロパティ2: ファイル形式検証
*任意の*ファイルについて、対応形式（PNG、JPG、JPEG、PDF）以外のファイルはアップロードが拒否される
**検証: 要件 1.5**

プロパティ3: アップロード失敗時の状態保持
*任意の*アップロード操作について、R2への保存が失敗した場合、データベースの状態は変更されない
**検証: 要件 1.4**

プロパティ4: 領収書取得の一貫性
*任意の*有効なreceipt_urlについて、取得要求時にPresigned URLを使用してR2からファイルが取得される
**検証: 要件 2.1, 2.2**

プロパティ5: ファイル表示の対応形式
*任意の*有効な領収書ファイル（PNG、JPG、JPEG、PDF）について、システムは正しく表示処理を実行する
**検証: 要件 2.5**

プロパティ6: 認証情報の環境変数読み込み
*任意の*有効な環境変数設定について、システムはR2認証情報を正しく読み込み、検証する
**検証: 要件 3.1**

プロパティ7: 認証情報のログ保護
*任意の*ログ出力について、認証情報（アクセスキー、シークレットキー）は含まれない
**検証: 要件 3.5**

プロパティ8: ネットワークエラーリトライ
*任意の*ネットワークエラーについて、システムは最大3回まで自動リトライを実行する
**検証: 要件 4.2**

プロパティ9: ファイルサイズ事前検証
*任意の*ファイルについて、サイズ制限（10MB）を超える場合、アップロード前に拒否される
**検証: 要件 4.3**

プロパティ10: 並列アップロード対応
*任意の*複数ファイルセットについて、システムは同時アップロードを正しく処理する
**検証: 要件 4.5**

プロパティ11: URL形式制約
*任意の*receipt_urlについて、HTTPS形式以外のURLはデータベースで拒否される
**検証: 要件 5.2**

プロパティ12: マイグレーション失敗時のロールバック
*任意の*マイグレーション失敗について、システムは自動的にデータベースを元の状態に復元する
**検証: 要件 5.4**

プロパティ13: 経費削除時の領収書削除
*任意の*領収書付き経費について、経費削除時にR2からの領収書削除が完了してからデータベースレコードが削除される
**検証: 要件 6.1, 6.2**

プロパティ14: R2削除失敗時の状態保持
*任意の*削除操作について、R2からの削除が失敗した場合、データベースの状態は変更されない
**検証: 要件 6.3**

プロパティ15: 削除操作のログ記録
*任意の*削除操作について、システムは操作ログを記録する
**検証: 要件 6.5**

プロパティ16: R2接続テスト
*任意の*R2設定について、接続テスト機能は設定の有効性を正しく判定する
**検証: 要件 7.1**

プロパティ17: 環境別設定
*任意の*環境設定について、システムは環境に応じて適切なR2バケットを使用する
**検証: 要件 7.3**

プロパティ18: 領収書キャッシュ保存
*任意の*表示された領収書について、システムはローカルキャッシュに保存する
**検証: 要件 8.1**

プロパティ19: オフライン時キャッシュ表示
*任意の*キャッシュされた領収書について、オフライン時でもシステムは表示可能である
**検証: 要件 8.2**

プロパティ20: キャッシュライフサイクル管理
*任意の*キャッシュファイルについて、有効期限切れまたはサイズ上限超過時に適切に削除される
**検証: 要件 8.3, 8.5**

## テスト戦略

### 単体テスト

**Rust（バックエンド）**
- R2クライアントの各操作（アップロード、ダウンロード、削除）
- キャッシュマネージャーの動作
- 設定読み込みと検証
- エラーハンドリング
- データベースマイグレーション

**TypeScript（フロントエンド）**
- ファイルアップロードUI
- プログレス表示
- エラー表示
- キャッシュ状態管理

### プロパティベーステスト

プロパティベーステストには**QuickCheck for Rust**を使用し、各テストは最小100回の反復実行を行います。

各プロパティベーステストは、設計書の対応する正確性プロパティを実装し、以下の形式でコメントタグを付けます：
`**Feature: cloudflare-r2-receipts, Property {番号}: {プロパティテキスト}**`

### 統合テスト

**R2統合**
- 実際のR2サービスとの通信
- Presigned URL生成と使用
- 大容量ファイルのアップロード
- ネットワーク障害シミュレーション

**データベース統合**
- マイグレーション実行
- 制約検証
- トランザクション処理

### 手動テストチェックリスト

- [ ] 各種ファイル形式のアップロード
- [ ] オフライン/オンライン切り替え
- [ ] キャッシュ動作確認
- [ ] エラーハンドリング
- [ ] パフォーマンス測定
- [ ] セキュリティ検証

## 依存関係

### Cargo.toml追加項目

```toml
[dependencies]
# 既存の依存関係...

# R2/S3互換API
aws-config = "1.0"
aws-sdk-s3 = "1.0"

# 非同期処理
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"

# UUID生成
uuid = { version = "1.0", features = ["v4"] }

# 設定管理
serde_json = "1.0"

# エラーハンドリング
thiserror = "1.0"

# テスト用
[dev-dependencies]
quickcheck = "1.0"
quickcheck_macros = "1.0"
tempfile = "3.0"
```

## デプロイメント考慮事項

### 環境変数設定

```bash
# 開発環境
export R2_ACCOUNT_ID="dev_account_id"
export R2_BUCKET_NAME="expense-receipts-dev"

# 本番環境
export R2_ACCOUNT_ID="prod_account_id"
export R2_BUCKET_NAME="expense-receipts-prod"
```

### R2バケット設定

```bash
# バケット作成（Cloudflare CLI使用）
wrangler r2 bucket create expense-receipts-dev
wrangler r2 bucket create expense-receipts-prod

# CORS設定
wrangler r2 bucket cors put expense-receipts-dev --file cors.json
```

### セキュリティ設定

- R2 APIトークンの最小権限設定
- バケットアクセス制限
- Presigned URLの有効期限設定
- ローカルキャッシュの暗号化（将来的に）

## 監視とログ

### メトリクス

- アップロード成功/失敗率
- 平均アップロード時間
- キャッシュヒット率
- R2 API使用量
- エラー発生頻度

### ログ形式

```rust
// 構造化ログ
info!(
    target: "r2_operations",
    operation = "upload",
    expense_id = expense_id,
    file_size = file_size,
    duration_ms = duration.as_millis(),
    "Receipt uploaded successfully"
);
```

## 将来の拡張

- 複数クラウドプロバイダー対応
- 自動バックアップ機能
- 領収書の自動分類
- OCR機能統合
- 暗号化オプション
- 監査ログ機能
# 設計書

## 概要

現在のバックエンドアーキテクチャを技術レイヤー別（commands, models, db, services）から機能別（package by feature）アプローチにリファクタリングする。各機能モジュールは、その機能に関連するすべてのコード（モデル、コマンド、データベース操作、サービス）を含む自己完結型のユニットとなる。

### 現在の構造

```
src-tauri/src/
├── commands/
│   ├── expense_commands.rs
│   ├── subscription_commands.rs
│   ├── receipt_commands.rs
│   ├── security_commands.rs
│   └── migration_commands.rs
├── models/
│   ├── expense.rs
│   └── subscription.rs
├── db/
│   ├── expense_operations.rs
│   ├── subscription_operations.rs
│   └── migrations.rs
├── services/
│   ├── r2_client.rs
│   ├── cache_manager.rs
│   └── security.rs
└── config/
    ├── environment.rs
    └── initialization.rs
```

### 目標の構造

```
src-tauri/src/
├── features/
│   ├── expenses/
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   ├── commands.rs
│   │   ├── repository.rs
│   │   └── tests.rs
│   ├── subscriptions/
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   ├── commands.rs
│   │   ├── repository.rs
│   │   └── tests.rs
│   ├── receipts/
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   ├── commands.rs
│   │   ├── service.rs
│   │   ├── cache.rs
│   │   └── tests.rs
│   ├── security/
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   ├── commands.rs
│   │   ├── service.rs
│   │   └── tests.rs
│   └── migrations/
│       ├── mod.rs
│       ├── commands.rs
│       ├── service.rs
│       └── tests.rs
└── shared/
    ├── database/
    │   ├── mod.rs
    │   └── connection.rs
    ├── config/
    │   ├── mod.rs
    │   ├── environment.rs
    │   └── initialization.rs
    ├── errors/
    │   └── mod.rs
    └── utils/
        └── mod.rs
```

## アーキテクチャ

### 機能モジュール構造

各機能モジュールは以下の標準的な構造を持つ：

1. **mod.rs**: モジュールの公開インターフェースを定義
2. **models.rs**: データモデルとDTO（Data Transfer Objects）
3. **commands.rs**: Tauriコマンドハンドラー
4. **repository.rs**: データベース操作（CRUD）
5. **service.rs**: ビジネスロジックと外部サービス連携（必要な場合）
6. **tests.rs**: ユニットテストと統合テスト

### レイヤー分離

```
┌─────────────────────────────────────┐
│     Tauri Commands Layer            │  ← フロントエンドとの境界
│  (commands.rs in each feature)      │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│     Business Logic Layer            │
│  (service.rs in each feature)       │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│     Data Access Layer               │
│  (repository.rs in each feature)    │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│     Shared Database Connection      │
│  (shared/database)                  │
└─────────────────────────────────────┘
```

## コンポーネントとインターフェース

### 1. Expenses Feature（経費機能）

#### 責務
- 経費の作成、読み取り、更新、削除
- 経費データのバリデーション
- 月別・カテゴリ別の経費取得

#### 公開インターフェース
```rust
// commands.rs
pub async fn create_expense(dto: CreateExpenseDto, state: State<AppState>) -> Result<Expense, String>
pub async fn get_expenses(month: Option<String>, category: Option<String>, state: State<AppState>) -> Result<Vec<Expense>, String>
pub async fn update_expense(id: i64, dto: UpdateExpenseDto, state: State<AppState>) -> Result<Expense, String>
pub async fn delete_expense(id: i64, state: State<AppState>) -> Result<(), String>

// repository.rs
pub fn create(conn: &Connection, dto: CreateExpenseDto) -> Result<Expense>
pub fn find_by_id(conn: &Connection, id: i64) -> Result<Expense>
pub fn find_all(conn: &Connection, month: Option<&str>, category: Option<&str>) -> Result<Vec<Expense>>
pub fn update(conn: &Connection, id: i64, dto: UpdateExpenseDto) -> Result<Expense>
pub fn delete(conn: &Connection, id: i64) -> Result<()>
pub fn set_receipt_url(conn: &Connection, id: i64, url: String) -> Result<Expense>
```

#### 内部構造
- バリデーションロジック（金額、日付、説明文字数）
- 日付処理（JST対応）
- エラーハンドリング

### 2. Subscriptions Feature（サブスクリプション機能）

#### 責務
- サブスクリプションの作成、読み取り、更新、削除
- サブスクリプションの有効/無効切り替え
- 月額合計の計算

#### 公開インターフェース
```rust
// commands.rs
pub async fn create_subscription(dto: CreateSubscriptionDto, state: State<AppState>) -> Result<Subscription, String>
pub async fn get_subscriptions(active_only: Option<bool>, state: State<AppState>) -> Result<Vec<Subscription>, String>
pub async fn update_subscription(id: i64, dto: UpdateSubscriptionDto, state: State<AppState>) -> Result<Subscription, String>
pub async fn toggle_subscription_status(id: i64, state: State<AppState>) -> Result<Subscription, String>
pub async fn get_monthly_subscription_total(state: State<AppState>) -> Result<f64, String>

// repository.rs
pub fn create(conn: &Connection, dto: CreateSubscriptionDto) -> Result<Subscription>
pub fn find_by_id(conn: &Connection, id: i64) -> Result<Subscription>
pub fn find_all(conn: &Connection, active_only: bool) -> Result<Vec<Subscription>>
pub fn update(conn: &Connection, id: i64, dto: UpdateSubscriptionDto) -> Result<Subscription>
pub fn toggle_status(conn: &Connection, id: i64) -> Result<Subscription>
pub fn calculate_monthly_total(conn: &Connection) -> Result<f64>
```

### 3. Receipts Feature（領収書機能）

#### 責務
- 領収書のアップロード、取得、削除（R2連携）
- 領収書のキャッシュ管理
- オフライン対応
- 複数ファイルの並列アップロード
- パフォーマンス統計とデバッグ情報

#### 公開インターフェース
```rust
// commands.rs
pub async fn upload_receipt_to_r2(expense_id: i64, file_path: String, state: State<AppState>) -> Result<String, String>
pub async fn get_receipt_from_r2(receipt_url: String, state: State<AppState>) -> Result<Vec<u8>, String>
pub async fn delete_receipt_from_r2(receipt_url: String, state: State<AppState>) -> Result<(), String>
pub async fn upload_multiple_receipts_to_r2(uploads: Vec<ReceiptUpload>, state: State<AppState>) -> Result<Vec<UploadResult>, String>
pub async fn get_receipt_offline(receipt_url: String, state: State<AppState>) -> Result<Vec<u8>, String>
pub async fn sync_cache_on_online(state: State<AppState>) -> Result<SyncResult, String>

// service.rs (R2Client)
pub async fn upload_file(&self, file_path: &str, expense_id: i64) -> Result<String>
pub async fn download_file(&self, file_key: &str) -> Result<Vec<u8>>
pub async fn delete_file(&self, file_key: &str) -> Result<()>
pub async fn test_connection(&self) -> Result<bool>

// cache.rs (CacheManager)
pub fn cache_receipt(&self, receipt_url: &str, data: &[u8]) -> Result<String>
pub fn get_cached_receipt(&self, receipt_url: &str) -> Result<Option<Vec<u8>>>
pub fn clear_old_cache(&self, days: i64) -> Result<usize>
```

### 4. Security Feature（セキュリティ機能）

#### 責務
- 環境設定の管理と検証
- セキュリティイベントのログ記録
- システム診断情報の提供
- R2接続のセキュアなテスト

#### 公開インターフェース
```rust
// commands.rs
pub async fn get_system_diagnostic_info(state: State<AppState>) -> Result<DiagnosticInfo, String>
pub async fn validate_security_configuration(state: State<AppState>) -> Result<ValidationResult, String>
pub async fn test_r2_connection_secure(state: State<AppState>) -> Result<ConnectionTestResult, String>
pub async fn log_security_event(event_type: String, details: String, state: State<AppState>) -> Result<(), String>

// service.rs (SecurityManager)
pub fn validate_configuration(&self) -> Result<()>
pub fn log_security_event(&self, event_type: &str, details: &str)
pub fn get_diagnostic_info(&self) -> DiagnosticInfo
pub fn is_production(&self) -> bool
```

### 5. Migrations Feature（マイグレーション機能）

#### 責務
- データベーススキーマのマイグレーション
- バックアップの作成と復元
- マイグレーション状態の確認

#### 公開インターフェース
```rust
// commands.rs
pub async fn check_migration_status(state: State<AppState>) -> Result<MigrationStatus, String>
pub async fn execute_receipt_url_migration(state: State<AppState>) -> Result<MigrationResult, String>
pub async fn restore_database_from_backup(backup_path: String, state: State<AppState>) -> Result<RestoreResult, String>
pub async fn drop_receipt_path_column_command(state: State<AppState>) -> Result<MigrationResult, String>

// service.rs
pub fn is_receipt_url_migration_complete(conn: &Connection) -> Result<bool>
pub fn migrate_receipt_path_to_url(conn: &Connection) -> Result<MigrationResult>
pub fn create_backup(conn: &Connection) -> Result<String>
pub fn drop_receipt_path_column(conn: &Connection) -> Result<MigrationResult>
```

### 6. Shared Modules（共有モジュール）

#### Database Module
```rust
// shared/database/connection.rs
pub fn initialize_database(app_handle: &AppHandle) -> Result<Connection>
pub fn get_database_path(app_handle: &AppHandle) -> Result<PathBuf>
pub fn create_tables(conn: &Connection) -> Result<()>
```

#### Config Module
```rust
// shared/config/environment.rs
pub struct EnvironmentConfig {
    pub environment: String,
    pub debug_mode: bool,
    pub log_level: String,
}

impl EnvironmentConfig {
    pub fn from_env() -> Self
    pub fn is_production(&self) -> bool
    pub fn is_development(&self) -> bool
}

// shared/config/initialization.rs
pub struct InitializationResult {
    pub is_first_run: bool,
    pub database_path: PathBuf,
    pub environment: String,
}

pub fn initialize_app(app_handle: &AppHandle) -> Result<InitializationResult>
```

#### Errors Module
```rust
// shared/errors/mod.rs
pub enum AppError {
    DatabaseError(String),
    ValidationError(String),
    NotFoundError(String),
    ExternalServiceError(String),
    SecurityError(String),
}

impl AppError {
    pub fn user_message(&self) -> &str
    pub fn details(&self) -> &str
    pub fn severity(&self) -> ErrorSeverity
}
```

## データモデル

### Expense（経費）
```rust
pub struct Expense {
    pub id: i64,
    pub date: String,           // YYYY-MM-DD形式
    pub amount: f64,            // 正の数値、10桁以内
    pub category: String,       // カテゴリ名
    pub description: Option<String>, // 500文字以内
    pub receipt_url: Option<String>, // R2のURL
    pub created_at: String,     // RFC3339形式（JST）
    pub updated_at: String,     // RFC3339形式（JST）
}

pub struct CreateExpenseDto {
    pub date: String,
    pub amount: f64,
    pub category: String,
    pub description: Option<String>,
}

pub struct UpdateExpenseDto {
    pub date: Option<String>,
    pub amount: Option<f64>,
    pub category: Option<String>,
    pub description: Option<String>,
}
```

### Subscription（サブスクリプション）
```rust
pub struct Subscription {
    pub id: i64,
    pub name: String,           // サービス名、100文字以内
    pub amount: f64,            // 正の数値、10桁以内
    pub billing_cycle: String,  // "monthly" または "annual"
    pub start_date: String,     // YYYY-MM-DD形式
    pub category: String,       // カテゴリ名
    pub is_active: bool,        // 有効/無効
    pub receipt_path: Option<String>, // 領収書パス（将来的にreceipt_urlに移行）
    pub created_at: String,     // RFC3339形式（JST）
    pub updated_at: String,     // RFC3339形式（JST）
}

pub struct CreateSubscriptionDto {
    pub name: String,
    pub amount: f64,
    pub billing_cycle: String,
    pub start_date: String,
    pub category: String,
}

pub struct UpdateSubscriptionDto {
    pub name: Option<String>,
    pub amount: Option<f64>,
    pub billing_cycle: Option<String>,
    pub start_date: Option<String>,
    pub category: Option<String>,
}
```

### Receipt Cache（領収書キャッシュ）
```rust
pub struct ReceiptCache {
    pub id: i64,
    pub receipt_url: String,    // R2のURL
    pub local_path: String,     // ローカルキャッシュパス
    pub cached_at: String,      // キャッシュ作成日時
    pub file_size: i64,         // ファイルサイズ（バイト）
    pub last_accessed: String,  // 最終アクセス日時
}
```

## 正当性プロパティ

*プロパティとは、システムのすべての有効な実行において真であるべき特性や振る舞いのことです。これは、人間が読める仕様と機械で検証可能な正当性保証の橋渡しとなります。*

### プロパティ反映

プロパティの冗長性を排除するため、以下の統合を行う：

- **プロパティ1-5**: 各機能モジュールの配置確認は、単一のディレクトリ構造検証プロパティに統合
- **プロパティ2.1-2.5**: モジュール間の依存関係とインターフェース規則は、モジュール境界検証プロパティに統合
- **プロパティ3.1-3.4**: リファクタリング後の機能保持は、後方互換性検証プロパティに統合
- **プロパティ4.1-4.5**: 共通機能の使用は、共有モジュール使用検証プロパティに統合

### 正当性プロパティ

**プロパティ1: 機能別ディレクトリ構造の正当性**
*すべての*機能について、その機能に関連するすべてのコード（モデル、コマンド、データベース操作、サービス）が対応するfeatures/{feature_name}/ディレクトリ内に配置されている
**検証対象: 要件 1.1, 1.2, 1.3, 1.4, 1.5**

**プロパティ2: モジュール境界の正当性**
*すべての*機能モジュール間の相互作用について、明確に定義されたパブリックインターフェースを通じてのみアクセスが行われ、内部実装が適切に隠蔽されている
**検証対象: 要件 2.1, 2.2, 2.3, 2.4, 2.5**

**プロパティ3: 後方互換性の保持**
*すべての*既存のTauriコマンド、データベース操作、エラーハンドリング、セキュリティ機能について、リファクタリング前後で同じシグネチャと動作を維持している
**検証対象: 要件 3.1, 3.2, 3.3, 3.4**

**プロパティ4: 共有モジュールの一貫した使用**
*すべての*機能モジュールについて、共通機能（データベース接続、設定、エラーハンドリング、ログ、バリデーション）へのアクセスがshared/モジュールを通じて行われている
**検証対象: 要件 4.1, 4.2, 4.3, 4.4, 4.5**

**プロパティ5: 段階的リファクタリングの独立性**
*任意の*機能モジュールをリファクタリングする際、他の機能モジュールの動作に影響を与えず、アプリケーション全体が正常に動作し続ける
**検証対象: 要件 5.1, 5.2, 5.5**

## エラーハンドリング

### 統一されたエラー処理

```rust
// shared/errors/mod.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("データベースエラー: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("バリデーションエラー: {0}")]
    Validation(String),
    
    #[error("リソースが見つかりません: {0}")]
    NotFound(String),
    
    #[error("外部サービスエラー: {0}")]
    ExternalService(String),
    
    #[error("セキュリティエラー: {0}")]
    Security(String),
    
    #[error("設定エラー: {0}")]
    Configuration(String),
}

impl AppError {
    pub fn user_message(&self) -> &str {
        match self {
            AppError::Database(_) => "データベース操作でエラーが発生しました",
            AppError::Validation(msg) => msg,
            AppError::NotFound(msg) => msg,
            AppError::ExternalService(_) => "外部サービスとの通信でエラーが発生しました",
            AppError::Security(_) => "セキュリティエラーが発生しました",
            AppError::Configuration(_) => "設定エラーが発生しました",
        }
    }
    
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            AppError::Database(_) => ErrorSeverity::High,
            AppError::Validation(_) => ErrorSeverity::Low,
            AppError::NotFound(_) => ErrorSeverity::Low,
            AppError::ExternalService(_) => ErrorSeverity::Medium,
            AppError::Security(_) => ErrorSeverity::Critical,
            AppError::Configuration(_) => ErrorSeverity::High,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}
```

### 機能別エラーハンドリング

各機能モジュールは、共通のAppErrorを使用しつつ、機能固有のバリデーションとエラーメッセージを提供する：

```rust
// features/expenses/commands.rs
impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.user_message().to_string()
    }
}

pub async fn create_expense(dto: CreateExpenseDto, state: State<AppState>) -> Result<Expense, String> {
    // バリデーション
    validate_expense_dto(&dto)?;
    
    // リポジトリ操作
    let db = state.db.lock().map_err(|_| AppError::Database("ロック取得失敗".into()))?;
    let expense = expenses::repository::create(&db, dto)?;
    
    Ok(expense)
}

fn validate_expense_dto(dto: &CreateExpenseDto) -> Result<(), AppError> {
    if dto.amount <= 0.0 {
        return Err(AppError::Validation("金額は正の数値である必要があります".to_string()));
    }
    
    if dto.amount > 9999999999.0 {
        return Err(AppError::Validation("金額は10桁以内で入力してください".to_string()));
    }
    
    // 日付バリデーション
    shared::utils::validate_date(&dto.date)?;
    
    Ok(())
}
```

## テスト戦略

### デュアルテストアプローチ

**ユニットテスト**と**プロパティベーステスト**の両方を使用して包括的なテストカバレッジを提供する：

- **ユニットテスト**: 特定の例、エッジケース、エラー条件を検証
- **プロパティベーステスト**: すべての入力にわたって保持すべき普遍的なプロパティを検証

### プロパティベーステスト要件

- **使用ライブラリ**: `proptest` crate（Rust標準のプロパティベーステストライブラリ）
- **実行回数**: 各プロパティテストは最低100回の反復実行を行う
- **テストタグ**: 各プロパティベーステストには対応する設計書のプロパティを明示的に参照するコメントを付ける
- **タグ形式**: `**Feature: backend-feature-refactoring, Property {number}: {property_text}**`

### テスト構造

```rust
// features/expenses/tests.rs
use proptest::prelude::*;

#[cfg(test)]
mod tests {
    use super::*;
    
    // ユニットテスト例
    #[test]
    fn test_create_expense_with_valid_data() {
        // 特定の有効なデータでの経費作成テスト
    }
    
    #[test]
    fn test_create_expense_with_invalid_amount() {
        // 無効な金額でのエラーハンドリングテスト
    }
    
    // プロパティベーステスト例
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        /**
         * Feature: backend-feature-refactoring, Property 3: 後方互換性の保持
         * すべての既存のTauriコマンド、データベース操作、エラーハンドリング、セキュリティ機能について、
         * リファクタリング前後で同じシグネチャと動作を維持している
         */
        #[test]
        fn test_expense_operations_backward_compatibility(
            amount in 0.01f64..9999999999.0,
            category in "[a-zA-Z]{1,50}",
            date in r"\d{4}-\d{2}-\d{2}"
        ) {
            // リファクタリング前後でのデータベース操作の一貫性をテスト
        }
    }
}
```

### 統合テスト

```rust
// tests/integration/mod.rs
/**
 * Feature: backend-feature-refactoring, Property 1: 機能別ディレクトリ構造の正当性
 * すべての機能について、その機能に関連するすべてのコードが
 * 対応するfeatures/{feature_name}/ディレクトリ内に配置されている
 */
#[test]
fn test_feature_directory_structure() {
    // ディレクトリ構造の検証
    assert!(Path::new("src/features/expenses").exists());
    assert!(Path::new("src/features/subscriptions").exists());
    assert!(Path::new("src/features/receipts").exists());
    assert!(Path::new("src/features/security").exists());
    assert!(Path::new("src/features/migrations").exists());
    
    // 各機能モジュールの必要ファイルの存在確認
    assert!(Path::new("src/features/expenses/mod.rs").exists());
    assert!(Path::new("src/features/expenses/models.rs").exists());
    assert!(Path::new("src/features/expenses/commands.rs").exists());
    assert!(Path::new("src/features/expenses/repository.rs").exists());
}

/**
 * Feature: backend-feature-refactoring, Property 2: モジュール境界の正当性
 * すべての機能モジュール間の相互作用について、明確に定義された
 * パブリックインターフェースを通じてのみアクセスが行われている
 */
#[test]
fn test_module_boundary_integrity() {
    // モジュール間の依存関係の検証
    // パブリックインターフェースのみが使用されていることの確認
}
```

## 実装計画の概要

### フェーズ1: 共有モジュールの準備
1. shared/database, shared/config, shared/errors モジュールの作成
2. 既存のコードから共通機能を抽出
3. 統一されたエラーハンドリングの実装

### フェーズ2: 機能モジュールの段階的移行
1. Expenses機能の移行
2. Subscriptions機能の移行
3. Receipts機能の移行
4. Security機能の移行
5. Migrations機能の移行

### フェーズ3: 統合とクリーンアップ
1. 古い技術レイヤーディレクトリの削除
2. lib.rsの更新
3. 包括的なテストの実行
4. ドキュメントの更新

各フェーズでは、既存の機能が正常に動作し続けることを確認しながら段階的に進める。
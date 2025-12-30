# 自動マイグレーションシステム 使用方法

## 概要

自動マイグレーションシステムは、アプリケーション起動時に実行されていないマイグレーションを自動で適用するシステムです。`migrations`テーブルを使用してマイグレーション実行履歴を管理し、既存のマイグレーション機能と統合して、安全で効率的な自動マイグレーション機能を提供します。

## 主な機能

- **自動マイグレーション実行**: アプリケーション起動時に未適用のマイグレーションを自動実行
- **実行履歴管理**: `migrations`テーブルでマイグレーション実行履歴を管理
- **並行実行制御**: 複数のアプリケーションインスタンスが同時に起動した場合の重複実行防止
- **バックアップ機能**: マイグレーション実行前の自動バックアップ作成
- **エラーハンドリング**: 失敗時の適切なロールバックとエラー報告
- **状態確認機能**: 現在のマイグレーション状態を確認するコマンド

## システム構成

### アーキテクチャ

```
アプリケーション起動
        ↓
データベース接続初期化
        ↓
自動マイグレーションシステム初期化
        ↓
migrationsテーブル確認・作成
        ↓
利用可能マイグレーション一覧取得
        ↓
適用済みマイグレーション一覧取得
        ↓
未適用マイグレーション判定
        ↓
バックアップ作成 → マイグレーション実行 → 実行記録保存
        ↓
アプリケーション起動完了
```

### 主要コンポーネント

1. **AutoMigrationService**: メインの自動マイグレーション管理サービス
2. **MigrationRegistry**: 利用可能なマイグレーションの登録と管理
3. **MigrationExecutor**: マイグレーションの実行とトランザクション管理
4. **MigrationTable**: migrationsテーブルの管理

## 使用方法

### 基本的な使用方法

自動マイグレーションシステムは、アプリケーション起動時に自動的に実行されます。特別な操作は必要ありません。

```rust
// src-tauri/src/shared/database/connection.rs
pub fn initialize_database(app_handle: &AppHandle) -> AppResult<Connection> {
    let database_path = get_database_path(app_handle)?;
    let conn = Connection::open(&database_path)?;
    
    // 既存のテーブル作成
    create_tables(&conn)?;
    
    // 自動マイグレーションシステム初期化（自動実行）
    let auto_migration_service = AutoMigrationService::new(&conn)?;
    auto_migration_service.run_startup_migrations(&conn)?;
    
    Ok(conn)
}
```

### マイグレーション状態の確認

Tauriコマンドを使用して、現在のマイグレーション状態を確認できます。

```javascript
// フロントエンド（JavaScript/TypeScript）
import { invoke } from '@tauri-apps/api/tauri';

// マイグレーション状態を取得
const migrationStatus = await invoke('get_migration_status');
console.log('マイグレーション状態:', migrationStatus);
```

```rust
// バックエンド（Rust）
use crate::features::migrations::commands::get_migration_status;

#[tauri::command]
pub fn get_migration_status(
    state: tauri::State<AppState>,
) -> Result<DetailedMigrationInfo, String> {
    let conn = state.get_connection()?;
    get_migration_status(&conn).map_err(|e| e.to_string())
}
```

### 手動でのマイグレーション実行

通常は自動実行されますが、必要に応じて手動でマイグレーションを実行することも可能です。

```rust
use crate::features::migrations::auto_migration::AutoMigrationService;

// 手動でマイグレーションを実行
let service = AutoMigrationService::new(&conn)?;
let result = service.run_startup_migrations(&conn)?;

if result.success {
    println!("マイグレーション成功: {}", result.message);
    println!("適用されたマイグレーション: {:?}", result.applied_migrations);
} else {
    eprintln!("マイグレーション失敗: {}", result.message);
}
```

## 設定とカスタマイズ

### 新しいマイグレーションの追加

新しいマイグレーションを追加するには、以下の手順を実行します：

1. **マイグレーション実行器の作成**

```rust
// src-tauri/src/features/migrations/auto_migration/executor.rs
pub struct NewFeatureMigrationExecutor;

impl MigrationExecutorTrait for NewFeatureMigrationExecutor {
    fn execute(&self, conn: &Connection) -> Result<(), String> {
        // マイグレーション処理を実装
        conn.execute(
            "CREATE TABLE new_feature (id INTEGER PRIMARY KEY, name TEXT)",
            [],
        ).map_err(|e| e.to_string())?;
        
        Ok(())
    }

    fn name(&self) -> &str {
        "004_add_new_feature"
    }
}
```

2. **マイグレーション定義の登録**

```rust
// src-tauri/src/features/migrations/auto_migration/registry.rs
impl MigrationRegistry {
    pub fn register_default_migrations() -> Result<Self, MigrationError> {
        let mut registry = Self::new();
        
        // 既存のマイグレーション...
        
        // 新しいマイグレーションを追加
        registry.register(MigrationDefinition::new(
            "004_add_new_feature".to_string(),
            "3.0.0".to_string(),
            "新機能の追加".to_string(),
            calculate_checksum("new_feature_migration_content"),
        ))?;
        
        registry.register_executable(ExecutableMigrationDefinition::new(
            registry.find_migration("004_add_new_feature").unwrap().clone(),
            Box::new(NewFeatureMigrationExecutor),
        ));
        
        Ok(registry)
    }
}
```

### バックアップ設定

バックアップは自動的に作成されますが、設定をカスタマイズできます：

```rust
// バックアップパスのカスタマイズ
impl MigrationExecutor {
    pub fn create_backup_with_custom_path(
        &self, 
        conn: &Connection, 
        backup_dir: &str
    ) -> Result<String, MigrationError> {
        let now_jst = Utc::now().with_timezone(&Tokyo);
        let backup_path = format!("{}/database_backup_{}.db", backup_dir, now_jst.timestamp());
        
        create_backup(conn, &backup_path)?;
        Ok(backup_path)
    }
}
```

## トラブルシューティング

### よくある問題と解決方法

#### 1. マイグレーション実行エラー

**症状**: マイグレーション実行時にエラーが発生する

**原因と解決方法**:
- **権限不足**: データベースファイルの書き込み権限を確認
- **ディスク容量不足**: 十分な空き容量があることを確認
- **データベース破損**: バックアップからの復旧を検討

```bash
# ログファイルでエラー詳細を確認
tail -f src-tauri/logs/app.log
```

#### 2. 並行実行制御エラー

**症状**: "別のインスタンスがマイグレーション実行中です" エラー

**原因**: 複数のアプリケーションインスタンスが同時に起動した

**解決方法**:
```sql
-- migration_lockテーブルの状態を確認
SELECT * FROM migration_lock;

-- 必要に応じて実行中フラグをリセット
UPDATE migration_lock SET in_progress = 0, started_at = NULL WHERE id = 1;
```

#### 3. チェックサム不一致エラー

**症状**: "チェックサム不一致" エラー

**原因**: マイグレーション内容が変更された

**解決方法**:
1. マイグレーション内容の変更を確認
2. 必要に応じて新しいマイグレーションとして追加
3. 既存の記録を削除（注意が必要）

```sql
-- 問題のあるマイグレーション記録を確認
SELECT * FROM migrations WHERE name = 'migration_name';

-- 必要に応じて記録を削除（慎重に実行）
DELETE FROM migrations WHERE name = 'migration_name';
```

#### 4. バックアップ作成失敗

**症状**: バックアップ作成に失敗する

**原因と解決方法**:
- **ディスク容量不足**: 空き容量を確保
- **権限不足**: バックアップディレクトリの書き込み権限を確認
- **パス不正**: バックアップパスが有効であることを確認

### ログの確認方法

```bash
# アプリケーションログを確認
tail -f src-tauri/logs/app.log

# マイグレーション関連のログのみを表示
grep "マイグレーション\|migration" src-tauri/logs/app.log
```

### デバッグモードでの実行

```bash
# デバッグモードでアプリケーションを起動
RUST_LOG=debug cargo tauri dev
```

## パフォーマンス最適化

### インデックスの活用

migrationsテーブルには以下のインデックスが自動的に作成されます：

```sql
CREATE INDEX idx_migrations_name ON migrations(name);
CREATE INDEX idx_migrations_applied_at ON migrations(applied_at);
CREATE INDEX idx_migrations_version ON migrations(version);
```

### 大量マイグレーション履歴の管理

マイグレーション履歴が大量になった場合の対処法：

```sql
-- 古いマイグレーション記録のアーカイブ
CREATE TABLE migrations_archive AS 
SELECT * FROM migrations 
WHERE applied_at < date('now', '-1 year');

-- アーカイブ後の削除（慎重に実行）
DELETE FROM migrations 
WHERE applied_at < date('now', '-1 year');
```

## セキュリティ考慮事項

### データベースアクセス制御

- マイグレーション実行には適切なデータベース権限が必要
- 本番環境では最小権限の原則を適用
- バックアップファイルのアクセス権限を適切に設定

### 監査ログ

全てのマイグレーション実行は自動的に記録されます：

```sql
-- マイグレーション実行履歴の確認
SELECT 
    name,
    version,
    applied_at,
    execution_time_ms
FROM migrations 
ORDER BY applied_at DESC;
```

## 運用ガイドライン

### 本番環境での注意事項

1. **事前テスト**: 本番環境と同じ構成でのテスト実行
2. **バックアップ確認**: 自動バックアップが正常に作成されることを確認
3. **ロールバック計画**: 失敗時の復旧手順を事前に準備
4. **監視設定**: マイグレーション実行状況の監視

### 開発環境での推奨事項

1. **頻繁なテスト**: マイグレーション追加時の動作確認
2. **ログレベル設定**: デバッグログの有効化
3. **テストデータ**: 様々なデータ状態でのテスト実行

## API リファレンス

### AutoMigrationService

```rust
impl AutoMigrationService {
    /// 自動マイグレーションシステムを初期化
    pub fn new(conn: &Connection) -> Result<Self, MigrationError>;
    
    /// アプリケーション起動時の自動マイグレーション実行
    pub fn run_startup_migrations(&self, conn: &Connection) -> Result<AutoMigrationResult, MigrationError>;
    
    /// マイグレーション状態の確認
    pub fn check_migration_status(&self, conn: &Connection) -> Result<MigrationStatusReport, MigrationError>;
}
```

### MigrationTable

```rust
impl MigrationTable {
    /// migrationsテーブルを初期化
    pub fn initialize(conn: &Connection) -> Result<(), MigrationError>;
    
    /// 適用済みマイグレーション一覧を取得
    pub fn get_applied_migrations(conn: &Connection) -> Result<Vec<AppliedMigration>, MigrationError>;
    
    /// マイグレーション実行記録を保存
    pub fn record_migration(/* ... */) -> Result<(), MigrationError>;
    
    /// マイグレーションが適用済みかチェック
    pub fn is_migration_applied(conn: &Connection, name: &str) -> Result<bool, MigrationError>;
}
```

### Tauriコマンド

```rust
#[tauri::command]
pub fn get_migration_status(state: tauri::State<AppState>) -> Result<DetailedMigrationInfo, String>;
```

## 更新履歴

- **v1.0.0**: 初期リリース
  - 基本的な自動マイグレーション機能
  - 既存マイグレーション機能との統合
  - 並行実行制御機能

## サポート

問題が発生した場合は、以下の情報を含めてサポートに連絡してください：

1. エラーメッセージの詳細
2. アプリケーションログ
3. データベースの状態（migrations テーブルの内容）
4. 実行環境の情報

```sql
-- サポート用の情報収集クエリ
SELECT 
    'Migration Status' as info_type,
    name,
    version,
    applied_at,
    execution_time_ms
FROM migrations 
ORDER BY applied_at DESC
LIMIT 10;
```
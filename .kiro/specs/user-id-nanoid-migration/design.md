# 設計書

## 概要

本設計書は、ユーザーIDを連番（INTEGER AUTOINCREMENT）からnanoId（TEXT）に変更するための詳細な設計を記述します。この変更により、ユーザーIDの予測可能性を排除し、セキュリティとプライバシーを向上させます。

nanoIdは、URLセーフで短く、暗号学的に安全な一意識別子生成ライブラリです。21文字のデフォルト長で、UUIDと同等の衝突確率を持ちながら、より短く読みやすいIDを生成します。

## アーキテクチャ

### 変更対象コンポーネント

1. **データベーススキーマ**
   - usersテーブル
   - 外部キー参照を持つすべてのテーブル（expenses, subscriptions, sessions, receipt_cache, migration_logs, security_audit_logs）

2. **Rustデータモデル**
   - `User`構造体
   - `Session`構造体
   - すべてのDTO（Data Transfer Object）

3. **リポジトリ層**
   - `UserRepository`
   - `ExpenseRepository`
   - `SubscriptionRepository`
   - `SessionManager`

4. **マイグレーション層**
   - 新しいマイグレーション関数の追加
   - データ移行ロジック

### 依存関係

```
nanoid (0.4.0) - nanoId生成ライブラリ
  ↓
User Repository - ユーザー作成時にnanoIdを生成
  ↓
Migration Service - 既存データの移行
  ↓
All Repositories - 新しいString型のuser_idを使用
```

## コンポーネントとインターフェース

### 1. NanoId生成ユーティリティ

新しいユーティリティモジュールを作成し、nanoId生成機能を提供します。

```rust
// packages/desktop/src-tauri/src/shared/utils/nanoid.rs

use nanoid::nanoid;

/// ユーザーID用のnanoIdを生成する
///
/// # 戻り値
/// 21文字のURL-safeなnanoId
///
/// # 特性
/// - 文字セット: A-Za-z0-9_- (64文字)
/// - 長さ: 21文字
/// - 衝突確率: 1兆個のIDで1%未満
pub fn generate_user_id() -> String {
    nanoid!()
}

/// カスタム長のnanoIdを生成する（テスト用）
///
/// # 引数
/// * `length` - 生成するIDの長さ
///
/// # 戻り値
/// 指定された長さのnanoId
pub fn generate_user_id_with_length(length: usize) -> String {
    nanoid!(length)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_user_id_length() {
        let id = generate_user_id();
        assert_eq!(id.len(), 21);
    }

    #[test]
    fn test_generate_user_id_uniqueness() {
        let id1 = generate_user_id();
        let id2 = generate_user_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_user_id_url_safe() {
        let id = generate_user_id();
        // URL-safeな文字のみを含むことを確認
        assert!(id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
    }

    #[test]
    fn test_generate_user_id_with_custom_length() {
        let id = generate_user_id_with_length(10);
        assert_eq!(id.len(), 10);
    }
}
```

### 2. データモデルの更新

#### User構造体

```rust
// packages/desktop/src-tauri/src/features/auth/models.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// ユーザーID（nanoId形式）
    pub id: String,  // 変更: i64 → String
    /// GoogleユーザーID
    pub google_id: String,
    /// メールアドレス
    pub email: String,
    /// 表示名
    pub name: String,
    /// プロフィール画像URL
    pub picture_url: Option<String>,
    /// 作成日時
    pub created_at: DateTime<Utc>,
    /// 更新日時
    pub updated_at: DateTime<Utc>,
}
```

#### Session構造体

```rust
// packages/desktop/src-tauri/src/features/auth/models.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// セッションID
    pub id: String,
    /// ユーザーID（nanoId形式）
    pub user_id: String,  // 変更: i64 → String
    /// 有効期限
    pub expires_at: DateTime<Utc>,
    /// 作成日時
    pub created_at: DateTime<Utc>,
}
```

### 3. リポジトリの更新

#### UserRepository

```rust
// packages/desktop/src-tauri/src/features/auth/repository.rs

impl UserRepository {
    /// ユーザーIDでユーザーを取得する
    ///
    /// # 引数
    /// * `user_id` - ユーザーID（nanoId形式）
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<User>, AuthError> {
        // 実装
    }

    /// 新規ユーザーを作成する
    fn create_new_user(
        &self,
        conn: &Connection,
        google_user: &GoogleUser,
    ) -> Result<User, AuthError> {
        // nanoIdを生成
        let user_id = crate::shared::utils::nanoid::generate_user_id();
        
        let now_jst = Utc::now().with_timezone(&Tokyo);
        let timestamp = now_jst.to_rfc3339();

        conn.execute(
            "INSERT INTO users (id, google_id, email, name, picture_url, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                user_id,
                google_user.id,
                google_user.email,
                google_user.name,
                google_user.picture,
                timestamp,
                timestamp
            ],
        )?;

        // 作成されたユーザー情報を取得して返す
        self.get_user_by_id_internal(conn, &user_id)?
            .ok_or_else(|| AuthError::DatabaseError("作成されたユーザーの取得に失敗".to_string()))
    }

    /// ユーザーを削除する
    pub async fn delete_user(&self, user_id: &str) -> Result<(), AuthError> {
        // 実装
    }
}
```

### 4. マイグレーション設計

#### マイグレーション戦略

1. **新しいテーブル構造の作成**
2. **既存データの移行**
3. **外部キー制約の再構築**
4. **旧テーブルの削除**

#### マイグレーション関数

```rust
// packages/desktop/src-tauri/src/features/migrations/service.rs

/// ユーザーIDをnanoIdに移行する
///
/// # 処理フロー
/// 1. 一時的なマッピングテーブルを作成
/// 2. 新しいスキーマでテーブルを再作成
/// 3. 既存データを新しいIDで移行
/// 4. 外部キー参照を更新
/// 5. マッピングテーブルを削除
fn execute_user_id_nanoid_migration(tx: &Transaction) -> Result<(), rusqlite::Error> {
    // 1. IDマッピングテーブルを作成
    tx.execute(
        "CREATE TEMPORARY TABLE user_id_mapping (
            old_id INTEGER PRIMARY KEY,
            new_id TEXT NOT NULL UNIQUE
        )",
        [],
    )?;

    // 2. 既存ユーザーに新しいIDを割り当て
    let mut stmt = tx.prepare("SELECT id FROM users")?;
    let old_ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    for old_id in old_ids {
        let new_id = nanoid::nanoid!();
        tx.execute(
            "INSERT INTO user_id_mapping (old_id, new_id) VALUES (?1, ?2)",
            params![old_id, new_id],
        )?;
    }

    // 3. 新しいusersテーブルを作成
    tx.execute(
        "CREATE TABLE users_new (
            id TEXT PRIMARY KEY,
            google_id TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL,
            name TEXT NOT NULL,
            picture_url TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // 4. データを移行
    tx.execute(
        "INSERT INTO users_new (id, google_id, email, name, picture_url, created_at, updated_at)
         SELECT m.new_id, u.google_id, u.email, u.name, u.picture_url, u.created_at, u.updated_at
         FROM users u
         INNER JOIN user_id_mapping m ON u.id = m.old_id",
        [],
    )?;

    // 5. 外部キー参照を持つテーブルを更新
    migrate_expenses_table(tx)?;
    migrate_subscriptions_table(tx)?;
    migrate_sessions_table(tx)?;
    migrate_receipt_cache_table(tx)?;
    migrate_migration_logs_table(tx)?;
    migrate_security_audit_logs_table(tx)?;

    // 6. 旧テーブルを削除して新テーブルをリネーム
    tx.execute("DROP TABLE users", [])?;
    tx.execute("ALTER TABLE users_new RENAME TO users", [])?;

    // 7. インデックスを再作成
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_users_google_id ON users(google_id)",
        [],
    )?;

    Ok(())
}

/// expensesテーブルのuser_idを移行
fn migrate_expenses_table(tx: &Transaction) -> Result<(), rusqlite::Error> {
    // 新しいテーブルを作成
    tx.execute(
        "CREATE TABLE expenses_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            amount REAL NOT NULL,
            category TEXT NOT NULL,
            description TEXT,
            receipt_url TEXT CHECK(receipt_url IS NULL OR receipt_url LIKE 'https://%'),
            user_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // データを移行
    tx.execute(
        "INSERT INTO expenses_new (id, date, amount, category, description, receipt_url, user_id, created_at, updated_at)
         SELECT e.id, e.date, e.amount, e.category, e.description, e.receipt_url, m.new_id, e.created_at, e.updated_at
         FROM expenses e
         INNER JOIN user_id_mapping m ON e.user_id = m.old_id",
        [],
    )?;

    // 旧テーブルを削除して新テーブルをリネーム
    tx.execute("DROP TABLE expenses", [])?;
    tx.execute("ALTER TABLE expenses_new RENAME TO expenses", [])?;

    // インデックスを再作成
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_user_id ON expenses(user_id)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date)",
        [],
    )?;

    Ok(())
}

/// subscriptionsテーブルのuser_idを移行
fn migrate_subscriptions_table(tx: &Transaction) -> Result<(), rusqlite::Error> {
    // 同様の処理
    Ok(())
}

/// sessionsテーブルのuser_idを移行
fn migrate_sessions_table(tx: &Transaction) -> Result<(), rusqlite::Error> {
    // 同様の処理
    Ok(())
}

/// receipt_cacheテーブルのuser_idを移行
fn migrate_receipt_cache_table(tx: &Transaction) -> Result<(), rusqlite::Error> {
    // 同様の処理
    Ok(())
}

/// migration_logsテーブルのuser_idを移行
fn migrate_migration_logs_table(tx: &Transaction) -> Result<(), rusqlite::Error> {
    // 同様の処理
    Ok(())
}

/// security_audit_logsテーブルのuser_idを移行
fn migrate_security_audit_logs_table(tx: &Transaction) -> Result<(), rusqlite::Error> {
    // 同様の処理
    Ok(())
}
```

## データモデル

### データベーススキーマ

#### usersテーブル（変更後）

```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,                    -- 変更: INTEGER → TEXT
    google_id TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL,
    name TEXT NOT NULL,
    picture_url TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_users_google_id ON users(google_id);
```

#### expensesテーブル（変更後）

```sql
CREATE TABLE expenses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    amount REAL NOT NULL,
    category TEXT NOT NULL,
    description TEXT,
    receipt_url TEXT CHECK(receipt_url IS NULL OR receipt_url LIKE 'https://%'),
    user_id TEXT NOT NULL,                  -- 変更: INTEGER → TEXT
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_expenses_user_id ON expenses(user_id);
CREATE INDEX idx_expenses_date ON expenses(date);
```

#### subscriptionsテーブル（変更後）

```sql
CREATE TABLE subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    amount REAL NOT NULL,
    billing_cycle TEXT NOT NULL CHECK(billing_cycle IN ('monthly', 'yearly')),
    start_date TEXT NOT NULL,
    category TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    receipt_path TEXT,
    user_id TEXT NOT NULL,                  -- 変更: INTEGER → TEXT
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_subscriptions_user_id ON subscriptions(user_id);
```

#### sessionsテーブル（変更後）

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,                  -- 変更: INTEGER → TEXT
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
```

#### receipt_cacheテーブル（変更後）

```sql
CREATE TABLE receipt_cache (
    receipt_url TEXT PRIMARY KEY,
    local_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    last_accessed TEXT NOT NULL,
    user_id TEXT NOT NULL,                  -- 変更: INTEGER → TEXT
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_receipt_cache_user_id ON receipt_cache(user_id);
```

### 型変更の影響範囲

| ファイル | 変更内容 |
|---------|---------|
| `auth/models.rs` | `User.id: i64 → String`, `Session.user_id: i64 → String` |
| `auth/repository.rs` | すべてのメソッドの`user_id`パラメータを`&str`に変更 |
| `auth/session.rs` | `create_session(&self, user_id: &str)` |
| `auth/commands.rs` | コマンドハンドラの型を更新 |
| `expenses/models.rs` | `CreateExpenseDto.user_id: Option<i64> → Option<String>` |
| `expenses/repository.rs` | すべてのメソッドの`user_id`パラメータを`&str`に変更 |
| `expenses/commands.rs` | コマンドハンドラの型を更新 |
| `subscriptions/models.rs` | DTOの`user_id`型を更新 |
| `subscriptions/repository.rs` | すべてのメソッドの`user_id`パラメータを`&str`に変更 |
| `subscriptions/commands.rs` | コマンドハンドラの型を更新 |
| `receipts/user_path_manager.rs` | `generate_user_receipt_path(user_id: &str, ...)` |
| `migrations/r2_user_directory_migration.rs` | マイグレーション関連の型を更新 |

## 正確性プロパティ

プロパティとは、システムのすべての有効な実行において真であるべき特性や振る舞いのことです。プロパティは、人間が読める仕様と機械が検証可能な正確性保証の橋渡しをします。


### プロパティ反映

受入基準を分析した結果、以下のプロパティを特定しました：

- **プロパティ1-3**: nanoId生成の特性（長さ、文字セット、一意性）
- **プロパティ4**: スキーマ変更の検証（複数テーブルを統合）
- **プロパティ5**: 外部キー制約の動作
- **プロパティ6-7**: マイグレーション処理の正確性
- **プロパティ8**: エラー時のロールバック（3.4と8.2を統合）
- **プロパティ9**: セッションの移行

### 正確性プロパティ

#### プロパティ1: NanoId長さの一貫性

*すべての*新規ユーザー作成において、生成されるユーザーIDは正確に21文字である必要があります。

**検証方法**: Requirements 1.2

#### プロパティ2: NanoId文字セットの安全性

*すべての*生成されたユーザーIDは、URL-safe文字セット（A-Za-z0-9_-）のみを含む必要があります。

**検証方法**: Requirements 1.3

#### プロパティ3: NanoId一意性

*任意の*2つの異なるユーザー作成において、生成されるユーザーIDは異なる必要があります（衝突確率が極めて低い）。

**検証方法**: Requirements 1.4

#### プロパティ4: 外部キー制約の機能性

*任意の*ユーザーを削除した場合、そのユーザーに関連するすべてのデータ（expenses, subscriptions, sessions, receipt_cache）がカスケード削除される必要があります。

**検証方法**: Requirements 2.7

#### プロパティ5: マイグレーション後のユーザーID形式

*すべての*既存ユーザーに対して、マイグレーション実行後、ユーザーIDは21文字のnanoId形式（TEXT型）である必要があります。

**検証方法**: Requirements 3.1

#### プロパティ6: マイグレーション後のデータ整合性

*すべての*関連テーブル（expenses, subscriptions, sessions, receipt_cache）において、マイグレーション実行後、user_id参照が新しいnanoId形式で正しく更新されている必要があります。

**検証方法**: Requirements 3.3

#### プロパティ7: マイグレーションエラー時のロールバック

*任意の*マイグレーション実行において、エラーが発生した場合、すべてのデータベース変更がロールバックされ、元の状態に戻る必要があります。

**検証方法**: Requirements 3.4, 8.2

#### プロパティ8: セッション移行の正確性

*すべての*アクティブセッションに対して、マイグレーション実行後、セッションのuser_idが新しいnanoId形式で正しく更新されている必要があります。

**検証方法**: Requirements 6.1

## エラーハンドリング

### エラーの種類

1. **NanoId生成エラー**
   - 発生条件: 極めて稀（ランダム生成の失敗）
   - 対応: エラーログを記録し、ユーザー作成を中止
   - エラーメッセージ: "ユーザーIDの生成に失敗しました。もう一度お試しください。"

2. **マイグレーションエラー**
   - 発生条件: データベースロックエラー、ディスク容量不足、外部キー制約違反
   - 対応: トランザクションをロールバックし、詳細なエラーログを記録
   - エラーメッセージ: "データベースの移行に失敗しました。システム管理者に連絡してください。"

3. **型変換エラー**
   - 発生条件: 旧ID形式（整数）でのアクセス試行
   - 対応: エラーを返し、新しい認証を要求
   - エラーメッセージ: "セッションが無効です。再度ログインしてください。"

4. **外部キー制約違反**
   - 発生条件: 存在しないuser_idの参照
   - 対応: エラーログを記録し、操作を中止
   - エラーメッセージ: "ユーザー情報が見つかりません。"

### エラーハンドリング戦略

```rust
// NanoId生成エラー
pub fn generate_user_id() -> Result<String, String> {
    // nanoid!()は通常失敗しないが、念のためResult型で包む
    Ok(nanoid!())
}

// マイグレーションエラー
pub fn execute_migration(conn: &Connection) -> Result<(), MigrationError> {
    let tx = conn.transaction()?;
    
    match execute_user_id_nanoid_migration(&tx) {
        Ok(_) => {
            tx.commit()?;
            log::info!("ユーザーIDマイグレーションが正常に完了しました");
            Ok(())
        }
        Err(e) => {
            log::error!("マイグレーションエラー: {}", e);
            // トランザクションは自動的にロールバックされる
            Err(MigrationError::ExecutionFailed(e.to_string()))
        }
    }
}

// 型変換エラー
pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<User>, AuthError> {
    // user_idがnanoId形式（21文字、URL-safe）であることを検証
    if !is_valid_nanoid(user_id) {
        return Err(AuthError::InvalidUserId(
            "無効なユーザーID形式です".to_string()
        ));
    }
    
    // データベースクエリを実行
    // ...
}

fn is_valid_nanoid(id: &str) -> bool {
    id.len() == 21 && id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}
```

## テスト戦略

### デュアルテストアプローチ

本プロジェクトでは、ユニットテストとプロパティベーステストの両方を使用します：

- **ユニットテスト**: 特定の例、エッジケース、エラー条件を検証
- **プロパティベーステスト**: すべての入力にわたって普遍的なプロパティを検証

両方のアプローチは補完的であり、包括的なカバレッジに必要です。

### ユニットテストのバランス

- ユニットテストは特定の例とエッジケースに焦点を当てる
- プロパティベーステストが多数の入力をカバーするため、過度なユニットテストは避ける
- ユニットテストは以下に焦点を当てる：
  - 正しい動作を示す特定の例
  - コンポーネント間の統合ポイント
  - エッジケースとエラー条件

### プロパティベーステスト設定

- プロパティテストごとに最低100回の反復実行
- 各プロパティテストは設計書のプロパティを参照
- タグ形式: **Feature: user-id-nanoid-migration, Property {番号}: {プロパティテキスト}**

### テストケース

#### 1. NanoId生成のテスト

**ユニットテスト:**
```rust
#[test]
fn test_generate_user_id_returns_21_chars() {
    let id = generate_user_id();
    assert_eq!(id.len(), 21);
}

#[test]
fn test_generate_user_id_url_safe() {
    let id = generate_user_id();
    assert!(id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
}
```

**プロパティベーステスト:**
```rust
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    // Feature: user-id-nanoid-migration, Property 1: NanoId長さの一貫性
    #[test]
    fn prop_nanoid_always_21_chars(_seed in any::<u64>()) {
        let id = generate_user_id();
        prop_assert_eq!(id.len(), 21);
    }
    
    // Feature: user-id-nanoid-migration, Property 2: NanoId文字セットの安全性
    #[test]
    fn prop_nanoid_url_safe(_seed in any::<u64>()) {
        let id = generate_user_id();
        prop_assert!(id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
    }
    
    // Feature: user-id-nanoid-migration, Property 3: NanoId一意性
    #[test]
    fn prop_nanoid_uniqueness(_seed in any::<u64>()) {
        let id1 = generate_user_id();
        let id2 = generate_user_id();
        prop_assert_ne!(id1, id2);
    }
}
```

#### 2. ユーザー作成のテスト

**ユニットテスト:**
```rust
#[tokio::test]
async fn test_create_user_with_nanoid() {
    let repository = create_test_repository();
    let google_user = create_test_google_user();
    
    let user = repository.find_or_create_user(google_user).await.unwrap();
    
    // IDがnanoId形式であることを確認
    assert_eq!(user.id.len(), 21);
    assert!(user.id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
}
```

**プロパティベーステスト:**
```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    // Feature: user-id-nanoid-migration, Property 1: NanoId長さの一貫性
    #[test]
    fn prop_created_user_has_valid_nanoid(
        email in "[a-z]{5,10}@example\\.com",
        name in "[A-Za-z ]{5,20}"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let repository = create_test_repository();
            let google_user = GoogleUser {
                id: format!("google_{}", nanoid!()),
                email,
                name,
                picture: None,
                verified_email: true,
            };
            
            let user = repository.find_or_create_user(google_user).await.unwrap();
            prop_assert_eq!(user.id.len(), 21);
        });
    }
}
```

#### 3. マイグレーションのテスト

**ユニットテスト:**
```rust
#[test]
fn test_migration_creates_mapping_table() {
    let conn = create_test_connection();
    let tx = conn.transaction().unwrap();
    
    // 既存ユーザーを作成（旧スキーマ）
    create_legacy_users(&tx);
    
    // マイグレーションを実行
    execute_user_id_nanoid_migration(&tx).unwrap();
    
    // マッピングテーブルが作成されていることを確認
    let count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM user_id_mapping",
        [],
        |row| row.get(0)
    ).unwrap();
    
    assert!(count > 0);
}

#[test]
fn test_migration_rollback_on_error() {
    let conn = create_test_connection();
    
    // 既存ユーザーを作成
    create_legacy_users(&conn);
    
    // エラーを発生させるマイグレーションを実行
    let result = execute_faulty_migration(&conn);
    
    assert!(result.is_err());
    
    // 元のデータが保持されていることを確認
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM users",
        [],
        |row| row.get(0)
    ).unwrap();
    
    assert!(count > 0);
}
```

**プロパティベーステスト:**
```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    // Feature: user-id-nanoid-migration, Property 5: マイグレーション後のユーザーID形式
    #[test]
    fn prop_migration_converts_all_user_ids(user_count in 1..100usize) {
        let conn = create_test_connection();
        
        // 指定された数の旧ユーザーを作成
        for i in 0..user_count {
            create_legacy_user(&conn, i as i64);
        }
        
        // マイグレーションを実行
        execute_user_id_nanoid_migration(&conn).unwrap();
        
        // すべてのユーザーIDがnanoId形式であることを確認
        let mut stmt = conn.prepare("SELECT id FROM users").unwrap();
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        
        for id in ids {
            prop_assert_eq!(id.len(), 21);
            prop_assert!(id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
        }
    }
    
    // Feature: user-id-nanoid-migration, Property 6: マイグレーション後のデータ整合性
    #[test]
    fn prop_migration_preserves_data_integrity(
        user_count in 1..50usize,
        expenses_per_user in 1..10usize
    ) {
        let conn = create_test_connection();
        
        // ユーザーと経費を作成
        for i in 0..user_count {
            let user_id = create_legacy_user(&conn, i as i64);
            for _ in 0..expenses_per_user {
                create_legacy_expense(&conn, user_id);
            }
        }
        
        // マイグレーション前のデータ数を記録
        let expense_count_before: i64 = conn.query_row(
            "SELECT COUNT(*) FROM expenses",
            [],
            |row| row.get(0)
        ).unwrap();
        
        // マイグレーションを実行
        execute_user_id_nanoid_migration(&conn).unwrap();
        
        // マイグレーション後のデータ数を確認
        let expense_count_after: i64 = conn.query_row(
            "SELECT COUNT(*) FROM expenses",
            [],
            |row| row.get(0)
        ).unwrap();
        
        prop_assert_eq!(expense_count_before, expense_count_after);
        
        // すべての経費が有効なuser_idを持つことを確認
        let mut stmt = conn.prepare(
            "SELECT e.user_id FROM expenses e 
             LEFT JOIN users u ON e.user_id = u.id 
             WHERE u.id IS NULL"
        ).unwrap();
        let orphaned_count = stmt.query_map([], |_| Ok(())).unwrap().count();
        
        prop_assert_eq!(orphaned_count, 0);
    }
    
    // Feature: user-id-nanoid-migration, Property 7: マイグレーションエラー時のロールバック
    #[test]
    fn prop_migration_rollback_on_error(user_count in 1..50usize) {
        let conn = create_test_connection();
        
        // ユーザーを作成
        for i in 0..user_count {
            create_legacy_user(&conn, i as i64);
        }
        
        // マイグレーション前のユーザー数を記録
        let user_count_before: i64 = conn.query_row(
            "SELECT COUNT(*) FROM users",
            [],
            |row| row.get(0)
        ).unwrap();
        
        // エラーを発生させるマイグレーションを実行
        let result = execute_faulty_migration(&conn);
        
        prop_assert!(result.is_err());
        
        // ユーザー数が変わっていないことを確認
        let user_count_after: i64 = conn.query_row(
            "SELECT COUNT(*) FROM users",
            [],
            |row| row.get(0)
        ).unwrap();
        
        prop_assert_eq!(user_count_before, user_count_after);
    }
}
```

#### 4. 外部キー制約のテスト

**プロパティベーステスト:**
```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    // Feature: user-id-nanoid-migration, Property 4: 外部キー制約の機能性
    #[test]
    fn prop_cascade_delete_works(
        expense_count in 1..20usize,
        subscription_count in 1..10usize
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let repository = create_test_repository();
            let google_user = create_test_google_user();
            
            // ユーザーを作成
            let user = repository.find_or_create_user(google_user).await.unwrap();
            
            // 関連データを作成
            for _ in 0..expense_count {
                create_test_expense(&repository, &user.id).await;
            }
            for _ in 0..subscription_count {
                create_test_subscription(&repository, &user.id).await;
            }
            
            // ユーザーを削除
            repository.delete_user(&user.id).await.unwrap();
            
            // 関連データが削除されていることを確認
            let expense_count: i64 = repository.count_expenses_for_user(&user.id).await.unwrap();
            let subscription_count: i64 = repository.count_subscriptions_for_user(&user.id).await.unwrap();
            
            prop_assert_eq!(expense_count, 0);
            prop_assert_eq!(subscription_count, 0);
        });
    }
}
```

### テストライブラリ

- **プロパティベーステスト**: `proptest` クレート（Rustの標準的なプロパティベーステストライブラリ）
- **ユニットテスト**: Rustの標準テストフレームワーク
- **非同期テスト**: `tokio::test` マクロ

### テスト実行

```bash
# すべてのテストを実行
cargo test

# プロパティベーステストのみを実行
cargo test prop_

# 特定のモジュールのテストを実行
cargo test --package desktop --lib features::auth::repository

# テストカバレッジを確認
cargo tarpaulin --out Html
```

# 設計書

## 概要

この設計書では、TauriデスクトップアプリケーションのローカルSQLiteデータベースからCloudflare D1（クラウドベースのSQLite互換データベース）への移行を定義します。

### 移行の目的

1. **マルチデバイス対応**: 複数のデバイスから同じデータにアクセス可能にする
2. **データの永続化**: クラウドにデータを保存することで、デバイスの故障や紛失時にもデータを保護
3. **スケーラビリティ**: Cloudflare Workersのグローバルネットワークを活用した高速アクセス
4. **セキュリティ**: JWT認証とアクセス制御による安全なデータアクセス

### アーキテクチャの変更

**現在のアーキテクチャ:**
```
Tauri App (Rust) → ローカルSQLite
```

**新しいアーキテクチャ:**
```
Tauri App (Rust) → API Server (Cloudflare Workers) → D1 Database
```

## アーキテクチャ

### システム構成

1. **Tauri Desktop App (クライアント)**
   - Rust + Svelte
   - ローカルSQLiteは認証情報のみ保持（セッショントークン、キャッシュ）
   - すべてのデータ操作はAPI Server経由で実行

2. **API Server (Cloudflare Workers)**
   - TypeScript + Hono
   - JWT認証
   - D1データベースへのアクセス制御
   - RESTful APIエンドポイント

3. **D1 Database (データストア)**
   - Cloudflare D1（SQLite互換）
   - ユーザーデータ、経費、サブスクリプションを保存
   - 自動バックアップとレプリケーション

### データフロー

```
[Tauri App] --HTTP/HTTPS--> [API Server] --D1 Binding--> [D1 Database]
     ↓                            ↓
[Local Cache]              [JWT Validation]
```

## コンポーネントとインターフェース

### 1. D1データベーススキーマ

#### usersテーブル

```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,              -- nanoId形式（21文字）
    google_id TEXT NOT NULL UNIQUE,   -- Google OAuth ID
    email TEXT NOT NULL,              -- メールアドレス
    name TEXT NOT NULL,               -- ユーザー名
    picture_url TEXT,                 -- プロフィール画像URL
    created_at TEXT NOT NULL,         -- RFC3339形式（JST）
    updated_at TEXT NOT NULL          -- RFC3339形式（JST）
);

CREATE INDEX idx_users_google_id ON users(google_id);
CREATE INDEX idx_users_email ON users(email);
```

#### expensesテーブル

```sql
CREATE TABLE expenses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,            -- ユーザーID（nanoId形式）
    date TEXT NOT NULL,               -- YYYY-MM-DD形式
    amount REAL NOT NULL,             -- 金額
    category TEXT NOT NULL,           -- カテゴリ
    description TEXT,                 -- 説明（オプション）
    receipt_url TEXT,                 -- 領収書URL（HTTPS）
    created_at TEXT NOT NULL,         -- RFC3339形式（JST）
    updated_at TEXT NOT NULL,         -- RFC3339形式（JST）
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CHECK (receipt_url IS NULL OR receipt_url LIKE 'https://%')
);

CREATE INDEX idx_expenses_user_id ON expenses(user_id);
CREATE INDEX idx_expenses_date ON expenses(date);
CREATE INDEX idx_expenses_category ON expenses(category);
```

#### subscriptionsテーブル

```sql
CREATE TABLE subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,            -- ユーザーID（nanoId形式）
    name TEXT NOT NULL,               -- サービス名
    amount REAL NOT NULL,             -- 金額
    billing_cycle TEXT NOT NULL,      -- "monthly" または "annual"
    start_date TEXT NOT NULL,         -- YYYY-MM-DD形式
    category TEXT NOT NULL,           -- カテゴリ
    is_active INTEGER NOT NULL DEFAULT 1, -- 0=無効, 1=有効
    receipt_path TEXT,                -- 領収書パス（将来的にreceipt_urlに移行）
    created_at TEXT NOT NULL,         -- RFC3339形式（JST）
    updated_at TEXT NOT NULL,         -- RFC3339形式（JST）
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CHECK (billing_cycle IN ('monthly', 'annual'))
);

CREATE INDEX idx_subscriptions_user_id ON subscriptions(user_id);
CREATE INDEX idx_subscriptions_is_active ON subscriptions(is_active);
```

### 2. TypeScript型定義

#### User型

```typescript
interface User {
    id: string;              // nanoId形式
    google_id: string;       // Google OAuth ID
    email: string;           // メールアドレス
    name: string;            // ユーザー名
    picture_url: string | null; // プロフィール画像URL
    created_at: string;      // RFC3339形式
    updated_at: string;      // RFC3339形式
}
```

#### Expense型

```typescript
interface Expense {
    id: number;              // 自動採番ID
    user_id: string;         // ユーザーID
    date: string;            // YYYY-MM-DD形式
    amount: number;          // 金額
    category: string;        // カテゴリ
    description: string | null; // 説明
    receipt_url: string | null; // 領収書URL
    created_at: string;      // RFC3339形式
    updated_at: string;      // RFC3339形式
}
```

#### Subscription型

```typescript
interface Subscription {
    id: number;              // 自動採番ID
    user_id: string;         // ユーザーID
    name: string;            // サービス名
    amount: number;          // 金額
    billing_cycle: 'monthly' | 'annual'; // 請求サイクル
    start_date: string;      // YYYY-MM-DD形式
    category: string;        // カテゴリ
    is_active: boolean;      // 有効/無効
    receipt_path: string | null; // 領収書パス
    created_at: string;      // RFC3339形式
    updated_at: string;      // RFC3339形式
}
```

### 3. リポジトリ層の実装

#### UserRepository

```typescript
class UserRepository {
    constructor(private db: D1Database) {}

    async findOrCreateUser(googleUser: GoogleUser): Promise<User>
    async getUserById(userId: string): Promise<User | null>
    async getUserByGoogleId(googleId: string): Promise<User | null>
    async updateUser(user: User): Promise<User>
    async deleteUser(userId: string): Promise<void>
    async getAllUsers(): Promise<User[]>
}
```

#### ExpenseRepository

```typescript
class ExpenseRepository {
    constructor(private db: D1Database) {}

    async create(dto: CreateExpenseDto, userId: string): Promise<Expense>
    async findById(id: number, userId: string): Promise<Expense | null>
    async findAll(userId: string, month?: string, category?: string): Promise<Expense[]>
    async update(id: number, dto: UpdateExpenseDto, userId: string): Promise<Expense>
    async delete(id: number, userId: string): Promise<void>
    async setReceiptUrl(id: number, receiptUrl: string, userId: string): Promise<Expense>
    async getReceiptUrl(id: number, userId: string): Promise<string | null>
}
```

#### SubscriptionRepository

```typescript
class SubscriptionRepository {
    constructor(private db: D1Database) {}

    async create(dto: CreateSubscriptionDto, userId: string): Promise<Subscription>
    async findById(id: number, userId: string): Promise<Subscription | null>
    async findAll(userId: string, activeOnly: boolean): Promise<Subscription[]>
    async update(id: number, dto: UpdateSubscriptionDto, userId: string): Promise<Subscription>
    async toggleStatus(id: number, userId: string): Promise<Subscription>
    async delete(id: number, userId: string): Promise<void>
    async calculateMonthlyTotal(userId: string): Promise<number>
    async setReceiptPath(id: number, receiptPath: string, userId: string): Promise<void>
    async getReceiptPath(id: number, userId: string): Promise<string | null>
}
```

### 4. APIエンドポイント

#### ユーザー関連エンドポイント

- `GET /api/v1/users/me` - 現在のユーザー情報を取得
- `PUT /api/v1/users/me` - 現在のユーザー情報を更新
- `DELETE /api/v1/users/me` - 現在のユーザーを削除

#### 経費関連エンドポイント

- `POST /api/v1/expenses` - 経費を作成
- `GET /api/v1/expenses/:id` - 経費を取得
- `GET /api/v1/expenses` - 経費一覧を取得（クエリパラメータ: month, category）
- `PUT /api/v1/expenses/:id` - 経費を更新
- `DELETE /api/v1/expenses/:id` - 経費を削除
- `PUT /api/v1/expenses/:id/receipt` - 領収書URLを設定
- `GET /api/v1/expenses/:id/receipt` - 領収書URLを取得

#### サブスクリプション関連エンドポイント

- `POST /api/v1/subscriptions` - サブスクリプションを作成
- `GET /api/v1/subscriptions/:id` - サブスクリプションを取得
- `GET /api/v1/subscriptions` - サブスクリプション一覧を取得（クエリパラメータ: activeOnly）
- `PUT /api/v1/subscriptions/:id` - サブスクリプションを更新
- `PATCH /api/v1/subscriptions/:id/toggle` - サブスクリプションのステータスを切り替え
- `DELETE /api/v1/subscriptions/:id` - サブスクリプションを削除
- `GET /api/v1/subscriptions/monthly-total` - 月額合計を取得

## データモデル

### DTO（Data Transfer Object）

#### CreateExpenseDto

```typescript
interface CreateExpenseDto {
    date: string;            // YYYY-MM-DD形式
    amount: number;          // 金額
    category: string;        // カテゴリ
    description?: string;    // 説明（オプション）
}
```

#### UpdateExpenseDto

```typescript
interface UpdateExpenseDto {
    date?: string;           // YYYY-MM-DD形式
    amount?: number;         // 金額
    category?: string;       // カテゴリ
    description?: string;    // 説明
    receipt_url?: string;    // 領収書URL
}
```

#### CreateSubscriptionDto

```typescript
interface CreateSubscriptionDto {
    name: string;            // サービス名
    amount: number;          // 金額
    billing_cycle: 'monthly' | 'annual'; // 請求サイクル
    start_date: string;      // YYYY-MM-DD形式
    category: string;        // カテゴリ
}
```

#### UpdateSubscriptionDto

```typescript
interface UpdateSubscriptionDto {
    name?: string;           // サービス名
    amount?: number;         // 金額
    billing_cycle?: 'monthly' | 'annual'; // 請求サイクル
    start_date?: string;     // YYYY-MM-DD形式
    category?: string;       // カテゴリ
}
```

## 正確性プロパティ

プロパティとは、すべての有効な実行において真であるべき特性や動作のことです。プロパティは、人間が読める仕様と機械で検証可能な正確性保証の橋渡しとなります。

### プロパティ 1: ユーザーデータの分離

*すべての*ユーザーについて、そのユーザーが自分のデータにアクセスする場合、システムは他のユーザーのデータを返してはならない

**検証: 要件 5.1, 5.2, 5.3, 8.3**

### プロパティ 2: データの整合性

*すべての*データ作成操作について、作成されたデータをIDで取得した場合、作成時に指定したデータと同じ内容が返される

**検証: 要件 2.1, 2.2, 3.1, 3.2, 4.1, 4.2**

### プロパティ 3: データの更新

*すべての*データ更新操作について、更新後にデータを取得した場合、更新した内容が反映されている

**検証: 要件 2.4, 3.4, 4.5**

### プロパティ 4: データの削除

*すべての*データ削除操作について、削除後にデータを取得しようとした場合、404エラーが返される

**検証: 要件 2.5, 3.6, 4.6, 6.2**

### プロパティ 5: 認証の必須性

*すべての*APIリクエストについて、有効なJWTトークンが含まれていない場合、システムは401エラーを返す

**検証: 要件 5.4, 8.1**

### プロパティ 6: アクセス制御

*すべての*データアクセス操作について、ユーザーが他のユーザーのデータにアクセスしようとした場合、システムは403エラーを返す

**検証: 要件 5.5, 8.3**

### プロパティ 7: フィルタリングの正確性

*すべての*経費一覧取得操作について、月フィルターを指定した場合、返されるすべての経費の日付は指定した月に属する

**検証: 要件 3.4**

### プロパティ 8: アクティブフィルターの正確性

*すべての*サブスクリプション一覧取得操作について、activeOnlyフィルターをtrueに指定した場合、返されるすべてのサブスクリプションのis_activeはtrueである

**検証: 要件 4.4**

### プロパティ 9: 月額合計の正確性

*すべての*月額合計計算について、計算結果はアクティブなサブスクリプションの月額換算金額の合計と等しい

**検証: 要件 4.7**

### プロパティ 10: SQLインジェクション防止

*すべての*データベースクエリについて、ユーザー入力はパラメータ化されたクエリで処理され、SQLインジェクション攻撃を防ぐ

**検証: 要件 8.4**

## エラーハンドリング

### エラーコード

```typescript
enum ErrorCode {
    // 認証エラー
    UNAUTHORIZED = 'UNAUTHORIZED',           // 401
    FORBIDDEN = 'FORBIDDEN',                 // 403
    
    // データエラー
    NOT_FOUND = 'NOT_FOUND',                 // 404
    VALIDATION_ERROR = 'VALIDATION_ERROR',   // 400
    CONFLICT = 'CONFLICT',                   // 409
    
    // データベースエラー
    DATABASE_ERROR = 'DATABASE_ERROR',       // 500
    CONNECTION_ERROR = 'CONNECTION_ERROR',   // 503
    
    // その他
    INTERNAL_ERROR = 'INTERNAL_ERROR'        // 500
}
```

### エラーレスポンス形式

```typescript
interface ErrorResponse {
    error: {
        code: ErrorCode;
        message: string;
        details?: any;
        timestamp: string;
        requestId: string;
    }
}
```

### エラーハンドリング戦略

1. **データベース接続エラー**: リトライ機構（最大3回、指数バックオフ）
2. **バリデーションエラー**: 詳細なエラーメッセージをクライアントに返す
3. **認証エラー**: セキュリティログに記録し、401/403を返す
4. **予期しないエラー**: エラーログに記録し、500を返す（詳細は隠す）

## テスト戦略

### ユニットテスト

各リポジトリメソッドの動作を検証：
- データの作成、取得、更新、削除
- エラーケース（存在しないID、無効なデータなど）
- バリデーション

### プロパティベーステスト

正確性プロパティを検証：
- ランダムなデータを生成してプロパティが成立することを確認
- 最低100回の反復実行
- fast-checkライブラリを使用

### 統合テスト

APIエンドポイントとD1データベースの連携を検証：
- 認証フロー
- CRUD操作
- エラーハンドリング
- アクセス制御

### テスト環境

- テスト用のD1データベースを使用
- テストデータは各テスト後にクリーンアップ
- Vitestを使用したテスト実行

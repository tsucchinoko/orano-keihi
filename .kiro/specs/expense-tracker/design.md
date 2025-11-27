# Design Document

## Overview

経費管理アプリケーションは、SvelteKit 5（フロントエンド）とTauri 2（デスクトップラッパー）、Rust（バックエンドロジック）、SQLite（データベース）を使用して構築されます。TailwindCSS v4を使用してモダンなグラデーションUIを実装し、確定申告に必要な経費データを効率的に管理します。

### Technology Stack

- **Frontend**: SvelteKit 5 (Svelte 5 runes API)
- **Styling**: TailwindCSS v4
- **Desktop Framework**: Tauri 2
- **Backend Logic**: Rust
- **Database**: SQLite (via rusqlite)
- **File System**: Tauri File System API

## Architecture

### Application Structure

```
┌─────────────────────────────────────────┐
│         SvelteKit Frontend              │
│  ┌─────────────────────────────────┐   │
│  │  UI Components (Svelte 5)       │   │
│  │  - ExpenseForm                   │   │
│  │  - ExpenseList                   │   │
│  │  - SubscriptionManager           │   │
│  │  - CategoryFilter                │   │
│  └─────────────────────────────────┘   │
│              ↕                          │
│  ┌─────────────────────────────────┐   │
│  │  Tauri Commands (invoke)        │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
              ↕
┌─────────────────────────────────────────┐
│         Rust Backend (Tauri)            │
│  ┌─────────────────────────────────┐   │
│  │  Command Handlers               │   │
│  │  - expense_commands.rs          │   │
│  │  - subscription_commands.rs     │   │
│  └─────────────────────────────────┘   │
│              ↕                          │
│  ┌─────────────────────────────────┐   │
│  │  Database Layer                 │   │
│  │  - db.rs (SQLite connection)    │   │
│  │  - models.rs                    │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
              ↕
┌─────────────────────────────────────────┐
│         SQLite Database                 │
│  - expenses table                       │
│  - subscriptions table                  │
│  - categories table                     │
└─────────────────────────────────────────┘
```

### Directory Structure

```
src/
├── routes/
│   ├── +page.svelte              # メインダッシュボード
│   ├── +layout.svelte            # グローバルレイアウト
│   └── expenses/
│       └── +page.svelte          # 経費一覧ページ
├── lib/
│   ├── components/
│   │   ├── ExpenseForm.svelte
│   │   ├── ExpenseList.svelte
│   │   ├── ExpenseItem.svelte
│   │   ├── SubscriptionForm.svelte
│   │   ├── SubscriptionList.svelte
│   │   ├── CategoryFilter.svelte
│   │   ├── MonthSelector.svelte
│   │   └── ReceiptViewer.svelte
│   ├── stores/
│   │   └── expenses.svelte.ts    # Svelte 5 runes state
│   ├── types/
│   │   └── index.ts
│   └── utils/
│       └── tauri.ts              # Tauri command wrappers
└── app.css                       # TailwindCSS + グラデーション

src-tauri/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── expense_commands.rs
│   │   └── subscription_commands.rs
│   ├── db/
│   │   ├── mod.rs
│   │   ├── connection.rs
│   │   └── migrations.rs
│   └── models/
│       ├── mod.rs
│       ├── expense.rs
│       └── subscription.rs
└── Cargo.toml
```

## Components and Interfaces

### Frontend Components

#### 1. ExpenseForm.svelte
経費入力フォーム

**Props:**
- `expense?: Expense` - 編集時の既存データ（オプション）

**Events:**
- `onSave(expense: Expense)` - 保存時
- `onCancel()` - キャンセル時

**Features:**
- 日付ピッカー
- 金額入力（数値バリデーション）
- カテゴリドロップダウン
- 説明テキストエリア
- 領収書ファイルアップロード（画像プレビュー付き）

#### 2. ExpenseList.svelte
経費一覧表示

**Props:**
- `expenses: Expense[]`
- `selectedMonth: string`

**Features:**
- 月別フィルタリング
- カテゴリ別グループ表示
- 合計金額表示
- 編集・削除ボタン
- 領収書サムネイル表示

#### 3. SubscriptionForm.svelte
サブスクリプション入力フォーム

**Props:**
- `subscription?: Subscription`

**Features:**
- サービス名入力
- 金額入力
- 支払いサイクル選択（月払い/年払い）
- 開始日選択
- カテゴリ選択

#### 4. SubscriptionList.svelte
サブスクリプション一覧

**Features:**
- アクティブ/非アクティブ切り替え
- 月額換算表示
- 次回支払日表示
- 合計月額コスト表示

#### 5. CategoryFilter.svelte
カテゴリフィルター

**Props:**
- `selectedCategories: string[]`
- `onFilterChange(categories: string[])`

**Features:**
- マルチセレクトチェックボックス
- カテゴリ別カラーコーディング

#### 6. ReceiptViewer.svelte
領収書ビューアー（モーダル）

**Props:**
- `receiptPath: string`
- `onClose()`

**Features:**
- 画像/PDF表示
- ズーム機能
- ダウンロードボタン

### Tauri Commands (Rust)

#### Expense Commands

```rust
#[tauri::command]
async fn create_expense(
    expense: CreateExpenseDto,
    state: State<'_, AppState>
) -> Result<Expense, String>

#[tauri::command]
async fn get_expenses(
    month: Option<String>,
    category: Option<String>,
    state: State<'_, AppState>
) -> Result<Vec<Expense>, String>

#[tauri::command]
async fn update_expense(
    id: i64,
    expense: UpdateExpenseDto,
    state: State<'_, AppState>
) -> Result<Expense, String>

#[tauri::command]
async fn delete_expense(
    id: i64,
    state: State<'_, AppState>
) -> Result<(), String>

#[tauri::command]
async fn save_receipt(
    expense_id: i64,
    file_path: String,
    state: State<'_, AppState>
) -> Result<String, String>
```

#### Subscription Commands

```rust
#[tauri::command]
async fn create_subscription(
    subscription: CreateSubscriptionDto,
    state: State<'_, AppState>
) -> Result<Subscription, String>

#[tauri::command]
async fn get_subscriptions(
    active_only: bool,
    state: State<'_, AppState>
) -> Result<Vec<Subscription>, String>

#[tauri::command]
async fn update_subscription(
    id: i64,
    subscription: UpdateSubscriptionDto,
    state: State<'_, AppState>
) -> Result<Subscription, String>

#[tauri::command]
async fn toggle_subscription_status(
    id: i64,
    state: State<'_, AppState>
) -> Result<Subscription, String>

#[tauri::command]
async fn get_monthly_subscription_total(
    state: State<'_, AppState>
) -> Result<f64, String>
```

## Data Models

### Database Schema

#### expenses table

```sql
CREATE TABLE expenses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    amount REAL NOT NULL,
    category TEXT NOT NULL,
    description TEXT,
    receipt_path TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_expenses_date ON expenses(date);
CREATE INDEX idx_expenses_category ON expenses(category);
```

#### subscriptions table

```sql
CREATE TABLE subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    amount REAL NOT NULL,
    billing_cycle TEXT NOT NULL CHECK(billing_cycle IN ('monthly', 'annual')),
    start_date TEXT NOT NULL,
    category TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_subscriptions_active ON subscriptions(is_active);
```

#### categories table (predefined)

```sql
CREATE TABLE categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL,
    icon TEXT
);

-- Initial data
INSERT INTO categories (name, color, icon) VALUES
    ('交通費', '#3B82F6', '🚗'),
    ('飲食費', '#EF4444', '🍽️'),
    ('通信費', '#8B5CF6', '📱'),
    ('消耗品費', '#10B981', '📦'),
    ('接待交際費', '#F59E0B', '🤝'),
    ('その他', '#6B7280', '📋');
```

### TypeScript Types

```typescript
// src/lib/types/index.ts

export interface Expense {
  id: number;
  date: string; // ISO 8601 format
  amount: number;
  category: string;
  description?: string;
  receipt_path?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateExpenseDto {
  date: string;
  amount: number;
  category: string;
  description?: string;
}

export interface Subscription {
  id: number;
  name: string;
  amount: number;
  billing_cycle: 'monthly' | 'annual';
  start_date: string;
  category: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateSubscriptionDto {
  name: string;
  amount: number;
  billing_cycle: 'monthly' | 'annual';
  start_date: string;
  category: string;
}

export interface Category {
  id: number;
  name: string;
  color: string;
  icon?: string;
}

export interface MonthlyTotal {
  category: string;
  total: number;
}
```

### Rust Models

```rust
// src-tauri/src/models/expense.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Expense {
    pub id: i64,
    pub date: String,
    pub amount: f64,
    pub category: String,
    pub description: Option<String>,
    pub receipt_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateExpenseDto {
    pub date: String,
    pub amount: f64,
    pub category: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExpenseDto {
    pub date: Option<String>,
    pub amount: Option<f64>,
    pub category: Option<String>,
    pub description: Option<String>,
}
```

## UI Design System

### Color Palette & Gradients

```css
/* Primary Gradients */
--gradient-primary: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
--gradient-success: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
--gradient-info: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
--gradient-warning: linear-gradient(135deg, #fa709a 0%, #fee140 100%);

/* Category Colors */
--color-transport: #3B82F6;
--color-meals: #EF4444;
--color-communication: #8B5CF6;
--color-supplies: #10B981;
--color-entertainment: #F59E0B;
--color-other: #6B7280;

/* Background Gradients */
--bg-gradient-light: linear-gradient(to bottom right, #fafafa, #e5e7eb);
--bg-gradient-dark: linear-gradient(to bottom right, #1f2937, #111827);
```

### Component Styling Guidelines

1. **Cards**: 白背景、subtle shadow、rounded corners (12px)
2. **Buttons**: グラデーション背景、hover時に明度変化、smooth transition
3. **Input Fields**: border-2、focus時にグラデーションborder
4. **Lists**: alternating background、hover時にグラデーション overlay
5. **Typography**: Inter font family、見出しは font-bold、本文は font-normal

### Responsive Design

- Desktop-first approach (Tauriはデスクトップアプリ)
- Minimum window size: 800x600px
- Maximum content width: 1200px
- Grid layout for expense cards (2-3 columns)

## Error Handling

### Frontend Error Handling

```typescript
// src/lib/utils/tauri.ts

export async function handleTauriCommand<T>(
  command: Promise<T>
): Promise<{ data?: T; error?: string }> {
  try {
    const data = await command;
    return { data };
  } catch (error) {
    console.error('Tauri command error:', error);
    return { error: String(error) };
  }
}
```

### Error Display

- Toast notifications for user-facing errors
- Inline validation errors for form fields
- Error boundary for unexpected errors

### Rust Error Handling

```rust
// Custom error type
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("File system error: {0}")]
    FileSystem(String),
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}
```

### Validation Rules

- **Amount**: 正の数値のみ、最大10桁
- **Date**: 未来の日付は不可、ISO 8601形式
- **Category**: 定義済みカテゴリのみ
- **Receipt File**: 最大10MB、PNG/JPG/PDF形式のみ
- **Description**: 最大500文字

## Testing Strategy

### Unit Tests

**Rust (Backend)**
- Database CRUD operations
- Data validation logic
- Date/amount calculations
- Error handling

**TypeScript (Frontend)**
- Utility functions
- Data transformation
- Validation logic

### Integration Tests

**Tauri Commands**
- Command invocation from frontend
- Data flow between frontend and backend
- File system operations
- Database transactions

### Manual Testing Checklist

- [ ] 経費の作成・編集・削除
- [ ] 領収書のアップロードと表示
- [ ] サブスクリプションの管理
- [ ] 月別フィルタリング
- [ ] カテゴリ別フィルタリング
- [ ] 合計金額の計算
- [ ] UI responsiveness
- [ ] グラデーションとアニメーション
- [ ] エラーハンドリング
- [ ] データの永続化

### Performance Considerations

- SQLiteクエリの最適化（インデックス使用）
- 大量の経費データでのページネーション（将来的に）
- 画像サムネイルの遅延読み込み
- Svelte 5 runesによる効率的なリアクティビティ

## Database Management

### Initialization

アプリ起動時に以下を実行：
1. データベースファイルの存在確認
2. 存在しない場合は作成
3. テーブルスキーマのマイグレーション実行
4. 初期カテゴリデータの挿入

### File Location

```rust
// データベースファイルパス
// macOS: ~/Library/Application Support/com.daichitsuchiya.subscription-memo/expenses.db
// Windows: C:\Users\{username}\AppData\Roaming\com.daichitsuchiya.subscription-memo\expenses.db
// Linux: ~/.local/share/com.daichitsuchiya.subscription-memo/expenses.db

use tauri::api::path::app_data_dir;

pub fn get_db_path(config: &Config) -> PathBuf {
    let app_data = app_data_dir(config).expect("Failed to get app data dir");
    app_data.join("expenses.db")
}
```

### Receipt File Storage

```rust
// 領収書ファイルパス
// {app_data_dir}/receipts/{expense_id}_{timestamp}.{ext}

pub fn save_receipt_file(
    app_data_dir: &Path,
    expense_id: i64,
    source_path: &str
) -> Result<String, AppError> {
    let receipts_dir = app_data_dir.join("receipts");
    std::fs::create_dir_all(&receipts_dir)?;
    
    let timestamp = chrono::Utc::now().timestamp();
    let ext = Path::new(source_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jpg");
    
    let filename = format!("{}_{}.{}", expense_id, timestamp, ext);
    let dest_path = receipts_dir.join(&filename);
    
    std::fs::copy(source_path, &dest_path)?;
    
    Ok(dest_path.to_string_lossy().to_string())
}
```

## Security Considerations

- SQLインジェクション対策: rusqliteのパラメータバインディング使用
- ファイルパス検証: 領収書ファイルパスのサニタイゼーション
- ローカルストレージのみ: 外部通信なし、データは全てローカル
- Tauri CSP設定: 必要に応じて制限

## Future Enhancements (Out of Scope for MVP)

- データのエクスポート機能（CSV/PDF）
- 年間レポート生成
- クラウドバックアップ
- OCRによる領収書自動読み取り
- 複数通貨対応
- タグ機能
- 検索機能
- データ分析グラフ

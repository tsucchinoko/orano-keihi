-- D1データベーススキーマ定義
-- Cloudflare D1 (SQLite互換) 用

-- usersテーブル
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,              -- nanoId形式（21文字）
    google_id TEXT NOT NULL UNIQUE,   -- Google OAuth ID
    email TEXT NOT NULL,              -- メールアドレス
    name TEXT NOT NULL,               -- ユーザー名
    picture_url TEXT,                 -- プロフィール画像URL
    created_at TEXT NOT NULL,         -- RFC3339形式（JST）
    updated_at TEXT NOT NULL          -- RFC3339形式（JST）
);

-- usersテーブルのインデックス
CREATE INDEX IF NOT EXISTS idx_users_google_id ON users(google_id);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- expensesテーブル
CREATE TABLE IF NOT EXISTS expenses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,            -- ユーザーID（nanoId形式）
    date TEXT NOT NULL,               -- YYYY-MM-DD形式
    amount REAL NOT NULL,             -- 金額
    category TEXT NOT NULL,           -- カテゴリ（後方互換性のため残す）
    category_id INTEGER,              -- カテゴリID（categoriesテーブルへの外部キー）
    description TEXT,                 -- 説明（オプション）
    receipt_url TEXT,                 -- 領収書URL（HTTPS）
    created_at TEXT NOT NULL,         -- RFC3339形式（JST）
    updated_at TEXT NOT NULL,         -- RFC3339形式（JST）
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(id),
    CHECK (receipt_url IS NULL OR receipt_url LIKE 'https://%')
);

-- expensesテーブルのインデックス
CREATE INDEX IF NOT EXISTS idx_expenses_user_id ON expenses(user_id);
CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date);
CREATE INDEX IF NOT EXISTS idx_expenses_category ON expenses(category);
CREATE INDEX IF NOT EXISTS idx_expenses_category_id ON expenses(category_id);

-- subscriptionsテーブル
CREATE TABLE IF NOT EXISTS subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,            -- ユーザーID（nanoId形式）
    name TEXT NOT NULL,               -- サービス名
    amount REAL NOT NULL,             -- 金額
    billing_cycle TEXT NOT NULL,      -- "monthly" または "annual"
    start_date TEXT NOT NULL,         -- YYYY-MM-DD形式
    category TEXT NOT NULL,           -- カテゴリ（後方互換性のため残す）
    category_id INTEGER,              -- カテゴリID（categoriesテーブルへの外部キー）
    is_active INTEGER NOT NULL DEFAULT 1, -- 0=無効, 1=有効
    receipt_path TEXT,                -- 領収書パス（将来的にreceipt_urlに移行）
    created_at TEXT NOT NULL,         -- RFC3339形式（JST）
    updated_at TEXT NOT NULL,         -- RFC3339形式（JST）
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(id),
    CHECK (billing_cycle IN ('monthly', 'annual'))
);

-- subscriptionsテーブルのインデックス
CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_is_active ON subscriptions(is_active);
CREATE INDEX IF NOT EXISTS idx_subscriptions_category_id ON subscriptions(category_id);

-- categoriesテーブル
CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,            -- カテゴリ名（例：交通費）
    icon TEXT NOT NULL,                   -- 絵文字アイコン（例：🚗）
    display_order INTEGER NOT NULL DEFAULT 0, -- 表示順序
    is_active INTEGER NOT NULL DEFAULT 1,     -- 有効/無効フラグ (0=無効, 1=有効)
    created_at TEXT NOT NULL,             -- RFC3339形式（JST）
    updated_at TEXT NOT NULL              -- RFC3339形式（JST）
);

-- categoriesテーブルのインデックス
CREATE INDEX IF NOT EXISTS idx_categories_display_order ON categories(display_order);
CREATE INDEX IF NOT EXISTS idx_categories_is_active ON categories(is_active);

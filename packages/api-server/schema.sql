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
    category TEXT NOT NULL,           -- カテゴリ
    description TEXT,                 -- 説明（オプション）
    receipt_url TEXT,                 -- 領収書URL（HTTPS）
    created_at TEXT NOT NULL,         -- RFC3339形式（JST）
    updated_at TEXT NOT NULL,         -- RFC3339形式（JST）
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CHECK (receipt_url IS NULL OR receipt_url LIKE 'https://%')
);

-- expensesテーブルのインデックス
CREATE INDEX IF NOT EXISTS idx_expenses_user_id ON expenses(user_id);
CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date);
CREATE INDEX IF NOT EXISTS idx_expenses_category ON expenses(category);

-- subscriptionsテーブル
CREATE TABLE IF NOT EXISTS subscriptions (
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

-- subscriptionsテーブルのインデックス
CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_is_active ON subscriptions(is_active);

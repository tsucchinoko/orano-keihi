-- Migration: カテゴリーテーブルの追加
-- 実行日時: 2024-01-XX
-- 説明: カテゴリーをDBで一元管理するためのテーブル作成とデータ移行

-- ============================================
-- Step 1: categoriesテーブルの作成
-- ============================================
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

-- ============================================
-- Step 2: 初期カテゴリーデータの投入
-- ============================================
INSERT OR IGNORE INTO categories (name, icon, display_order, is_active, created_at, updated_at) VALUES
    ('交通費', '🚗', 1, 1, datetime('now'), datetime('now')),
    ('飲食費', '🍽️', 2, 1, datetime('now'), datetime('now')),
    ('通信費', '📱', 3, 1, datetime('now'), datetime('now')),
    ('消耗品費', '📦', 4, 1, datetime('now'), datetime('now')),
    ('接待交際費', '🤝', 5, 1, datetime('now'), datetime('now')),
    ('その他', '📋', 6, 1, datetime('now'), datetime('now'));

-- ============================================
-- Step 3: expensesテーブルに category_id カラムを追加
-- ============================================
ALTER TABLE expenses ADD COLUMN category_id INTEGER REFERENCES categories(id);

-- ============================================
-- Step 4: subscriptionsテーブルに category_id カラムを追加
-- ============================================
ALTER TABLE subscriptions ADD COLUMN category_id INTEGER REFERENCES categories(id);

-- ============================================
-- Step 5: 既存データのマイグレーション
-- expenses.category (TEXT) → expenses.category_id (INTEGER)
-- ============================================
UPDATE expenses SET category_id = (
    SELECT id FROM categories WHERE categories.name = expenses.category
) WHERE category_id IS NULL;

-- 不明なカテゴリーは「その他」にマッピング
UPDATE expenses SET category_id = (
    SELECT id FROM categories WHERE name = 'その他'
) WHERE category_id IS NULL;

-- ============================================
-- Step 6: 既存データのマイグレーション
-- subscriptions.category (TEXT) → subscriptions.category_id (INTEGER)
-- ============================================
UPDATE subscriptions SET category_id = (
    SELECT id FROM categories WHERE categories.name = subscriptions.category
) WHERE category_id IS NULL;

-- 不明なカテゴリーは「その他」にマッピング
UPDATE subscriptions SET category_id = (
    SELECT id FROM categories WHERE name = 'その他'
) WHERE category_id IS NULL;

-- ============================================
-- Step 7: category_idカラムのインデックス作成
-- ============================================
CREATE INDEX IF NOT EXISTS idx_expenses_category_id ON expenses(category_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_category_id ON subscriptions(category_id);

-- ============================================
-- 注意事項:
-- - SQLiteではALTER TABLEでNOT NULL制約を後から追加できないため、
--   category_idはNULL許容のまま運用し、アプリケーション側でバリデーションを行う
-- - 移行完了後、categoryカラム（TEXT）は後方互換性のため残しておく
-- - 将来的に不要になった場合は、新テーブル作成→データ移行→旧テーブル削除の手順で対応
-- ============================================

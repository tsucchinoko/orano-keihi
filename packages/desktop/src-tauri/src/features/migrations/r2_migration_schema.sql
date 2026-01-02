-- R2ユーザーディレクトリ移行用データベーススキーマ
-- 
-- このSQLファイルは、R2バケット内のレシートファイルを
-- `/receipts/`構造から`/users/{user_id}/receipts/`構造に移行する
-- プロセスを追跡するためのテーブル構造を定義します。

-- 移行ログテーブル
-- 移行プロセス全体の状態を追跡します（要件4.1, 4.2）
CREATE TABLE IF NOT EXISTS migration_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    migration_type TEXT NOT NULL, -- 'r2_user_directory'
    status TEXT NOT NULL CHECK(status IN ('started', 'in_progress', 'completed', 'failed', 'paused')),
    total_items INTEGER NOT NULL DEFAULT 0,
    processed_items INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,
    error_details TEXT, -- JSON形式のエラー詳細
    started_at TEXT NOT NULL,
    completed_at TEXT,
    created_by TEXT, -- システムまたはユーザーID
    metadata TEXT -- JSON形式の追加情報
);

-- 移行アイテムテーブル
-- 個別ファイルの移行状態を追跡します（要件4.1, 4.2, 4.3）
CREATE TABLE IF NOT EXISTS migration_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    migration_log_id INTEGER NOT NULL,
    old_path TEXT NOT NULL,
    new_path TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    file_size INTEGER NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending', 'processing', 'completed', 'failed')),
    error_message TEXT,
    started_at TEXT,
    completed_at TEXT,
    file_hash TEXT, -- SHA256ハッシュ
    FOREIGN KEY (migration_log_id) REFERENCES migration_log(id)
);

-- migration_logテーブルのインデックス
CREATE INDEX IF NOT EXISTS idx_migration_log_type ON migration_log(migration_type);
CREATE INDEX IF NOT EXISTS idx_migration_log_status ON migration_log(status);
CREATE INDEX IF NOT EXISTS idx_migration_log_started_at ON migration_log(started_at);

-- migration_itemsテーブルのインデックス
CREATE INDEX IF NOT EXISTS idx_migration_items_log_id ON migration_items(migration_log_id);
CREATE INDEX IF NOT EXISTS idx_migration_items_status ON migration_items(status);
CREATE INDEX IF NOT EXISTS idx_migration_items_user_id ON migration_items(user_id);
CREATE INDEX IF NOT EXISTS idx_migration_items_old_path ON migration_items(old_path);
CREATE INDEX IF NOT EXISTS idx_migration_items_new_path ON migration_items(new_path);
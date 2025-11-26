use rusqlite::{Connection, Result};

/// すべてのデータベースマイグレーションを実行する
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // 経費テーブルを作成
    conn.execute(
        "CREATE TABLE IF NOT EXISTS expenses (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            amount REAL NOT NULL,
            category TEXT NOT NULL,
            description TEXT,
            receipt_path TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // 経費テーブルのインデックスを作成
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_expenses_category ON expenses(category)",
        [],
    )?;

    // サブスクリプションテーブルを作成
    conn.execute(
        "CREATE TABLE IF NOT EXISTS subscriptions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            amount REAL NOT NULL,
            billing_cycle TEXT NOT NULL CHECK(billing_cycle IN ('monthly', 'annual')),
            start_date TEXT NOT NULL,
            category TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // サブスクリプションテーブルのインデックスを作成
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_subscriptions_active ON subscriptions(is_active)",
        [],
    )?;

    // カテゴリテーブルを作成
    conn.execute(
        "CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL,
            icon TEXT
        )",
        [],
    )?;

    // テーブルが空の場合、初期カテゴリデータを挿入
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM categories",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        let categories = [
            ("交通費", "#3B82F6", "🚗"),
            ("飲食費", "#EF4444", "🍽️"),
            ("通信費", "#8B5CF6", "📱"),
            ("消耗品費", "#10B981", "📦"),
            ("接待交際費", "#F59E0B", "🤝"),
            ("その他", "#6B7280", "📋"),
        ];

        for (name, color, icon) in categories.iter() {
            conn.execute(
                "INSERT INTO categories (name, color, icon) VALUES (?1, ?2, ?3)",
                [name, color, icon],
            )?;
        }
    }

    Ok(())
}

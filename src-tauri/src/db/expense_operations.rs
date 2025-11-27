use crate::models::{CreateExpenseDto, Expense, UpdateExpenseDto};
use chrono::Utc;
use rusqlite::{params, Connection, Result};

/// 経費を作成する
///
/// # 引数
/// * `conn` - データベース接続
/// * `dto` - 経費作成用DTO
///
/// # 戻り値
/// 作成された経費、または失敗時はエラー
pub fn create_expense(conn: &Connection, dto: CreateExpenseDto) -> Result<Expense> {
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO expenses (date, amount, category, description, receipt_path, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)",
        params![dto.date, dto.amount, dto.category, dto.description, now, now],
    )?;

    let id = conn.last_insert_rowid();
    get_expense_by_id(conn, id)
}

/// IDで経費を取得する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - 経費ID
///
/// # 戻り値
/// 経費、または失敗時はエラー
pub fn get_expense_by_id(conn: &Connection, id: i64) -> Result<Expense> {
    conn.query_row(
        "SELECT id, date, amount, category, description, receipt_path, created_at, updated_at
         FROM expenses WHERE id = ?1",
        params![id],
        |row| {
            Ok(Expense {
                id: row.get(0)?,
                date: row.get(1)?,
                amount: row.get(2)?,
                category: row.get(3)?,
                description: row.get(4)?,
                receipt_path: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
}

/// 経費一覧を取得する（月とカテゴリでフィルタリング可能）
///
/// # 引数
/// * `conn` - データベース接続
/// * `month` - 月フィルター（YYYY-MM形式、オプション）
/// * `category` - カテゴリフィルター（オプション）
///
/// # 戻り値
/// 経費のリスト、または失敗時はエラー
pub fn get_expenses(
    conn: &Connection,
    month: Option<String>,
    category: Option<String>,
) -> Result<Vec<Expense>> {
    let mut query = String::from(
        "SELECT id, date, amount, category, description, receipt_path, created_at, updated_at
         FROM expenses WHERE 1=1",
    );

    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    // 月フィルター
    if let Some(m) = month {
        query.push_str(" AND date LIKE ?");
        params.push(Box::new(format!("{m}%")));
    }

    // カテゴリフィルター
    if let Some(c) = category {
        query.push_str(" AND category = ?");
        params.push(Box::new(c));
    }

    query.push_str(" ORDER BY date DESC");

    let mut stmt = conn.prepare(&query)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let expenses = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(Expense {
            id: row.get(0)?,
            date: row.get(1)?,
            amount: row.get(2)?,
            category: row.get(3)?,
            description: row.get(4)?,
            receipt_path: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    expenses.collect()
}

/// 経費を更新する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - 経費ID
/// * `dto` - 経費更新用DTO
///
/// # 戻り値
/// 更新された経費、または失敗時はエラー
pub fn update_expense(conn: &Connection, id: i64, dto: UpdateExpenseDto) -> Result<Expense> {
    let now = Utc::now().to_rfc3339();

    // 既存の経費を取得
    let existing = get_expense_by_id(conn, id)?;

    // 更新するフィールドを決定
    let date = dto.date.unwrap_or(existing.date);
    let amount = dto.amount.unwrap_or(existing.amount);
    let category = dto.category.unwrap_or(existing.category);
    let description = dto.description.or(existing.description);

    conn.execute(
        "UPDATE expenses SET date = ?1, amount = ?2, category = ?3, description = ?4, updated_at = ?5
         WHERE id = ?6",
        params![date, amount, category, description, now, id],
    )?;

    get_expense_by_id(conn, id)
}

/// 経費を削除する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - 経費ID
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
pub fn delete_expense(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM expenses WHERE id = ?1", params![id])?;
    Ok(())
}

/// 経費に領収書パスを設定する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - 経費ID
/// * `receipt_path` - 領収書ファイルパス
///
/// # 戻り値
/// 更新された経費、または失敗時はエラー
pub fn set_receipt_path(conn: &Connection, id: i64, receipt_path: String) -> Result<Expense> {
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE expenses SET receipt_path = ?1, updated_at = ?2 WHERE id = ?3",
        params![receipt_path, now, id],
    )?;

    get_expense_by_id(conn, id)
}

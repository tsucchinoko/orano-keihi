use crate::models::{CreateExpenseDto, Expense, UpdateExpenseDto};
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
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
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

    conn.execute(
        "INSERT INTO expenses (date, amount, category, description, receipt_url, created_at, updated_at)
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
        "SELECT id, date, amount, category, description, receipt_url, created_at, updated_at
         FROM expenses WHERE id = ?1",
        params![id],
        |row| {
            Ok(Expense {
                id: row.get(0)?,
                date: row.get(1)?,
                amount: row.get(2)?,
                category: row.get(3)?,
                description: row.get(4)?,
                receipt_url: row.get(5)?,
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
        "SELECT id, date, amount, category, description, receipt_url, created_at, updated_at
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
            receipt_url: row.get(5)?,
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
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

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

/// 経費に領収書URLを設定する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - 経費ID
/// * `receipt_url` - 領収書URL（空文字列の場合はNULLに設定）
///
/// # 戻り値
/// 更新された経費、または失敗時はエラー
pub fn set_receipt_url(conn: &Connection, id: i64, receipt_url: String) -> Result<Expense> {
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

    // 空文字列の場合はNULLに設定
    let url_value = if receipt_url.is_empty() {
        None
    } else {
        // HTTPS URLの検証
        if !receipt_url.starts_with("https://") {
            return Err(rusqlite::Error::InvalidColumnType(
                0,
                "receipt_url".to_string(),
                rusqlite::types::Type::Text,
            ));
        }
        Some(receipt_url)
    };

    conn.execute(
        "UPDATE expenses SET receipt_url = ?1, updated_at = ?2 WHERE id = ?3",
        params![url_value, now, id],
    )?;

    get_expense_by_id(conn, id)
}

/// 経費の領収書URLを取得する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - 経費ID
///
/// # 戻り値
/// 領収書URL（存在する場合）、または失敗時はエラー
pub fn get_receipt_url(conn: &Connection, id: i64) -> Result<Option<String>> {
    conn.query_row(
        "SELECT receipt_url FROM expenses WHERE id = ?1",
        params![id],
        |row| {
            let url: Option<String> = row.get(0)?;
            // 空文字列の場合はNoneとして扱う
            Ok(url.filter(|u| !u.is_empty()))
        },
    )
}

/// 後方互換性のための関数（廃止予定）
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - 経費ID
/// * `receipt_path` - 領収書パス（receipt_urlとして扱われる）
///
/// # 戻り値
/// 更新された経費、または失敗時はエラー
#[deprecated(note = "set_receipt_urlを使用してください")]
pub fn set_receipt_path(conn: &Connection, id: i64, receipt_path: String) -> Result<Expense> {
    set_receipt_url(conn, id, receipt_path)
}

/// 後方互換性のための関数（廃止予定）
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - 経費ID
///
/// # 戻り値
/// 領収書URL（存在する場合）、または失敗時はエラー
#[deprecated(note = "get_receipt_urlを使用してください")]
pub fn get_receipt_path(conn: &Connection, id: i64) -> Result<Option<String>> {
    get_receipt_url(conn, id)
}
/// 領収書キャッシュを保存する
///
/// # 引数
/// * `conn` - データベース接続
/// * `receipt_url` - 領収書URL
/// * `local_path` - ローカルファイルパス
/// * `file_size` - ファイルサイズ
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
pub fn save_receipt_cache(
    conn: &Connection,
    receipt_url: &str,
    local_path: &str,
    file_size: i64,
) -> Result<()> {
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

    conn.execute(
        "INSERT OR REPLACE INTO receipt_cache 
         (receipt_url, local_path, cached_at, file_size, last_accessed)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![receipt_url, local_path, now, file_size, now],
    )?;

    Ok(())
}

/// 領収書キャッシュを取得する
///
/// # 引数
/// * `conn` - データベース接続
/// * `receipt_url` - 領収書URL
///
/// # 戻り値
/// キャッシュ情報（存在する場合）、または失敗時はエラー
pub fn get_receipt_cache(
    conn: &Connection,
    receipt_url: &str,
) -> Result<Option<crate::models::expense::ReceiptCache>> {
    match conn.query_row(
        "SELECT id, receipt_url, local_path, cached_at, file_size, last_accessed
         FROM receipt_cache WHERE receipt_url = ?1",
        params![receipt_url],
        |row| {
            Ok(crate::models::expense::ReceiptCache {
                id: row.get(0)?,
                receipt_url: row.get(1)?,
                local_path: row.get(2)?,
                cached_at: row.get(3)?,
                file_size: row.get(4)?,
                last_accessed: row.get(5)?,
            })
        },
    ) {
        Ok(cache) => Ok(Some(cache)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// 領収書キャッシュのアクセス時刻を更新する
///
/// # 引数
/// * `conn` - データベース接続
/// * `receipt_url` - 領収書URL
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
pub fn update_cache_access_time(conn: &Connection, receipt_url: &str) -> Result<()> {
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

    conn.execute(
        "UPDATE receipt_cache SET last_accessed = ?1 WHERE receipt_url = ?2",
        params![now, receipt_url],
    )?;

    Ok(())
}

/// 古い領収書キャッシュを削除する
///
/// # 引数
/// * `conn` - データベース接続
/// * `max_age_days` - 最大保持日数
///
/// # 戻り値
/// 削除されたレコード数、または失敗時はエラー
pub fn cleanup_old_cache(conn: &Connection, max_age_days: i64) -> Result<usize> {
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo);
    let cutoff_date = now - chrono::Duration::days(max_age_days);
    let cutoff_str = cutoff_date.to_rfc3339();

    let changes = conn.execute(
        "DELETE FROM receipt_cache WHERE last_accessed < ?1",
        params![cutoff_str],
    )?;

    Ok(changes)
}

/// 領収書キャッシュを削除する
///
/// # 引数
/// * `conn` - データベース接続
/// * `receipt_url` - 領収書URL
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
pub fn delete_receipt_cache(conn: &Connection, receipt_url: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM receipt_cache WHERE receipt_url = ?1",
        params![receipt_url],
    )?;

    Ok(())
}

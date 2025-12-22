use crate::features::expenses::models::{CreateExpenseDto, Expense, UpdateExpenseDto, ReceiptCache};
use crate::shared::errors::{AppError, AppResult};
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use rusqlite::{params, Connection};

/// 経費を作成する
///
/// # 引数
/// * `conn` - データベース接続
/// * `dto` - 経費作成用DTO
///
/// # 戻り値
/// 作成された経費、または失敗時はエラー
pub fn create(conn: &Connection, dto: CreateExpenseDto) -> AppResult<Expense> {
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

    conn.execute(
        "INSERT INTO expenses (date, amount, category, description, receipt_url, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)",
        params![dto.date, dto.amount, dto.category, dto.description, now, now],
    )?;

    let id = conn.last_insert_rowid();
    find_by_id(conn, id)
}

/// IDで経費を取得する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - 経費ID
///
/// # 戻り値
/// 経費、または失敗時はエラー
pub fn find_by_id(conn: &Connection, id: i64) -> AppResult<Expense> {
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
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::not_found("経費"),
        _ => AppError::Database(e),
    })
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
pub fn find_all(
    conn: &Connection,
    month: Option<&str>,
    category: Option<&str>,
) -> AppResult<Vec<Expense>> {
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
        params.push(Box::new(c.to_string()));
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

    expenses.collect::<Result<Vec<_>, _>>().map_err(AppError::Database)
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
pub fn update(conn: &Connection, id: i64, dto: UpdateExpenseDto) -> AppResult<Expense> {
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

    // 既存の経費を取得
    let existing = find_by_id(conn, id)?;

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

    find_by_id(conn, id)
}

/// 経費を削除する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - 経費ID
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
pub fn delete(conn: &Connection, id: i64) -> AppResult<()> {
    let affected_rows = conn.execute("DELETE FROM expenses WHERE id = ?1", params![id])?;
    
    if affected_rows == 0 {
        return Err(AppError::not_found("経費"));
    }
    
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
pub fn set_receipt_url(conn: &Connection, id: i64, receipt_url: String) -> AppResult<Expense> {
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

    // 空文字列の場合はNULLに設定
    let url_value = if receipt_url.is_empty() {
        None
    } else {
        // HTTPS URLの検証
        if !receipt_url.starts_with("https://") {
            return Err(AppError::validation("領収書URLはHTTPS形式である必要があります"));
        }
        Some(receipt_url)
    };

    let affected_rows = conn.execute(
        "UPDATE expenses SET receipt_url = ?1, updated_at = ?2 WHERE id = ?3",
        params![url_value, now, id],
    )?;

    if affected_rows == 0 {
        return Err(AppError::not_found("経費"));
    }

    find_by_id(conn, id)
}

/// 経費の領収書URLを取得する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - 経費ID
///
/// # 戻り値
/// 領収書URL（存在する場合）、または失敗時はエラー
pub fn get_receipt_url(conn: &Connection, id: i64) -> AppResult<Option<String>> {
    conn.query_row(
        "SELECT receipt_url FROM expenses WHERE id = ?1",
        params![id],
        |row| {
            let url: Option<String> = row.get(0)?;
            // 空文字列の場合はNoneとして扱う
            Ok(url.filter(|u| !u.is_empty()))
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::not_found("経費"),
        _ => AppError::Database(e),
    })
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
) -> AppResult<()> {
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
) -> AppResult<Option<ReceiptCache>> {
    match conn.query_row(
        "SELECT id, receipt_url, local_path, cached_at, file_size, last_accessed
         FROM receipt_cache WHERE receipt_url = ?1",
        params![receipt_url],
        |row| {
            Ok(ReceiptCache {
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
        Err(e) => Err(AppError::Database(e)),
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
pub fn update_cache_access_time(conn: &Connection, receipt_url: &str) -> AppResult<()> {
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
pub fn cleanup_old_cache(conn: &Connection, max_age_days: i64) -> AppResult<usize> {
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
pub fn delete_receipt_cache(conn: &Connection, receipt_url: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM receipt_cache WHERE receipt_url = ?1",
        params![receipt_url],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn create_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        // テスト用のテーブルを作成
        conn.execute(
            "CREATE TABLE expenses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                amount REAL NOT NULL,
                category TEXT NOT NULL,
                description TEXT,
                receipt_url TEXT CHECK(receipt_url IS NULL OR receipt_url LIKE 'https://%'),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE receipt_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                receipt_url TEXT NOT NULL UNIQUE,
                local_path TEXT NOT NULL,
                cached_at TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                last_accessed TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_expense_crud_operations() {
        let conn = create_test_db();

        // 経費作成のテスト
        let dto = CreateExpenseDto {
            date: "2024-01-01".to_string(),
            amount: 1000.0,
            category: "食費".to_string(),
            description: Some("テスト経費".to_string()),
        };

        let expense = create(&conn, dto).unwrap();
        assert_eq!(expense.amount, 1000.0);
        assert_eq!(expense.category, "食費");

        // 経費取得のテスト
        let retrieved = find_by_id(&conn, expense.id).unwrap();
        assert_eq!(retrieved.id, expense.id);
        assert_eq!(retrieved.amount, 1000.0);

        // 経費更新のテスト
        let update_dto = UpdateExpenseDto {
            date: None,
            amount: Some(1500.0),
            category: None,
            description: Some("更新されたテスト経費".to_string()),
        };

        let updated = update(&conn, expense.id, update_dto).unwrap();
        assert_eq!(updated.amount, 1500.0);
        assert_eq!(
            updated.description,
            Some("更新されたテスト経費".to_string())
        );

        // 経費削除のテスト
        delete(&conn, expense.id).unwrap();
        assert!(find_by_id(&conn, expense.id).is_err());
    }

    #[test]
    fn test_receipt_url_operations() {
        let conn = create_test_db();

        // 経費を作成
        let dto = CreateExpenseDto {
            date: "2024-01-01".to_string(),
            amount: 1000.0,
            category: "食費".to_string(),
            description: None,
        };

        let expense = create(&conn, dto).unwrap();

        // receipt_urlの設定テスト
        let receipt_url = "https://example.com/receipt.pdf".to_string();
        let updated = set_receipt_url(&conn, expense.id, receipt_url.clone()).unwrap();
        assert_eq!(updated.receipt_url, Some(receipt_url.clone()));

        // receipt_urlの取得テスト
        let retrieved_url = get_receipt_url(&conn, expense.id).unwrap();
        assert_eq!(retrieved_url, Some(receipt_url));

        // receipt_urlの削除テスト（空文字列設定）
        let cleared = set_receipt_url(&conn, expense.id, "".to_string()).unwrap();
        assert_eq!(cleared.receipt_url, None);
    }

    #[test]
    fn test_receipt_url_validation() {
        let conn = create_test_db();

        // 経費を作成
        let dto = CreateExpenseDto {
            date: "2024-01-01".to_string(),
            amount: 1000.0,
            category: "食費".to_string(),
            description: None,
        };

        let expense = create(&conn, dto).unwrap();

        // 有効なHTTPS URLのテスト
        let valid_url = "https://example.com/receipt.pdf".to_string();
        assert!(set_receipt_url(&conn, expense.id, valid_url).is_ok());

        // 無効なURL（HTTP）のテスト
        let invalid_url = "http://example.com/receipt.pdf".to_string();
        let result = set_receipt_url(&conn, expense.id, invalid_url);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn test_receipt_cache_operations() {
        let conn = create_test_db();

        let receipt_url = "https://example.com/receipt.pdf";
        let local_path = "/tmp/cached_receipt.pdf";
        let file_size = 1024;

        // キャッシュ保存のテスト
        save_receipt_cache(&conn, receipt_url, local_path, file_size).unwrap();

        // キャッシュ取得のテスト
        let cache = get_receipt_cache(&conn, receipt_url).unwrap();
        assert!(cache.is_some());
        let cache = cache.unwrap();
        assert_eq!(cache.receipt_url, receipt_url);
        assert_eq!(cache.local_path, local_path);
        assert_eq!(cache.file_size, file_size);

        // アクセス時刻更新のテスト
        update_cache_access_time(&conn, receipt_url).unwrap();

        // キャッシュ削除のテスト
        delete_receipt_cache(&conn, receipt_url).unwrap();
        let cache_after_delete = get_receipt_cache(&conn, receipt_url).unwrap();
        assert!(cache_after_delete.is_none());
    }

    #[test]
    fn test_expense_filtering() {
        let conn = create_test_db();

        // テスト用の経費を複数作成
        let expenses = vec![
            CreateExpenseDto {
                date: "2024-01-15".to_string(),
                amount: 1000.0,
                category: "食費".to_string(),
                description: Some("1月の食費".to_string()),
            },
            CreateExpenseDto {
                date: "2024-02-10".to_string(),
                amount: 2000.0,
                category: "交通費".to_string(),
                description: Some("2月の交通費".to_string()),
            },
            CreateExpenseDto {
                date: "2024-01-20".to_string(),
                amount: 1500.0,
                category: "食費".to_string(),
                description: Some("1月の食費2".to_string()),
            },
        ];

        for dto in expenses {
            create(&conn, dto).unwrap();
        }

        // 月フィルターのテスト
        let jan_expenses = find_all(&conn, Some("2024-01"), None).unwrap();
        assert_eq!(jan_expenses.len(), 2);

        // カテゴリフィルターのテスト
        let food_expenses = find_all(&conn, None, Some("食費")).unwrap();
        assert_eq!(food_expenses.len(), 2);

        // 月とカテゴリの組み合わせフィルターのテスト
        let jan_food_expenses = find_all(&conn, Some("2024-01"), Some("食費")).unwrap();
        assert_eq!(jan_food_expenses.len(), 2);

        // フィルターなしのテスト
        let all_expenses = find_all(&conn, None, None).unwrap();
        assert_eq!(all_expenses.len(), 3);
    }

    #[test]
    fn test_not_found_errors() {
        let conn = create_test_db();

        // 存在しない経費の取得テスト
        let result = find_by_id(&conn, 999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));

        // 存在しない経費の更新テスト
        let update_dto = UpdateExpenseDto {
            date: None,
            amount: Some(1500.0),
            category: None,
            description: None,
        };
        let result = update(&conn, 999, update_dto);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));

        // 存在しない経費の削除テスト
        let result = delete(&conn, 999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }
}
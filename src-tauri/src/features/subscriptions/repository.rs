use super::models::{CreateSubscriptionDto, Subscription, UpdateSubscriptionDto};
use crate::shared::errors::AppError;
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use rusqlite::{params, Connection};

/// サブスクリプションを作成する
///
/// # 引数
/// * `conn` - データベース接続
/// * `dto` - サブスクリプション作成用DTO
///
/// # 戻り値
/// 作成されたサブスクリプション、または失敗時はエラー
pub fn create(conn: &Connection, dto: CreateSubscriptionDto) -> Result<Subscription, AppError> {
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

    conn.execute(
        "INSERT INTO subscriptions (name, amount, billing_cycle, start_date, category, is_active, receipt_path, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, NULL, ?6, ?7)",
        params![dto.name, dto.amount, dto.billing_cycle, dto.start_date, dto.category, now, now],
    )?;

    let id = conn.last_insert_rowid();
    find_by_id(conn, id)
}

/// IDでサブスクリプションを取得する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - サブスクリプションID
///
/// # 戻り値
/// サブスクリプション、または失敗時はエラー
pub fn find_by_id(conn: &Connection, id: i64) -> Result<Subscription, AppError> {
    conn.query_row(
        "SELECT id, name, amount, billing_cycle, start_date, category, is_active, receipt_path, created_at, updated_at
         FROM subscriptions WHERE id = ?1",
        params![id],
        |row| {
            Ok(Subscription {
                id: row.get(0)?,
                name: row.get(1)?,
                amount: row.get(2)?,
                billing_cycle: row.get(3)?,
                start_date: row.get(4)?,
                category: row.get(5)?,
                is_active: row.get::<_, i64>(6)? != 0,
                receipt_path: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("ID {id} のサブスクリプションが見つかりません"))
        }
        _ => AppError::Database(e.to_string()),
    })
}

/// サブスクリプション一覧を取得する
///
/// # 引数
/// * `conn` - データベース接続
/// * `active_only` - アクティブなサブスクリプションのみを取得するか
///
/// # 戻り値
/// サブスクリプションのリスト、または失敗時はエラー
pub fn find_all(conn: &Connection, active_only: bool) -> Result<Vec<Subscription>, AppError> {
    let query = if active_only {
        "SELECT id, name, amount, billing_cycle, start_date, category, is_active, receipt_path, created_at, updated_at
         FROM subscriptions WHERE is_active = 1 ORDER BY name"
    } else {
        "SELECT id, name, amount, billing_cycle, start_date, category, is_active, receipt_path, created_at, updated_at
         FROM subscriptions ORDER BY name"
    };

    let mut stmt = conn.prepare(query)?;
    let subscriptions = stmt.query_map([], |row| {
        Ok(Subscription {
            id: row.get(0)?,
            name: row.get(1)?,
            amount: row.get(2)?,
            billing_cycle: row.get(3)?,
            start_date: row.get(4)?,
            category: row.get(5)?,
            is_active: row.get::<_, i64>(6)? != 0,
            receipt_path: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?;

    subscriptions
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Database(e.to_string()))
}

/// サブスクリプションを更新する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - サブスクリプションID
/// * `dto` - サブスクリプション更新用DTO
///
/// # 戻り値
/// 更新されたサブスクリプション、または失敗時はエラー
pub fn update(
    conn: &Connection,
    id: i64,
    dto: UpdateSubscriptionDto,
) -> Result<Subscription, AppError> {
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

    // 既存のサブスクリプションを取得
    let existing = find_by_id(conn, id)?;

    // 更新するフィールドを決定
    let name = dto.name.unwrap_or(existing.name);
    let amount = dto.amount.unwrap_or(existing.amount);
    let billing_cycle = dto.billing_cycle.unwrap_or(existing.billing_cycle);
    let start_date = dto.start_date.unwrap_or(existing.start_date);
    let category = dto.category.unwrap_or(existing.category);

    conn.execute(
        "UPDATE subscriptions 
         SET name = ?1, amount = ?2, billing_cycle = ?3, start_date = ?4, category = ?5, updated_at = ?6
         WHERE id = ?7",
        params![name, amount, billing_cycle, start_date, category, now, id],
    )?;

    find_by_id(conn, id)
}

/// サブスクリプションのアクティブ状態を切り替える
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - サブスクリプションID
///
/// # 戻り値
/// 更新されたサブスクリプション、または失敗時はエラー
pub fn toggle_status(conn: &Connection, id: i64) -> Result<Subscription, AppError> {
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

    conn.execute(
        "UPDATE subscriptions SET is_active = NOT is_active, updated_at = ?1 WHERE id = ?2",
        params![now, id],
    )?;

    find_by_id(conn, id)
}

/// サブスクリプションを削除する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - サブスクリプションID
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
#[allow(dead_code)]
pub fn delete(conn: &Connection, id: i64) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM subscriptions WHERE id = ?1", params![id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "ID {id} のサブスクリプションが見つかりません"
        )));
    }

    Ok(())
}

/// アクティブなサブスクリプションの月額合計を計算する
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// 月額合計金額、または失敗時はエラー
pub fn calculate_monthly_total(conn: &Connection) -> Result<f64, AppError> {
    let subscriptions = find_all(conn, true)?;

    let total = subscriptions.iter().fold(0.0, |acc, sub| {
        let monthly_amount = match sub.billing_cycle.as_str() {
            "monthly" => sub.amount,
            "annual" => sub.amount / 12.0,
            _ => 0.0,
        };
        acc + monthly_amount
    });

    Ok(total)
}

/// サブスクリプションの領収書パスを設定する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - サブスクリプションID
/// * `receipt_path` - 領収書ファイルパス（空文字列の場合はNULLに設定）
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
pub fn set_receipt_path(conn: &Connection, id: i64, receipt_path: String) -> Result<(), AppError> {
    // JSTで現在時刻を取得
    let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

    // 空文字列の場合はNULLに設定
    let path_value = if receipt_path.is_empty() {
        None
    } else {
        Some(receipt_path)
    };

    let rows_affected = conn.execute(
        "UPDATE subscriptions SET receipt_path = ?1, updated_at = ?2 WHERE id = ?3",
        params![path_value, now, id],
    )?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "ID {id} のサブスクリプションが見つかりません"
        )));
    }

    Ok(())
}

/// サブスクリプションの領収書パスを取得する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - サブスクリプションID
///
/// # 戻り値
/// 領収書パス（存在する場合）、または失敗時はエラー
pub fn get_receipt_path(conn: &Connection, id: i64) -> Result<Option<String>, AppError> {
    conn.query_row(
        "SELECT receipt_path FROM subscriptions WHERE id = ?1",
        params![id],
        |row| {
            let path: Option<String> = row.get(0)?;
            // 空文字列の場合はNoneとして扱う
            Ok(path.filter(|p| !p.is_empty()))
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("ID {id} のサブスクリプションが見つかりません"))
        }
        _ => AppError::Database(e.to_string()),
    })
}

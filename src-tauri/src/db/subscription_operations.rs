use crate::models::{Subscription, CreateSubscriptionDto, UpdateSubscriptionDto};
use chrono::Utc;
use rusqlite::{Connection, Result, params};

/// サブスクリプションを作成する
///
/// # 引数
/// * `conn` - データベース接続
/// * `dto` - サブスクリプション作成用DTO
///
/// # 戻り値
/// 作成されたサブスクリプション、または失敗時はエラー
pub fn create_subscription(conn: &Connection, dto: CreateSubscriptionDto) -> Result<Subscription> {
    let now = Utc::now().to_rfc3339();
    
    conn.execute(
        "INSERT INTO subscriptions (name, amount, billing_cycle, start_date, category, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)",
        params![dto.name, dto.amount, dto.billing_cycle, dto.start_date, dto.category, now, now],
    )?;

    let id = conn.last_insert_rowid();
    get_subscription_by_id(conn, id)
}

/// IDでサブスクリプションを取得する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - サブスクリプションID
///
/// # 戻り値
/// サブスクリプション、または失敗時はエラー
pub fn get_subscription_by_id(conn: &Connection, id: i64) -> Result<Subscription> {
    conn.query_row(
        "SELECT id, name, amount, billing_cycle, start_date, category, is_active, created_at, updated_at
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
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )
}

/// サブスクリプション一覧を取得する
///
/// # 引数
/// * `conn` - データベース接続
/// * `active_only` - アクティブなサブスクリプションのみを取得するか
///
/// # 戻り値
/// サブスクリプションのリスト、または失敗時はエラー
pub fn get_subscriptions(conn: &Connection, active_only: bool) -> Result<Vec<Subscription>> {
    let query = if active_only {
        "SELECT id, name, amount, billing_cycle, start_date, category, is_active, created_at, updated_at
         FROM subscriptions WHERE is_active = 1 ORDER BY name"
    } else {
        "SELECT id, name, amount, billing_cycle, start_date, category, is_active, created_at, updated_at
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
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?;
    
    subscriptions.collect()
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
pub fn update_subscription(
    conn: &Connection,
    id: i64,
    dto: UpdateSubscriptionDto,
) -> Result<Subscription> {
    let now = Utc::now().to_rfc3339();
    
    // 既存のサブスクリプションを取得
    let existing = get_subscription_by_id(conn, id)?;
    
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
    
    get_subscription_by_id(conn, id)
}

/// サブスクリプションのアクティブ状態を切り替える
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - サブスクリプションID
///
/// # 戻り値
/// 更新されたサブスクリプション、または失敗時はエラー
pub fn toggle_subscription_status(conn: &Connection, id: i64) -> Result<Subscription> {
    let now = Utc::now().to_rfc3339();
    
    conn.execute(
        "UPDATE subscriptions SET is_active = NOT is_active, updated_at = ?1 WHERE id = ?2",
        params![now, id],
    )?;
    
    get_subscription_by_id(conn, id)
}

/// サブスクリプションを削除する
///
/// # 引数
/// * `conn` - データベース接続
/// * `id` - サブスクリプションID
///
/// # 戻り値
/// 成功時はOk(())、失敗時はエラー
pub fn delete_subscription(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM subscriptions WHERE id = ?1", params![id])?;
    Ok(())
}

/// アクティブなサブスクリプションの月額合計を計算する
///
/// # 引数
/// * `conn` - データベース接続
///
/// # 戻り値
/// 月額合計金額、または失敗時はエラー
pub fn get_monthly_subscription_total(conn: &Connection) -> Result<f64> {
    let subscriptions = get_subscriptions(conn, true)?;
    
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

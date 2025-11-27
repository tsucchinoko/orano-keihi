use serde::{Deserialize, Serialize};

/// サブスクリプションデータモデル
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subscription {
    pub id: i64,
    pub name: String,
    pub amount: f64,
    pub billing_cycle: String,
    pub start_date: String,
    pub category: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// サブスクリプション作成用DTO
#[derive(Debug, Deserialize)]
pub struct CreateSubscriptionDto {
    pub name: String,
    pub amount: f64,
    pub billing_cycle: String,
    pub start_date: String,
    pub category: String,
}

/// サブスクリプション更新用DTO
#[derive(Debug, Deserialize)]
pub struct UpdateSubscriptionDto {
    pub name: Option<String>,
    pub amount: Option<f64>,
    pub billing_cycle: Option<String>,
    pub start_date: Option<String>,
    pub category: Option<String>,
}

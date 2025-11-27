use serde::{Deserialize, Serialize};

/// 経費データモデル
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Expense {
    pub id: i64,
    pub date: String,
    pub amount: f64,
    pub category: String,
    pub description: Option<String>,
    pub receipt_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 経費作成用DTO
#[derive(Debug, Deserialize)]
pub struct CreateExpenseDto {
    pub date: String,
    pub amount: f64,
    pub category: String,
    pub description: Option<String>,
}

/// 経費更新用DTO
#[derive(Debug, Deserialize)]
pub struct UpdateExpenseDto {
    pub date: Option<String>,
    pub amount: Option<f64>,
    pub category: Option<String>,
    pub description: Option<String>,
}

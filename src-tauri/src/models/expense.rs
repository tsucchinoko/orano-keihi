use serde::{Deserialize, Serialize};

/// 経費データモデル
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Expense {
    pub id: i64,
    pub date: String,
    pub amount: f64,
    pub category: String,
    pub description: Option<String>,
    pub receipt_url: Option<String>, // receipt_pathからreceipt_urlに変更
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
/// 領収書キャッシュデータモデル
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReceiptCache {
    pub id: i64,
    pub receipt_url: String,
    pub local_path: String,
    pub cached_at: String,
    pub file_size: i64,
    pub last_accessed: String,
}

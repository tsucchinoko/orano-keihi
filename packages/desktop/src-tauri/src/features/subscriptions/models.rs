use serde::{Deserialize, Serialize};

/// サブスクリプションデータモデル
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subscription {
    pub id: i64,
    pub name: String,                 // サービス名、100文字以内
    pub amount: f64,                  // 正の数値、10桁以内
    pub billing_cycle: String,        // "monthly" または "annual"
    pub start_date: String,           // YYYY-MM-DD形式
    pub category: String,             // カテゴリ名（後方互換性のため残す）
    pub category_id: Option<i64>,     // カテゴリーID（categoriesテーブルへの外部キー）
    pub is_active: bool,              // 有効/無効
    pub receipt_path: Option<String>, // 領収書パス（将来的にreceipt_urlに移行）
    pub created_at: String,           // RFC3339形式（JST）
    pub updated_at: String,           // RFC3339形式（JST）
}

/// サブスクリプション作成用DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSubscriptionDto {
    pub name: String,
    pub amount: f64,
    pub billing_cycle: String,
    pub start_date: String,
    pub category: String,
    pub category_id: Option<i64>, // カテゴリーID（推奨）
}

/// サブスクリプション更新用DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSubscriptionDto {
    pub name: Option<String>,
    pub amount: Option<f64>,
    pub billing_cycle: Option<String>,
    pub start_date: Option<String>,
    pub category: Option<String>,
    pub category_id: Option<i64>, // カテゴリーID（推奨）
    pub receipt_path: Option<String>,
}

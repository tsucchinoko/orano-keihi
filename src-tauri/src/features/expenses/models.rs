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
    /// ユーザーID（認証後に設定される）
    pub user_id: Option<i64>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expense_serialization() {
        // 経費データのシリアライゼーションテスト
        let expense = Expense {
            id: 1,
            date: "2024-01-01".to_string(),
            amount: 1000.0,
            category: "食費".to_string(),
            description: Some("テスト経費".to_string()),
            receipt_url: Some("https://example.com/receipt.pdf".to_string()),
            created_at: "2024-01-01T00:00:00+09:00".to_string(),
            updated_at: "2024-01-01T00:00:00+09:00".to_string(),
        };

        // JSONシリアライゼーション
        let json = serde_json::to_string(&expense).unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"amount\":1000.0"));
        assert!(json.contains("\"category\":\"食費\""));

        // JSONデシリアライゼーション
        let deserialized: Expense = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, expense.id);
        assert_eq!(deserialized.amount, expense.amount);
        assert_eq!(deserialized.category, expense.category);
    }

    #[test]
    fn test_create_expense_dto_deserialization() {
        // 経費作成DTOのデシリアライゼーションテスト
        let json = r#"{
            "date": "2024-01-01",
            "amount": 1500.0,
            "category": "交通費",
            "description": "電車代"
        }"#;

        let dto: CreateExpenseDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.date, "2024-01-01");
        assert_eq!(dto.amount, 1500.0);
        assert_eq!(dto.category, "交通費");
        assert_eq!(dto.description, Some("電車代".to_string()));
    }

    #[test]
    fn test_create_expense_dto_without_description() {
        // 説明なしの経費作成DTOテスト
        let json = r#"{
            "date": "2024-01-01",
            "amount": 1500.0,
            "category": "交通費"
        }"#;

        let dto: CreateExpenseDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.description, None);
    }

    #[test]
    fn test_update_expense_dto_partial() {
        // 部分更新DTOのテスト
        let json = r#"{
            "amount": 2000.0,
            "description": "更新された説明"
        }"#;

        let dto: UpdateExpenseDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.date, None);
        assert_eq!(dto.amount, Some(2000.0));
        assert_eq!(dto.category, None);
        assert_eq!(dto.description, Some("更新された説明".to_string()));
    }

    #[test]
    fn test_receipt_cache_model() {
        // 領収書キャッシュモデルのテスト
        let cache = ReceiptCache {
            id: 1,
            receipt_url: "https://example.com/receipt.pdf".to_string(),
            local_path: "/tmp/cached_receipt.pdf".to_string(),
            cached_at: "2024-01-01T00:00:00+09:00".to_string(),
            file_size: 1024,
            last_accessed: "2024-01-01T00:00:00+09:00".to_string(),
        };

        // シリアライゼーション
        let json = serde_json::to_string(&cache).unwrap();
        assert!(json.contains("\"receipt_url\":\"https://example.com/receipt.pdf\""));
        assert!(json.contains("\"file_size\":1024"));

        // デシリアライゼーション
        let deserialized: ReceiptCache = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, cache.id);
        assert_eq!(deserialized.receipt_url, cache.receipt_url);
        assert_eq!(deserialized.file_size, cache.file_size);
    }
}

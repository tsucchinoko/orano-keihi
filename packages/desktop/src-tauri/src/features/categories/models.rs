use serde::{Deserialize, Serialize};

/// カテゴリーデータモデル
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub display_order: i64,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// カテゴリー一覧レスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoriesResponse {
    pub success: bool,
    pub categories: Vec<Category>,
    pub count: usize,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_serialization() {
        let category = Category {
            id: 1,
            name: "交通費".to_string(),
            icon: "🚗".to_string(),
            display_order: 1,
            is_active: true,
            created_at: "2024-01-01T00:00:00+09:00".to_string(),
            updated_at: "2024-01-01T00:00:00+09:00".to_string(),
        };

        let json = serde_json::to_string(&category).unwrap();
        assert!(json.contains("\"name\":\"交通費\""));
        assert!(json.contains("\"icon\":\"🚗\""));
        assert!(json.contains("\"is_active\":true"));

        let deserialized: Category = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, category.id);
        assert_eq!(deserialized.name, category.name);
        assert_eq!(deserialized.is_active, category.is_active);
    }

    #[test]
    fn test_categories_response() {
        let response = CategoriesResponse {
            success: true,
            categories: vec![Category {
                id: 1,
                name: "交通費".to_string(),
                icon: "🚗".to_string(),
                display_order: 1,
                is_active: true,
                created_at: "2024-01-01T00:00:00+09:00".to_string(),
                updated_at: "2024-01-01T00:00:00+09:00".to_string(),
            }],
            count: 1,
            timestamp: "2024-01-01T00:00:00+09:00".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"count\":1"));
    }
}

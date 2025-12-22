// 領収書機能のデータモデル

use serde::{Deserialize, Serialize};

/// 領収書キャッシュデータモデル
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReceiptCache {
    pub id: i64,
    pub receipt_url: String,    // R2のHTTPS URL
    pub local_path: String,     // ローカルキャッシュファイルのパス
    pub cached_at: String,      // キャッシュ作成日時（RFC3339形式、JST）
    pub file_size: i64,         // ファイルサイズ（バイト）
    pub last_accessed: String,  // 最終アクセス日時（RFC3339形式、JST）
}

/// 複数ファイルアップロード用の入力構造体
#[derive(Debug, Clone, Deserialize)]
pub struct MultipleFileUploadInput {
    pub expense_id: i64,
    pub file_path: String,
}

/// 複数ファイルアップロード結果の構造体
#[derive(Debug, Clone, Serialize)]
pub struct MultipleUploadResult {
    pub total_files: usize,
    pub successful_uploads: usize,
    pub failed_uploads: usize,
    pub results: Vec<SingleUploadResult>,
    pub total_duration_ms: u64,
}

/// 単一アップロード結果の構造体
#[derive(Debug, Clone, Serialize)]
pub struct SingleUploadResult {
    pub expense_id: i64,
    pub success: bool,
    pub url: Option<String>,
    pub error: Option<String>,
    pub file_size: u64,
    pub duration_ms: u64,
}

/// キャッシュ統計情報の構造体
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub max_size_bytes: u64,
    pub cache_hit_rate: f64,
}

/// R2接続テスト結果
#[derive(Debug, Clone, Serialize)]
pub struct R2ConnectionTestResult {
    pub overall_success: bool,
    pub config_validation: TestStepResult,
    pub client_initialization: TestStepResult,
    pub bucket_access: TestStepResult,
    pub upload_test: TestStepResult,
    pub download_test: TestStepResult,
    pub delete_test: TestStepResult,
    pub performance_metrics: Option<PerformanceStats>,
    pub total_duration_ms: u64,
    pub environment: String,
}

/// テストステップ結果
#[derive(Debug, Clone, Serialize)]
pub struct TestStepResult {
    pub success: bool,
    pub message: String,
    pub duration_ms: u64,
    pub details: Option<String>,
}

impl Default for TestStepResult {
    fn default() -> Self {
        Self {
            success: false,
            message: "未実行".to_string(),
            duration_ms: 0,
            details: None,
        }
    }
}

/// パフォーマンス統計の構造体
#[derive(Debug, Clone, Serialize)]
pub struct PerformanceStats {
    pub latency_ms: u64,
    pub throughput_bps: u64,
    pub connection_status: String,
    pub last_measured: String,
}

/// R2使用量情報
#[derive(Debug, Clone, Serialize)]
pub struct R2UsageInfo {
    pub total_files: u64,
    pub estimated_storage_mb: u64,
    pub monthly_uploads: u64,
    pub daily_uploads: u64,
    pub cache_stats: Option<CacheStats>,
    pub bucket_name: String,
    pub region: String,
    pub last_updated: String,
    pub cost_estimate_usd: f64,
}

/// R2デバッグ情報
#[derive(Debug, Clone, Serialize)]
pub struct R2DebugInfo {
    pub environment_variables: std::collections::HashMap<String, String>,
    pub r2_config: Option<std::collections::HashMap<String, String>>,
    pub system_info: std::collections::HashMap<String, String>,
    pub database_stats: std::collections::HashMap<String, String>,
    pub recent_errors: Vec<String>,
    pub timestamp: String,
}

/// アップロードプログレスの構造体
#[derive(Debug, Clone, Serialize)]
pub struct UploadProgress {
    pub file_index: usize,
    pub file_key: String,
    pub status: UploadStatus,
    pub bytes_uploaded: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
}

/// アップロードステータス
#[derive(Debug, Clone, Serialize)]
pub enum UploadStatus {
    Started,
    Completed,
    Failed,
}

/// アップロード結果の構造体（R2Client用）
#[derive(Debug, Clone, Serialize)]
pub struct UploadResult {
    pub file_key: String,
    pub success: bool,
    pub url: Option<String>,
    pub error: Option<String>,
    pub file_size: u64,
    #[serde(serialize_with = "serialize_duration")]
    pub duration: std::time::Duration,
}

/// Duration を milliseconds として serialize する
fn serialize_duration<S>(duration: &std::time::Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(duration.as_millis() as u64)
}

/// 複数ファイルアップロード用の構造体（R2Client用）
#[derive(Debug, Clone)]
pub struct MultipleFileUpload {
    pub file_key: String,
    pub file_data: Vec<u8>,
    pub content_type: String,
    pub expense_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receipt_cache_model() {
        // 領収書キャッシュモデルのテスト
        let cache = ReceiptCache {
            id: 1,
            receipt_url: "https://example.com/receipt.pdf".to_string(),
            local_path: "/path/to/cache/receipt_123.pdf".to_string(),
            cached_at: "2024-01-01T12:00:00+09:00".to_string(),
            file_size: 1024,
            last_accessed: "2024-01-01T12:00:00+09:00".to_string(),
        };

        // シリアライゼーション
        let json = serde_json::to_string(&cache).unwrap();
        assert!(json.contains("receipt.pdf"));

        // デシリアライゼーション
        let deserialized: ReceiptCache = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, cache.id);
        assert_eq!(deserialized.receipt_url, cache.receipt_url);
        assert_eq!(deserialized.local_path, cache.local_path);
        assert_eq!(deserialized.file_size, cache.file_size);
    }

    #[test]
    fn test_multiple_upload_result_model() {
        // 複数アップロード結果モデルのテスト
        let result = MultipleUploadResult {
            total_files: 2,
            successful_uploads: 1,
            failed_uploads: 1,
            results: vec![
                SingleUploadResult {
                    expense_id: 1,
                    success: true,
                    url: Some("https://example.com/receipt1.pdf".to_string()),
                    error: None,
                    file_size: 1024,
                    duration_ms: 500,
                },
                SingleUploadResult {
                    expense_id: 2,
                    success: false,
                    url: None,
                    error: Some("アップロードエラー".to_string()),
                    file_size: 2048,
                    duration_ms: 300,
                },
            ],
            total_duration_ms: 800,
        };

        // シリアライゼーション
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("successful_uploads"));

        // デシリアライゼーション
        let deserialized: MultipleUploadResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_files, 2);
        assert_eq!(deserialized.successful_uploads, 1);
        assert_eq!(deserialized.failed_uploads, 1);
        assert_eq!(deserialized.results.len(), 2);
    }

    #[test]
    fn test_cache_stats_model() {
        // キャッシュ統計モデルのテスト
        let stats = CacheStats {
            total_files: 10,
            total_size_bytes: 1024 * 1024, // 1MB
            max_size_bytes: 100 * 1024 * 1024, // 100MB
            cache_hit_rate: 0.85,
        };

        // シリアライゼーション
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("total_files"));

        // デシリアライゼーション
        let deserialized: CacheStats = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_files, 10);
        assert_eq!(deserialized.total_size_bytes, 1024 * 1024);
        assert_eq!(deserialized.cache_hit_rate, 0.85);
    }

    #[test]
    fn test_upload_status_enum() {
        // アップロードステータス列挙型のテスト
        let statuses = vec![
            UploadStatus::Started,
            UploadStatus::Completed,
            UploadStatus::Failed,
        ];

        for status in statuses {
            // シリアライゼーション
            let json = serde_json::to_string(&status).unwrap();
            assert!(!json.is_empty());

            // デシリアライゼーション
            let _deserialized: UploadStatus = serde_json::from_str(&json).unwrap();
        }
    }
}
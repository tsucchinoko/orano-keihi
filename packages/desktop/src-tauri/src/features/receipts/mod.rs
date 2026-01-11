// 領収書機能モジュール

pub mod api_client;
pub mod api_commands;
pub mod auth_commands;
pub mod cache;
pub mod commands;
pub mod models;
pub mod user_path_manager;

// 公開インターフェース

// モデル
pub use models::{
    CacheStats, MultipleFileUpload, MultipleFileUploadInput, MultipleUploadResult,
    PerformanceStats, R2ConnectionTestResult, R2DebugInfo, R2UsageInfo, ReceiptCache,
    SingleUploadResult, TestStepResult, UploadProgress, UploadResult, UploadStatus,
};

// ユーザーパス管理
pub use user_path_manager::UserPathManager;

// キャッシュマネージャー
pub use cache::CacheManager;

// APIクライアント
pub use api_client::{
    ApiClient, ApiClientConfig, ErrorResponse, MultipleUploadResponse, UploadResponse,
};

// APIコマンド（APIサーバー経由）
pub use api_commands::{
    check_api_server_health, get_receipt_via_api, upload_multiple_receipts_via_api,
    upload_receipt_via_api,
};

// コマンド（Tauriコマンドハンドラー）
pub use commands::{get_cache_stats, get_receipt_offline, sync_cache_on_online};

/// 領収書機能の初期化とセットアップ
pub fn initialize() {
    log::info!("領収書機能モジュールを初期化しています...");

    // 必要に応じて初期化処理を追加
    // 例：キャッシュディレクトリの作成、設定の検証など

    log::info!("領収書機能モジュールの初期化が完了しました");
}

/// 領収書機能の統計情報を取得
pub fn get_feature_stats() -> std::collections::HashMap<String, String> {
    let mut stats = std::collections::HashMap::new();

    stats.insert("feature_name".to_string(), "receipts".to_string());
    stats.insert("version".to_string(), "1.0.0".to_string());
    stats.insert("status".to_string(), "active".to_string());

    // 利用可能なコマンド数（既存9 + 新規5 + API経由3 = 17）
    stats.insert("available_commands".to_string(), "17".to_string());

    // サポートされるファイル形式
    stats.insert(
        "supported_formats".to_string(),
        "PNG,JPG,JPEG,PDF".to_string(),
    );

    // 最大ファイルサイズ（MB）
    stats.insert("max_file_size_mb".to_string(), "10".to_string());

    // デフォルトキャッシュサイズ（MB）
    stats.insert("default_cache_size_mb".to_string(), "100".to_string());

    log::debug!("領収書機能の統計情報: {stats:?}");

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // モジュールのエクスポートが正しく機能することを確認

        // モデルのテスト
        let _cache_stats: Option<CacheStats> = None;
        let _receipt_cache: Option<ReceiptCache> = None;
        let _upload_result: Option<UploadResult> = None;
        let _multiple_upload_input: Option<MultipleFileUploadInput> = None;
        let _multiple_upload_result: Option<MultipleUploadResult> = None;
        let _single_upload_result: Option<SingleUploadResult> = None;
        let _performance_stats: Option<PerformanceStats> = None;
        let _test_step_result: Option<TestStepResult> = None;
        let _r2_connection_test_result: Option<R2ConnectionTestResult> = None;
        let _r2_usage_info: Option<R2UsageInfo> = None;
        let _r2_debug_info: Option<R2DebugInfo> = None;
        let _upload_progress: Option<UploadProgress> = None;
        let _upload_status: Option<UploadStatus> = None;
        let _multiple_file_upload: Option<MultipleFileUpload> = None;

        // この時点でコンパイルが通れば、エクスポートは正しく機能している
    }

    #[test]
    fn test_feature_stats() {
        let stats = get_feature_stats();

        assert_eq!(stats.get("feature_name"), Some(&"receipts".to_string()));
        assert_eq!(stats.get("version"), Some(&"1.0.0".to_string()));
        assert_eq!(stats.get("status"), Some(&"active".to_string()));
        assert_eq!(stats.get("available_commands"), Some(&"17".to_string()));
        assert_eq!(
            stats.get("supported_formats"),
            Some(&"PNG,JPG,JPEG,PDF".to_string())
        );
        assert_eq!(stats.get("max_file_size_mb"), Some(&"10".to_string()));
        assert_eq!(stats.get("default_cache_size_mb"), Some(&"100".to_string()));
    }

    #[test]
    fn test_initialize() {
        // 初期化関数が正常に実行されることを確認
        initialize();
        // パニックしなければ成功
    }
}

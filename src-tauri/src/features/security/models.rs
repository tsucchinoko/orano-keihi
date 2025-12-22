// セキュリティ機能のデータモデル

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// システム診断情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    /// 環境名（development, production等）
    pub environment: String,
    /// デバッグモードが有効かどうか
    pub debug_mode: bool,
    /// ログレベル
    pub log_level: String,
    /// マスクされた認証情報
    pub credentials: HashMap<String, String>,
    /// システム情報
    pub system_info: SystemInfo,
    /// セキュリティ設定の検証結果
    pub validation_status: ValidationStatus,
}

/// システム情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Rustバージョン
    pub rust_version: String,
    /// ターゲットアーキテクチャ
    pub target_arch: String,
    /// ターゲットOS
    pub target_os: String,
    /// アプリケーションバージョン
    pub app_version: String,
}

/// 設定検証結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// 検証が成功したかどうか
    pub is_valid: bool,
    /// 検証メッセージ
    pub message: String,
    /// 検証された項目の詳細
    pub details: Vec<ValidationDetail>,
}

/// 検証詳細
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationDetail {
    /// 検証項目名
    pub item: String,
    /// 検証結果
    pub status: ValidationStatus,
    /// 詳細メッセージ
    pub message: String,
}

/// 検証ステータス
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationStatus {
    /// 成功
    Success,
    /// 警告
    Warning,
    /// エラー
    Error,
    /// 未検証
    NotChecked,
}

/// R2接続テスト結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    /// 接続が成功したかどうか
    pub is_connected: bool,
    /// テスト実行時刻
    pub tested_at: String,
    /// レスポンス時間（ミリ秒）
    pub response_time_ms: Option<u64>,
    /// エラーメッセージ（失敗時）
    pub error_message: Option<String>,
    /// 接続詳細情報
    pub connection_details: ConnectionDetails,
}

/// 接続詳細情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionDetails {
    /// エンドポイントURL（マスク済み）
    pub endpoint: String,
    /// バケット名（マスク済み）
    pub bucket_name: String,
    /// アカウントID（マスク済み）
    pub account_id: String,
    /// 使用されたアクセスキー（マスク済み）
    pub access_key: String,
}

/// セキュリティイベント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// イベントID
    pub id: String,
    /// イベントタイプ
    pub event_type: String,
    /// イベント詳細
    pub details: String,
    /// 発生時刻
    pub timestamp: String,
    /// 重要度
    pub severity: EventSeverity,
    /// 関連するユーザー情報（存在する場合）
    pub user_context: Option<String>,
}

/// イベント重要度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventSeverity {
    /// 情報
    Info,
    /// 警告
    Warning,
    /// エラー
    Error,
    /// 重大
    Critical,
}

/// セキュリティ監査ログ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditLog {
    /// ログエントリのリスト
    pub entries: Vec<SecurityEvent>,
    /// ログの生成時刻
    pub generated_at: String,
    /// ログの期間（開始）
    pub period_start: String,
    /// ログの期間（終了）
    pub period_end: String,
    /// 総イベント数
    pub total_events: usize,
}

/// 環境設定情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    /// 環境名
    pub environment: String,
    /// デバッグモードが有効かどうか
    pub debug_mode: bool,
    /// ログレベル
    pub log_level: String,
    /// 本番環境かどうか
    pub is_production: bool,
    /// 開発環境かどうか
    pub is_development: bool,
    /// 設定の読み込み元
    pub config_source: ConfigSource,
}

/// 設定の読み込み元
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigSource {
    /// コンパイル時埋め込み
    Embedded,
    /// 実行時環境変数
    Environment,
    /// デフォルト値
    Default,
}

impl Default for ValidationStatus {
    fn default() -> Self {
        ValidationStatus::NotChecked
    }
}

impl Default for EventSeverity {
    fn default() -> Self {
        EventSeverity::Info
    }
}

impl Default for ConfigSource {
    fn default() -> Self {
        ConfigSource::Default
    }
}

impl DiagnosticInfo {
    /// 新しい診断情報を作成
    pub fn new(
        environment: String,
        debug_mode: bool,
        log_level: String,
        credentials: HashMap<String, String>,
        system_info: SystemInfo,
    ) -> Self {
        Self {
            environment,
            debug_mode,
            log_level,
            credentials,
            system_info,
            validation_status: ValidationStatus::NotChecked,
        }
    }

    /// 検証ステータスを更新
    pub fn set_validation_status(&mut self, status: ValidationStatus) {
        self.validation_status = status;
    }
}

impl SystemInfo {
    /// 新しいシステム情報を作成
    pub fn new(
        rust_version: String,
        target_arch: String,
        target_os: String,
        app_version: String,
    ) -> Self {
        Self {
            rust_version,
            target_arch,
            target_os,
            app_version,
        }
    }
}

impl ValidationResult {
    /// 成功結果を作成
    pub fn success(message: String, details: Vec<ValidationDetail>) -> Self {
        Self {
            is_valid: true,
            message,
            details,
        }
    }

    /// 失敗結果を作成
    pub fn failure(message: String, details: Vec<ValidationDetail>) -> Self {
        Self {
            is_valid: false,
            message,
            details,
        }
    }
}

impl ValidationDetail {
    /// 新しい検証詳細を作成
    pub fn new(item: String, status: ValidationStatus, message: String) -> Self {
        Self {
            item,
            status,
            message,
        }
    }
}

impl SecurityEvent {
    /// 新しいセキュリティイベントを作成
    pub fn new(
        event_type: String,
        details: String,
        severity: EventSeverity,
        user_context: Option<String>,
    ) -> Self {
        use chrono::Utc;
        use chrono_tz::Asia::Tokyo;
        use uuid::Uuid;

        let now_jst = Utc::now().with_timezone(&Tokyo);
        let timestamp = now_jst.to_rfc3339();

        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            details,
            timestamp,
            severity,
            user_context,
        }
    }
}

impl ConnectionTestResult {
    /// 成功結果を作成
    pub fn success(response_time_ms: u64, connection_details: ConnectionDetails) -> Self {
        use chrono::Utc;
        use chrono_tz::Asia::Tokyo;

        let now_jst = Utc::now().with_timezone(&Tokyo);
        let tested_at = now_jst.to_rfc3339();

        Self {
            is_connected: true,
            tested_at,
            response_time_ms: Some(response_time_ms),
            error_message: None,
            connection_details,
        }
    }

    /// 失敗結果を作成
    pub fn failure(error_message: String, connection_details: ConnectionDetails) -> Self {
        use chrono::Utc;
        use chrono_tz::Asia::Tokyo;

        let now_jst = Utc::now().with_timezone(&Tokyo);
        let tested_at = now_jst.to_rfc3339();

        Self {
            is_connected: false,
            tested_at,
            response_time_ms: None,
            error_message: Some(error_message),
            connection_details,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_info_creation() {
        let mut credentials = HashMap::new();
        credentials.insert("test_key".to_string(), "test****value".to_string());

        let system_info = SystemInfo::new(
            "1.70.0".to_string(),
            "x86_64".to_string(),
            "macos".to_string(),
            "1.0.0".to_string(),
        );

        let diagnostic_info = DiagnosticInfo::new(
            "development".to_string(),
            true,
            "debug".to_string(),
            credentials,
            system_info,
        );

        assert_eq!(diagnostic_info.environment, "development");
        assert!(diagnostic_info.debug_mode);
        assert_eq!(diagnostic_info.log_level, "debug");
        assert_eq!(
            diagnostic_info.validation_status,
            ValidationStatus::NotChecked
        );
    }

    #[test]
    fn test_validation_result_creation() {
        let details = vec![ValidationDetail::new(
            "test_item".to_string(),
            ValidationStatus::Success,
            "テスト成功".to_string(),
        )];

        let success_result = ValidationResult::success("すべて成功".to_string(), details.clone());
        assert!(success_result.is_valid);

        let failure_result = ValidationResult::failure("失敗しました".to_string(), details);
        assert!(!failure_result.is_valid);
    }

    #[test]
    fn test_security_event_creation() {
        let event = SecurityEvent::new(
            "test_event".to_string(),
            "テストイベント".to_string(),
            EventSeverity::Info,
            Some("test_user".to_string()),
        );

        assert_eq!(event.event_type, "test_event");
        assert_eq!(event.details, "テストイベント");
        assert_eq!(event.severity, EventSeverity::Info);
        assert_eq!(event.user_context, Some("test_user".to_string()));
        assert!(!event.id.is_empty());
        assert!(!event.timestamp.is_empty());
    }

    #[test]
    fn test_connection_test_result_success() {
        let connection_details = ConnectionDetails {
            endpoint: "https://****".to_string(),
            bucket_name: "test****bucket".to_string(),
            account_id: "acc****123".to_string(),
            access_key: "key****456".to_string(),
        };

        let result = ConnectionTestResult::success(150, connection_details);
        assert!(result.is_connected);
        assert_eq!(result.response_time_ms, Some(150));
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_connection_test_result_failure() {
        let connection_details = ConnectionDetails {
            endpoint: "https://****".to_string(),
            bucket_name: "test****bucket".to_string(),
            account_id: "acc****123".to_string(),
            access_key: "key****456".to_string(),
        };

        let result = ConnectionTestResult::failure("接続失敗".to_string(), connection_details);
        assert!(!result.is_connected);
        assert!(result.response_time_ms.is_none());
        assert_eq!(result.error_message, Some("接続失敗".to_string()));
    }
}

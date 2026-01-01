//! セキュリティ監査ログ機能
//!
//! R2ユーザーディレクトリ移行におけるセキュリティイベントの
//! 監査ログ記録と分析機能を提供します。

use super::errors::MigrationError;
use super::logging::{LogLevel, StructuredLogEntry, StructuredLogger};
use crate::shared::errors::ErrorSeverity;
use chrono::{DateTime, Utc};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// セキュリティイベントタイプ
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum SecurityEventType {
    /// 認証失敗
    AuthenticationFailure,
    /// 認可失敗
    AuthorizationFailure,
    /// 不正アクセス試行
    UnauthorizedAccess,
    /// データ整合性違反
    DataIntegrityViolation,
    /// 権限昇格試行
    PrivilegeEscalation,
    /// 設定改ざん検出
    ConfigurationTampering,
    /// 異常なファイルアクセス
    AbnormalFileAccess,
    /// セキュリティポリシー違反
    SecurityPolicyViolation,
    /// 監査ログ改ざん試行
    AuditLogTampering,
    /// システム侵入検出
    SystemIntrusion,
}

impl SecurityEventType {
    /// イベントタイプの重要度を取得
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            SecurityEventType::AuthenticationFailure => ErrorSeverity::Medium,
            SecurityEventType::AuthorizationFailure => ErrorSeverity::High,
            SecurityEventType::UnauthorizedAccess => ErrorSeverity::Critical,
            SecurityEventType::DataIntegrityViolation => ErrorSeverity::Critical,
            SecurityEventType::PrivilegeEscalation => ErrorSeverity::Critical,
            SecurityEventType::ConfigurationTampering => ErrorSeverity::Critical,
            SecurityEventType::AbnormalFileAccess => ErrorSeverity::High,
            SecurityEventType::SecurityPolicyViolation => ErrorSeverity::High,
            SecurityEventType::AuditLogTampering => ErrorSeverity::Critical,
            SecurityEventType::SystemIntrusion => ErrorSeverity::Critical,
        }
    }

    /// イベントタイプの説明を取得
    pub fn description(&self) -> &'static str {
        match self {
            SecurityEventType::AuthenticationFailure => "認証に失敗しました",
            SecurityEventType::AuthorizationFailure => "認可に失敗しました",
            SecurityEventType::UnauthorizedAccess => "不正なアクセスが試行されました",
            SecurityEventType::DataIntegrityViolation => "データの整合性に問題が検出されました",
            SecurityEventType::PrivilegeEscalation => "権限昇格が試行されました",
            SecurityEventType::ConfigurationTampering => "設定の改ざんが検出されました",
            SecurityEventType::AbnormalFileAccess => "異常なファイルアクセスが検出されました",
            SecurityEventType::SecurityPolicyViolation => "セキュリティポリシーに違反しました",
            SecurityEventType::AuditLogTampering => "監査ログの改ざんが試行されました",
            SecurityEventType::SystemIntrusion => "システムへの侵入が検出されました",
        }
    }

    /// イベントコードを取得
    pub fn event_code(&self) -> &'static str {
        match self {
            SecurityEventType::AuthenticationFailure => "SEC_AUTH_FAIL",
            SecurityEventType::AuthorizationFailure => "SEC_AUTHZ_FAIL",
            SecurityEventType::UnauthorizedAccess => "SEC_UNAUTH_ACCESS",
            SecurityEventType::DataIntegrityViolation => "SEC_DATA_INTEGRITY",
            SecurityEventType::PrivilegeEscalation => "SEC_PRIV_ESC",
            SecurityEventType::ConfigurationTampering => "SEC_CONFIG_TAMPER",
            SecurityEventType::AbnormalFileAccess => "SEC_ABNORMAL_FILE",
            SecurityEventType::SecurityPolicyViolation => "SEC_POLICY_VIOLATION",
            SecurityEventType::AuditLogTampering => "SEC_AUDIT_TAMPER",
            SecurityEventType::SystemIntrusion => "SEC_INTRUSION",
        }
    }
}

/// セキュリティ監査エントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEntry {
    /// 監査ID（一意識別子）
    pub audit_id: String,
    /// タイムスタンプ（JST）
    pub timestamp: DateTime<Utc>,
    /// イベントタイプ
    pub event_type: SecurityEventType,
    /// 重要度
    pub severity: ErrorSeverity,
    /// イベントの説明
    pub description: String,
    /// 関連するユーザーID
    pub user_id: Option<i64>,
    /// 関連する移行ログID
    pub migration_log_id: Option<i64>,
    /// 影響を受けたリソース
    pub affected_resource: Option<String>,
    /// 送信元IPアドレス
    pub source_ip: Option<String>,
    /// ユーザーエージェント
    pub user_agent: Option<String>,
    /// セッションID
    pub session_id: Option<String>,
    /// 追加のコンテキスト情報
    pub context: HashMap<String, serde_json::Value>,
    /// 関連するエラー情報
    pub related_error: Option<String>,
    /// 対応アクション
    pub response_action: Option<String>,
    /// 調査状況
    pub investigation_status: InvestigationStatus,
}

/// 調査状況
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum InvestigationStatus {
    /// 未調査
    Pending,
    /// 調査中
    InProgress,
    /// 誤検知
    FalsePositive,
    /// 確認済み脅威
    ConfirmedThreat,
    /// 対応完了
    Resolved,
    /// 無視
    Ignored,
}

impl SecurityAuditEntry {
    /// 新しい監査エントリを作成
    pub fn new(
        event_type: SecurityEventType,
        description: &str,
        user_id: Option<i64>,
        migration_log_id: Option<i64>,
    ) -> Self {
        Self {
            audit_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            severity: event_type.severity(),
            event_type,
            description: description.to_string(),
            user_id,
            migration_log_id,
            affected_resource: None,
            source_ip: None,
            user_agent: None,
            session_id: None,
            context: HashMap::new(),
            related_error: None,
            response_action: None,
            investigation_status: InvestigationStatus::Pending,
        }
    }

    /// 影響を受けたリソースを設定
    pub fn with_affected_resource(mut self, resource: &str) -> Self {
        self.affected_resource = Some(resource.to_string());
        self
    }

    /// 送信元IPを設定
    pub fn with_source_ip(mut self, ip: &str) -> Self {
        self.source_ip = Some(ip.to_string());
        self
    }

    /// セッションIDを設定
    pub fn with_session_id(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    /// コンテキスト情報を追加
    pub fn with_context(mut self, key: &str, value: serde_json::Value) -> Self {
        self.context.insert(key.to_string(), value);
        self
    }

    /// 関連エラーを設定
    pub fn with_related_error(mut self, error: &str) -> Self {
        self.related_error = Some(error.to_string());
        self
    }

    /// 対応アクションを設定
    pub fn with_response_action(mut self, action: &str) -> Self {
        self.response_action = Some(action.to_string());
        self
    }

    /// 調査状況を更新
    pub fn update_investigation_status(&mut self, status: InvestigationStatus) {
        self.investigation_status = status;
    }

    /// JSON形式でシリアライズ
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// 人間が読みやすい形式でフォーマット
    pub fn to_human_readable(&self) -> String {
        let severity_str = match self.severity {
            ErrorSeverity::Low => "LOW",
            ErrorSeverity::Medium => "MEDIUM",
            ErrorSeverity::High => "HIGH",
            ErrorSeverity::Critical => "CRITICAL",
        };

        let mut parts = vec![
            format!(
                "[{}]",
                self.timestamp
                    .with_timezone(&chrono_tz::Asia::Tokyo)
                    .format("%Y-%m-%d %H:%M:%S JST")
            ),
            format!("[SECURITY]"),
            format!("[{}]", severity_str),
            format!("[{}]", self.event_type.event_code()),
            format!("[ID:{}]", &self.audit_id[..8]), // 短縮ID
        ];

        if let Some(user_id) = self.user_id {
            parts.push(format!("[USER:{}]", user_id));
        }

        if let Some(migration_id) = self.migration_log_id {
            parts.push(format!("[MIG:{}]", migration_id));
        }

        parts.push(self.description.clone());

        if let Some(resource) = &self.affected_resource {
            parts.push(format!("Resource: {}", resource));
        }

        if let Some(ip) = &self.source_ip {
            parts.push(format!("IP: {}", ip));
        }

        parts.join(" ")
    }
}

/// セキュリティ監査ログ管理
pub struct SecurityAuditLogger {
    /// 監査エントリのバッファ
    audit_buffer: Arc<Mutex<Vec<SecurityAuditEntry>>>,
    /// 構造化ロガー
    structured_logger: Arc<Mutex<StructuredLogger>>,
    /// 最大バッファサイズ
    max_buffer_size: usize,
    /// アラート閾値設定
    alert_thresholds: HashMap<SecurityEventType, usize>,
}

impl SecurityAuditLogger {
    /// 新しいセキュリティ監査ロガーを作成
    pub fn new(
        structured_logger: Arc<Mutex<StructuredLogger>>,
        max_buffer_size: Option<usize>,
    ) -> Self {
        let mut alert_thresholds = HashMap::new();

        // デフォルトのアラート閾値を設定
        alert_thresholds.insert(SecurityEventType::AuthenticationFailure, 5);
        alert_thresholds.insert(SecurityEventType::AuthorizationFailure, 3);
        alert_thresholds.insert(SecurityEventType::UnauthorizedAccess, 1);
        alert_thresholds.insert(SecurityEventType::DataIntegrityViolation, 1);
        alert_thresholds.insert(SecurityEventType::PrivilegeEscalation, 1);
        alert_thresholds.insert(SecurityEventType::ConfigurationTampering, 1);
        alert_thresholds.insert(SecurityEventType::AbnormalFileAccess, 10);
        alert_thresholds.insert(SecurityEventType::SecurityPolicyViolation, 3);
        alert_thresholds.insert(SecurityEventType::AuditLogTampering, 1);
        alert_thresholds.insert(SecurityEventType::SystemIntrusion, 1);

        Self {
            audit_buffer: Arc::new(Mutex::new(Vec::new())),
            structured_logger,
            max_buffer_size: max_buffer_size.unwrap_or(10000),
            alert_thresholds,
        }
    }

    /// セキュリティイベントを記録
    pub fn log_security_event(&self, entry: SecurityAuditEntry) {
        let human_readable = entry.to_human_readable();

        // 構造化ログにも記録
        if let Ok(logger) = self.structured_logger.lock() {
            let log_entry = StructuredLogEntry::new(
                LogLevel::from(entry.severity),
                "security_audit",
                &entry.description,
                entry.migration_log_id,
            )
            .with_error_code(entry.event_type.event_code())
            .with_context(
                "audit_id",
                serde_json::Value::String(entry.audit_id.clone()),
            )
            .with_context(
                "event_type",
                serde_json::Value::String(format!("{:?}", entry.event_type)),
            )
            .with_context(
                "investigation_status",
                serde_json::Value::String(format!("{:?}", entry.investigation_status)),
            );

            logger.log(log_entry);
        }

        // 標準ログにも出力
        match entry.severity {
            ErrorSeverity::Critical => error!("SECURITY CRITICAL: {}", human_readable),
            ErrorSeverity::High => error!("SECURITY HIGH: {}", human_readable),
            ErrorSeverity::Medium => warn!("SECURITY MEDIUM: {}", human_readable),
            ErrorSeverity::Low => info!("SECURITY LOW: {}", human_readable),
        }

        // 監査バッファに追加
        if let Ok(mut buffer) = self.audit_buffer.lock() {
            buffer.push(entry.clone());

            // バッファサイズ制限
            if buffer.len() > self.max_buffer_size {
                buffer.remove(0);
            }
        }

        // アラート閾値チェック
        self.check_alert_thresholds(&entry.event_type);
    }

    /// MigrationErrorからセキュリティイベントを記録
    pub fn log_migration_security_event(
        &self,
        error: &MigrationError,
        migration_log_id: Option<i64>,
        user_id: Option<i64>,
    ) {
        let (event_type, description) = self.classify_migration_error(error);

        let entry = SecurityAuditEntry::new(event_type, &description, user_id, migration_log_id)
            .with_related_error(&error.to_string())
            .with_context("error_code", serde_json::Value::String(error.error_code()))
            .with_context(
                "error_category",
                serde_json::Value::String(error.category().to_string()),
            )
            .with_context("error_metadata", error.log_metadata());

        self.log_security_event(entry);
    }

    /// MigrationErrorをセキュリティイベントに分類
    fn classify_migration_error(&self, error: &MigrationError) -> (SecurityEventType, String) {
        match error {
            MigrationError::Permission { message, user_id, required_permission, resource } => {
                (
                    SecurityEventType::AuthorizationFailure,
                    format!(
                        "移行処理で権限エラーが発生: ユーザー{} が {} に対する {} 権限を持っていません - {}",
                        user_id, resource, required_permission, message
                    ),
                )
            }
            MigrationError::IntegrityValidation { message, validation_type, expected, actual } => {
                (
                    SecurityEventType::DataIntegrityViolation,
                    format!(
                        "データ整合性違反が検出: {} - 期待値: {}, 実際値: {} - {}",
                        validation_type, expected, actual, message
                    ),
                )
            }
            MigrationError::Configuration { message, config_key, .. } => {
                (
                    SecurityEventType::ConfigurationTampering,
                    format!("設定エラーが検出: {} の設定に問題があります - {}", config_key, message),
                )
            }
            MigrationError::FileMigration { message, old_path, new_path, user_id, .. } => {
                (
                    SecurityEventType::AbnormalFileAccess,
                    format!(
                        "ファイル移行で異常なアクセス: ユーザー{} が {} から {} への移行で問題が発生 - {}",
                        user_id, old_path, new_path, message
                    ),
                )
            }
            _ => {
                (
                    SecurityEventType::SecurityPolicyViolation,
                    format!("移行処理でセキュリティポリシー違反の可能性: {}", error),
                )
            }
        }
    }

    /// アラート閾値をチェック
    fn check_alert_thresholds(&self, event_type: &SecurityEventType) {
        if let Some(&threshold) = self.alert_thresholds.get(event_type) {
            let recent_count =
                self.count_recent_events(event_type, std::time::Duration::from_secs(3600)); // 1時間以内

            if recent_count >= threshold {
                self.trigger_security_alert(event_type, recent_count, threshold);
            }
        }
    }

    /// 最近のイベント数をカウント
    fn count_recent_events(
        &self,
        event_type: &SecurityEventType,
        duration: std::time::Duration,
    ) -> usize {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(duration).unwrap_or_default();

        if let Ok(buffer) = self.audit_buffer.lock() {
            buffer
                .iter()
                .filter(|entry| {
                    entry.event_type == *event_type
                        && entry.timestamp.with_timezone(&Utc) > cutoff_time
                })
                .count()
        } else {
            0
        }
    }

    /// セキュリティアラートをトリガー
    fn trigger_security_alert(
        &self,
        event_type: &SecurityEventType,
        count: usize,
        threshold: usize,
    ) {
        let alert_entry = SecurityAuditEntry::new(
            SecurityEventType::SecurityPolicyViolation,
            &format!(
                "セキュリティアラート: {} イベントが閾値を超過しました (発生回数: {}, 閾値: {})",
                event_type.description(),
                count,
                threshold
            ),
            None,
            None,
        )
        .with_context(
            "alert_type",
            serde_json::Value::String("threshold_exceeded".to_string()),
        )
        .with_context(
            "event_type",
            serde_json::Value::String(format!("{:?}", event_type)),
        )
        .with_context(
            "count",
            serde_json::Value::Number(serde_json::Number::from(count)),
        )
        .with_context(
            "threshold",
            serde_json::Value::Number(serde_json::Number::from(threshold)),
        )
        .with_response_action("immediate_investigation_required");

        error!(
            "SECURITY ALERT: {} イベントが閾値({})を超過しました。発生回数: {}",
            event_type.description(),
            threshold,
            count
        );

        // アラートエントリを記録（再帰を避けるため直接バッファに追加）
        if let Ok(mut buffer) = self.audit_buffer.lock() {
            buffer.push(alert_entry);
        }
    }

    /// 監査ログを取得
    pub fn get_audit_logs(&self) -> Vec<SecurityAuditEntry> {
        self.audit_buffer
            .lock()
            .unwrap_or_else(|poisoned| {
                warn!("監査ログバッファのロック取得に失敗しました");
                poisoned.into_inner()
            })
            .clone()
    }

    /// 指定された重要度以上の監査ログを取得
    pub fn get_audit_logs_by_severity(
        &self,
        min_severity: ErrorSeverity,
    ) -> Vec<SecurityAuditEntry> {
        let logs = self.get_audit_logs();
        logs.into_iter()
            .filter(|entry| {
                self.severity_priority(entry.severity) >= self.severity_priority(min_severity)
            })
            .collect()
    }

    /// 指定されたイベントタイプの監査ログを取得
    pub fn get_audit_logs_by_event_type(
        &self,
        event_type: SecurityEventType,
    ) -> Vec<SecurityAuditEntry> {
        let logs = self.get_audit_logs();
        logs.into_iter()
            .filter(|entry| entry.event_type == event_type)
            .collect()
    }

    /// 指定された移行ログIDの監査ログを取得
    pub fn get_audit_logs_by_migration_id(&self, migration_log_id: i64) -> Vec<SecurityAuditEntry> {
        let logs = self.get_audit_logs();
        logs.into_iter()
            .filter(|entry| entry.migration_log_id == Some(migration_log_id))
            .collect()
    }

    /// 重要度の優先度を取得
    fn severity_priority(&self, severity: ErrorSeverity) -> u8 {
        match severity {
            ErrorSeverity::Low => 0,
            ErrorSeverity::Medium => 1,
            ErrorSeverity::High => 2,
            ErrorSeverity::Critical => 3,
        }
    }

    /// 監査統計を取得
    pub fn get_audit_statistics(&self) -> SecurityAuditStatistics {
        let logs = self.get_audit_logs();
        let mut stats = SecurityAuditStatistics::default();

        for entry in logs {
            match entry.severity {
                ErrorSeverity::Low => stats.low_severity_count += 1,
                ErrorSeverity::Medium => stats.medium_severity_count += 1,
                ErrorSeverity::High => stats.high_severity_count += 1,
                ErrorSeverity::Critical => stats.critical_severity_count += 1,
            }

            // イベントタイプ別統計
            *stats.event_type_counts.entry(entry.event_type).or_insert(0) += 1;

            // 調査状況別統計
            *stats
                .investigation_status_counts
                .entry(entry.investigation_status)
                .or_insert(0) += 1;
        }

        stats.total_events = stats.low_severity_count
            + stats.medium_severity_count
            + stats.high_severity_count
            + stats.critical_severity_count;
        stats
    }

    /// 監査ログをクリア
    pub fn clear_audit_logs(&self) {
        if let Ok(mut buffer) = self.audit_buffer.lock() {
            buffer.clear();
        }
    }

    /// アラート閾値を設定
    pub fn set_alert_threshold(&mut self, event_type: SecurityEventType, threshold: usize) {
        self.alert_thresholds.insert(event_type, threshold);
    }
}

/// セキュリティ監査統計
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SecurityAuditStatistics {
    /// 総イベント数
    pub total_events: usize,
    /// 低重要度イベント数
    pub low_severity_count: usize,
    /// 中重要度イベント数
    pub medium_severity_count: usize,
    /// 高重要度イベント数
    pub high_severity_count: usize,
    /// 致命的重要度イベント数
    pub critical_severity_count: usize,
    /// イベントタイプ別カウント
    pub event_type_counts: HashMap<SecurityEventType, usize>,
    /// 調査状況別カウント
    pub investigation_status_counts: HashMap<InvestigationStatus, usize>,
}

/// グローバルセキュリティ監査ロガーインスタンス
static GLOBAL_SECURITY_LOGGER: std::sync::OnceLock<Arc<Mutex<SecurityAuditLogger>>> =
    std::sync::OnceLock::new();

/// グローバルセキュリティ監査ロガーを初期化
pub fn init_global_security_logger(
    structured_logger: Arc<Mutex<StructuredLogger>>,
    max_buffer_size: Option<usize>,
) {
    let logger = SecurityAuditLogger::new(structured_logger, max_buffer_size);
    GLOBAL_SECURITY_LOGGER
        .set(Arc::new(Mutex::new(logger)))
        .ok();
}

/// グローバルセキュリティ監査ロガーを取得
pub fn get_global_security_logger() -> Option<Arc<Mutex<SecurityAuditLogger>>> {
    GLOBAL_SECURITY_LOGGER.get().cloned()
}

/// 便利な関数：セキュリティイベントを記録
pub fn log_security_event(
    event_type: SecurityEventType,
    description: &str,
    user_id: Option<i64>,
    migration_log_id: Option<i64>,
) {
    if let Some(logger) = get_global_security_logger() {
        if let Ok(logger) = logger.lock() {
            let entry = SecurityAuditEntry::new(event_type, description, user_id, migration_log_id);
            logger.log_security_event(entry);
        }
    }
}

/// 便利な関数：移行エラーからセキュリティイベントを記録
pub fn log_migration_security_error(
    error: &MigrationError,
    migration_log_id: Option<i64>,
    user_id: Option<i64>,
) {
    if let Some(logger) = get_global_security_logger() {
        if let Ok(logger) = logger.lock() {
            logger.log_migration_security_event(error, migration_log_id, user_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::migrations::logging::StructuredLogger;

    #[test]
    fn test_security_event_type_properties() {
        let event = SecurityEventType::UnauthorizedAccess;

        assert_eq!(event.severity(), ErrorSeverity::Critical);
        assert_eq!(event.event_code(), "SEC_UNAUTH_ACCESS");
        assert_eq!(event.description(), "不正なアクセスが試行されました");
    }

    #[test]
    fn test_security_audit_entry_creation() {
        let entry = SecurityAuditEntry::new(
            SecurityEventType::AuthorizationFailure,
            "テスト認可失敗",
            Some(123),
            Some(456),
        )
        .with_affected_resource("test_resource")
        .with_source_ip("192.168.1.1")
        .with_context(
            "test_key",
            serde_json::Value::String("test_value".to_string()),
        );

        assert_eq!(entry.event_type, SecurityEventType::AuthorizationFailure);
        assert_eq!(entry.severity, ErrorSeverity::High);
        assert_eq!(entry.description, "テスト認可失敗");
        assert_eq!(entry.user_id, Some(123));
        assert_eq!(entry.migration_log_id, Some(456));
        assert_eq!(entry.affected_resource, Some("test_resource".to_string()));
        assert_eq!(entry.source_ip, Some("192.168.1.1".to_string()));
        assert_eq!(entry.investigation_status, InvestigationStatus::Pending);
    }

    #[test]
    fn test_security_audit_entry_json_serialization() {
        let entry = SecurityAuditEntry::new(
            SecurityEventType::DataIntegrityViolation,
            "データ整合性エラー",
            None,
            None,
        );

        let json = entry.to_json();
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("DataIntegrityViolation"));
        assert!(json_str.contains("データ整合性エラー"));
    }

    #[test]
    fn test_security_audit_entry_human_readable() {
        let entry = SecurityAuditEntry::new(
            SecurityEventType::PrivilegeEscalation,
            "権限昇格試行",
            Some(789),
            Some(101),
        )
        .with_source_ip("10.0.0.1");

        let readable = entry.to_human_readable();
        assert!(readable.contains("SECURITY"));
        assert!(readable.contains("CRITICAL"));
        assert!(readable.contains("SEC_PRIV_ESC"));
        assert!(readable.contains("USER:789"));
        assert!(readable.contains("MIG:101"));
        assert!(readable.contains("権限昇格試行"));
        assert!(readable.contains("IP: 10.0.0.1"));
    }

    #[tokio::test]
    async fn test_security_audit_logger_basic_operations() {
        let structured_logger = Arc::new(Mutex::new(StructuredLogger::new(Some(100))));
        let audit_logger = SecurityAuditLogger::new(structured_logger, Some(50));

        let entry1 = SecurityAuditEntry::new(
            SecurityEventType::AuthenticationFailure,
            "認証失敗テスト",
            Some(1),
            None,
        );

        let entry2 = SecurityAuditEntry::new(
            SecurityEventType::UnauthorizedAccess,
            "不正アクセステスト",
            Some(2),
            None,
        );

        audit_logger.log_security_event(entry1);
        audit_logger.log_security_event(entry2);

        let logs = audit_logger.get_audit_logs();
        assert_eq!(logs.len(), 2);

        let stats = audit_logger.get_audit_statistics();
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.medium_severity_count, 1); // AuthenticationFailure
        assert_eq!(stats.critical_severity_count, 1); // UnauthorizedAccess
    }

    #[test]
    fn test_migration_error_classification() {
        let structured_logger = Arc::new(Mutex::new(StructuredLogger::new(Some(100))));
        let audit_logger = SecurityAuditLogger::new(structured_logger, Some(50));

        let permission_error = MigrationError::Permission {
            message: "アクセス拒否".to_string(),
            user_id: 123,
            required_permission: "admin".to_string(),
            resource: "migration".to_string(),
        };

        let (event_type, description) = audit_logger.classify_migration_error(&permission_error);
        assert_eq!(event_type, SecurityEventType::AuthorizationFailure);
        assert!(description.contains("権限エラー"));
        assert!(description.contains("ユーザー123"));

        let integrity_error = MigrationError::IntegrityValidation {
            message: "ハッシュ不一致".to_string(),
            validation_type: "file_hash".to_string(),
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };

        let (event_type, description) = audit_logger.classify_migration_error(&integrity_error);
        assert_eq!(event_type, SecurityEventType::DataIntegrityViolation);
        assert!(description.contains("データ整合性違反"));
    }

    #[test]
    fn test_audit_log_filtering() {
        let structured_logger = Arc::new(Mutex::new(StructuredLogger::new(Some(100))));
        let audit_logger = SecurityAuditLogger::new(structured_logger, Some(50));

        // 異なる重要度のイベントを追加
        audit_logger.log_security_event(SecurityAuditEntry::new(
            SecurityEventType::AuthenticationFailure, // Medium
            "認証失敗",
            None,
            None,
        ));

        audit_logger.log_security_event(SecurityAuditEntry::new(
            SecurityEventType::UnauthorizedAccess, // Critical
            "不正アクセス",
            None,
            None,
        ));

        audit_logger.log_security_event(SecurityAuditEntry::new(
            SecurityEventType::AbnormalFileAccess, // High
            "異常ファイルアクセス",
            None,
            None,
        ));

        // 重要度でフィルタリング
        let high_and_above = audit_logger.get_audit_logs_by_severity(ErrorSeverity::High);
        assert_eq!(high_and_above.len(), 2); // High + Critical

        let critical_only = audit_logger.get_audit_logs_by_severity(ErrorSeverity::Critical);
        assert_eq!(critical_only.len(), 1); // Critical only

        // イベントタイプでフィルタリング
        let auth_failures =
            audit_logger.get_audit_logs_by_event_type(SecurityEventType::AuthenticationFailure);
        assert_eq!(auth_failures.len(), 1);
    }
}

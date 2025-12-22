// セキュリティ機能のTauriコマンド

use super::models::*;
use super::service::SecurityManager;
use crate::shared::errors::AppError;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::time::Instant;
use tauri::State;

/// システム診断情報を取得
#[tauri::command]
pub async fn get_system_diagnostic_info() -> Result<DiagnosticInfo, String> {
    info!("システム診断情報の取得要求を受信しました");

    let security_manager = SecurityManager::new();
    let diagnostic_info = security_manager.get_diagnostic_info();

    debug!("診断情報を返却します: {diagnostic_info:?}");
    Ok(diagnostic_info)
}

/// セキュリティ設定の検証
#[tauri::command]
pub async fn validate_security_configuration() -> Result<ValidationResult, String> {
    info!("セキュリティ設定の検証要求を受信しました");

    let security_manager = SecurityManager::new();

    match security_manager.validate_configuration() {
        Ok(validation_result) => {
            if validation_result.is_valid {
                info!("セキュリティ設定の検証が成功しました");
            } else {
                warn!(
                    "セキュリティ設定の検証で問題が見つかりました: {}",
                    validation_result.message
                );
            }
            Ok(validation_result)
        }
        Err(e) => {
            let error_msg = format!("セキュリティ設定の検証でエラーが発生しました: {e}");
            warn!("{error_msg}");
            Err(error_msg)
        }
    }
}

/// R2接続テスト（セキュリティログ付き）
#[tauri::command]
pub async fn test_r2_connection_secure() -> Result<ConnectionTestResult, String> {
    info!("セキュアなR2接続テストを開始します");

    let mut security_manager = SecurityManager::new();
    security_manager.log_security_event(
        "r2_connection_test_requested",
        "フロントエンドからの接続テスト要求",
    );

    let start_time = Instant::now();

    // R2設定を取得
    let config = match crate::services::config::R2Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            let error_msg = format!("R2設定の読み込みに失敗しました: {e:?}");
            security_manager.log_security_event_with_severity(
                "r2_config_load_failed",
                &error_msg,
                EventSeverity::Error,
            );

            let connection_details = ConnectionDetails {
                endpoint: "設定読み込み失敗".to_string(),
                bucket_name: "****".to_string(),
                account_id: "****".to_string(),
                access_key: "****".to_string(),
            };

            return Ok(ConnectionTestResult::failure(error_msg, connection_details));
        }
    };

    // 接続詳細情報を作成（マスク済み）
    let credentials = security_manager.get_credentials();
    let connection_details = ConnectionDetails {
        endpoint: format!(
            "https://{}.r2.cloudflarestorage.com",
            credentials
                .get_masked_credential("R2_ACCOUNT_ID")
                .unwrap_or(&"****".to_string())
        ),
        bucket_name: credentials
            .get_masked_credential("R2_BUCKET_NAME")
            .unwrap_or(&"****".to_string())
            .clone(),
        account_id: credentials
            .get_masked_credential("R2_ACCOUNT_ID")
            .unwrap_or(&"****".to_string())
            .clone(),
        access_key: credentials
            .get_masked_credential("R2_ACCESS_KEY")
            .unwrap_or(&"****".to_string())
            .clone(),
    };

    // R2クライアントを作成
    let client = match crate::features::receipts::service::R2Client::new(config).await {
        Ok(client) => client,
        Err(e) => {
            let error_msg = format!("R2クライアントの作成に失敗しました: {e}");
            security_manager.log_security_event_with_severity(
                "r2_client_creation_failed",
                &error_msg,
                EventSeverity::Error,
            );
            return Ok(ConnectionTestResult::failure(error_msg, connection_details));
        }
    };

    // 接続テストを実行
    match client.test_connection().await {
        Ok(()) => {
            let response_time = start_time.elapsed().as_millis() as u64;
            info!("R2接続テストが成功しました（応答時間: {response_time}ms）");
            security_manager.log_security_event("r2_connection_test_success", "接続テスト成功");
            Ok(ConnectionTestResult::success(
                response_time,
                connection_details,
            ))
        }
        Err(e) => {
            let error_msg = format!("R2接続テストに失敗しました: {e}");
            warn!("{error_msg}");
            security_manager.log_security_event_with_severity(
                "r2_connection_test_failed",
                &error_msg,
                EventSeverity::Warning,
            );
            Ok(ConnectionTestResult::failure(error_msg, connection_details))
        }
    }
}

/// 環境情報を取得
#[tauri::command]
pub async fn get_environment_info() -> Result<EnvironmentInfo, String> {
    info!("環境情報の取得要求を受信しました");

    let security_manager = SecurityManager::new();
    let env_info = security_manager.get_environment_info();

    debug!("環境情報を返却します: {env_info:?}");
    Ok(env_info)
}

/// セキュリティイベントをログに記録
#[tauri::command]
pub async fn log_security_event(event_type: String, details: String) -> Result<(), String> {
    info!("セキュリティイベントのログ記録要求: type={event_type}, details={details}");

    let mut security_manager = SecurityManager::new();
    security_manager.log_security_event(&event_type, &details);

    Ok(())
}

/// セキュリティイベントをログに記録（重要度指定）
#[tauri::command]
pub async fn log_security_event_with_severity(
    event_type: String,
    details: String,
    severity: EventSeverity,
) -> Result<(), String> {
    info!("セキュリティイベントのログ記録要求（重要度指定）: type={event_type}, details={details}, severity={severity:?}");

    let mut security_manager = SecurityManager::new();
    security_manager.log_security_event_with_severity(&event_type, &details, severity);

    Ok(())
}

/// 最近のセキュリティイベントを取得
#[tauri::command]
pub async fn get_recent_security_events(
    limit: Option<usize>,
) -> Result<Vec<SecurityEvent>, String> {
    info!("最近のセキュリティイベント取得要求: limit={limit:?}");

    let security_manager = SecurityManager::new();
    let events = security_manager.get_recent_security_events(limit.unwrap_or(50));

    debug!("セキュリティイベントを返却します: {} 件", events.len());
    Ok(events)
}

/// セキュリティ監査ログを生成
#[tauri::command]
pub async fn generate_security_audit_log(
    period_start: String,
    period_end: String,
) -> Result<SecurityAuditLog, String> {
    info!("セキュリティ監査ログ生成要求: period_start={period_start}, period_end={period_end}");

    let security_manager = SecurityManager::new();
    let audit_log = security_manager.generate_audit_log(period_start, period_end);

    debug!(
        "監査ログを生成しました: {} 件のイベント",
        audit_log.total_events
    );
    Ok(audit_log)
}

/// R2クライアントの診断情報を取得
#[tauri::command]
pub async fn get_r2_diagnostic_info() -> Result<HashMap<String, String>, String> {
    info!("R2診断情報の取得要求を受信しました");

    let mut security_manager = SecurityManager::new();
    security_manager.log_security_event("r2_diagnostic_requested", "R2診断情報の取得要求");

    // R2設定を取得
    let config = match crate::services::config::R2Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            let error_msg = format!("R2設定の読み込みに失敗しました: {e:?}");
            security_manager.log_security_event_with_severity(
                "r2_config_load_failed",
                &error_msg,
                EventSeverity::Error,
            );
            return Err(error_msg);
        }
    };

    // R2クライアントを作成
    let client = match crate::features::receipts::service::R2Client::new(config).await {
        Ok(client) => client,
        Err(e) => {
            let error_msg = format!("R2クライアントの作成に失敗しました: {e}");
            security_manager.log_security_event_with_severity(
                "r2_client_creation_failed",
                &error_msg,
                EventSeverity::Error,
            );
            return Err(error_msg);
        }
    };

    let diagnostic_info = client.get_diagnostic_info();

    debug!("R2診断情報を返却します: {diagnostic_info:?}");
    Ok(diagnostic_info)
}

/// アプリケーション初期化時のセキュリティチェック
#[tauri::command]
pub async fn perform_security_initialization_check() -> Result<ValidationResult, String> {
    info!("アプリケーション初期化時のセキュリティチェックを開始します");

    let mut security_manager = SecurityManager::new();
    security_manager.log_security_event(
        "security_initialization_check_started",
        "アプリケーション初期化時のセキュリティチェック開始",
    );

    match security_manager.validate_configuration() {
        Ok(validation_result) => {
            if validation_result.is_valid {
                security_manager.log_security_event(
                    "security_initialization_check_success",
                    "セキュリティ初期化チェック成功",
                );
                info!("セキュリティ初期化チェックが成功しました");
            } else {
                security_manager.log_security_event_with_severity(
                    "security_initialization_check_warning",
                    &format!(
                        "セキュリティ初期化チェックで警告: {}",
                        validation_result.message
                    ),
                    EventSeverity::Warning,
                );
                warn!(
                    "セキュリティ初期化チェックで警告が発生しました: {}",
                    validation_result.message
                );
            }
            Ok(validation_result)
        }
        Err(e) => {
            let error_msg = format!("セキュリティ初期化チェックでエラーが発生しました: {e}");
            security_manager.log_security_event_with_severity(
                "security_initialization_check_failed",
                &error_msg,
                EventSeverity::Error,
            );
            warn!("{error_msg}");
            Err(error_msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_environment_info() {
        let result = get_environment_info().await;
        assert!(result.is_ok());

        let info = result.unwrap();
        assert!(!info.environment.is_empty());
    }

    #[tokio::test]
    async fn test_log_security_event() {
        let result =
            log_security_event("test_event".to_string(), "テストイベントの詳細".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_system_diagnostic_info() {
        let result = get_system_diagnostic_info().await;
        assert!(result.is_ok());

        let info = result.unwrap();
        assert!(!info.environment.is_empty());
        assert!(!info.system_info.app_version.is_empty());
    }

    #[tokio::test]
    async fn test_validate_security_configuration() {
        let result = validate_security_configuration().await;
        assert!(result.is_ok());

        let validation_result = result.unwrap();
        assert!(!validation_result.details.is_empty());
    }

    #[tokio::test]
    async fn test_log_security_event_with_severity() {
        let result = log_security_event_with_severity(
            "test_event".to_string(),
            "テストイベント".to_string(),
            EventSeverity::Warning,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_recent_security_events() {
        let result = get_recent_security_events(Some(10)).await;
        assert!(result.is_ok());

        let events = result.unwrap();
        // 新しいSecurityManagerなので、イベントは空のはず
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_perform_security_initialization_check() {
        let result = perform_security_initialization_check().await;
        assert!(result.is_ok());

        let validation_result = result.unwrap();
        assert!(!validation_result.details.is_empty());
    }
}

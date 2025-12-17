// セキュリティ関連のTauriコマンド

use crate::services::security::SecurityManager;
use log::{debug, info, warn};
use std::collections::HashMap;
// use tauri::State; // 現在未使用

/// システム診断情報を取得
#[tauri::command]
pub async fn get_system_diagnostic_info() -> Result<HashMap<String, String>, String> {
    info!("システム診断情報の取得要求を受信しました");

    let security_manager = SecurityManager::new();
    let diagnostic_info = security_manager.get_diagnostic_info();

    debug!("診断情報を返却します: {diagnostic_info:?}");
    Ok(diagnostic_info)
}

/// セキュリティ設定の検証
#[tauri::command]
pub async fn validate_security_configuration() -> Result<bool, String> {
    info!("セキュリティ設定の検証要求を受信しました");

    let security_manager = SecurityManager::new();

    match security_manager.validate_configuration() {
        Ok(()) => {
            info!("セキュリティ設定の検証が成功しました");
            Ok(true)
        }
        Err(e) => {
            warn!("セキュリティ設定の検証に失敗しました: {e}");
            Err(e)
        }
    }
}

/// R2接続テスト（セキュリティログ付き）
#[tauri::command]
pub async fn test_r2_connection_secure() -> Result<bool, String> {
    info!("セキュアなR2接続テストを開始します");

    let security_manager = SecurityManager::new();
    security_manager.log_security_event(
        "r2_connection_test_requested",
        "フロントエンドからの接続テスト要求",
    );

    // R2設定を取得
    let config = crate::services::config::R2Config::from_env().map_err(|e| {
        let error_msg = format!("R2設定の読み込みに失敗しました: {e:?}");
        security_manager.log_security_event("r2_config_load_failed", &error_msg);
        error_msg
    })?;

    // R2クライアントを作成
    let client = crate::services::r2_client::R2Client::new(config)
        .await
        .map_err(|e| {
            let error_msg = format!("R2クライアントの作成に失敗しました: {e}");
            security_manager.log_security_event("r2_client_creation_failed", &error_msg);
            error_msg
        })?;

    // 接続テストを実行
    match client.test_connection().await {
        Ok(()) => {
            info!("R2接続テストが成功しました");
            security_manager.log_security_event("r2_connection_test_success", "接続テスト成功");
            Ok(true)
        }
        Err(e) => {
            let error_msg = format!("R2接続テストに失敗しました: {e}");
            warn!("{error_msg}");
            security_manager.log_security_event("r2_connection_test_failed", &error_msg);
            Err(error_msg)
        }
    }
}

/// 環境情報を取得
#[tauri::command]
pub async fn get_environment_info() -> Result<HashMap<String, String>, String> {
    info!("環境情報の取得要求を受信しました");

    let security_manager = SecurityManager::new();
    let env_config = security_manager.get_env_config();

    let mut info = HashMap::new();
    info.insert("environment".to_string(), env_config.environment.clone());
    info.insert("debug_mode".to_string(), env_config.debug_mode.to_string());
    info.insert("log_level".to_string(), env_config.log_level.clone());
    info.insert(
        "is_production".to_string(),
        env_config.is_production().to_string(),
    );
    info.insert(
        "is_development".to_string(),
        env_config.is_development().to_string(),
    );

    debug!("環境情報を返却します: {info:?}");
    Ok(info)
}

/// セキュリティイベントをログに記録
#[tauri::command]
pub async fn log_security_event(event_type: String, details: String) -> Result<(), String> {
    info!("セキュリティイベントのログ記録要求: type={event_type}, details={details}");

    let security_manager = SecurityManager::new();
    security_manager.log_security_event(&event_type, &details);

    Ok(())
}

/// R2クライアントの診断情報を取得
#[tauri::command]
pub async fn get_r2_diagnostic_info() -> Result<HashMap<String, String>, String> {
    info!("R2診断情報の取得要求を受信しました");

    let security_manager = SecurityManager::new();
    security_manager.log_security_event("r2_diagnostic_requested", "R2診断情報の取得要求");

    // R2設定を取得
    let config = crate::services::config::R2Config::from_env().map_err(|e| {
        let error_msg = format!("R2設定の読み込みに失敗しました: {e:?}");
        security_manager.log_security_event("r2_config_load_failed", &error_msg);
        error_msg
    })?;

    // R2クライアントを作成
    let client = crate::services::r2_client::R2Client::new(config)
        .await
        .map_err(|e| {
            let error_msg = format!("R2クライアントの作成に失敗しました: {e}");
            security_manager.log_security_event("r2_client_creation_failed", &error_msg);
            error_msg
        })?;

    let diagnostic_info = client.get_diagnostic_info();

    debug!("R2診断情報を返却します: {diagnostic_info:?}");
    Ok(diagnostic_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_environment_info() {
        let result = get_environment_info().await;
        assert!(result.is_ok());

        let info = result.unwrap();
        assert!(info.contains_key("environment"));
        assert!(info.contains_key("debug_mode"));
        assert!(info.contains_key("log_level"));
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
        assert!(info.contains_key("environment"));
        assert!(info.contains_key("rust_version"));
    }
}

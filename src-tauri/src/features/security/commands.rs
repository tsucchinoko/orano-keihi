use crate::features::security::service::SecurityService;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

/// トークン暗号化リクエスト
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptTokenRequest {
    /// トークンID
    pub token_id: String,
    /// 暗号化するトークン
    pub token: String,
}

/// トークン暗号化レスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptTokenResponse {
    /// 暗号化されたトークン
    pub encrypted_token: String,
}

/// トークン復号化リクエスト
#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptTokenRequest {
    /// トークンID
    pub token_id: String,
    /// 暗号化されたトークン
    pub encrypted_token: String,
}

/// トークン復号化レスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptTokenResponse {
    /// 復号化されたトークン
    pub decrypted_token: String,
}

/// 複数トークン暗号化リクエスト
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptMultipleTokensRequest {
    /// 暗号化するトークンのマップ
    pub tokens: HashMap<String, String>,
}

/// 複数トークン暗号化レスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptMultipleTokensResponse {
    /// 暗号化されたトークンのマップ
    pub encrypted_tokens: HashMap<String, String>,
}

/// API認証検証リクエスト
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyApiRequestRequest {
    /// 認証トークン
    pub token: String,
}

/// API認証検証レスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyApiRequestResponse {
    /// 認証が有効かどうか
    pub is_valid: bool,
}

/// セキュリティ統計レスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityStatsResponse {
    /// 統計情報
    pub stats: HashMap<String, serde_json::Value>,
}

/// 認証トークンを暗号化して保存する
///
/// # 引数
/// * `request` - 暗号化リクエスト
/// * `security_service` - セキュリティサービス
///
/// # 戻り値
/// 暗号化されたトークン
#[tauri::command]
pub async fn encrypt_and_store_token(
    request: EncryptTokenRequest,
    security_service: State<'_, SecurityService>,
) -> Result<EncryptTokenResponse, String> {
    log::debug!(
        "トークン暗号化コマンドを実行: token_id={}",
        request.token_id
    );

    let encrypted_token = security_service
        .encrypt_and_store_token(&request.token_id, &request.token)
        .map_err(|e| {
            log::error!("トークン暗号化エラー: {e}");
            format!("トークンの暗号化に失敗しました: {e}")
        })?;

    log::info!(
        "トークン暗号化コマンドが完了しました: token_id={}",
        request.token_id
    );
    Ok(EncryptTokenResponse { encrypted_token })
}

/// 暗号化されたトークンを復号化する
///
/// # 引数
/// * `request` - 復号化リクエスト
/// * `security_service` - セキュリティサービス
///
/// # 戻り値
/// 復号化されたトークン
#[tauri::command]
pub async fn decrypt_token(
    request: DecryptTokenRequest,
    security_service: State<'_, SecurityService>,
) -> Result<DecryptTokenResponse, String> {
    log::debug!(
        "トークン復号化コマンドを実行: token_id={}",
        request.token_id
    );

    let decrypted_token = security_service
        .decrypt_token(&request.token_id, &request.encrypted_token)
        .map_err(|e| {
            log::error!("トークン復号化エラー: {e}");
            format!("トークンの復号化に失敗しました: {e}")
        })?;

    log::debug!(
        "トークン復号化コマンドが完了しました: token_id={}",
        request.token_id
    );
    Ok(DecryptTokenResponse { decrypted_token })
}

/// 複数のトークンを一括暗号化する
///
/// # 引数
/// * `request` - 複数トークン暗号化リクエスト
/// * `security_service` - セキュリティサービス
///
/// # 戻り値
/// 暗号化されたトークンのマップ
#[tauri::command]
pub async fn encrypt_multiple_tokens(
    request: EncryptMultipleTokensRequest,
    security_service: State<'_, SecurityService>,
) -> Result<EncryptMultipleTokensResponse, String> {
    log::debug!(
        "複数トークン暗号化コマンドを実行: count={}",
        request.tokens.len()
    );

    let encrypted_tokens = security_service
        .encrypt_multiple_tokens(&request.tokens)
        .map_err(|e| {
            log::error!("複数トークン暗号化エラー: {e}");
            format!("複数トークンの暗号化に失敗しました: {e}")
        })?;

    log::info!(
        "複数トークン暗号化コマンドが完了しました: count={}",
        encrypted_tokens.len()
    );
    Ok(EncryptMultipleTokensResponse { encrypted_tokens })
}

/// APIリクエストの認証を検証する
///
/// # 引数
/// * `request` - API認証検証リクエスト
/// * `security_service` - セキュリティサービス
///
/// # 戻り値
/// 認証検証結果
#[tauri::command]
pub async fn verify_api_request(
    request: VerifyApiRequestRequest,
    security_service: State<'_, SecurityService>,
) -> Result<VerifyApiRequestResponse, String> {
    log::debug!("API認証検証コマンドを実行");

    let is_valid = security_service
        .verify_api_request(&request.token)
        .map_err(|e| {
            log::error!("API認証検証エラー: {e}");
            format!("API認証の検証に失敗しました: {e}")
        })?;

    log::debug!("API認証検証コマンドが完了しました: is_valid={is_valid}");
    Ok(VerifyApiRequestResponse { is_valid })
}

/// トークンを無効化する
///
/// # 引数
/// * `token_id` - 無効化するトークンID
/// * `security_service` - セキュリティサービス
///
/// # 戻り値
/// 処理結果
#[tauri::command]
pub async fn invalidate_token(
    token_id: String,
    security_service: State<'_, SecurityService>,
) -> Result<(), String> {
    log::debug!("トークン無効化コマンドを実行: token_id={token_id}");

    security_service.invalidate_token(&token_id).map_err(|e| {
        log::error!("トークン無効化エラー: {e}");
        format!("トークンの無効化に失敗しました: {e}")
    })?;

    log::info!("トークン無効化コマンドが完了しました: token_id={token_id}");
    Ok(())
}

/// すべてのトークンを無効化する
///
/// # 引数
/// * `security_service` - セキュリティサービス
///
/// # 戻り値
/// 無効化されたトークン数
#[tauri::command]
pub async fn invalidate_all_tokens(
    security_service: State<'_, SecurityService>,
) -> Result<usize, String> {
    log::debug!("全トークン無効化コマンドを実行");

    let count = security_service.invalidate_all_tokens().map_err(|e| {
        log::error!("全トークン無効化エラー: {e}");
        format!("全トークンの無効化に失敗しました: {e}")
    })?;

    log::info!("全トークン無効化コマンドが完了しました: count={count}");
    Ok(count)
}

/// セキュリティ統計情報を取得する
///
/// # 引数
/// * `security_service` - セキュリティサービス
///
/// # 戻り値
/// セキュリティ統計情報
#[tauri::command]
pub async fn get_security_stats(
    security_service: State<'_, SecurityService>,
) -> Result<SecurityStatsResponse, String> {
    log::debug!("セキュリティ統計取得コマンドを実行");

    let stats = security_service.get_security_stats().map_err(|e| {
        log::error!("セキュリティ統計取得エラー: {e}");
        format!("セキュリティ統計の取得に失敗しました: {e}")
    })?;

    log::debug!("セキュリティ統計取得コマンドが完了しました");
    Ok(SecurityStatsResponse { stats })
}

/// 期限切れトークンをクリーンアップする
///
/// # 引数
/// * `max_age_hours` - 最大保持時間（時間）
/// * `security_service` - セキュリティサービス
///
/// # 戻り値
/// 削除されたトークン数
#[tauri::command]
pub async fn cleanup_expired_tokens(
    max_age_hours: i64,
    security_service: State<'_, SecurityService>,
) -> Result<usize, String> {
    log::debug!("期限切れトークンクリーンアップコマンドを実行: max_age_hours={max_age_hours}");

    let removed_count = security_service
        .cleanup_expired_tokens(max_age_hours)
        .map_err(|e| {
            log::error!("期限切れトークンクリーンアップエラー: {e}");
            format!("期限切れトークンのクリーンアップに失敗しました: {e}")
        })?;

    log::info!(
        "期限切れトークンクリーンアップコマンドが完了しました: removed_count={removed_count}"
    );
    Ok(removed_count)
}

/// 不正アクセスを検出して処理する
///
/// # 引数
/// * `request_info` - リクエスト情報
/// * `token` - 認証トークン（オプション）
/// * `security_service` - セキュリティサービス
///
/// # 戻り値
/// 処理結果
#[tauri::command]
pub async fn detect_unauthorized_access(
    request_info: String,
    token: Option<String>,
    security_service: State<'_, SecurityService>,
) -> Result<(), String> {
    log::debug!("不正アクセス検出コマンドを実行: request_info={request_info}");

    security_service
        .detect_unauthorized_access(&request_info, token.as_deref())
        .map_err(|e| {
            log::error!("不正アクセス検出エラー: {e}");
            format!("不正アクセスの検出処理に失敗しました: {e}")
        })?;

    log::info!("不正アクセス検出コマンドが完了しました");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::security::models::SecurityConfig;

    fn setup_test_security_service() -> SecurityService {
        let config = SecurityConfig {
            encryption_key: "test_encryption_key_32_bytes_long".to_string(),
            max_token_age_hours: 24,
            enable_audit_logging: true,
        };

        SecurityService::new(config).unwrap()
    }

    #[tokio::test]
    async fn test_encrypt_and_store_token_command() {
        let service = setup_test_security_service();
        let request = EncryptTokenRequest {
            token_id: "test_token".to_string(),
            token: "test_value".to_string(),
        };

        // Stateをモックするのは複雑なので、直接サービスメソッドをテスト
        let result = service.encrypt_and_store_token(&request.token_id, &request.token);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_api_request_command() {
        let service = setup_test_security_service();
        let token = "test_token";

        let encrypted_token = service.token_encryption.encrypt_token(token).unwrap();
        let result = service.verify_api_request(&encrypted_token);

        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}

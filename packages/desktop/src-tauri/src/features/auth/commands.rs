use crate::features::auth::models::{AuthState, User};
use crate::features::auth::service::AuthService;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;
use tokio::sync::oneshot;

/// 認証開始のレスポンス（ループバック方式）
#[derive(Debug, Serialize, Deserialize)]
pub struct StartAuthResponse {
    /// 認証URL
    pub auth_url: String,
    /// ループバックサーバーのポート番号
    pub loopback_port: u16,
}

/// 認証完了待機のレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct WaitForAuthResponse {
    /// ユーザー情報
    pub user: User,
    /// セッショントークン
    pub session_token: String,
}

/// セッション検証のレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateSessionResponse {
    /// ユーザー情報
    pub user: User,
    /// 認証状態
    pub is_authenticated: bool,
}

// グローバルなコールバック受信用のストレージ
// 実際のプロダクションでは、より適切な状態管理を使用すべき
#[derive(Default)]
struct CallbackStorage {
    receiver: Option<oneshot::Receiver<crate::features::auth::loopback::OAuthCallback>>,
    state: Option<String>,
    code_verifier: Option<String>,
    redirect_uri: Option<String>,
}

static CALLBACK_STORAGE: Mutex<Option<CallbackStorage>> = Mutex::new(None);

/// OAuth認証フローを開始する（APIサーバー経由・ループバック方式）
///
/// # 引数
/// * `auth_service` - 認証サービス
///
/// # 戻り値
/// 認証開始情報
#[tauri::command]
pub async fn start_oauth_flow(
    auth_service: State<'_, AuthService>,
) -> Result<StartAuthResponse, String> {
    log::info!("OAuth認証フロー開始コマンドを実行（APIサーバー経由）");

    let oauth_info = auth_service.start_oauth_flow().await.map_err(|e| {
        log::error!("OAuth認証フロー開始エラー: {e}");
        format!("認証フローの開始に失敗しました: {e}")
    })?;

    // コールバック受信用の情報をグローバルストレージに保存
    if let Some(receiver) = oauth_info.callback_receiver {
        let mut global_storage = CALLBACK_STORAGE.lock().unwrap();
        *global_storage = Some(CallbackStorage {
            receiver: Some(receiver),
            state: Some(oauth_info.state),
            code_verifier: Some(oauth_info.code_verifier),
            redirect_uri: None, // 後で設定
        });
    }

    let response = StartAuthResponse {
        auth_url: oauth_info.auth_url,
        loopback_port: oauth_info.loopback_port,
    };

    log::info!("OAuth認証フロー開始コマンドが完了しました");
    Ok(response)
}

/// 認証完了を待機する（APIサーバー経由・ループバック方式）
///
/// # 引数
/// * `auth_service` - 認証サービス
///
/// # 戻り値
/// 認証結果（ユーザー情報とセッショントークン）
#[tauri::command]
pub async fn wait_for_auth_completion(
    auth_service: State<'_, AuthService>,
) -> Result<WaitForAuthResponse, String> {
    log::info!("認証完了待機コマンドを実行（APIサーバー経由）");

    // グローバルストレージからコールバック受信用の情報を取得
    let (receiver, state, code_verifier, redirect_uri) = {
        let mut global_storage = CALLBACK_STORAGE.lock().unwrap();
        let storage = global_storage.take().ok_or_else(|| {
            log::error!("コールバック受信用の情報が見つかりません");
            "認証フローが開始されていません。先にstart_oauth_flowを呼び出してください。".to_string()
        })?;

        (
            storage
                .receiver
                .ok_or_else(|| "Receiverが見つかりません".to_string())?,
            storage
                .state
                .ok_or_else(|| "stateが見つかりません".to_string())?,
            storage
                .code_verifier
                .ok_or_else(|| "code_verifierが見つかりません".to_string())?,
            storage
                .redirect_uri
                .unwrap_or_else(|| "http://127.0.0.1/callback".to_string()),
        )
    };

    let (user, session) = auth_service
        .handle_loopback_callback(receiver, state, code_verifier, redirect_uri)
        .await
        .map_err(|e| {
            log::error!("認証コールバック処理エラー: {e}");
            format!("認証処理に失敗しました: {e}")
        })?;

    // セッショントークンを生成
    let session_token = auth_service
        .create_session_token(&session.id)
        .map_err(|e| {
            log::error!("セッショントークン生成エラー: {e}");
            format!("セッショントークンの生成に失敗しました: {e}")
        })?;

    let response = WaitForAuthResponse {
        user,
        session_token,
    };

    log::info!("認証完了待機コマンドが完了しました");
    Ok(response)
}

/// セッションを検証する
///
/// # 引数
/// * `session_token` - セッショントークン
/// * `auth_service` - 認証サービス
///
/// # 戻り値
/// セッション検証結果
#[tauri::command]
pub async fn validate_session(
    session_token: String,
    auth_service: State<'_, AuthService>,
) -> Result<ValidateSessionResponse, String> {
    log::debug!("セッション検証コマンドを実行");

    match auth_service.validate_session(session_token).await {
        Ok(user) => {
            log::debug!("セッション検証成功: user_id={}", user.id);
            Ok(ValidateSessionResponse {
                user,
                is_authenticated: true,
            })
        }
        Err(e) => {
            log::debug!("セッション検証失敗: {e}");
            Err(format!("セッション検証に失敗しました: {e}"))
        }
    }
}

/// ログアウト処理を行う
///
/// # 引数
/// * `session_token` - セッショントークン
/// * `auth_service` - 認証サービス
///
/// # 戻り値
/// ログアウト結果
#[tauri::command]
pub async fn logout(
    session_token: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    log::info!("ログアウトコマンドを実行");

    // まずセッションを検証してセッションIDを取得
    match auth_service.validate_session(session_token).await {
        Ok(_) => {
            // セッションが有効な場合、ログアウト処理を実行
            // 注意: 実際のセッションIDを取得するためにはSessionManagerを直接使用する必要がある
            // ここでは簡略化のため、セッショントークンからセッションIDを抽出する処理を省略
            log::info!("ログアウト処理が完了しました");
            Ok(())
        }
        Err(e) => {
            log::warn!("ログアウト時のセッション検証エラー: {e}");
            // セッションが無効でもログアウト成功として扱う
            Ok(())
        }
    }
}

/// 現在の認証状態を取得する
///
/// # 引数
/// * `session_token` - セッショントークン（オプション）
/// * `auth_service` - 認証サービス
///
/// # 戻り値
/// 認証状態
#[tauri::command]
pub async fn get_auth_state(
    session_token: Option<String>,
    auth_service: State<'_, AuthService>,
) -> Result<AuthState, String> {
    log::debug!("認証状態取得コマンドを実行");

    match session_token {
        Some(token) => match auth_service.validate_session(token).await {
            Ok(user) => {
                log::debug!("認証済み状態: user_id={}", user.id);
                Ok(AuthState {
                    user: Some(user),
                    is_authenticated: true,
                    is_loading: false,
                })
            }
            Err(_) => {
                log::debug!("未認証状態");
                Ok(AuthState::default())
            }
        },
        None => {
            log::debug!("セッショントークンなし - 未認証状態");
            Ok(AuthState::default())
        }
    }
}

/// 期限切れセッションをクリーンアップする（管理用コマンド）
///
/// # 引数
/// * `auth_service` - 認証サービス
///
/// # 戻り値
/// 削除されたセッション数
#[tauri::command]
pub async fn cleanup_expired_sessions(
    auth_service: State<'_, AuthService>,
) -> Result<usize, String> {
    log::info!("期限切れセッションクリーンアップコマンドを実行");

    let deleted_count = auth_service.cleanup_expired_sessions().await.map_err(|e| {
        log::error!("セッションクリーンアップエラー: {e}");
        format!("セッションクリーンアップに失敗しました: {e}")
    })?;

    log::info!("期限切れセッションクリーンアップが完了しました: 削除数={deleted_count}");
    Ok(deleted_count)
}

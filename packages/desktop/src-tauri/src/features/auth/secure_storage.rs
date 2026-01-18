/// セキュアストレージモジュール
///
/// Tauri Storeプラグインを使用して、セッショントークンやその他の秘匿情報を
/// 安全に保存・取得します。
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

/// セキュアストレージのキー定義
pub struct SecureStorageKeys;

impl SecureStorageKeys {
    /// セッショントークンのキー
    pub const SESSION_TOKEN: &'static str = "session_token";
    /// ユーザーIDのキー
    pub const USER_ID: &'static str = "user_id";
    /// 最終ログイン日時のキー
    pub const LAST_LOGIN: &'static str = "last_login";
}

/// セキュアストレージに保存する認証情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAuthInfo {
    /// セッショントークン
    pub session_token: String,
    /// ユーザーID
    pub user_id: String,
    /// 最終ログイン日時（RFC3339形式）
    pub last_login: String,
}

/// セキュアストレージサービス
#[derive(Clone)]
pub struct SecureStorage {
    /// Tauriアプリハンドル
    app_handle: Arc<AppHandle>,
    /// ストアファイル名
    store_name: String,
}

impl SecureStorage {
    /// 新しいSecureStorageを作成する
    ///
    /// # 引数
    /// * `app_handle` - Tauriアプリハンドル
    ///
    /// # 戻り値
    /// SecureStorageインスタンス
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle: Arc::new(app_handle),
            store_name: "secure.json".to_string(),
        }
    }

    /// セッショントークンを保存する
    ///
    /// # 引数
    /// * `token` - セッショントークン
    ///
    /// # 戻り値
    /// 処理結果
    pub fn save_session_token(&self, token: &str) -> Result<(), String> {
        let store = self
            .app_handle
            .store(&self.store_name)
            .map_err(|e| format!("ストアの取得に失敗しました: {e}"))?;

        store.set(SecureStorageKeys::SESSION_TOKEN, token);

        store
            .save()
            .map_err(|e| format!("ストアの保存に失敗しました: {e}"))?;

        log::info!("セッショントークンを保存しました");
        Ok(())
    }

    /// セッショントークンを取得する
    ///
    /// # 戻り値
    /// セッショントークン（存在しない場合はNone）
    pub fn get_session_token(&self) -> Result<Option<String>, String> {
        let store = self
            .app_handle
            .store(&self.store_name)
            .map_err(|e| format!("ストアの取得に失敗しました: {e}"))?;

        let token = store
            .get(SecureStorageKeys::SESSION_TOKEN)
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        Ok(token)
    }

    /// ユーザーIDを保存する
    ///
    /// # 引数
    /// * `user_id` - ユーザーID
    ///
    /// # 戻り値
    /// 処理結果
    pub fn save_user_id(&self, user_id: &str) -> Result<(), String> {
        let store = self
            .app_handle
            .store(&self.store_name)
            .map_err(|e| format!("ストアの取得に失敗しました: {e}"))?;

        store.set(SecureStorageKeys::USER_ID, user_id);

        store
            .save()
            .map_err(|e| format!("ストアの保存に失敗しました: {e}"))?;

        log::debug!("ユーザーIDを保存しました: user_id={user_id}");
        Ok(())
    }

    /// ユーザーIDを取得する
    ///
    /// # 戻り値
    /// ユーザーID（存在しない場合はNone）
    pub fn get_user_id(&self) -> Result<Option<String>, String> {
        let store = self
            .app_handle
            .store(&self.store_name)
            .map_err(|e| format!("ストアの取得に失敗しました: {e}"))?;

        let user_id = store
            .get(SecureStorageKeys::USER_ID)
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        Ok(user_id)
    }

    /// 最終ログイン日時を保存する
    ///
    /// # 引数
    /// * `last_login` - 最終ログイン日時（RFC3339形式）
    ///
    /// # 戻り値
    /// 処理結果
    pub fn save_last_login(&self, last_login: &str) -> Result<(), String> {
        let store = self
            .app_handle
            .store(&self.store_name)
            .map_err(|e| format!("ストアの取得に失敗しました: {e}"))?;

        store.set(SecureStorageKeys::LAST_LOGIN, last_login);

        store
            .save()
            .map_err(|e| format!("ストアの保存に失敗しました: {e}"))?;

        log::debug!("最終ログイン日時を保存しました: last_login={last_login}");
        Ok(())
    }

    /// 最終ログイン日時を取得する
    ///
    /// # 戻り値
    /// 最終ログイン日時（存在しない場合はNone）
    pub fn get_last_login(&self) -> Result<Option<String>, String> {
        let store = self
            .app_handle
            .store(&self.store_name)
            .map_err(|e| format!("ストアの取得に失敗しました: {e}"))?;

        let last_login = store
            .get(SecureStorageKeys::LAST_LOGIN)
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        Ok(last_login)
    }

    /// 認証情報をまとめて保存する
    ///
    /// # 引数
    /// * `auth_info` - 認証情報
    ///
    /// # 戻り値
    /// 処理結果
    pub fn save_auth_info(&self, auth_info: &StoredAuthInfo) -> Result<(), String> {
        self.save_session_token(&auth_info.session_token)?;
        self.save_user_id(&auth_info.user_id)?;
        self.save_last_login(&auth_info.last_login)?;

        log::info!("認証情報を保存しました: user_id={}", auth_info.user_id);
        Ok(())
    }

    /// 認証情報をまとめて取得する
    ///
    /// # 戻り値
    /// 認証情報（存在しない場合はNone）
    pub fn get_auth_info(&self) -> Result<Option<StoredAuthInfo>, String> {
        let session_token = self.get_session_token()?;
        let user_id = self.get_user_id()?;
        let last_login = self.get_last_login()?;

        match (session_token, user_id, last_login) {
            (Some(session_token), Some(user_id), Some(last_login)) => Ok(Some(StoredAuthInfo {
                session_token,
                user_id,
                last_login,
            })),
            _ => Ok(None),
        }
    }

    /// すべての認証情報を削除する（ログアウト時）
    ///
    /// # 戻り値
    /// 処理結果
    pub fn clear_auth_info(&self) -> Result<(), String> {
        let store = self
            .app_handle
            .store(&self.store_name)
            .map_err(|e| format!("ストアの取得に失敗しました: {e}"))?;

        store.delete(SecureStorageKeys::SESSION_TOKEN);
        store.delete(SecureStorageKeys::USER_ID);
        store.delete(SecureStorageKeys::LAST_LOGIN);

        store
            .save()
            .map_err(|e| format!("ストアの保存に失敗しました: {e}"))?;

        log::info!("認証情報を削除しました");
        Ok(())
    }

    /// ストアをクリアする（デバッグ用）
    ///
    /// # 戻り値
    /// 処理結果
    pub fn clear_all(&self) -> Result<(), String> {
        let store = self
            .app_handle
            .store(&self.store_name)
            .map_err(|e| format!("ストアの取得に失敗しました: {e}"))?;

        store.clear();

        store
            .save()
            .map_err(|e| format!("ストアの保存に失敗しました: {e}"))?;

        log::warn!("ストアをクリアしました");
        Ok(())
    }
}

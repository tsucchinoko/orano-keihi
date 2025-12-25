use crate::features::auth::models::{Session, SessionError};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

/// セッション管理を行う構造体
#[derive(Clone)]
pub struct SessionManager {
    /// データベース接続
    db_connection: Arc<Mutex<Connection>>,
    /// 暗号化キー
    encryption_key: Vec<u8>,
}

impl SessionManager {
    /// 新しいSessionManagerを作成する
    ///
    /// # 引数
    /// * `db_connection` - データベース接続
    /// * `encryption_key` - セッション暗号化用のキー
    ///
    /// # 戻り値
    /// SessionManagerインスタンス
    pub fn new(db_connection: Arc<Mutex<Connection>>, encryption_key: String) -> Self {
        // 暗号化キーを32バイトに調整
        let mut key_bytes = encryption_key.as_bytes().to_vec();
        key_bytes.resize(32, 0); // 32バイトに調整（不足分は0で埋める）

        Self {
            db_connection,
            encryption_key: key_bytes,
        }
    }

    /// セッションを作成する
    ///
    /// # 引数
    /// * `user_id` - ユーザーID
    ///
    /// # 戻り値
    /// 作成されたセッション情報
    pub fn create_session(&self, user_id: i64) -> Result<Session, SessionError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + Duration::days(30); // 30日間有効

        let session = Session {
            id: session_id.clone(),
            user_id,
            expires_at,
            created_at: now,
        };

        // データベースにセッションを保存
        let conn = self.db_connection.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, user_id, expires_at, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                session_id,
                user_id,
                expires_at.to_rfc3339(),
                now.to_rfc3339()
            ],
        )?;

        log::info!("セッションを作成しました: user_id={user_id}, session_id={session_id}");
        Ok(session)
    }

    /// セッションを検証する
    ///
    /// # 引数
    /// * `token` - 暗号化されたセッショントークン
    ///
    /// # 戻り値
    /// 検証されたセッション情報
    pub fn validate_session(&self, token: String) -> Result<Session, SessionError> {
        // トークンを復号化してセッションIDを取得
        let session_id = self.decrypt_token(&token)?;

        // データベースからセッションを取得
        let conn = self.db_connection.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, user_id, expires_at, created_at FROM sessions WHERE id = ?1")
            .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

        let session_result = stmt.query_row(params![session_id], |row| {
            let expires_at_str: String = row.get(2)?;
            let created_at_str: String = row.get(3)?;

            let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)
                .map_err(|_e| {
                    rusqlite::Error::InvalidColumnType(
                        2,
                        "expires_at".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?
                .with_timezone(&Utc);

            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|_e| {
                    rusqlite::Error::InvalidColumnType(
                        3,
                        "created_at".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?
                .with_timezone(&Utc);

            Ok(Session {
                id: row.get(0)?,
                user_id: row.get(1)?,
                expires_at,
                created_at,
            })
        });

        let session = match session_result {
            Ok(session) => session,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(SessionError::NotFound);
            }
            Err(e) => {
                return Err(SessionError::DatabaseError(e.to_string()));
            }
        };

        // セッションの有効期限をチェック
        if session.expires_at < Utc::now() {
            // 期限切れセッションを削除
            let _ = self.invalidate_session(&session.id);
            return Err(SessionError::Expired);
        }

        log::debug!(
            "セッションを検証しました: user_id={}, session_id={}",
            session.user_id,
            session.id
        );
        Ok(session)
    }

    /// セッションを無効化する
    ///
    /// # 引数
    /// * `session_id` - セッションID
    ///
    /// # 戻り値
    /// 処理結果
    pub fn invalidate_session(&self, session_id: &str) -> Result<(), SessionError> {
        let conn = self.db_connection.lock().unwrap();
        let affected_rows =
            conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;

        if affected_rows > 0 {
            log::info!("セッションを無効化しました: session_id={session_id}");
        } else {
            log::warn!("無効化対象のセッションが見つかりませんでした: session_id={session_id}");
        }

        Ok(())
    }

    /// セッションIDを暗号化してトークンを生成する
    ///
    /// # 引数
    /// * `session_id` - セッションID
    ///
    /// # 戻り値
    /// 暗号化されたトークン
    pub fn encrypt_session_id(&self, session_id: &str) -> Result<String, SessionError> {
        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
            .map_err(|e| SessionError::EncryptionError(e.to_string()))?;

        // ランダムなナンス（12バイト）を生成
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // セッションIDを暗号化
        let ciphertext = cipher
            .encrypt(nonce, session_id.as_bytes())
            .map_err(|e| SessionError::EncryptionError(e.to_string()))?;

        // ナンスと暗号文を結合してBase64エンコード
        let mut token_bytes = nonce_bytes.to_vec();
        token_bytes.extend_from_slice(&ciphertext);
        let token = general_purpose::STANDARD.encode(&token_bytes);

        Ok(token)
    }

    /// トークンを復号化してセッションIDを取得する
    ///
    /// # 引数
    /// * `token` - 暗号化されたトークン
    ///
    /// # 戻り値
    /// セッションID
    fn decrypt_token(&self, token: &str) -> Result<String, SessionError> {
        // Base64デコード
        let token_bytes = general_purpose::STANDARD
            .decode(token)
            .map_err(|e| SessionError::DecryptionError(format!("Base64デコードエラー: {e}")))?;

        if token_bytes.len() < 12 {
            return Err(SessionError::DecryptionError(
                "トークンが短すぎます".to_string(),
            ));
        }

        // ナンスと暗号文を分離
        let (nonce_bytes, ciphertext) = token_bytes.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
            .map_err(|e| SessionError::DecryptionError(e.to_string()))?;

        // 復号化
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| SessionError::DecryptionError(e.to_string()))?;

        let session_id = String::from_utf8(plaintext)
            .map_err(|e| SessionError::DecryptionError(format!("UTF-8変換エラー: {e}")))?;

        Ok(session_id)
    }

    /// 期限切れセッションをクリーンアップする
    ///
    /// # 戻り値
    /// 削除されたセッション数
    pub fn cleanup_expired_sessions(&self) -> Result<usize, SessionError> {
        let now = Utc::now();
        let conn = self.db_connection.lock().unwrap();

        let affected_rows = conn.execute(
            "DELETE FROM sessions WHERE expires_at < ?1",
            params![now.to_rfc3339()],
        )?;

        if affected_rows > 0 {
            log::info!("期限切れセッションを{affected_rows}件削除しました");
        }

        Ok(affected_rows)
    }

    /// ユーザーのすべてのセッションを無効化する
    ///
    /// # 引数
    /// * `user_id` - ユーザーID
    ///
    /// # 戻り値
    /// 削除されたセッション数
    pub fn invalidate_user_sessions(&self, user_id: i64) -> Result<usize, SessionError> {
        let conn = self.db_connection.lock().unwrap();

        let affected_rows =
            conn.execute("DELETE FROM sessions WHERE user_id = ?1", params![user_id])?;

        log::info!("ユーザー{user_id}のセッションを{affected_rows}件無効化しました");
        Ok(affected_rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::database::connection::create_in_memory_connection;

    fn setup_test_session_manager() -> SessionManager {
        let conn = create_in_memory_connection().unwrap();

        // テスト用のセッションテーブルを作成
        conn.execute(
            "CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        SessionManager::new(
            Arc::new(Mutex::new(conn)),
            "test_encryption_key_32_bytes_long".to_string(),
        )
    }

    #[test]
    fn test_create_session() {
        let session_manager = setup_test_session_manager();
        let user_id = 1;

        let session = session_manager.create_session(user_id).unwrap();

        assert_eq!(session.user_id, user_id);
        assert!(!session.id.is_empty());
        assert!(session.expires_at > Utc::now());
    }

    #[test]
    fn test_encrypt_decrypt_session_id() {
        let session_manager = setup_test_session_manager();
        let session_id = "test-session-id";

        let token = session_manager.encrypt_session_id(session_id).unwrap();
        let decrypted_id = session_manager.decrypt_token(&token).unwrap();

        assert_eq!(session_id, decrypted_id);
    }

    #[test]
    fn test_validate_session() {
        let session_manager = setup_test_session_manager();
        let user_id = 1;

        // セッションを作成
        let session = session_manager.create_session(user_id).unwrap();

        // セッションIDを暗号化
        let token = session_manager.encrypt_session_id(&session.id).unwrap();

        // セッションを検証
        let validated_session = session_manager.validate_session(token).unwrap();

        assert_eq!(validated_session.id, session.id);
        assert_eq!(validated_session.user_id, user_id);
    }

    #[test]
    fn test_invalidate_session() {
        let session_manager = setup_test_session_manager();
        let user_id = 1;

        // セッションを作成
        let session = session_manager.create_session(user_id).unwrap();

        // セッションを無効化
        session_manager.invalidate_session(&session.id).unwrap();

        // セッションIDを暗号化して検証を試行
        let token = session_manager.encrypt_session_id(&session.id).unwrap();
        let result = session_manager.validate_session(token);

        assert!(matches!(result, Err(SessionError::NotFound)));
    }
}

use crate::features::auth::models::{AuthError, GoogleUser, User};
use chrono::{DateTime, Utc};
use chrono_tz::Asia::Tokyo;
use rusqlite::{params, Connection, Row};
use std::sync::{Arc, Mutex};

/// ユーザーデータのリポジトリ
///
/// Googleユーザー情報からのユーザー作成・取得、
/// ユーザーデータのCRUD操作、
/// ユーザーIDによるデータフィルタリングを提供する
#[derive(Clone)]
pub struct UserRepository {
    /// データベース接続
    db_connection: Arc<Mutex<Connection>>,
}

impl UserRepository {
    /// 新しいUserRepositoryインスタンスを作成する
    ///
    /// # 引数
    /// * `db_connection` - データベース接続
    ///
    /// # 戻り値
    /// UserRepositoryインスタンス
    pub fn new(db_connection: Arc<Mutex<Connection>>) -> Self {
        Self { db_connection }
    }

    /// Googleユーザー情報からユーザーを作成または取得する
    ///
    /// # 引数
    /// * `google_user` - Googleから取得したユーザー情報
    ///
    /// # 戻り値
    /// 作成または取得されたユーザー情報、失敗時はエラー
    ///
    /// # 処理内容
    /// 1. GoogleIDでユーザーを検索
    /// 2. 存在する場合は既存ユーザーを返す
    /// 3. 存在しない場合は新規ユーザーを作成して返す
    pub async fn find_or_create_user(&self, google_user: GoogleUser) -> Result<User, AuthError> {
        let conn = self
            .db_connection
            .lock()
            .map_err(|e| AuthError::DatabaseError(format!("データベースロック取得失敗: {e}")))?;

        // usersテーブルが存在するかチェック
        if !self.check_users_table_exists(&conn)? {
            return Err(AuthError::DatabaseError(
                "usersテーブルが存在しません。データベースマイグレーションを実行してください。"
                    .to_string(),
            ));
        }

        // 既存ユーザーを検索
        if let Some(existing_user) = self.get_user_by_google_id_internal(&conn, &google_user.id)? {
            // 既存ユーザーの情報を更新
            self.update_user_info(&conn, &existing_user, &google_user)?;
            // 更新後のユーザー情報を取得して返す
            return self
                .get_user_by_google_id_internal(&conn, &google_user.id)?
                .ok_or_else(|| AuthError::DatabaseError("更新後のユーザー取得に失敗".to_string()));
        }

        // 新規ユーザーを作成
        self.create_new_user(&conn, &google_user)
    }

    /// ユーザーIDでユーザーを取得する
    ///
    /// # 引数
    /// * `user_id` - ユーザーID（nanoId形式）
    ///
    /// # 戻り値
    /// ユーザー情報（存在しない場合はNone）、失敗時はエラー
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<User>, AuthError> {
        let conn = self
            .db_connection
            .lock()
            .map_err(|e| AuthError::DatabaseError(format!("データベースロック取得失敗: {e}")))?;

        // usersテーブルが存在するかチェック
        if !self.check_users_table_exists(&conn)? {
            log::warn!("usersテーブルが存在しないため、ユーザー取得をスキップします");
            return Ok(None);
        }

        self.get_user_by_id_internal(&conn, user_id)
    }

    /// GoogleIDでユーザーを取得する
    ///
    /// # 引数
    /// * `google_id` - GoogleユーザーID
    ///
    /// # 戻り値
    /// ユーザー情報（存在しない場合はNone）、失敗時はエラー
    pub async fn get_user_by_google_id(
        &self,
        google_id: String,
    ) -> Result<Option<User>, AuthError> {
        let conn = self
            .db_connection
            .lock()
            .map_err(|e| AuthError::DatabaseError(format!("データベースロック取得失敗: {e}")))?;

        // usersテーブルが存在するかチェック
        if !self.check_users_table_exists(&conn)? {
            log::warn!("usersテーブルが存在しないため、ユーザー取得をスキップします");
            return Ok(None);
        }

        self.get_user_by_google_id_internal(&conn, &google_id)
    }

    /// ユーザー情報を更新する
    ///
    /// # 引数
    /// * `user` - 更新するユーザー情報
    ///
    /// # 戻り値
    /// 更新されたユーザー情報、失敗時はエラー
    pub async fn update_user(&self, user: &User) -> Result<User, AuthError> {
        let conn = self
            .db_connection
            .lock()
            .map_err(|e| AuthError::DatabaseError(format!("データベースロック取得失敗: {e}")))?;

        // 更新日時をJSTで生成
        let now_jst = Utc::now().with_timezone(&Tokyo);
        let updated_at = now_jst.to_rfc3339();

        conn.execute(
            "UPDATE users SET email = ?1, name = ?2, picture_url = ?3, updated_at = ?4 WHERE id = ?5",
            params![user.email, user.name, user.picture_url, updated_at, user.id],
        )?;

        // 更新後のユーザー情報を取得
        self.get_user_by_id_internal(&conn, &user.id)?
            .ok_or_else(|| AuthError::DatabaseError("更新後のユーザー取得に失敗".to_string()))
    }

    /// ユーザーを削除する
    ///
    /// # 引数
    /// * `user_id` - 削除するユーザーのID（nanoId形式）
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー
    ///
    /// # 注意
    /// 外部キー制約により、関連するセッションも自動的に削除される
    pub async fn delete_user(&self, user_id: &str) -> Result<(), AuthError> {
        let conn = self
            .db_connection
            .lock()
            .map_err(|e| AuthError::DatabaseError(format!("データベースロック取得失敗: {e}")))?;

        let affected_rows = conn.execute("DELETE FROM users WHERE id = ?1", params![user_id])?;

        if affected_rows == 0 {
            return Err(AuthError::DatabaseError(
                "削除対象のユーザーが見つかりません".to_string(),
            ));
        }

        Ok(())
    }

    /// すべてのユーザーを取得する（管理用）
    ///
    /// # 戻り値
    /// ユーザーリスト、失敗時はエラー
    pub async fn get_all_users(&self) -> Result<Vec<User>, AuthError> {
        let conn = self
            .db_connection
            .lock()
            .map_err(|e| AuthError::DatabaseError(format!("データベースロック取得失敗: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, google_id, email, name, picture_url, created_at, updated_at 
             FROM users 
             ORDER BY created_at DESC",
        )?;

        let user_iter = stmt.query_map([], |row| self.row_to_user(row))?;

        let mut users = Vec::new();
        for user in user_iter {
            users.push(user?);
        }

        Ok(users)
    }

    /// 内部用：ユーザーIDでユーザーを取得する
    fn get_user_by_id_internal(
        &self,
        conn: &Connection,
        user_id: &str,
    ) -> Result<Option<User>, AuthError> {
        let mut stmt = conn.prepare(
            "SELECT id, google_id, email, name, picture_url, created_at, updated_at 
             FROM users 
             WHERE id = ?1",
        )?;

        let mut user_iter = stmt.query_map(params![user_id], |row| self.row_to_user(row))?;

        match user_iter.next() {
            Some(user) => Ok(Some(user?)),
            None => Ok(None),
        }
    }

    /// 内部用：GoogleIDでユーザーを取得する
    fn get_user_by_google_id_internal(
        &self,
        conn: &Connection,
        google_id: &str,
    ) -> Result<Option<User>, AuthError> {
        let mut stmt = conn.prepare(
            "SELECT id, google_id, email, name, picture_url, created_at, updated_at 
             FROM users 
             WHERE google_id = ?1",
        )?;

        let mut user_iter = stmt.query_map(params![google_id], |row| self.row_to_user(row))?;

        match user_iter.next() {
            Some(user) => Ok(Some(user?)),
            None => Ok(None),
        }
    }

    /// 新規ユーザーを作成する
    fn create_new_user(
        &self,
        conn: &Connection,
        google_user: &GoogleUser,
    ) -> Result<User, AuthError> {
        // nanoIdを生成
        let user_id = crate::shared::utils::nanoid::generate_user_id();

        // 作成日時をJSTで生成
        let now_jst = Utc::now().with_timezone(&Tokyo);
        let timestamp = now_jst.to_rfc3339();

        // ユーザーを挿入
        conn.execute(
            "INSERT INTO users (id, google_id, email, name, picture_url, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                user_id,
                google_user.id,
                google_user.email,
                google_user.name,
                google_user.picture,
                timestamp,
                timestamp
            ],
        )?;

        // 作成されたユーザー情報を取得して返す
        self.get_user_by_id_internal(conn, &user_id)?
            .ok_or_else(|| AuthError::DatabaseError("作成されたユーザーの取得に失敗".to_string()))
    }

    /// 既存ユーザーの情報を更新する
    fn update_user_info(
        &self,
        conn: &Connection,
        existing_user: &User,
        google_user: &GoogleUser,
    ) -> Result<(), AuthError> {
        // 更新が必要かチェック
        let needs_update = existing_user.email != google_user.email
            || existing_user.name != google_user.name
            || existing_user.picture_url != google_user.picture;

        if !needs_update {
            return Ok(());
        }

        // 更新日時をJSTで生成
        let now_jst = Utc::now().with_timezone(&Tokyo);
        let updated_at = now_jst.to_rfc3339();

        conn.execute(
            "UPDATE users SET email = ?1, name = ?2, picture_url = ?3, updated_at = ?4 WHERE id = ?5",
            params![
                google_user.email,
                google_user.name,
                google_user.picture,
                updated_at,
                &existing_user.id
            ],
        )?;

        Ok(())
    }

    /// データベース行からUserオブジェクトを作成する
    fn row_to_user(&self, row: &Row) -> Result<User, rusqlite::Error> {
        // id: String型（nanoId形式）
        let id: String = row.get(0)?;
        let created_at_str: String = row.get(5)?;
        let updated_at_str: String = row.get(6)?;

        // RFC3339形式の文字列をDateTime<Utc>に変換
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .with_timezone(&Utc);

        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .with_timezone(&Utc);

        Ok(User {
            id,
            google_id: row.get(1)?,
            email: row.get(2)?,
            name: row.get(3)?,
            picture_url: row.get(4)?,
            created_at,
            updated_at,
        })
    }

    /// usersテーブルが存在するかチェックする
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// テーブルが存在する場合はtrue、失敗時はエラー
    fn check_users_table_exists(&self, conn: &Connection) -> Result<bool, AuthError> {
        let table_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='users'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AuthError::DatabaseError(format!("テーブル存在確認エラー: {e}")))?;

        Ok(table_exists > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::database::connection::create_in_memory_connection;
    use std::sync::{Arc, Mutex};

    /// テスト用のUserRepositoryを作成する
    fn create_test_repository() -> UserRepository {
        let conn = create_in_memory_connection().unwrap();
        let db_connection = Arc::new(Mutex::new(conn));
        UserRepository::new(db_connection)
    }

    /// テスト用のGoogleUserを作成する
    fn create_test_google_user() -> GoogleUser {
        GoogleUser {
            id: "google_123".to_string(),
            email: "test@example.com".to_string(),
            name: "テストユーザー".to_string(),
            picture: Some("https://example.com/picture.jpg".to_string()),
            verified_email: true,
        }
    }

    #[tokio::test]
    async fn test_find_or_create_user_new() {
        let repository = create_test_repository();
        let google_user = create_test_google_user();

        // 新規ユーザーを作成
        let user = repository
            .find_or_create_user(google_user.clone())
            .await
            .unwrap();

        // ユーザー情報が正しく設定されていることを確認
        assert_eq!(user.google_id, google_user.id);
        assert_eq!(user.email, google_user.email);
        assert_eq!(user.name, google_user.name);
        assert_eq!(user.picture_url, google_user.picture);
        // IDがnanoId形式（21文字）であることを確認
        assert_eq!(user.id.len(), 21);
    }

    #[tokio::test]
    async fn test_find_or_create_user_existing() {
        let repository = create_test_repository();
        let google_user = create_test_google_user();

        // 最初にユーザーを作成
        let user1 = repository
            .find_or_create_user(google_user.clone())
            .await
            .unwrap();

        // 同じGoogleユーザーで再度呼び出し
        let user2 = repository.find_or_create_user(google_user).await.unwrap();

        // 同じユーザーが返されることを確認
        assert_eq!(user1.id, user2.id);
        assert_eq!(user1.google_id, user2.google_id);
    }

    #[tokio::test]
    async fn test_get_user_by_id() {
        let repository = create_test_repository();
        let google_user = create_test_google_user();

        // ユーザーを作成
        let created_user = repository.find_or_create_user(google_user).await.unwrap();

        // IDで取得
        let retrieved_user = repository
            .get_user_by_id(&created_user.id)
            .await
            .unwrap()
            .unwrap();

        // 同じユーザー情報であることを確認
        assert_eq!(created_user.id, retrieved_user.id);
        assert_eq!(created_user.google_id, retrieved_user.google_id);
        assert_eq!(created_user.email, retrieved_user.email);
    }

    #[tokio::test]
    async fn test_get_user_by_id_not_found() {
        let repository = create_test_repository();

        // 存在しないIDで取得
        let result = repository.get_user_by_id("nonexistent_id").await.unwrap();

        // Noneが返されることを確認
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_user_by_google_id() {
        let repository = create_test_repository();
        let google_user = create_test_google_user();

        // ユーザーを作成
        let created_user = repository
            .find_or_create_user(google_user.clone())
            .await
            .unwrap();

        // GoogleIDで取得
        let retrieved_user = repository
            .get_user_by_google_id(google_user.id)
            .await
            .unwrap()
            .unwrap();

        // 同じユーザー情報であることを確認
        assert_eq!(created_user.id, retrieved_user.id);
        assert_eq!(created_user.google_id, retrieved_user.google_id);
    }

    #[tokio::test]
    async fn test_update_user() {
        let repository = create_test_repository();
        let google_user = create_test_google_user();

        // ユーザーを作成
        let mut user = repository.find_or_create_user(google_user).await.unwrap();

        // ユーザー情報を更新
        user.name = "更新されたユーザー".to_string();
        user.email = "updated@example.com".to_string();

        let updated_user = repository.update_user(&user).await.unwrap();

        // 更新された情報が反映されていることを確認
        assert_eq!(updated_user.name, "更新されたユーザー");
        assert_eq!(updated_user.email, "updated@example.com");
        assert_ne!(updated_user.updated_at, updated_user.created_at);
    }

    #[tokio::test]
    async fn test_delete_user() {
        let repository = create_test_repository();
        let google_user = create_test_google_user();

        // ユーザーを作成
        let user = repository.find_or_create_user(google_user).await.unwrap();

        // ユーザーを削除
        repository.delete_user(&user.id).await.unwrap();

        // ユーザーが削除されていることを確認
        let result = repository.get_user_by_id(&user.id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_user_not_found() {
        let repository = create_test_repository();

        // 存在しないユーザーを削除しようとする
        let result = repository.delete_user("nonexistent_id").await;

        // エラーが返されることを確認
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_all_users() {
        let repository = create_test_repository();

        // 複数のユーザーを作成
        let google_user1 = GoogleUser {
            id: "google_1".to_string(),
            email: "user1@example.com".to_string(),
            name: "ユーザー1".to_string(),
            picture: None,
            verified_email: true,
        };

        let google_user2 = GoogleUser {
            id: "google_2".to_string(),
            email: "user2@example.com".to_string(),
            name: "ユーザー2".to_string(),
            picture: None,
            verified_email: true,
        };

        repository.find_or_create_user(google_user1).await.unwrap();
        repository.find_or_create_user(google_user2).await.unwrap();

        // すべてのユーザーを取得
        let users = repository.get_all_users().await.unwrap();

        // デフォルトユーザー（ID=1）も含めて3人のユーザーが存在することを確認
        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_update_user_info_no_changes() {
        let repository = create_test_repository();
        let google_user = create_test_google_user();

        // ユーザーを作成
        let user = repository
            .find_or_create_user(google_user.clone())
            .await
            .unwrap();
        let original_updated_at = user.updated_at;

        // 同じ情報で再度find_or_create_userを呼び出し
        let user2 = repository.find_or_create_user(google_user).await.unwrap();

        // updated_atが変更されていないことを確認（変更がないため）
        assert_eq!(user2.updated_at, original_updated_at);
    }

    #[tokio::test]
    async fn test_update_user_info_with_changes() {
        let repository = create_test_repository();
        let google_user = create_test_google_user();

        // ユーザーを作成
        let user = repository
            .find_or_create_user(google_user.clone())
            .await
            .unwrap();
        let original_updated_at = user.updated_at;

        // 情報を変更してfind_or_create_userを呼び出し
        let mut updated_google_user = google_user;
        updated_google_user.name = "更新されたユーザー".to_string();

        let user2 = repository
            .find_or_create_user(updated_google_user)
            .await
            .unwrap();

        // 情報が更新されていることを確認
        assert_eq!(user2.name, "更新されたユーザー");
        assert_ne!(user2.updated_at, original_updated_at);
    }
}

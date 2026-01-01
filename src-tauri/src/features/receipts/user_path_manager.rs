// ユーザーパス管理システム
// R2ユーザーディレクトリ移行機能のためのパス管理

use crate::shared::errors::{AppError, AppResult};
use regex::Regex;

/// ユーザーパス管理システム
pub struct UserPathManager;

impl UserPathManager {
    /// 新しいユーザー別ファイルパスを生成
    ///
    /// # 引数
    /// * `user_id` - ユーザーID
    /// * `expense_id` - 経費ID
    /// * `filename` - ファイル名
    ///
    /// # 戻り値
    /// 新しいユーザー別パス（`users/{user_id}/receipts/{expense_id}/{timestamp}-{uuid}-{filename}`）
    pub fn generate_user_receipt_path(user_id: i64, expense_id: i64, filename: &str) -> String {
        let timestamp = chrono::Utc::now().timestamp();
        let uuid = uuid::Uuid::new_v4();
        format!("users/{user_id}/receipts/{expense_id}/{timestamp}-{uuid}-{filename}")
    }

    /// 既存のレガシーパスからユーザーパスに変換
    ///
    /// # 引数
    /// * `legacy_path` - レガシーパス（`receipts/{expense_id}/...`）
    /// * `user_id` - 変換先のユーザーID
    ///
    /// # 戻り値
    /// 新しいユーザーパス、または変換エラー
    ///
    /// # 例
    /// ```
    /// let legacy = "receipts/123/timestamp-uuid-filename.pdf";
    /// let user_path = UserPathManager::convert_legacy_to_user_path(legacy, 456)?;
    /// // => "users/456/receipts/123/timestamp-uuid-filename.pdf"
    /// ```
    pub fn convert_legacy_to_user_path(legacy_path: &str, user_id: i64) -> AppResult<String> {
        if let Some(stripped) = legacy_path.strip_prefix("receipts/") {
            Ok(format!("users/{user_id}/receipts/{stripped}"))
        } else {
            Err(AppError::Validation(format!(
                "無効なレガシーパス: {legacy_path}"
            )))
        }
    }

    /// パスからユーザーIDを抽出
    ///
    /// # 引数
    /// * `path` - ユーザーパス（`users/{user_id}/receipts/...`）
    ///
    /// # 戻り値
    /// ユーザーID、または抽出エラー
    ///
    /// # 例
    /// ```
    /// let path = "users/123/receipts/456/file.pdf";
    /// let user_id = UserPathManager::extract_user_id_from_path(path)?;
    /// // => 123
    /// ```
    pub fn extract_user_id_from_path(path: &str) -> AppResult<i64> {
        let regex = Regex::new(r"^users/(\d+)/receipts/")
            .map_err(|e| AppError::validation(format!("正規表現エラー: {e}")))?;

        if let Some(captures) = regex.captures(path) {
            captures[1]
                .parse::<i64>()
                .map_err(|_| AppError::validation("ユーザーIDの解析に失敗"))
        } else {
            Err(AppError::validation("ユーザーパス形式が無効"))
        }
    }

    /// ユーザーがパスにアクセス権限を持つかチェック
    ///
    /// # 引数
    /// * `user_id` - 認証されたユーザーID
    /// * `path` - アクセス対象のパス
    ///
    /// # 戻り値
    /// アクセス許可の場合はOk(())、拒否の場合はErr
    pub fn validate_user_access(user_id: i64, path: &str) -> AppResult<()> {
        let path_user_id = Self::extract_user_id_from_path(path)?;
        if user_id == path_user_id {
            Ok(())
        } else {
            Err(AppError::security("アクセス権限がありません"))
        }
    }

    /// レガシーパスかどうかを判定
    ///
    /// # 引数
    /// * `path` - 判定対象のパス
    ///
    /// # 戻り値
    /// レガシーパスの場合はtrue
    ///
    /// # 例
    /// ```
    /// assert!(UserPathManager::is_legacy_path("receipts/123/file.pdf"));
    /// assert!(!UserPathManager::is_legacy_path("users/456/receipts/123/file.pdf"));
    /// ```
    pub fn is_legacy_path(path: &str) -> bool {
        path.starts_with("receipts/") && !path.starts_with("receipts/users/")
    }

    /// ユーザーパスかどうかを判定
    ///
    /// # 引数
    /// * `path` - 判定対象のパス
    ///
    /// # 戻り値
    /// ユーザーパスの場合はtrue
    ///
    /// # 例
    /// ```
    /// assert!(UserPathManager::is_user_path("users/123/receipts/456/file.pdf"));
    /// assert!(!UserPathManager::is_user_path("receipts/123/file.pdf"));
    /// ```
    pub fn is_user_path(path: &str) -> bool {
        let regex = Regex::new(r"^users/\d+/receipts/").unwrap();
        regex.is_match(path)
    }

    /// サブスクリプション用のユーザー別ファイルパスを生成
    ///
    /// # 引数
    /// * `user_id` - ユーザーID
    /// * `subscription_id` - サブスクリプションID
    /// * `filename` - ファイル名
    ///
    /// # 戻り値
    /// 新しいユーザー別パス（`users/{user_id}/subscriptions/{subscription_id}/{timestamp}-{uuid}-{filename}`）
    pub fn generate_user_subscription_path(
        user_id: i64,
        subscription_id: i64,
        filename: &str,
    ) -> String {
        let timestamp = chrono::Utc::now().timestamp();
        let uuid = uuid::Uuid::new_v4();
        format!("users/{user_id}/subscriptions/{subscription_id}/{timestamp}-{uuid}-{filename}")
    }

    /// サブスクリプションのレガシーパスからユーザーパスに変換
    ///
    /// # 引数
    /// * `legacy_path` - レガシーパス（`subscriptions/{subscription_id}/...`）
    /// * `user_id` - 変換先のユーザーID
    ///
    /// # 戻り値
    /// 新しいユーザーパス、または変換エラー
    ///
    /// # 例
    /// ```
    /// let legacy = "subscriptions/123/timestamp-uuid-filename.pdf";
    /// let user_path = UserPathManager::convert_legacy_subscription_to_user_path(legacy, 456)?;
    /// // => "users/456/subscriptions/123/timestamp-uuid-filename.pdf"
    /// ```
    pub fn convert_legacy_subscription_to_user_path(
        legacy_path: &str,
        user_id: i64,
    ) -> AppResult<String> {
        if let Some(stripped) = legacy_path.strip_prefix("subscriptions/") {
            Ok(format!("users/{user_id}/subscriptions/{stripped}"))
        } else {
            Err(AppError::Validation(format!(
                "無効なレガシーサブスクリプションパス: {legacy_path}"
            )))
        }
    }

    /// サブスクリプションパスからsubscription_idを抽出（ユーザーパス用）
    ///
    /// # 引数
    /// * `path` - ユーザーパス（`users/{user_id}/subscriptions/{subscription_id}/...`）
    ///
    /// # 戻り値
    /// サブスクリプションID、または抽出エラー
    ///
    /// # 例
    /// ```
    /// let path = "users/123/subscriptions/456/file.pdf";
    /// let subscription_id = UserPathManager::extract_subscription_id_from_user_path(path)?;
    /// // => 456
    /// ```
    pub fn extract_subscription_id_from_user_path(path: &str) -> AppResult<i64> {
        let regex = Regex::new(r"^users/\d+/subscriptions/(\d+)/")
            .map_err(|e| AppError::validation(format!("正規表現エラー: {e}")))?;

        if let Some(captures) = regex.captures(path) {
            captures[1]
                .parse::<i64>()
                .map_err(|_| AppError::validation("サブスクリプションIDの解析に失敗"))
        } else {
            Err(AppError::validation(
                "ユーザーサブスクリプションパス形式が無効",
            ))
        }
    }

    /// サブスクリプションのレガシーパスかどうかを判定
    ///
    /// # 引数
    /// * `path` - 判定対象のパス
    ///
    /// # 戻り値
    /// レガシーサブスクリプションパスの場合はtrue
    ///
    /// # 例
    /// ```
    /// assert!(UserPathManager::is_legacy_subscription_path("subscriptions/123/file.pdf"));
    /// assert!(!UserPathManager::is_legacy_subscription_path("users/456/subscriptions/123/file.pdf"));
    /// ```
    pub fn is_legacy_subscription_path(path: &str) -> bool {
        path.starts_with("subscriptions/") && !path.starts_with("subscriptions/users/")
    }

    /// サブスクリプションのユーザーパスかどうかを判定
    ///
    /// # 引数
    /// * `path` - 判定対象のパス
    ///
    /// # 戻り値
    /// ユーザーサブスクリプションパスの場合はtrue
    ///
    /// # 例
    /// ```
    /// assert!(UserPathManager::is_user_subscription_path("users/123/subscriptions/456/file.pdf"));
    /// assert!(!UserPathManager::is_user_subscription_path("subscriptions/123/file.pdf"));
    /// ```
    pub fn is_user_subscription_path(path: &str) -> bool {
        let regex = Regex::new(r"^users/\d+/subscriptions/").unwrap();
        regex.is_match(path)
    }
    ///
    /// # 引数
    /// * `path` - ユーザーパス
    ///
    /// # 戻り値
    /// expense_id、または抽出エラー
    pub fn extract_expense_id_from_user_path(path: &str) -> AppResult<i64> {
        let regex = Regex::new(r"^users/\d+/receipts/(\d+)/")
            .map_err(|e| AppError::validation(format!("正規表現エラー: {e}")))?;

        if let Some(captures) = regex.captures(path) {
            captures[1]
                .parse::<i64>()
                .map_err(|_| AppError::validation("expense_idの解析に失敗"))
        } else {
            Err(AppError::validation("ユーザーパス形式が無効"))
        }
    }

    /// パスからexpense_idを抽出（レガシーパス用）
    ///
    /// # 引数
    /// * `path` - レガシーパス
    ///
    /// # 戻り値
    /// expense_id、または抽出エラー
    pub fn extract_expense_id_from_legacy_path(path: &str) -> AppResult<i64> {
        let regex = Regex::new(r"^receipts/(\d+)/")
            .map_err(|e| AppError::validation(format!("正規表現エラー: {e}")))?;

        if let Some(captures) = regex.captures(path) {
            captures[1]
                .parse::<i64>()
                .map_err(|_| AppError::validation("expense_idの解析に失敗"))
        } else {
            Err(AppError::validation("レガシーパス形式が無効"))
        }
    }

    /// ファイル名を抽出（パス形式に関係なく）
    ///
    /// # 引数
    /// * `path` - ファイルパス
    ///
    /// # 戻り値
    /// ファイル名、または抽出エラー
    pub fn extract_filename_from_path(path: &str) -> AppResult<String> {
        if let Some(filename) = path.split('/').next_back() {
            if filename.is_empty() {
                Err(AppError::validation("ファイル名が空です"))
            } else {
                Ok(filename.to_string())
            }
        } else {
            Err(AppError::validation("ファイル名の抽出に失敗"))
        }
    }

    /// パスの正規化（重複スラッシュの除去など）
    ///
    /// # 引数
    /// * `path` - 正規化対象のパス
    ///
    /// # 戻り値
    /// 正規化されたパス
    pub fn normalize_path(path: &str) -> String {
        // 重複スラッシュを単一スラッシュに変換
        let regex = Regex::new(r"/+").unwrap();
        let normalized = regex.replace_all(path, "/");

        // 先頭と末尾のスラッシュを除去
        normalized.trim_matches('/').to_string()
    }

    /// 管理者権限でのアクセス許可チェック
    ///
    /// # 引数
    /// * `user_id` - ユーザーID
    /// * `is_admin` - 管理者フラグ
    /// * `path` - アクセス対象のパス
    ///
    /// # 戻り値
    /// アクセス許可の場合はOk(())、拒否の場合はErr
    pub fn validate_admin_or_user_access(
        user_id: i64,
        is_admin: bool,
        path: &str,
    ) -> AppResult<()> {
        if is_admin {
            // 管理者は全てのパスにアクセス可能
            Ok(())
        } else {
            // 一般ユーザーは自分のパスのみアクセス可能
            Self::validate_user_access(user_id, path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_user_receipt_path() {
        let user_id = 123;
        let expense_id = 456;
        let filename = "receipt.pdf";

        let path1 = UserPathManager::generate_user_receipt_path(user_id, expense_id, filename);
        let path2 = UserPathManager::generate_user_receipt_path(user_id, expense_id, filename);

        // 異なるパスが生成されることを確認（UUIDとタイムスタンプのため）
        assert_ne!(path1, path2);

        // 正しい形式であることを確認
        assert!(path1.starts_with("users/123/receipts/456/"));
        assert!(path1.ends_with("-receipt.pdf"));
    }

    #[test]
    fn test_convert_legacy_to_user_path() {
        let legacy_path = "receipts/123/timestamp-uuid-filename.pdf";
        let user_id = 456;

        let result = UserPathManager::convert_legacy_to_user_path(legacy_path, user_id);
        assert!(result.is_ok());

        let user_path = result.unwrap();
        assert_eq!(
            user_path,
            "users/456/receipts/123/timestamp-uuid-filename.pdf"
        );
    }

    #[test]
    fn test_convert_legacy_to_user_path_invalid() {
        let invalid_path = "invalid/path/file.pdf";
        let user_id = 456;

        let result = UserPathManager::convert_legacy_to_user_path(invalid_path, user_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_user_id_from_path() {
        let path = "users/123/receipts/456/file.pdf";
        let result = UserPathManager::extract_user_id_from_path(path);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);
    }

    #[test]
    fn test_extract_user_id_from_path_invalid() {
        let invalid_path = "receipts/123/file.pdf";
        let result = UserPathManager::extract_user_id_from_path(invalid_path);

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_user_access() {
        let user_id = 123;
        let valid_path = "users/123/receipts/456/file.pdf";
        let invalid_path = "users/456/receipts/789/file.pdf";

        // 正当なアクセス
        let result = UserPathManager::validate_user_access(user_id, valid_path);
        assert!(result.is_ok());

        // 不正なアクセス
        let result = UserPathManager::validate_user_access(user_id, invalid_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_legacy_path() {
        assert!(UserPathManager::is_legacy_path("receipts/123/file.pdf"));
        assert!(UserPathManager::is_legacy_path(
            "receipts/456/timestamp-uuid-file.jpg"
        ));

        // ユーザーパスは除外
        assert!(!UserPathManager::is_legacy_path(
            "users/123/receipts/456/file.pdf"
        ));

        // 無関係なパスは除外
        assert!(!UserPathManager::is_legacy_path("other/path/file.pdf"));

        // receipts/users/ で始まるパスは除外（既に移行済みの可能性）
        assert!(!UserPathManager::is_legacy_path(
            "receipts/users/123/file.pdf"
        ));
    }

    #[test]
    fn test_is_user_path() {
        assert!(UserPathManager::is_user_path(
            "users/123/receipts/456/file.pdf"
        ));
        assert!(UserPathManager::is_user_path(
            "users/789/receipts/012/timestamp-uuid-file.jpg"
        ));

        // レガシーパスは除外
        assert!(!UserPathManager::is_user_path("receipts/123/file.pdf"));

        // 無関係なパスは除外
        assert!(!UserPathManager::is_user_path("other/path/file.pdf"));
    }

    #[test]
    fn test_extract_expense_id_from_user_path() {
        let path = "users/123/receipts/456/file.pdf";
        let result = UserPathManager::extract_expense_id_from_user_path(path);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 456);
    }

    #[test]
    fn test_extract_expense_id_from_legacy_path() {
        let path = "receipts/789/file.pdf";
        let result = UserPathManager::extract_expense_id_from_legacy_path(path);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 789);
    }

    #[test]
    fn test_extract_filename_from_path() {
        let user_path = "users/123/receipts/456/timestamp-uuid-file.pdf";
        let legacy_path = "receipts/789/timestamp-uuid-file.jpg";

        let result1 = UserPathManager::extract_filename_from_path(user_path);
        let result2 = UserPathManager::extract_filename_from_path(legacy_path);

        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), "timestamp-uuid-file.pdf");

        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), "timestamp-uuid-file.jpg");
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(
            UserPathManager::normalize_path("//users//123//receipts//456//file.pdf"),
            "users/123/receipts/456/file.pdf"
        );

        assert_eq!(
            UserPathManager::normalize_path("/receipts/123/file.pdf/"),
            "receipts/123/file.pdf"
        );
    }

    #[test]
    fn test_validate_admin_or_user_access() {
        let user_id = 123;
        let admin_user_id = 456;
        let path = "users/123/receipts/789/file.pdf";

        // 管理者アクセス
        let result = UserPathManager::validate_admin_or_user_access(admin_user_id, true, path);
        assert!(result.is_ok());

        // 所有者アクセス
        let result = UserPathManager::validate_admin_or_user_access(user_id, false, path);
        assert!(result.is_ok());

        // 不正アクセス
        let result = UserPathManager::validate_admin_or_user_access(admin_user_id, false, path);
        assert!(result.is_err());
    }
}

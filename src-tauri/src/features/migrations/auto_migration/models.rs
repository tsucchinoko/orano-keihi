//! 自動マイグレーションシステムのデータモデル
//!
//! このモジュールは、自動マイグレーションシステムで使用される
//! 基本的なデータ構造を定義します。

use serde::{Deserialize, Serialize};
use std::fmt;

/// マイグレーション定義
///
/// 利用可能なマイグレーションの定義を表します。
/// 各マイグレーションは一意の名前、バージョン、チェックサムを持ちます。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationDefinition {
    /// マイグレーション名（一意）
    pub name: String,
    /// マイグレーションバージョン（セマンティックバージョニング）
    pub version: String,
    /// マイグレーションの説明
    pub description: String,
    /// マイグレーション内容のチェックサム（SHA-256）
    pub checksum: String,
}

impl MigrationDefinition {
    /// 新しいマイグレーション定義を作成
    ///
    /// # 引数
    /// * `name` - マイグレーション名
    /// * `version` - バージョン
    /// * `description` - 説明
    /// * `checksum` - チェックサム
    ///
    /// # 戻り値
    /// 新しいマイグレーション定義
    pub fn new(name: String, version: String, description: String, checksum: String) -> Self {
        Self {
            name,
            version,
            description,
            checksum,
        }
    }

    /// マイグレーション定義の検証
    ///
    /// # 戻り値
    /// 有効な場合はOk(())、無効な場合はエラーメッセージ
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("マイグレーション名が空です".to_string());
        }

        if self.version.is_empty() {
            return Err("バージョンが空です".to_string());
        }

        if self.checksum.len() != 64 {
            return Err("チェックサムの長さが無効です（64文字のSHA-256が必要）".to_string());
        }

        // チェックサムが16進数文字列であることを確認
        if !self.checksum.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("チェックサムが無効な形式です（16進数文字列が必要）".to_string());
        }

        Ok(())
    }
}

impl fmt::Display for MigrationDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Migration[{}] v{}: {}",
            self.name, self.version, self.description
        )
    }
}

/// 適用済みマイグレーション
///
/// データベースに記録された適用済みマイグレーションの情報を表します。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedMigration {
    /// データベース内のID
    pub id: i64,
    /// マイグレーション名
    pub name: String,
    /// マイグレーションバージョン
    pub version: String,
    /// マイグレーションの説明
    pub description: Option<String>,
    /// マイグレーション内容のチェックサム
    pub checksum: String,
    /// 適用日時（JST、RFC3339形式）
    pub applied_at: String,
    /// 実行時間（ミリ秒）
    pub execution_time_ms: Option<i64>,
    /// レコード作成日時（JST、RFC3339形式）
    pub created_at: String,
}

impl AppliedMigration {
    /// 新しい適用済みマイグレーションを作成
    ///
    /// # 引数
    /// * `id` - データベースID
    /// * `name` - マイグレーション名
    /// * `version` - バージョン
    /// * `description` - 説明
    /// * `checksum` - チェックサム
    /// * `applied_at` - 適用日時
    /// * `execution_time_ms` - 実行時間
    /// * `created_at` - 作成日時
    ///
    /// # 戻り値
    /// 新しい適用済みマイグレーション
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        name: String,
        version: String,
        description: Option<String>,
        checksum: String,
        applied_at: String,
        execution_time_ms: Option<i64>,
        created_at: String,
    ) -> Self {
        Self {
            id,
            name,
            version,
            description,
            checksum,
            applied_at,
            execution_time_ms,
            created_at,
        }
    }

    /// チェックサムの整合性を検証
    ///
    /// # 引数
    /// * `expected_checksum` - 期待されるチェックサム
    ///
    /// # 戻り値
    /// 整合性が取れている場合はtrue
    pub fn verify_checksum(&self, expected_checksum: &str) -> bool {
        self.checksum == expected_checksum
    }
}

impl fmt::Display for AppliedMigration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Applied[{}] v{} at {}",
            self.name, self.version, self.applied_at
        )
    }
}

/// 自動マイグレーション実行結果
///
/// 自動マイグレーションシステムの実行結果を表します。
#[derive(Debug, Serialize, Deserialize)]
pub struct AutoMigrationResult {
    /// 実行成功フラグ
    pub success: bool,
    /// 結果メッセージ
    pub message: String,
    /// 適用されたマイグレーション名一覧
    pub applied_migrations: Vec<String>,
    /// バックアップファイルのパス
    pub backup_path: Option<String>,
    /// 総実行時間（ミリ秒）
    pub total_execution_time_ms: i64,
}

impl AutoMigrationResult {
    /// 成功結果を作成
    ///
    /// # 引数
    /// * `message` - 成功メッセージ
    /// * `applied_migrations` - 適用されたマイグレーション一覧
    /// * `backup_path` - バックアップパス
    /// * `total_execution_time_ms` - 総実行時間
    ///
    /// # 戻り値
    /// 成功結果
    pub fn success(
        message: String,
        applied_migrations: Vec<String>,
        backup_path: Option<String>,
        total_execution_time_ms: i64,
    ) -> Self {
        Self {
            success: true,
            message,
            applied_migrations,
            backup_path,
            total_execution_time_ms,
        }
    }

    /// 失敗結果を作成
    ///
    /// # 引数
    /// * `message` - エラーメッセージ
    /// * `backup_path` - バックアップパス（作成済みの場合）
    ///
    /// # 戻り値
    /// 失敗結果
    pub fn failure(message: String, backup_path: Option<String>) -> Self {
        Self {
            success: false,
            message,
            applied_migrations: Vec::new(),
            backup_path,
            total_execution_time_ms: 0,
        }
    }
}

impl fmt::Display for AutoMigrationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.success {
            write!(
                f,
                "成功: {} (適用: {}, 実行時間: {}ms)",
                self.message,
                self.applied_migrations.len(),
                self.total_execution_time_ms
            )
        } else {
            write!(f, "失敗: {}", self.message)
        }
    }
}

/// マイグレーション状態レポート
///
/// 現在のマイグレーション状態の詳細情報を表します。
#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationStatusReport {
    /// 利用可能なマイグレーション総数
    pub total_available: usize,
    /// 適用済みマイグレーション総数
    pub total_applied: usize,
    /// 未適用マイグレーション名一覧
    pub pending_migrations: Vec<String>,
    /// 最後のマイグレーション適用日時
    pub last_migration_date: Option<String>,
    /// データベースバージョン
    pub database_version: String,
}

impl MigrationStatusReport {
    /// 新しいマイグレーション状態レポートを作成
    ///
    /// # 引数
    /// * `total_available` - 利用可能なマイグレーション総数
    /// * `total_applied` - 適用済みマイグレーション総数
    /// * `pending_migrations` - 未適用マイグレーション一覧
    /// * `last_migration_date` - 最後のマイグレーション日時
    /// * `database_version` - データベースバージョン
    ///
    /// # 戻り値
    /// 新しいマイグレーション状態レポート
    pub fn new(
        total_available: usize,
        total_applied: usize,
        pending_migrations: Vec<String>,
        last_migration_date: Option<String>,
        database_version: String,
    ) -> Self {
        Self {
            total_available,
            total_applied,
            pending_migrations,
            last_migration_date,
            database_version,
        }
    }

    /// 未適用マイグレーションが存在するかチェック
    ///
    /// # 戻り値
    /// 未適用マイグレーションが存在する場合はtrue
    pub fn has_pending_migrations(&self) -> bool {
        !self.pending_migrations.is_empty()
    }

    /// マイグレーション完了率を計算
    ///
    /// # 戻り値
    /// 完了率（0.0-1.0）
    pub fn completion_rate(&self) -> f64 {
        if self.total_available == 0 {
            1.0
        } else {
            self.total_applied as f64 / self.total_available as f64
        }
    }
}

impl fmt::Display for MigrationStatusReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "マイグレーション状態: {}/{} 完了 ({:.1}%)",
            self.total_applied,
            self.total_available,
            self.completion_rate() * 100.0
        )?;

        if self.has_pending_migrations() {
            write!(f, ", 未適用: {}", self.pending_migrations.join(", "))?;
        }

        if let Some(last_date) = &self.last_migration_date {
            write!(f, ", 最終適用: {last_date}")?;
        }

        Ok(())
    }
}

/// マイグレーション実行結果
///
/// 個別のマイグレーション実行結果を表します。
#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationExecutionResult {
    /// 実行成功フラグ
    pub success: bool,
    /// 結果メッセージ
    pub message: String,
    /// 実行時間（ミリ秒）
    pub execution_time_ms: i64,
    /// バックアップファイルのパス
    pub backup_path: Option<String>,
}

impl MigrationExecutionResult {
    /// 成功結果を作成
    ///
    /// # 引数
    /// * `message` - 成功メッセージ
    /// * `execution_time_ms` - 実行時間
    /// * `backup_path` - バックアップパス
    ///
    /// # 戻り値
    /// 成功結果
    pub fn success(message: String, execution_time_ms: i64, backup_path: Option<String>) -> Self {
        Self {
            success: true,
            message,
            execution_time_ms,
            backup_path,
        }
    }

    /// 失敗結果を作成
    ///
    /// # 引数
    /// * `message` - エラーメッセージ
    /// * `backup_path` - バックアップパス（作成済みの場合）
    ///
    /// # 戻り値
    /// 失敗結果
    pub fn failure(message: String, backup_path: Option<String>) -> Self {
        Self {
            success: false,
            message,
            execution_time_ms: 0,
            backup_path,
        }
    }
}

impl fmt::Display for MigrationExecutionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.success {
            write!(
                f,
                "成功: {} (実行時間: {}ms)",
                self.message, self.execution_time_ms
            )
        } else {
            write!(f, "失敗: {}", self.message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_definition_validation() {
        // 有効なマイグレーション定義
        let valid_migration = MigrationDefinition::new(
            "test_migration".to_string(),
            "1.0.0".to_string(),
            "テストマイグレーション".to_string(),
            "a".repeat(64), // 64文字の16進数文字列
        );
        assert!(valid_migration.validate().is_ok());

        // 無効なマイグレーション定義（空の名前）
        let invalid_name = MigrationDefinition::new(
            "".to_string(),
            "1.0.0".to_string(),
            "テスト".to_string(),
            "a".repeat(64),
        );
        assert!(invalid_name.validate().is_err());

        // 無効なマイグレーション定義（無効なチェックサム長）
        let invalid_checksum = MigrationDefinition::new(
            "test".to_string(),
            "1.0.0".to_string(),
            "テスト".to_string(),
            "abc".to_string(),
        );
        assert!(invalid_checksum.validate().is_err());
    }

    #[test]
    fn test_applied_migration_checksum_verification() {
        let applied_migration = AppliedMigration::new(
            1,
            "test_migration".to_string(),
            "1.0.0".to_string(),
            Some("テスト".to_string()),
            "abc123".to_string(),
            "2024-01-01T00:00:00+09:00".to_string(),
            Some(1000),
            "2024-01-01T00:00:00+09:00".to_string(),
        );

        assert!(applied_migration.verify_checksum("abc123"));
        assert!(!applied_migration.verify_checksum("different"));
    }

    #[test]
    fn test_migration_status_report_completion_rate() {
        let report = MigrationStatusReport::new(
            10,
            7,
            vec!["migration1".to_string(), "migration2".to_string()],
            Some("2024-01-01T00:00:00+09:00".to_string()),
            "1.0.0".to_string(),
        );

        assert_eq!(report.completion_rate(), 0.7);
        assert!(report.has_pending_migrations());

        // 全て完了している場合
        let complete_report = MigrationStatusReport::new(
            5,
            5,
            vec![],
            Some("2024-01-01T00:00:00+09:00".to_string()),
            "1.0.0".to_string(),
        );

        assert_eq!(complete_report.completion_rate(), 1.0);
        assert!(!complete_report.has_pending_migrations());
    }

    #[test]
    fn test_auto_migration_result_creation() {
        let success_result = AutoMigrationResult::success(
            "マイグレーション完了".to_string(),
            vec!["migration1".to_string()],
            Some("backup.db".to_string()),
            5000,
        );

        assert!(success_result.success);
        assert_eq!(success_result.applied_migrations.len(), 1);
        assert_eq!(success_result.total_execution_time_ms, 5000);

        let failure_result = AutoMigrationResult::failure(
            "マイグレーション失敗".to_string(),
            Some("backup.db".to_string()),
        );

        assert!(!failure_result.success);
        assert!(failure_result.applied_migrations.is_empty());
        assert_eq!(failure_result.total_execution_time_ms, 0);
    }
}

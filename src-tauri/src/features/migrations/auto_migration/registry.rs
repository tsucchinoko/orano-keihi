//! マイグレーション登録管理
//!
//! このモジュールは、利用可能なマイグレーションの登録と管理を行います。

use super::errors::MigrationError;
use super::models::MigrationDefinition;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// マイグレーション登録管理
///
/// 利用可能なマイグレーションの登録と管理を行います。
pub struct MigrationRegistry {
    /// 登録されたマイグレーション定義
    migrations: Vec<MigrationDefinition>,
    /// 名前によるインデックス
    name_index: HashMap<String, usize>,
}

impl MigrationRegistry {
    /// 新しいレジストリを作成
    ///
    /// # 戻り値
    /// 新しいマイグレーションレジストリ
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
            name_index: HashMap::new(),
        }
    }

    /// マイグレーション定義を登録
    ///
    /// # 引数
    /// * `migration` - マイグレーション定義
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー
    pub fn register(&mut self, migration: MigrationDefinition) -> Result<(), MigrationError> {
        // バリデーション
        migration.validate().map_err(|msg| {
            MigrationError::validation(
                format!("マイグレーション定義が無効です: {msg}"),
                Some(migration.name.clone()),
                None,
            )
        })?;

        // 重複チェック
        if self.name_index.contains_key(&migration.name) {
            return Err(MigrationError::validation(
                format!(
                    "マイグレーション名 '{}' は既に登録されています",
                    migration.name
                ),
                Some(migration.name),
                None,
            ));
        }

        // 登録
        let index = self.migrations.len();
        self.name_index.insert(migration.name.clone(), index);
        self.migrations.push(migration);

        Ok(())
    }

    /// 利用可能なマイグレーション一覧を取得
    ///
    /// # 戻り値
    /// マイグレーション定義の参照一覧
    pub fn get_available_migrations(&self) -> &[MigrationDefinition] {
        &self.migrations
    }

    /// マイグレーション定義を名前で検索
    ///
    /// # 引数
    /// * `name` - マイグレーション名
    ///
    /// # 戻り値
    /// 見つかった場合はマイグレーション定義の参照、見つからない場合はNone
    pub fn find_migration(&self, name: &str) -> Option<&MigrationDefinition> {
        self.name_index
            .get(name)
            .and_then(|&index| self.migrations.get(index))
    }

    /// 登録されているマイグレーション数を取得
    ///
    /// # 戻り値
    /// マイグレーション数
    pub fn count(&self) -> usize {
        self.migrations.len()
    }

    /// レジストリが空かチェック
    ///
    /// # 戻り値
    /// 空の場合はtrue
    pub fn is_empty(&self) -> bool {
        self.migrations.is_empty()
    }

    /// 全てのマイグレーション名を取得
    ///
    /// # 戻り値
    /// マイグレーション名一覧
    pub fn get_migration_names(&self) -> Vec<String> {
        self.migrations.iter().map(|m| m.name.clone()).collect()
    }

    /// マイグレーション内容のチェックサムを計算
    ///
    /// # 引数
    /// * `content` - マイグレーション内容
    ///
    /// # 戻り値
    /// SHA-256チェックサム（16進数文字列）
    pub fn calculate_checksum(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// デフォルトマイグレーションを登録
    ///
    /// # 戻り値
    /// デフォルトマイグレーションが登録されたレジストリ
    pub fn register_default_migrations() -> Result<Self, MigrationError> {
        let mut registry = Self::new();

        // 基本スキーママイグレーション
        let basic_schema_migration = MigrationDefinition::new(
            "001_create_basic_schema".to_string(),
            "1.0.0".to_string(),
            "基本テーブル構造の作成".to_string(),
            Self::calculate_checksum("run_migrations"),
        );
        registry.register(basic_schema_migration)?;

        // ユーザー認証マイグレーション
        let user_auth_migration = MigrationDefinition::new(
            "002_add_user_authentication".to_string(),
            "2.0.0".to_string(),
            "ユーザー認証機能の追加".to_string(),
            Self::calculate_checksum("migrate_user_authentication"),
        );
        registry.register(user_auth_migration)?;

        // receipt_urlマイグレーション
        let receipt_url_migration = MigrationDefinition::new(
            "003_migrate_receipt_url".to_string(),
            "2.1.0".to_string(),
            "receipt_pathからreceipt_urlへの移行".to_string(),
            Self::calculate_checksum("migrate_receipt_path_to_url"),
        );
        registry.register(receipt_url_migration)?;

        Ok(registry)
    }
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_migration(name: &str, version: &str) -> MigrationDefinition {
        MigrationDefinition::new(
            name.to_string(),
            version.to_string(),
            format!("テストマイグレーション {name}"),
            "a".repeat(64), // 有効なチェックサム
        )
    }

    #[test]
    fn test_registry_creation() {
        let registry = MigrationRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_migration_registration() {
        let mut registry = MigrationRegistry::new();
        let migration = create_test_migration("test_migration", "1.0.0");

        assert!(registry.register(migration).is_ok());
        assert_eq!(registry.count(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_duplicate_migration_registration() {
        let mut registry = MigrationRegistry::new();
        let migration1 = create_test_migration("test_migration", "1.0.0");
        let migration2 = create_test_migration("test_migration", "2.0.0");

        assert!(registry.register(migration1).is_ok());
        assert!(registry.register(migration2).is_err());
    }

    #[test]
    fn test_migration_lookup() {
        let mut registry = MigrationRegistry::new();
        let migration = create_test_migration("test_migration", "1.0.0");

        registry.register(migration).unwrap();

        let found = registry.find_migration("test_migration");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test_migration");

        let not_found = registry.find_migration("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_migration_names() {
        let mut registry = MigrationRegistry::new();
        registry
            .register(create_test_migration("migration1", "1.0.0"))
            .unwrap();
        registry
            .register(create_test_migration("migration2", "1.0.0"))
            .unwrap();

        let names = registry.get_migration_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"migration1".to_string()));
        assert!(names.contains(&"migration2".to_string()));
    }

    #[test]
    fn test_calculate_checksum() {
        let content1 = "test content";
        let content2 = "different content";
        let content3 = "test content"; // 同じ内容

        let checksum1 = MigrationRegistry::calculate_checksum(content1);
        let checksum2 = MigrationRegistry::calculate_checksum(content2);
        let checksum3 = MigrationRegistry::calculate_checksum(content3);

        // チェックサムは64文字の16進数文字列
        assert_eq!(checksum1.len(), 64);
        assert!(checksum1.chars().all(|c| c.is_ascii_hexdigit()));

        // 異なる内容は異なるチェックサム
        assert_ne!(checksum1, checksum2);

        // 同じ内容は同じチェックサム
        assert_eq!(checksum1, checksum3);
    }

    #[test]
    fn test_register_default_migrations() {
        let registry = MigrationRegistry::register_default_migrations().unwrap();

        assert_eq!(registry.count(), 3);
        assert!(registry.find_migration("001_create_basic_schema").is_some());
        assert!(registry
            .find_migration("002_add_user_authentication")
            .is_some());
        assert!(registry.find_migration("003_migrate_receipt_url").is_some());

        // 各マイグレーションのチェックサムが正しく計算されていることを確認
        let basic_schema = registry.find_migration("001_create_basic_schema").unwrap();
        assert_eq!(
            basic_schema.checksum,
            MigrationRegistry::calculate_checksum("run_migrations")
        );

        let user_auth = registry
            .find_migration("002_add_user_authentication")
            .unwrap();
        assert_eq!(
            user_auth.checksum,
            MigrationRegistry::calculate_checksum("migrate_user_authentication")
        );

        let receipt_url = registry.find_migration("003_migrate_receipt_url").unwrap();
        assert_eq!(
            receipt_url.checksum,
            MigrationRegistry::calculate_checksum("migrate_receipt_path_to_url")
        );
    }
}

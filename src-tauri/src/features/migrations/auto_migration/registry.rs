//! マイグレーション登録管理
//!
//! このモジュールは、利用可能なマイグレーションの登録と管理を行います。

use super::errors::MigrationError;
use super::executor::{
    BasicSchemaMigrationExecutor, ReceiptUrlMigrationExecutor, UserAuthMigrationExecutor,
};
use super::models::{ExecutableMigrationDefinition, MigrationDefinition};
use crate::features::migrations::r2_user_directory_migration::get_r2_migration_schema_definition;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// マイグレーション登録管理
///
/// 利用可能なマイグレーションの登録と管理を行います。
pub struct MigrationRegistry {
    /// 登録されたマイグレーション定義
    migrations: Vec<MigrationDefinition>,
    /// 実行可能なマイグレーション定義
    executable_migrations: Vec<ExecutableMigrationDefinition>,
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
            executable_migrations: Vec::new(),
            name_index: HashMap::new(),
        }
    }

    /// 実行可能なマイグレーション定義を登録
    ///
    /// # 引数
    /// * `executable_migration` - 実行可能なマイグレーション定義
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラー
    pub fn register_executable(
        &mut self,
        executable_migration: ExecutableMigrationDefinition,
    ) -> Result<(), MigrationError> {
        // バリデーション
        executable_migration.definition.validate().map_err(|msg| {
            MigrationError::validation(
                format!("マイグレーション定義が無効です: {msg}"),
                Some(executable_migration.definition.name.clone()),
                None,
            )
        })?;

        // 重複チェック
        if self
            .name_index
            .contains_key(&executable_migration.definition.name)
        {
            return Err(MigrationError::validation(
                format!(
                    "マイグレーション名 '{}' は既に登録されています",
                    executable_migration.definition.name
                ),
                Some(executable_migration.definition.name),
                None,
            ));
        }

        // 登録
        let index = self.migrations.len();
        self.name_index
            .insert(executable_migration.definition.name.clone(), index);
        self.migrations
            .push(executable_migration.definition.clone());
        self.executable_migrations.push(executable_migration);

        Ok(())
    }

    /// 実行可能なマイグレーション一覧を取得
    ///
    /// # 戻り値
    /// 実行可能なマイグレーション定義の参照一覧
    pub fn get_executable_migrations(&self) -> &[ExecutableMigrationDefinition] {
        &self.executable_migrations
    }

    /// 実行可能なマイグレーション定義を名前で検索
    ///
    /// # 引数
    /// * `name` - マイグレーション名
    ///
    /// # 戻り値
    /// 見つかった場合は実行可能なマイグレーション定義の参照、見つからない場合はNone
    pub fn find_executable_migration(&self, name: &str) -> Option<&ExecutableMigrationDefinition> {
        self.name_index
            .get(name)
            .and_then(|&index| self.executable_migrations.get(index))
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
    /// 既存のマイグレーション機能を統合したデフォルトマイグレーションを登録します。
    /// 要件5.1, 5.2, 5.3, 5.4に対応します。
    ///
    /// # 戻り値
    /// デフォルトマイグレーションが登録されたレジストリ
    pub fn register_default_migrations() -> Result<Self, MigrationError> {
        let mut registry = Self::new();

        // 基本スキーママイグレーション（要件5.1）
        let basic_schema_definition = MigrationDefinition::new(
            "001_create_basic_schema".to_string(),
            "1.0.0".to_string(),
            "基本テーブル構造の作成".to_string(),
            Self::calculate_checksum("run_migrations"),
        );
        let basic_schema_executable = ExecutableMigrationDefinition::new(
            basic_schema_definition,
            Box::new(BasicSchemaMigrationExecutor),
        );
        registry.register_executable(basic_schema_executable)?;

        // ユーザー認証マイグレーション（要件5.2）
        let user_auth_definition = MigrationDefinition::new(
            "002_add_user_authentication".to_string(),
            "2.0.0".to_string(),
            "ユーザー認証機能の追加".to_string(),
            Self::calculate_checksum("migrate_user_authentication"),
        );
        let user_auth_executable = ExecutableMigrationDefinition::new(
            user_auth_definition,
            Box::new(UserAuthMigrationExecutor),
        );
        registry.register_executable(user_auth_executable)?;

        // receipt_urlマイグレーション（要件5.3）
        let receipt_url_definition = MigrationDefinition::new(
            "003_migrate_receipt_url".to_string(),
            "2.1.0".to_string(),
            "receipt_pathからreceipt_urlへの移行".to_string(),
            Self::calculate_checksum("migrate_receipt_path_to_url"),
        );
        let receipt_url_executable = ExecutableMigrationDefinition::new(
            receipt_url_definition,
            Box::new(ReceiptUrlMigrationExecutor),
        );
        registry.register_executable(receipt_url_executable)?;

        // R2移行用スキーママイグレーション（要件4.1, 4.2, 4.3）
        let r2_migration_schema = get_r2_migration_schema_definition();
        registry.register_executable(r2_migration_schema)?;

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
    use crate::features::migrations::auto_migration::executor::{
        BasicSchemaMigrationExecutor, UserAuthMigrationExecutor,
    };
    use crate::features::migrations::auto_migration::models::ExecutableMigrationDefinition;

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
    fn test_executable_migration_registration() {
        let mut registry = MigrationRegistry::new();
        let definition = create_test_migration("test_migration", "1.0.0");
        let executable_migration =
            ExecutableMigrationDefinition::new(definition, Box::new(BasicSchemaMigrationExecutor));

        assert!(registry.register_executable(executable_migration).is_ok());
        assert_eq!(registry.count(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_duplicate_executable_migration_registration() {
        let mut registry = MigrationRegistry::new();
        let definition1 = create_test_migration("test_migration", "1.0.0");
        let executable_migration1 =
            ExecutableMigrationDefinition::new(definition1, Box::new(BasicSchemaMigrationExecutor));

        let definition2 = create_test_migration("test_migration", "2.0.0");
        let executable_migration2 =
            ExecutableMigrationDefinition::new(definition2, Box::new(UserAuthMigrationExecutor));

        assert!(registry.register_executable(executable_migration1).is_ok());
        assert!(registry.register_executable(executable_migration2).is_err());
    }

    #[test]
    fn test_executable_migration_lookup() {
        let mut registry = MigrationRegistry::new();
        let definition = create_test_migration("test_migration", "1.0.0");
        let executable_migration =
            ExecutableMigrationDefinition::new(definition, Box::new(BasicSchemaMigrationExecutor));

        registry.register_executable(executable_migration).unwrap();

        let found = registry.find_executable_migration("test_migration");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "test_migration");

        let not_found = registry.find_executable_migration("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_migration_names() {
        let mut registry = MigrationRegistry::new();
        let definition1 = create_test_migration("migration1", "1.0.0");
        let executable_migration1 =
            ExecutableMigrationDefinition::new(definition1, Box::new(BasicSchemaMigrationExecutor));
        let definition2 = create_test_migration("migration2", "1.0.0");
        let executable_migration2 =
            ExecutableMigrationDefinition::new(definition2, Box::new(UserAuthMigrationExecutor));

        registry.register_executable(executable_migration1).unwrap();
        registry.register_executable(executable_migration2).unwrap();

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

        assert_eq!(registry.count(), 4);
        assert!(registry
            .find_executable_migration("001_create_basic_schema")
            .is_some());
        assert!(registry
            .find_executable_migration("002_add_user_authentication")
            .is_some());
        assert!(registry
            .find_executable_migration("003_migrate_receipt_url")
            .is_some());
        assert!(registry
            .find_executable_migration("r2_migration_schema")
            .is_some());

        // 各マイグレーションのチェックサムが正しく計算されていることを確認
        let basic_schema = registry
            .find_executable_migration("001_create_basic_schema")
            .unwrap();
        assert_eq!(
            basic_schema.definition.checksum,
            MigrationRegistry::calculate_checksum("run_migrations")
        );

        let user_auth = registry
            .find_executable_migration("002_add_user_authentication")
            .unwrap();
        assert_eq!(
            user_auth.definition.checksum,
            MigrationRegistry::calculate_checksum("migrate_user_authentication")
        );

        let receipt_url = registry
            .find_executable_migration("003_migrate_receipt_url")
            .unwrap();
        assert_eq!(
            receipt_url.definition.checksum,
            MigrationRegistry::calculate_checksum("migrate_receipt_path_to_url")
        );

        // 実行器の名前が正しいことを確認
        assert_eq!(basic_schema.executor.name(), "001_create_basic_schema");
        assert_eq!(user_auth.executor.name(), "002_add_user_authentication");
        assert_eq!(receipt_url.executor.name(), "003_migrate_receipt_url");
    }
}

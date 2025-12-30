//! マイグレーション登録管理
//!
//! このモジュールは、利用可能なマイグレーションの登録と管理を行います。

use super::errors::MigrationError;
use super::models::MigrationDefinition;
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
}

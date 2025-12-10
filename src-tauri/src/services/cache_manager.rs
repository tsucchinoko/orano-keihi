// ローカルキャッシュ管理モジュール

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// キャッシュエラー型
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("キャッシュ書き込みエラー: {0}")]
    WriteFailed(String),
    
    #[error("キャッシュ読み込みエラー: {0}")]
    ReadFailed(String),
    
    #[error("キャッシュクリーンアップエラー: {0}")]
    CleanupFailed(String),
    
    #[error("ディレクトリ作成エラー: {0}")]
    DirectoryCreationFailed(String),
    
    #[error("データベースエラー: {0}")]
    DatabaseError(String),
}

/// ローカルキャッシュマネージャー
pub struct CacheManager {
    cache_dir: PathBuf,
    max_cache_size: u64,
    max_age: Duration,
}

impl CacheManager {
    /// キャッシュマネージャーを初期化
    ///
    /// # 引数
    /// * `cache_dir` - キャッシュディレクトリのパス
    /// * `max_size_mb` - 最大キャッシュサイズ（MB）
    ///
    /// # 戻り値
    /// 初期化されたキャッシュマネージャー
    pub fn new(cache_dir: PathBuf, max_size_mb: u64) -> Self {
        Self {
            cache_dir,
            max_cache_size: max_size_mb * 1024 * 1024,
            max_age: Duration::from_secs(7 * 24 * 3600), // 7日間
        }
    }

    /// キャッシュディレクトリを初期化（同期版）
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はCacheError
    pub fn initialize_sync(&self) -> Result<(), CacheError> {
        if !self.cache_dir.exists() {
            std::fs::create_dir_all(&self.cache_dir)
                .map_err(|e| CacheError::DirectoryCreationFailed(format!("ディレクトリ作成失敗: {}", e)))?;
        }
        Ok(())
    }

    /// ファイルをキャッシュに保存
    ///
    /// # 引数
    /// * `receipt_url` - 領収書URL
    /// * `data` - ファイルデータ
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// キャッシュファイルのパス、または失敗時はCacheError
    pub fn cache_file(
        &self,
        receipt_url: &str,
        data: Vec<u8>,
        conn: &Connection,
    ) -> Result<PathBuf, CacheError> {
        // キャッシュディレクトリを確認・作成
        self.initialize_sync()?;

        // ファイル名を生成（URLからハッシュを作成）
        let filename = self.generate_cache_filename(receipt_url);
        let cache_path = self.cache_dir.join(&filename);

        // ファイルをキャッシュに保存
        std::fs::write(&cache_path, &data)
            .map_err(|e| CacheError::WriteFailed(format!("ファイル書き込み失敗: {}", e)))?;

        // データベースにキャッシュ情報を保存
        let local_path_str = cache_path
            .to_str()
            .ok_or_else(|| CacheError::WriteFailed("パス変換失敗".to_string()))?;

        crate::db::expense_operations::save_receipt_cache(
            conn,
            receipt_url,
            local_path_str,
            data.len() as i64,
        )
        .map_err(|e| CacheError::DatabaseError(format!("データベース保存失敗: {}", e)))?;

        Ok(cache_path)
    }

    /// キャッシュからファイルを取得
    ///
    /// # 引数
    /// * `receipt_url` - 領収書URL
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// ファイルデータ（存在する場合）、または失敗時はCacheError
    pub fn get_cached_file(
        &self,
        receipt_url: &str,
        conn: &Connection,
    ) -> Result<Option<Vec<u8>>, CacheError> {
        // データベースからキャッシュ情報を取得
        let cache_info = crate::db::expense_operations::get_receipt_cache(conn, receipt_url)
            .map_err(|e| CacheError::DatabaseError(format!("キャッシュ情報取得失敗: {}", e)))?;

        if let Some(cache) = cache_info {
            let cache_path = Path::new(&cache.local_path);
            
            // ファイルが存在するかチェック
            if cache_path.exists() {
                // アクセス時刻を更新
                crate::db::expense_operations::update_cache_access_time(conn, receipt_url)
                    .map_err(|e| CacheError::DatabaseError(format!("アクセス時刻更新失敗: {}", e)))?;

                // ファイルを読み込み
                let data = std::fs::read(cache_path)
                    .map_err(|e| CacheError::ReadFailed(format!("ファイル読み込み失敗: {}", e)))?;

                return Ok(Some(data));
            } else {
                // ファイルが存在しない場合はキャッシュ情報を削除
                crate::db::expense_operations::delete_receipt_cache(conn, receipt_url)
                    .map_err(|e| CacheError::DatabaseError(format!("キャッシュ削除失敗: {}", e)))?;
            }
        }

        Ok(None)
    }

    /// 古いキャッシュを削除
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 削除されたファイル数、または失敗時はCacheError
    pub fn cleanup_old_cache(&self, conn: &Connection) -> Result<usize, CacheError> {
        let max_age_days = self.max_age.as_secs() / (24 * 3600);
        
        // データベースから古いキャッシュ情報を取得
        let deleted_count = crate::db::expense_operations::cleanup_old_cache(conn, max_age_days as i64)
            .map_err(|e| CacheError::DatabaseError(format!("古いキャッシュ削除失敗: {}", e)))?;

        Ok(deleted_count)
    }

    /// キャッシュサイズを管理
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はCacheError
    pub fn manage_cache_size(&self, conn: &Connection) -> Result<(), CacheError> {
        // 現在のキャッシュサイズを計算
        let current_size = self.calculate_cache_size_sync()?;
        
        if current_size > self.max_cache_size {
            // サイズ超過時は古いファイルから削除
            self.cleanup_old_cache(conn)?;
            
            // まだサイズが超過している場合は、LRU方式で削除
            let remaining_size = self.calculate_cache_size_sync()?;
            if remaining_size > self.max_cache_size {
                self.cleanup_lru_cache(conn)?;
            }
        }

        Ok(())
    }

    /// キャッシュファイル名を生成
    ///
    /// # 引数
    /// * `receipt_url` - 領収書URL
    ///
    /// # 戻り値
    /// キャッシュファイル名
    fn generate_cache_filename(&self, receipt_url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        receipt_url.hash(&mut hasher);
        let hash = hasher.finish();

        // URLから拡張子を推定
        let extension = if receipt_url.contains(".pdf") {
            "pdf"
        } else if receipt_url.contains(".png") {
            "png"
        } else if receipt_url.contains(".jpg") || receipt_url.contains(".jpeg") {
            "jpg"
        } else {
            "bin"
        };

        format!("receipt_{:x}.{}", hash, extension)
    }

    /// 現在のキャッシュサイズを計算（同期版）
    ///
    /// # 戻り値
    /// キャッシュサイズ（バイト）、または失敗時はCacheError
    fn calculate_cache_size_sync(&self) -> Result<u64, CacheError> {
        let mut total_size = 0u64;

        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let entries = std::fs::read_dir(&self.cache_dir)
            .map_err(|e| CacheError::ReadFailed(format!("ディレクトリ読み込み失敗: {}", e)))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| CacheError::ReadFailed(format!("エントリ読み込み失敗: {}", e)))?;
            
            if entry.file_type()
                .map_err(|e| CacheError::ReadFailed(format!("ファイルタイプ取得失敗: {}", e)))?
                .is_file()
            {
                let metadata = entry
                    .metadata()
                    .map_err(|e| CacheError::ReadFailed(format!("メタデータ取得失敗: {}", e)))?;
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }

    /// LRU方式でキャッシュを削除
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はCacheError
    fn cleanup_lru_cache(&self, conn: &Connection) -> Result<(), CacheError> {
        // 最も古くアクセスされたファイルを削除
        // 実装は簡略化し、古いキャッシュクリーンアップを再実行
        self.cleanup_old_cache(conn)?;
        Ok(())
    }

    /// 特定のキャッシュファイルを削除
    ///
    /// # 引数
    /// * `receipt_url` - 領収書URL
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はCacheError
    pub fn delete_cache_file(
        &self,
        receipt_url: &str,
        conn: &Connection,
    ) -> Result<(), CacheError> {
        // データベースからキャッシュ情報を取得
        let cache_info = crate::db::expense_operations::get_receipt_cache(conn, receipt_url)
            .map_err(|e| CacheError::DatabaseError(format!("キャッシュ情報取得失敗: {}", e)))?;

        if let Some(cache) = cache_info {
            let cache_path = Path::new(&cache.local_path);
            
            // ファイルが存在する場合は削除
            if cache_path.exists() {
                std::fs::remove_file(cache_path)
                    .map_err(|e| CacheError::WriteFailed(format!("ファイル削除失敗: {}", e)))?;
            }

            // データベースからキャッシュ情報を削除
            crate::db::expense_operations::delete_receipt_cache(conn, receipt_url)
                .map_err(|e| CacheError::DatabaseError(format!("キャッシュ情報削除失敗: {}", e)))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_filename_generation() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = CacheManager::new(temp_dir.path().to_path_buf(), 100);

        let url1 = "https://example.com/receipt1.pdf";
        let url2 = "https://example.com/receipt2.jpg";

        let filename1 = cache_manager.generate_cache_filename(url1);
        let filename2 = cache_manager.generate_cache_filename(url2);

        // 異なるURLは異なるファイル名を生成
        assert_ne!(filename1, filename2);

        // 拡張子が正しく設定される
        assert!(filename1.ends_with(".pdf"));
        assert!(filename2.ends_with(".jpg"));

        // 同じURLは同じファイル名を生成
        let filename1_again = cache_manager.generate_cache_filename(url1);
        assert_eq!(filename1, filename1_again);
    }

    #[test]
    fn test_cache_size_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = CacheManager::new(temp_dir.path().to_path_buf(), 100);

        // 初期状態ではサイズは0
        let initial_size = cache_manager.calculate_cache_size_sync().unwrap();
        assert_eq!(initial_size, 0);
    }
}
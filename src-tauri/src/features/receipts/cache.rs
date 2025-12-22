// ローカルキャッシュ管理モジュール

use super::models::ReceiptCache;
use crate::shared::errors::{AppError, AppResult};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// ローカルキャッシュマネージャー
pub struct CacheManager {
    cache_dir: PathBuf,
    pub max_cache_size: u64,
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
    /// 成功時はOk(())、失敗時はAppError
    pub fn initialize_sync(&self) -> AppResult<()> {
        if !self.cache_dir.exists() {
            std::fs::create_dir_all(&self.cache_dir).map_err(|e| {
                AppError::ExternalService(format!("キャッシュディレクトリ作成失敗: {e}"))
            })?;
        }
        Ok(())
    }

    /// ファイルをキャッシュに保存（同期版）
    ///
    /// # 引数
    /// * `receipt_url` - 領収書URL
    /// * `data` - ファイルデータ
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// キャッシュファイルのパス、または失敗時はAppError
    pub fn cache_file(
        &self,
        receipt_url: &str,
        data: Vec<u8>,
        conn: &Connection,
    ) -> AppResult<PathBuf> {
        // キャッシュディレクトリを確認・作成
        self.initialize_sync()?;

        // ファイル名を生成（URLからハッシュを作成）
        let filename = self.generate_cache_filename(receipt_url);
        let cache_path = self.cache_dir.join(&filename);

        // ファイルをキャッシュに保存
        std::fs::write(&cache_path, &data)
            .map_err(|e| AppError::ExternalService(format!("キャッシュファイル書き込み失敗: {e}")))?;

        // データベースにキャッシュ情報を保存
        let local_path_str = cache_path
            .to_str()
            .ok_or_else(|| AppError::ExternalService("パス変換失敗".to_string()))?;

        self.save_receipt_cache(conn, receipt_url, local_path_str, data.len() as i64)?;

        Ok(cache_path)
    }

    /// キャッシュからファイルを取得（同期版）
    ///
    /// # 引数
    /// * `receipt_url` - 領収書URL
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// ファイルデータ（存在する場合）、または失敗時はAppError
    pub fn get_cached_file(
        &self,
        receipt_url: &str,
        conn: &Connection,
    ) -> AppResult<Option<Vec<u8>>> {
        // データベースからキャッシュ情報を取得
        let cache_info = self.get_receipt_cache(conn, receipt_url)?;

        if let Some(cache) = cache_info {
            let cache_path = Path::new(&cache.local_path);

            // ファイルが存在するかチェック
            if cache_path.exists() {
                // アクセス時刻を更新
                self.update_cache_access_time(conn, receipt_url)?;

                // ファイルを読み込み
                let data = std::fs::read(cache_path)
                    .map_err(|e| AppError::ExternalService(format!("キャッシュファイル読み込み失敗: {e}")))?;

                return Ok(Some(data));
            } else {
                // ファイルが存在しない場合はキャッシュ情報を削除
                self.delete_receipt_cache(conn, receipt_url)?;
            }
        }

        Ok(None)
    }

    /// 古いキャッシュを削除（同期版）
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 削除されたファイル数、または失敗時はAppError
    pub fn cleanup_old_cache(&self, conn: &Connection) -> AppResult<usize> {
        let max_age_days = self.max_age.as_secs() / (24 * 3600);

        // データベースから古いキャッシュ情報を取得して物理ファイルも削除
        let old_caches = self.get_old_cache_entries(conn, max_age_days as i64)?;

        let mut _deleted_count = 0;
        for cache in &old_caches {
            let cache_path = Path::new(&cache.local_path);
            if cache_path.exists() {
                if let Err(e) = std::fs::remove_file(cache_path) {
                    eprintln!("キャッシュファイル削除エラー: {} ({})", cache.local_path, e);
                } else {
                    _deleted_count += 1;
                }
            }
        }

        // データベースから古いキャッシュ情報を削除
        let db_deleted_count = self.cleanup_old_cache_db(conn, max_age_days as i64)?;

        Ok(db_deleted_count)
    }

    /// キャッシュサイズを管理（同期版）
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はAppError
    pub fn manage_cache_size(&self, conn: &Connection) -> AppResult<()> {
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

        format!("receipt_{hash:x}.{extension}")
    }

    /// 現在のキャッシュサイズを計算（同期版）
    ///
    /// # 戻り値
    /// キャッシュサイズ（バイト）、または失敗時はAppError
    pub fn calculate_cache_size_sync(&self) -> AppResult<u64> {
        let mut total_size = 0u64;

        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let entries = std::fs::read_dir(&self.cache_dir)
            .map_err(|e| AppError::ExternalService(format!("ディレクトリ読み込み失敗: {e}")))?;

        for entry in entries {
            let entry =
                entry.map_err(|e| AppError::ExternalService(format!("エントリ読み込み失敗: {e}")))?;

            if entry
                .file_type()
                .map_err(|e| AppError::ExternalService(format!("ファイルタイプ取得失敗: {e}")))?
                .is_file()
            {
                let metadata = entry
                    .metadata()
                    .map_err(|e| AppError::ExternalService(format!("メタデータ取得失敗: {e}")))?;
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }

    /// LRU方式でキャッシュを削除（同期版）
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はAppError
    fn cleanup_lru_cache(&self, conn: &Connection) -> AppResult<()> {
        // 最も古くアクセスされたファイルを取得して削除
        let lru_caches = self.get_lru_cache_entries(conn, 10)?; // 最大10個削除

        for cache in &lru_caches {
            let cache_path = Path::new(&cache.local_path);
            if cache_path.exists() {
                if let Err(e) = std::fs::remove_file(cache_path) {
                    eprintln!(
                        "LRUキャッシュファイル削除エラー: {} ({})",
                        cache.local_path, e
                    );
                }
            }

            // データベースからも削除
            if let Err(e) = self.delete_receipt_cache(conn, &cache.receipt_url) {
                eprintln!("LRUキャッシュDB削除エラー: {} ({})", cache.receipt_url, e);
            }
        }

        Ok(())
    }

    /// 特定のキャッシュファイルを削除（同期版）
    ///
    /// # 引数
    /// * `receipt_url` - 領収書URL
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はAppError
    pub fn delete_cache_file(&self, receipt_url: &str, conn: &Connection) -> AppResult<()> {
        // データベースからキャッシュ情報を取得
        let cache_info = self.get_receipt_cache(conn, receipt_url)?;

        if let Some(cache) = cache_info {
            let cache_path = Path::new(&cache.local_path);

            // ファイルが存在する場合は削除
            if cache_path.exists() {
                std::fs::remove_file(cache_path)
                    .map_err(|e| AppError::ExternalService(format!("ファイル削除失敗: {e}")))?;
            }

            // データベースからキャッシュ情報を削除
            self.delete_receipt_cache(conn, receipt_url)?;
        }

        Ok(())
    }

    /// オフライン時のキャッシュ表示機能
    ///
    /// # 引数
    /// * `receipt_url` - 領収書URL
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// キャッシュされたファイルデータ（存在する場合）、または失敗時はAppError
    pub fn get_offline_cached_file(
        &self,
        receipt_url: &str,
        conn: &Connection,
    ) -> AppResult<Option<Vec<u8>>> {
        // オフライン時はアクセス時刻を更新せずにキャッシュを取得
        let cache_info = self.get_receipt_cache(conn, receipt_url)?;

        if let Some(cache) = cache_info {
            let cache_path = Path::new(&cache.local_path);

            // ファイルが存在するかチェック
            if cache_path.exists() {
                // ファイルを読み込み（アクセス時刻は更新しない）
                let data = std::fs::read(cache_path)
                    .map_err(|e| AppError::ExternalService(format!("ファイル読み込み失敗: {e}")))?;

                return Ok(Some(data));
            }
        }

        Ok(None)
    }

    // ========== データベース操作ヘルパー関数 ==========

    /// 領収書キャッシュ情報をデータベースに保存
    fn save_receipt_cache(
        &self,
        conn: &Connection,
        receipt_url: &str,
        local_path: &str,
        file_size: i64,
    ) -> AppResult<()> {
        use chrono::Utc;
        use chrono_tz::Asia::Tokyo;

        let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

        conn.execute(
            "INSERT OR REPLACE INTO receipt_cache (receipt_url, local_path, cached_at, file_size, last_accessed)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![receipt_url, local_path, &now, file_size, &now],
        )
        .map_err(|e| AppError::Database(format!("キャッシュ情報保存失敗: {e}")))?;

        Ok(())
    }

    /// 領収書キャッシュ情報をデータベースから取得
    fn get_receipt_cache(
        &self,
        conn: &Connection,
        receipt_url: &str,
    ) -> AppResult<Option<ReceiptCache>> {
        match conn.query_row(
            "SELECT id, receipt_url, local_path, cached_at, file_size, last_accessed
             FROM receipt_cache WHERE receipt_url = ?1",
            rusqlite::params![receipt_url],
            |row| {
                Ok(ReceiptCache {
                    id: row.get(0)?,
                    receipt_url: row.get(1)?,
                    local_path: row.get(2)?,
                    cached_at: row.get(3)?,
                    file_size: row.get(4)?,
                    last_accessed: row.get(5)?,
                })
            },
        ) {
            Ok(cache) => Ok(Some(cache)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(format!("キャッシュ情報取得失敗: {e}"))),
        }
    }

    /// キャッシュアクセス時刻を更新
    fn update_cache_access_time(&self, conn: &Connection, receipt_url: &str) -> AppResult<()> {
        use chrono::Utc;
        use chrono_tz::Asia::Tokyo;

        let now = Utc::now().with_timezone(&Tokyo).to_rfc3339();

        conn.execute(
            "UPDATE receipt_cache SET last_accessed = ?1 WHERE receipt_url = ?2",
            rusqlite::params![&now, receipt_url],
        )
        .map_err(|e| AppError::Database(format!("アクセス時刻更新失敗: {e}")))?;

        Ok(())
    }

    /// 領収書キャッシュ情報をデータベースから削除
    fn delete_receipt_cache(&self, conn: &Connection, receipt_url: &str) -> AppResult<()> {
        conn.execute(
            "DELETE FROM receipt_cache WHERE receipt_url = ?1",
            rusqlite::params![receipt_url],
        )
        .map_err(|e| AppError::Database(format!("キャッシュ情報削除失敗: {e}")))?;

        Ok(())
    }

    /// 古いキャッシュエントリを取得するヘルパー関数
    fn get_old_cache_entries(
        &self,
        conn: &Connection,
        max_age_days: i64,
    ) -> AppResult<Vec<ReceiptCache>> {
        use chrono::Utc;
        use chrono_tz::Asia::Tokyo;

        // JSTで現在時刻を取得
        let now = Utc::now().with_timezone(&Tokyo);
        let cutoff_date = now - chrono::Duration::days(max_age_days);
        let cutoff_str = cutoff_date.to_rfc3339();

        let mut stmt = conn
            .prepare("SELECT id, receipt_url, local_path, cached_at, file_size, last_accessed FROM receipt_cache WHERE last_accessed < ?1")
            .map_err(|e| AppError::Database(format!("SQL準備失敗: {e}")))?;

        let cache_iter = stmt
            .query_map([cutoff_str], |row| {
                Ok(ReceiptCache {
                    id: row.get(0)?,
                    receipt_url: row.get(1)?,
                    local_path: row.get(2)?,
                    cached_at: row.get(3)?,
                    file_size: row.get(4)?,
                    last_accessed: row.get(5)?,
                })
            })
            .map_err(|e| AppError::Database(format!("クエリ実行失敗: {e}")))?;

        let mut caches = Vec::new();
        for cache_result in cache_iter {
            caches.push(
                cache_result.map_err(|e| AppError::Database(format!("行読み込み失敗: {e}")))?,
            );
        }

        Ok(caches)
    }

    /// LRUキャッシュエントリを取得するヘルパー関数
    fn get_lru_cache_entries(&self, conn: &Connection, limit: i64) -> AppResult<Vec<ReceiptCache>> {
        let mut stmt = conn
            .prepare("SELECT id, receipt_url, local_path, cached_at, file_size, last_accessed FROM receipt_cache ORDER BY last_accessed ASC LIMIT ?1")
            .map_err(|e| AppError::Database(format!("SQL準備失敗: {e}")))?;

        let cache_iter = stmt
            .query_map([limit], |row| {
                Ok(ReceiptCache {
                    id: row.get(0)?,
                    receipt_url: row.get(1)?,
                    local_path: row.get(2)?,
                    cached_at: row.get(3)?,
                    file_size: row.get(4)?,
                    last_accessed: row.get(5)?,
                })
            })
            .map_err(|e| AppError::Database(format!("クエリ実行失敗: {e}")))?;

        let mut caches = Vec::new();
        for cache_result in cache_iter {
            caches.push(
                cache_result.map_err(|e| AppError::Database(format!("行読み込み失敗: {e}")))?,
            );
        }

        Ok(caches)
    }

    /// データベースから古いキャッシュ情報を削除
    fn cleanup_old_cache_db(&self, conn: &Connection, max_age_days: i64) -> AppResult<usize> {
        use chrono::Utc;
        use chrono_tz::Asia::Tokyo;

        let now = Utc::now().with_timezone(&Tokyo);
        let cutoff_date = now - chrono::Duration::days(max_age_days);
        let cutoff_str = cutoff_date.to_rfc3339();

        let deleted_count = conn
            .execute(
                "DELETE FROM receipt_cache WHERE last_accessed < ?1",
                rusqlite::params![cutoff_str],
            )
            .map_err(|e| AppError::Database(format!("古いキャッシュ削除失敗: {e}")))?;

        Ok(deleted_count)
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

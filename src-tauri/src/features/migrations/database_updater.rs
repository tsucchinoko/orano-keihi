//! データベース更新機能
//!
//! R2ユーザーディレクトリ移行に伴うデータベース更新処理を提供します。
//! トランザクション処理による整合性保証、更新失敗時のロールバック機能、
//! 更新後検証機能を含みます。

use crate::shared::database::connection::get_database_connection;
use crate::shared::errors::{AppError, AppResult};
use log::{debug, error, info, warn};
use rusqlite::{OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// データベース更新結果
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseUpdateResult {
    /// 更新対象レコード数
    pub total_records: usize,
    /// 成功更新数
    pub updated_count: usize,
    /// 失敗数
    pub failed_count: usize,
    /// 検証成功数
    pub verified_count: usize,
    /// エラー詳細
    pub errors: Vec<String>,
    /// 実行時間（ミリ秒）
    pub duration_ms: u64,
}

/// URL更新アイテム
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlUpdateItem {
    /// 経費ID
    pub expense_id: i64,
    /// 古いURL
    pub old_url: String,
    /// 新しいURL
    pub new_url: String,
    /// ユーザーID
    pub user_id: i64,
}

/// データベース更新サービス
pub struct DatabaseUpdater;

impl DatabaseUpdater {
    /// receipt_url一括更新処理
    ///
    /// # 引数
    /// * `update_items` - 更新対象アイテム一覧
    /// * `batch_size` - バッチサイズ（デフォルト: 100）
    ///
    /// # 戻り値
    /// データベース更新結果
    pub async fn update_receipt_urls_batch(
        update_items: Vec<UrlUpdateItem>,
        batch_size: Option<usize>,
    ) -> AppResult<DatabaseUpdateResult> {
        let start_time = std::time::Instant::now();
        let batch_size = batch_size.unwrap_or(100);
        let total_records = update_items.len();

        info!(
            "receipt_url一括更新を開始します: 対象レコード数={}, バッチサイズ={}",
            total_records, batch_size
        );

        let mut updated_count = 0;
        let mut failed_count = 0;
        let mut errors = Vec::new();

        // バッチごとに処理
        let batches: Vec<_> = update_items.chunks(batch_size).collect();
        let total_batches = batches.len();

        for (batch_index, batch) in batches.into_iter().enumerate() {
            info!(
                "バッチ {}/{} を処理中 (レコード数: {})",
                batch_index + 1,
                total_batches,
                batch.len()
            );

            match Self::update_receipt_urls_transaction(batch.to_vec()).await {
                Ok(batch_updated_count) => {
                    updated_count += batch_updated_count;
                    info!(
                        "バッチ {}/{} 完了: {}件更新",
                        batch_index + 1,
                        total_batches,
                        batch_updated_count
                    );
                }
                Err(e) => {
                    failed_count += batch.len();
                    let error_msg = format!("バッチ{}更新失敗: {}", batch_index + 1, e);
                    error!("{}", error_msg);
                    errors.push(error_msg);
                }
            }
        }

        // 更新後検証
        let verified_count = if updated_count > 0 {
            match Self::verify_updated_urls(&update_items[..updated_count]).await {
                Ok(count) => count,
                Err(e) => {
                    let error_msg = format!("更新後検証失敗: {}", e);
                    warn!("{}", error_msg);
                    errors.push(error_msg);
                    0
                }
            }
        } else {
            0
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        let result = DatabaseUpdateResult {
            total_records,
            updated_count,
            failed_count,
            verified_count,
            errors,
            duration_ms,
        };

        info!(
            "receipt_url一括更新完了: 成功={}, 失敗={}, 検証成功={}, 実行時間={}ms",
            updated_count, failed_count, verified_count, duration_ms
        );

        Ok(result)
    }

    /// トランザクション内でreceipt_url更新
    ///
    /// # 引数
    /// * `update_items` - 更新対象アイテム一覧
    ///
    /// # 戻り値
    /// 更新されたレコード数
    async fn update_receipt_urls_transaction(update_items: Vec<UrlUpdateItem>) -> AppResult<usize> {
        let mut conn = get_database_connection().await?;

        // トランザクション開始
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(format!("トランザクション開始エラー: {e}")))?;

        // 更新前の状態を記録（ロールバック用）
        let mut rollback_data = HashMap::new();

        for item in &update_items {
            // 更新前の値を記録
            let original_url = Self::get_current_receipt_url(&tx, item.expense_id)?;
            if let Some(url) = original_url {
                rollback_data.insert(item.expense_id, url);
            }
        }

        // 一括更新実行
        let updated_count = match Self::execute_batch_update(&tx, &update_items) {
            Ok(count) => {
                // 更新後の整合性チェック
                if let Err(e) = Self::validate_update_integrity(&tx, &update_items) {
                    error!("整合性チェック失敗: {}", e);

                    // ロールバック実行
                    Self::rollback_updates(&tx, &rollback_data)?;

                    return Err(AppError::validation(format!(
                        "整合性チェック失敗によりロールバックしました: {}",
                        e
                    )));
                }

                // トランザクションコミット
                tx.commit().map_err(|e| {
                    AppError::Database(format!("トランザクションコミットエラー: {e}"))
                })?;

                debug!("トランザクション正常完了: {}件更新", count);
                count
            }
            Err(e) => {
                error!("バッチ更新失敗: {}", e);

                // 自動ロールバック（トランザクションがドロップされる）
                return Err(e);
            }
        };

        Ok(updated_count)
    }

    /// バッチ更新実行
    ///
    /// # 引数
    /// * `tx` - トランザクション
    /// * `update_items` - 更新対象アイテム一覧
    ///
    /// # 戻り値
    /// 更新されたレコード数
    fn execute_batch_update(
        tx: &Transaction<'_>,
        update_items: &[UrlUpdateItem],
    ) -> AppResult<usize> {
        let query = "
            UPDATE expenses 
            SET receipt_url = ?, 
                updated_at = ?
            WHERE id = ? AND user_id = ? AND receipt_url = ?
        ";

        let mut stmt = tx
            .prepare(query)
            .map_err(|e| AppError::Database(format!("UPDATE文準備エラー: {e}")))?;

        let now = chrono::Utc::now()
            .with_timezone(&chrono_tz::Asia::Tokyo)
            .to_rfc3339();

        let mut updated_count = 0;

        for item in update_items {
            let rows_affected = stmt
                .execute([
                    &item.new_url,
                    &now,
                    &item.expense_id.to_string(),
                    &item.user_id.to_string(),
                    &item.old_url,
                ])
                .map_err(|e| {
                    AppError::Database(format!(
                        "expense_id={}のreceipt_url更新エラー: {e}",
                        item.expense_id
                    ))
                })?;

            if rows_affected > 0 {
                updated_count += rows_affected;
                debug!(
                    "expense_id={} のreceipt_url更新完了: {} -> {}",
                    item.expense_id, item.old_url, item.new_url
                );
            } else {
                warn!(
                    "expense_id={} の更新対象レコードが見つかりません",
                    item.expense_id
                );
            }
        }

        Ok(updated_count)
    }

    /// 更新後の整合性チェック
    ///
    /// # 引数
    /// * `tx` - トランザクション
    /// * `update_items` - 更新対象アイテム一覧
    ///
    /// # 戻り値
    /// 検証結果
    fn validate_update_integrity(
        tx: &Transaction<'_>,
        update_items: &[UrlUpdateItem],
    ) -> AppResult<()> {
        debug!("更新後の整合性チェックを開始します");

        let query = "
            SELECT receipt_url 
            FROM expenses 
            WHERE id = ? AND user_id = ?
        ";

        let mut stmt = tx
            .prepare(query)
            .map_err(|e| AppError::Database(format!("整合性チェック用SELECT文準備エラー: {e}")))?;

        for item in update_items {
            let current_url: Option<String> = stmt
                .query_row(
                    [&item.expense_id.to_string(), &item.user_id.to_string()],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| {
                    AppError::Database(format!(
                        "expense_id={}の整合性チェックエラー: {e}",
                        item.expense_id
                    ))
                })?;

            match current_url {
                Some(url) => {
                    if url != item.new_url {
                        return Err(AppError::validation(format!(
                            "expense_id={}の整合性チェック失敗: 期待値={}, 実際値={}",
                            item.expense_id, item.new_url, url
                        )));
                    }
                }
                None => {
                    return Err(AppError::validation(format!(
                        "expense_id={}のレコードが見つかりません",
                        item.expense_id
                    )));
                }
            }
        }

        debug!("整合性チェック完了: {}件検証", update_items.len());
        Ok(())
    }

    /// 現在のreceipt_urlを取得
    ///
    /// # 引数
    /// * `tx` - トランザクション
    /// * `expense_id` - 経費ID
    ///
    /// # 戻り値
    /// 現在のreceipt_url
    fn get_current_receipt_url(tx: &Transaction<'_>, expense_id: i64) -> AppResult<Option<String>> {
        let query = "SELECT receipt_url FROM expenses WHERE id = ?";

        let url: Option<String> = tx
            .query_row(query, [expense_id], |row| row.get::<_, Option<String>>(0))
            .optional()
            .map_err(|e| {
                AppError::Database(format!("expense_id={}の現在URL取得エラー: {e}", expense_id))
            })?
            .flatten(); // Option<Option<String>> -> Option<String>

        Ok(url)
    }

    /// ロールバック実行
    ///
    /// # 引数
    /// * `tx` - トランザクション
    /// * `rollback_data` - ロールバック用データ（expense_id -> 元のURL）
    ///
    /// # 戻り値
    /// 処理結果
    fn rollback_updates(
        tx: &Transaction<'_>,
        rollback_data: &HashMap<i64, String>,
    ) -> AppResult<()> {
        warn!("ロールバックを実行します: {}件", rollback_data.len());

        let query = "UPDATE expenses SET receipt_url = ? WHERE id = ?";
        let mut stmt = tx
            .prepare(query)
            .map_err(|e| AppError::Database(format!("ロールバック用UPDATE文準備エラー: {e}")))?;

        for (expense_id, original_url) in rollback_data {
            stmt.execute([original_url, &expense_id.to_string()])
                .map_err(|e| {
                    AppError::Database(format!(
                        "expense_id={}のロールバックエラー: {e}",
                        expense_id
                    ))
                })?;

            debug!(
                "expense_id={}をロールバックしました: {}",
                expense_id, original_url
            );
        }

        warn!("ロールバック完了: {}件復元", rollback_data.len());
        Ok(())
    }

    /// 更新後検証
    ///
    /// # 引数
    /// * `update_items` - 更新対象アイテム一覧
    ///
    /// # 戻り値
    /// 検証成功数
    async fn verify_updated_urls(update_items: &[UrlUpdateItem]) -> AppResult<usize> {
        info!("更新後検証を開始します: {}件", update_items.len());

        let conn = get_database_connection().await?;
        let query = "
            SELECT receipt_url 
            FROM expenses 
            WHERE id = ? AND user_id = ?
        ";

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| AppError::Database(format!("検証用SELECT文準備エラー: {e}")))?;

        let mut verified_count = 0;

        for item in update_items {
            let current_url: Option<String> = stmt
                .query_row(
                    [&item.expense_id.to_string(), &item.user_id.to_string()],
                    |row| row.get::<_, Option<String>>(0),
                )
                .optional()
                .map_err(|e| {
                    AppError::Database(format!("expense_id={}の検証エラー: {e}", item.expense_id))
                })?
                .flatten(); // Option<Option<String>> -> Option<String>

            match current_url {
                Some(url) => {
                    if url == item.new_url {
                        verified_count += 1;
                        debug!("expense_id={}の検証成功", item.expense_id);
                    } else {
                        warn!(
                            "expense_id={}の検証失敗: 期待値={}, 実際値={}",
                            item.expense_id, item.new_url, url
                        );
                    }
                }
                None => {
                    warn!("expense_id={}のレコードが見つかりません", item.expense_id);
                }
            }
        }

        info!(
            "更新後検証完了: {}/{}件成功",
            verified_count,
            update_items.len()
        );
        Ok(verified_count)
    }

    /// レガシーURL検出
    ///
    /// # 戻り値
    /// レガシーURLを持つ経費一覧
    pub async fn detect_legacy_urls() -> AppResult<Vec<UrlUpdateItem>> {
        info!("レガシーURL検出を開始します");

        let conn = get_database_connection().await?;
        let query = "
            SELECT id, user_id, receipt_url 
            FROM expenses 
            WHERE receipt_url IS NOT NULL 
            AND receipt_url LIKE '%/receipts/%'
            AND receipt_url NOT LIKE '%/users/%/receipts/%'
            ORDER BY id
        ";

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| AppError::Database(format!("レガシーURL検出用SELECT文準備エラー: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,    // id
                    row.get::<_, i64>(1)?,    // user_id
                    row.get::<_, String>(2)?, // receipt_url
                ))
            })
            .map_err(|e| AppError::Database(format!("レガシーURL検出クエリエラー: {e}")))?;

        let mut legacy_items = Vec::new();

        for row in rows {
            let (expense_id, user_id, old_url) =
                row.map_err(|e| AppError::Database(format!("レガシーURL行読み取りエラー: {e}")))?;

            // 新しいURLを生成
            let new_url = Self::convert_legacy_url_to_user_url(&old_url, user_id)?;

            legacy_items.push(UrlUpdateItem {
                expense_id,
                old_url,
                new_url,
                user_id,
            });
        }

        info!("レガシーURL検出完了: {}件", legacy_items.len());
        Ok(legacy_items)
    }

    /// レガシーURLをユーザーURLに変換
    ///
    /// # 引数
    /// * `legacy_url` - レガシーURL
    /// * `user_id` - ユーザーID
    ///
    /// # 戻り値
    /// 新しいURL
    fn convert_legacy_url_to_user_url(legacy_url: &str, user_id: i64) -> AppResult<String> {
        // URL例: "https://bucket.r2.cloudflarestorage.com/receipts/123/file.pdf"
        // 変換後: "https://bucket.r2.cloudflarestorage.com/users/456/receipts/123/file.pdf"

        if let Some(receipts_pos) = legacy_url.find("/receipts/") {
            let base_url = &legacy_url[..receipts_pos];
            let path_part = &legacy_url[receipts_pos + 1..]; // "receipts/123/file.pdf"

            if let Some(stripped) = path_part.strip_prefix("receipts/") {
                let new_url = format!("{base_url}/users/{user_id}/receipts/{stripped}");
                return Ok(new_url);
            }
        }

        Err(AppError::validation(format!(
            "無効なレガシーURL形式: {}",
            legacy_url
        )))
    }

    /// データベース統計情報を取得
    ///
    /// # 戻り値
    /// 統計情報
    pub async fn get_database_statistics() -> AppResult<DatabaseStatistics> {
        info!("データベース統計情報を取得中");

        let conn = get_database_connection().await?;

        // 総経費レコード数
        let total_expenses: i64 = conn
            .query_row("SELECT COUNT(*) FROM expenses", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|e| AppError::Database(format!("総経費数取得エラー: {e}")))?;

        // receipt_urlを持つレコード数
        let with_receipt_url: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM expenses WHERE receipt_url IS NOT NULL",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| AppError::Database(format!("receipt_url付きレコード数取得エラー: {e}")))?;

        // レガシーURL数
        let legacy_urls: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM expenses WHERE receipt_url LIKE '%/receipts/%' AND receipt_url NOT LIKE '%/users/%/receipts/%'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| AppError::Database(format!("レガシーURL数取得エラー: {e}")))?;

        // 新形式URL数
        let user_urls: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM expenses WHERE receipt_url LIKE '%/users/%/receipts/%'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| AppError::Database(format!("ユーザーURL数取得エラー: {e}")))?;

        let stats = DatabaseStatistics {
            total_expenses: total_expenses as usize,
            with_receipt_url: with_receipt_url as usize,
            legacy_urls: legacy_urls as usize,
            user_urls: user_urls as usize,
        };

        info!("データベース統計情報取得完了: {stats:?}");
        Ok(stats)
    }
}

/// データベース統計情報
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseStatistics {
    /// 総経費レコード数
    pub total_expenses: usize,
    /// receipt_urlを持つレコード数
    pub with_receipt_url: usize,
    /// レガシーURL数
    pub legacy_urls: usize,
    /// 新形式URL数
    pub user_urls: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_legacy_url_to_user_url() {
        let legacy_url = "https://bucket.r2.cloudflarestorage.com/receipts/123/file.pdf";
        let user_id = 456;

        let result = DatabaseUpdater::convert_legacy_url_to_user_url(legacy_url, user_id).unwrap();
        let expected = "https://bucket.r2.cloudflarestorage.com/users/456/receipts/123/file.pdf";

        assert_eq!(result, expected);
    }

    #[test]
    fn test_convert_legacy_url_invalid_format() {
        let invalid_url = "https://bucket.r2.cloudflarestorage.com/invalid/path";
        let user_id = 456;

        let result = DatabaseUpdater::convert_legacy_url_to_user_url(invalid_url, user_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_url_update_item_creation() {
        let item = UrlUpdateItem {
            expense_id: 123,
            old_url: "https://example.com/receipts/123/file.pdf".to_string(),
            new_url: "https://example.com/users/456/receipts/123/file.pdf".to_string(),
            user_id: 456,
        };

        assert_eq!(item.expense_id, 123);
        assert_eq!(item.user_id, 456);
        assert!(item.old_url.contains("/receipts/"));
        assert!(item.new_url.contains("/users/456/receipts/"));
    }

    #[test]
    fn test_database_update_result_creation() {
        let result = DatabaseUpdateResult {
            total_records: 100,
            updated_count: 95,
            failed_count: 5,
            verified_count: 95,
            errors: vec!["テストエラー".to_string()],
            duration_ms: 1500,
        };

        assert_eq!(result.total_records, 100);
        assert_eq!(result.updated_count, 95);
        assert_eq!(result.failed_count, 5);
        assert_eq!(result.verified_count, 95);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.duration_ms, 1500);
    }

    #[tokio::test]
    async fn test_database_statistics_structure() {
        let stats = DatabaseStatistics {
            total_expenses: 1000,
            with_receipt_url: 800,
            legacy_urls: 300,
            user_urls: 500,
        };

        assert_eq!(stats.total_expenses, 1000);
        assert_eq!(stats.with_receipt_url, 800);
        assert_eq!(stats.legacy_urls, 300);
        assert_eq!(stats.user_urls, 500);

        // 整合性チェック
        assert_eq!(stats.legacy_urls + stats.user_urls, stats.with_receipt_url);
    }
}

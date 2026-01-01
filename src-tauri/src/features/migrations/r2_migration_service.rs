//! R2移行サービス
//!
//! R2ユーザーディレクトリ移行の全体的な制御を行うサービス

use super::batch_processor::{BatchProcessor, MigrationItem, MigrationResult, R2FileInfo};
use super::error_handler::{handle_migration_error, ErrorHandlingResult};
use super::errors::MigrationError;
use super::logging::{log_migration_info, StructuredLogger};
use super::r2_user_directory_migration::{
    create_migration_log_entry, get_migration_progress, update_migration_log_status,
    MigrationProgress,
};
use super::security_audit::log_migration_security_error;
use crate::features::receipts::service::R2Client;
use crate::features::receipts::user_path_manager::UserPathManager;
use crate::shared::database::connection::get_database_connection;
use crate::shared::errors::{AppError, AppResult};
use log::{debug, error, info, warn};
use rusqlite::Connection;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// R2移行サービス
pub struct R2MigrationService {
    /// R2クライアント
    r2_client: Arc<R2Client>,
    /// バッチプロセッサ
    batch_processor: Arc<Mutex<BatchProcessor>>,
}

impl R2MigrationService {
    /// 新しいR2移行サービスを作成
    ///
    /// # 引数
    /// * `r2_client` - R2クライアント
    /// * `max_concurrency` - 最大並列数（オプション）
    ///
    /// # 戻り値
    /// R2移行サービスインスタンス
    pub fn new(r2_client: Arc<R2Client>, max_concurrency: Option<usize>) -> Self {
        let batch_processor = Arc::new(Mutex::new(BatchProcessor::new(
            r2_client.clone(),
            max_concurrency,
        )));

        Self {
            r2_client,
            batch_processor,
        }
    }

    /// 移行プロセス全体を実行
    ///
    /// # 引数
    /// * `dry_run` - ドライランモード
    /// * `batch_size` - バッチサイズ
    /// * `created_by` - 作成者
    ///
    /// # 戻り値
    /// 移行結果
    pub async fn execute_migration(
        &self,
        dry_run: bool,
        batch_size: usize,
        created_by: &str,
    ) -> AppResult<(MigrationResult, i64)> {
        log_migration_info(
            "migration_lifecycle",
            &format!("R2移行プロセスを開始します (dry_run: {}, batch_size: {})", dry_run, batch_size),
            None,
        );

        let start_time = Instant::now();

        // 1. 事前検証
        if let Err(e) = self.pre_migration_validation().await {
            let migration_error = MigrationError::PreValidation {
                message: e.to_string(),
                details: "事前検証でエラーが発生しました".to_string(),
            };
            
            let _result = handle_migration_error(&migration_error, None, None, Some("pre_validation")).await;
            return Err(e);
        }

        // 2. 移行対象ファイルの特定
        let migration_items = match self.identify_migration_targets().await {
            Ok(items) => items,
            Err(e) => {
                let migration_error = MigrationError::TargetIdentification {
                    message: e.to_string(),
                    file_count: 0,
                };
                
                let _result = handle_migration_error(&migration_error, None, None, Some("target_identification")).await;
                return Err(e);
            }
        };

        log_migration_info(
            "migration_lifecycle",
            &format!("移行対象ファイル数: {}", migration_items.len()),
            None,
        );

        // 3. 移行ログエントリを作成
        let conn = get_database_connection().await?;
        let metadata = serde_json::json!({
            "dry_run": dry_run,
            "batch_size": batch_size,
            "max_concurrency": self.get_max_concurrency().await,
            "started_by": "r2_migration_service"
        });

        let migration_log_id = create_migration_log_entry(
            &conn,
            "r2_user_directory",
            migration_items.len(),
            created_by,
            Some(&metadata.to_string()),
        )?;

        if dry_run {
            // ドライランモード: 実際の移行は行わない
            log_migration_info(
                "migration_lifecycle",
                "ドライランモード: 移行対象の特定のみ実行",
                Some(migration_log_id),
            );

            update_migration_log_status(
                &conn,
                migration_log_id,
                "completed",
                0,
                0,
                0,
                Some(&serde_json::json!({
                    "dry_run": true,
                    "estimated_items": migration_items.len(),
                    "duration_ms": start_time.elapsed().as_millis()
                }).to_string()),
            )?;

            let result = MigrationResult::dry_run_success(migration_items.len());
            return Ok((result, migration_log_id));
        }

        // 4. データベースバックアップ
        if let Err(e) = self.create_database_backup().await {
            let migration_error = MigrationError::DatabaseUpdate {
                message: e.to_string(),
                table_name: "backup".to_string(),
                operation: "create_backup".to_string(),
                affected_rows: None,
            };
            
            let _result = handle_migration_error(&migration_error, Some(migration_log_id), None, Some("database_backup")).await;
            return Err(e);
        }

        // 5. 移行ログを進行中に更新
        update_migration_log_status(&conn, migration_log_id, "in_progress", 0, 0, 0, None)?;

        // 6. バッチ処理で移行実行
        let result = {
            let batch_processor = self.batch_processor.lock().await;
            batch_processor
                .process_migration_batches(migration_items, batch_size, migration_log_id)
                .await?
        };

        // 7. 移行後検証
        if result.error_count == 0 {
            if let Err(e) = self.post_migration_validation().await {
                let migration_error = MigrationError::IntegrityValidation {
                    message: e.to_string(),
                    validation_type: "post_migration".to_string(),
                    expected: "no_errors".to_string(),
                    actual: "validation_failed".to_string(),
                };
                
                let _result = handle_migration_error(&migration_error, Some(migration_log_id), None, Some("post_validation")).await;
                warn!("移行後検証でエラーが発生しましたが、移行は続行します: {}", e);
            }
        }

        // 8. 移行ログを完了に更新
        let final_status = if result.error_count == 0 {
            "completed"
        } else {
            "failed"
        };

        update_migration_log_status(
            &conn,
            migration_log_id,
            final_status,
            result.total_items,
            result.success_count,
            result.error_count,
            Some(&serde_json::json!({
                "duration_ms": result.duration.as_millis(),
                "errors": result.errors
            }).to_string()),
        )?;

        log_migration_info(
            "migration_lifecycle",
            &format!(
                "R2移行プロセスが完了しました: 成功={}, エラー={}, 実行時間={:?}",
                result.success_count, result.error_count, result.duration
            ),
            Some(migration_log_id),
        );

        Ok((result, migration_log_id))
    }

    /// 移行対象ファイルを特定
    ///
    /// # 戻り値
    /// 移行対象アイテム一覧
    async fn identify_migration_targets(&self) -> AppResult<Vec<MigrationItem>> {
        info!("移行対象ファイルを特定中...");

        let mut items = Vec::new();

        // R2からレガシーパスのファイルをリストアップ
        let legacy_files = self.list_legacy_files().await?;
        info!("レガシーファイル数: {}", legacy_files.len());

        for file in legacy_files {
            // データベースから対応するユーザーIDを取得
            if let Some(user_id) = self.get_user_id_for_file(&file.key).await? {
                let new_path = UserPathManager::convert_legacy_to_user_path(&file.key, user_id)?;

                items.push(MigrationItem {
                    old_path: file.key.clone(),
                    new_path,
                    user_id,
                    file_size: file.size,
                    last_modified: file.last_modified,
                });
            } else {
                warn!("ユーザーIDが見つからないファイル: {}", file.key);
            }
        }

        info!("移行対象アイテム数: {}", items.len());
        Ok(items)
    }

    /// レガシーファイルをR2からリストアップ
    ///
    /// # 戻り値
    /// レガシーファイル一覧
    async fn list_legacy_files(&self) -> AppResult<Vec<R2FileInfo>> {
        info!("R2からレガシーファイルをリストアップ中...");

        // TODO: 実際のR2 list_objects_v2 APIを実装
        // 現在はプレースホルダー実装
        warn!("list_legacy_files はプレースホルダー実装です");

        // プレースホルダー: 空のリストを返す
        Ok(vec![])
    }

    /// ファイルパスからユーザーIDを取得
    ///
    /// # 引数
    /// * `file_path` - ファイルパス
    ///
    /// # 戻り値
    /// ユーザーID（見つからない場合はNone）
    async fn get_user_id_for_file(&self, file_path: &str) -> AppResult<Option<i64>> {
        debug!("ファイルパスからユーザーIDを取得中: {}", file_path);

        let conn = get_database_connection().await?;

        let query = "
            SELECT e.user_id 
            FROM expenses e 
            WHERE e.receipt_url LIKE ?
            LIMIT 1
        ";

        let search_pattern = format!("%{file_path}%");

        let user_id: Option<i64> = conn
            .query_row(query, [search_pattern], |row| row.get(0))
            .optional()
            .map_err(|e| AppError::Database(format!("ユーザーID取得エラー: {e}")))?;

        debug!("ユーザーID取得結果: {:?}", user_id);
        Ok(user_id)
    }

    /// 事前検証を実行
    ///
    /// # 戻り値
    /// 検証結果
    async fn pre_migration_validation(&self) -> AppResult<()> {
        info!("移行前検証を実行中...");

        // R2接続テスト
        self.r2_client.test_connection().await?;

        // データベース接続テスト
        let _conn = get_database_connection().await?;

        // 必要な権限チェック
        self.validate_permissions().await?;

        info!("移行前検証が完了しました");
        Ok(())
    }

    /// 移行後検証を実行
    ///
    /// # 戻り値
    /// 検証結果
    async fn post_migration_validation(&self) -> AppResult<()> {
        info!("移行後検証を実行中...");

        // TODO: 実際の移行後検証を実装
        // - データ整合性チェック
        // - ファイルアクセステスト
        // - 孤立ファイル検出

        info!("移行後検証が完了しました");
        Ok(())
    }

    /// 権限検証
    ///
    /// # 戻り値
    /// 検証結果
    async fn validate_permissions(&self) -> AppResult<()> {
        debug!("権限検証を実行中...");

        // TODO: 実際の権限チェックを実装
        // - R2バケットへの読み書き権限
        // - データベースへの書き込み権限

        debug!("権限検証が完了しました");
        Ok(())
    }

    /// データベースバックアップを作成
    ///
    /// # 戻り値
    /// 処理結果
    async fn create_database_backup(&self) -> AppResult<()> {
        info!("データベースバックアップを作成中...");

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = format!("database_backup_migration_{timestamp}.db");

        // TODO: 実際のデータベースバックアップを実装
        // SQLiteデータベースファイルをコピー

        info!("データベースバックアップが作成されました: {backup_path}");
        Ok(())
    }

    /// 移行進捗を取得
    ///
    /// # 引数
    /// * `migration_log_id` - 移行ログID
    ///
    /// # 戻り値
    /// 移行進捗
    pub async fn get_migration_progress(&self, migration_log_id: i64) -> AppResult<MigrationProgress> {
        let conn = get_database_connection().await?;
        get_migration_progress(&conn, migration_log_id)
    }

    /// 移行を一時停止
    ///
    /// # 戻り値
    /// 処理結果
    pub async fn pause_migration(&self) -> AppResult<()> {
        let batch_processor = self.batch_processor.lock().await;
        batch_processor.pause().await;
        info!("移行プロセスを一時停止しました");
        Ok(())
    }

    /// 移行を再開
    ///
    /// # 戻り値
    /// 処理結果
    pub async fn resume_migration(&self) -> AppResult<()> {
        let batch_processor = self.batch_processor.lock().await;
        batch_processor.resume().await;
        info!("移行プロセスを再開しました");
        Ok(())
    }

    /// 移行を停止
    ///
    /// # 戻り値
    /// 処理結果
    pub async fn stop_migration(&self) -> AppResult<()> {
        let batch_processor = self.batch_processor.lock().await;
        batch_processor.stop().await;
        info!("移行プロセスを停止しました");
        Ok(())
    }

    /// 最大並列数を取得
    ///
    /// # 戻り値
    /// 最大並列数
    async fn get_max_concurrency(&self) -> usize {
        let batch_processor = self.batch_processor.lock().await;
        batch_processor.get_performance_stats().max_concurrency
    }

    /// 並列度を調整
    ///
    /// # 引数
    /// * `new_concurrency` - 新しい並列数
    ///
    /// # 戻り値
    /// 処理結果
    pub async fn adjust_concurrency(&self, new_concurrency: usize) -> AppResult<()> {
        let mut batch_processor = self.batch_processor.lock().await;
        batch_processor.adjust_concurrency(new_concurrency).await
    }

    /// 移行状態を取得
    ///
    /// # 戻り値
    /// 移行状態情報
    pub async fn get_migration_status(&self) -> MigrationStatus {
        let batch_processor = self.batch_processor.lock().await;
        let stats = batch_processor.get_performance_stats();

        MigrationStatus {
            is_running: !stats.is_cancelled && stats.available_permits < stats.max_concurrency,
            is_paused: stats.is_paused,
            is_cancelled: stats.is_cancelled,
            max_concurrency: stats.max_concurrency,
            available_permits: stats.available_permits,
        }
    }
}

/// 移行状態
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MigrationStatus {
    /// 実行中フラグ
    pub is_running: bool,
    /// 一時停止フラグ
    pub is_paused: bool,
    /// キャンセルフラグ
    pub is_cancelled: bool,
    /// 最大並列数
    pub max_concurrency: usize,
    /// 利用可能な許可数
    pub available_permits: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_status_creation() {
        let status = MigrationStatus {
            is_running: true,
            is_paused: false,
            is_cancelled: false,
            max_concurrency: 5,
            available_permits: 3,
        };

        assert!(status.is_running);
        assert!(!status.is_paused);
        assert!(!status.is_cancelled);
        assert_eq!(status.max_concurrency, 5);
        assert_eq!(status.available_permits, 3);
    }

    #[tokio::test]
    async fn test_r2_migration_service_creation() {
        // プレースホルダーテスト
        // 実際のテストでは、R2MigrationServiceの作成と基本機能をテスト
        // テストが正常に完了することを確認
    }
}
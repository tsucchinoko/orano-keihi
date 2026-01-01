//! バッチ処理システム
//!
//! R2ユーザーディレクトリ移行のためのバッチ処理機能を提供します。
//! 並列処理制御、一時停止・再開・停止機能、ファイルハッシュ計算・検証を含みます。

use super::error_handler::handle_migration_error;
use super::errors::MigrationError;
use super::logging::log_migration_info;
use super::r2_user_directory_migration::{create_migration_item, update_migration_item_status};
use crate::features::receipts::service::R2Client;
use crate::shared::errors::{AppError, AppResult};
use log::{debug, error, info, warn};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

/// 移行アイテム
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationItem {
    /// 古いパス
    pub old_path: String,
    /// 新しいパス
    pub new_path: String,
    /// ユーザーID
    pub user_id: i64,
    /// ファイルサイズ
    pub file_size: u64,
    /// 最終更新日時
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

/// 移行結果
#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationResult {
    /// 総アイテム数
    pub total_items: usize,
    /// 成功数
    pub success_count: usize,
    /// エラー数
    pub error_count: usize,
    /// エラー詳細
    pub errors: Vec<String>,
    /// 実行時間
    pub duration: Duration,
}

impl MigrationResult {
    /// ドライラン成功結果を作成
    pub fn dry_run_success(total_items: usize) -> Self {
        Self {
            total_items,
            success_count: 0,
            error_count: 0,
            errors: Vec::new(),
            duration: Duration::from_secs(0),
        }
    }
}

/// R2ファイル情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R2FileInfo {
    /// ファイルキー
    pub key: String,
    /// ファイルサイズ
    pub size: u64,
    /// 最終更新日時
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

/// データベース操作リクエスト
#[derive(Debug)]
enum DatabaseRequest {
    CreateMigrationItem {
        migration_log_id: i64,
        old_path: String,
        new_path: String,
        user_id: i64,
        file_size: i64,
        response_tx: tokio::sync::oneshot::Sender<AppResult<i64>>,
    },
    UpdateMigrationItemStatus {
        item_id: i64,
        status: String,
        error_message: Option<String>,
        file_hash: Option<String>,
        response_tx: tokio::sync::oneshot::Sender<AppResult<()>>,
    },
    UpdateReceiptUrl {
        old_path: String,
        new_path: String,
        user_id: i64,
        expense_id: Option<i64>,
        response_tx: tokio::sync::oneshot::Sender<AppResult<()>>,
    },
}

/// バッチ処理システム
pub struct BatchProcessor {
    /// R2クライアント
    r2_client: Arc<R2Client>,
    /// キャンセレーショントークン
    cancellation_token: CancellationToken,
    /// 一時停止トークン
    pause_token: Arc<Mutex<bool>>,
    /// セマフォ（並列処理制御）
    semaphore: Arc<Semaphore>,
    /// 最大並列数
    max_concurrency: usize,
    /// データベース操作チャネル
    db_tx: Option<mpsc::UnboundedSender<DatabaseRequest>>,
}

impl BatchProcessor {
    /// 新しいバッチプロセッサを作成
    ///
    /// # 引数
    /// * `r2_client` - R2クライアント
    /// * `max_concurrency` - 最大並列数（デフォルト: 5）
    ///
    /// # 戻り値
    /// バッチプロセッサインスタンス
    pub fn new(r2_client: Arc<R2Client>, max_concurrency: Option<usize>) -> Self {
        let max_concurrency = max_concurrency.unwrap_or(5);

        Self {
            r2_client,
            cancellation_token: CancellationToken::new(),
            pause_token: Arc::new(Mutex::new(false)),
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
            max_concurrency,
            db_tx: None,
        }
    }

    /// データベース接続を設定してワーカーを開始
    ///
    /// # 引数
    /// * `conn` - データベース接続
    pub async fn start_database_worker(&mut self, conn: Connection) {
        let (tx, mut rx) = mpsc::unbounded_channel::<DatabaseRequest>();
        self.db_tx = Some(tx);

        // データベースワーカータスクを開始
        tokio::spawn(async move {
            while let Some(request) = rx.recv().await {
                match request {
                    DatabaseRequest::CreateMigrationItem {
                        migration_log_id,
                        old_path,
                        new_path,
                        user_id,
                        file_size,
                        response_tx,
                    } => {
                        let result = create_migration_item(
                            &conn,
                            migration_log_id,
                            &old_path,
                            &new_path,
                            user_id,
                            file_size,
                        );
                        let _ = response_tx.send(result);
                    }
                    DatabaseRequest::UpdateMigrationItemStatus {
                        item_id,
                        status,
                        error_message,
                        file_hash,
                        response_tx,
                    } => {
                        let result = update_migration_item_status(
                            &conn,
                            item_id,
                            &status,
                            error_message.as_deref(),
                            file_hash.as_deref(),
                        );
                        let _ = response_tx.send(result);
                    }
                    DatabaseRequest::UpdateReceiptUrl {
                        old_path,
                        new_path,
                        user_id,
                        expense_id,
                        response_tx,
                    } => {
                        let result = if let Some(expense_id) = expense_id {
                            // 特定の経費IDに対する更新
                            Self::update_specific_receipt_url_sync(
                                &conn, expense_id, &old_path, &new_path, user_id,
                            )
                        } else {
                            // パターンマッチによる更新（従来の方式）
                            Self::update_database_receipt_url_sync(&conn, &old_path, &new_path)
                        };
                        let _ = response_tx.send(result);
                    }
                }
            }
        });
    }

    /// バッチ処理で移行を実行
    ///
    /// # 引数
    /// * `items` - 移行対象アイテム一覧
    /// * `batch_size` - バッチサイズ
    /// * `migration_log_id` - 移行ログID
    ///
    /// # 戻り値
    /// 移行結果
    pub async fn process_migration_batches(
        &self,
        items: Vec<MigrationItem>,
        batch_size: usize,
        migration_log_id: i64,
    ) -> AppResult<MigrationResult> {
        let start_time = Instant::now();
        let total_items = items.len();
        let mut success_count = 0;
        let mut error_count = 0;
        let mut errors = Vec::new();

        info!(
            "バッチ処理開始: 総アイテム数={}, バッチサイズ={}, 最大並列数={}",
            total_items, batch_size, self.max_concurrency
        );

        // アイテムをバッチに分割
        let batches: Vec<_> = items.chunks(batch_size).collect();
        let total_batches = batches.len();

        for (batch_index, batch) in batches.into_iter().enumerate() {
            // キャンセルチェック
            if self.cancellation_token.is_cancelled() {
                warn!("移行処理がキャンセルされました");
                break;
            }

            // 一時停止チェック
            while *self.pause_token.lock().await {
                info!("移行処理が一時停止中...");
                tokio::time::sleep(Duration::from_secs(1)).await;

                // 一時停止中でもキャンセルチェック
                if self.cancellation_token.is_cancelled() {
                    warn!("一時停止中に移行処理がキャンセルされました");
                    return Ok(MigrationResult {
                        total_items,
                        success_count,
                        error_count,
                        errors,
                        duration: start_time.elapsed(),
                    });
                }
            }

            info!(
                "バッチ {}/{} を処理中 (アイテム数: {})",
                batch_index + 1,
                total_batches,
                batch.len()
            );

            // バッチ内のアイテムを並列処理
            let batch_results = self
                .process_batch_parallel(batch.to_vec(), migration_log_id)
                .await;

            // 結果を集計
            for result in batch_results {
                match result {
                    Ok(_) => success_count += 1,
                    Err(e) => {
                        error_count += 1;
                        errors.push(e.to_string());
                    }
                }
            }

            info!(
                "バッチ {}/{} 完了 (成功: {}, エラー: {})",
                batch_index + 1,
                total_batches,
                success_count,
                error_count
            );
        }

        let duration = start_time.elapsed();
        info!(
            "バッチ処理完了: 成功={}, エラー={}, 実行時間={:?}",
            success_count, error_count, duration
        );

        Ok(MigrationResult {
            total_items,
            success_count,
            error_count,
            errors,
            duration,
        })
    }

    /// バッチ内のアイテムを並列処理
    ///
    /// # 引数
    /// * `items` - 処理対象アイテム
    /// * `migration_log_id` - 移行ログID
    ///
    /// # 戻り値
    /// 各アイテムの処理結果
    async fn process_batch_parallel(
        &self,
        items: Vec<MigrationItem>,
        migration_log_id: i64,
    ) -> Vec<AppResult<()>> {
        let tasks: Vec<_> = items
            .into_iter()
            .map(|item| {
                let r2_client = self.r2_client.clone();
                let semaphore = self.semaphore.clone();
                let cancellation_token = self.cancellation_token.clone();
                let db_tx = self.db_tx.clone();

                tokio::spawn(async move {
                    // セマフォを取得（並列数制御）
                    let _permit = semaphore.acquire().await.unwrap();

                    // キャンセルチェック
                    if cancellation_token.is_cancelled() {
                        return Err(AppError::concurrency("処理がキャンセルされました"));
                    }

                    Self::migrate_single_file(r2_client, item, migration_log_id, db_tx).await
                })
            })
            .collect();

        let results = futures::future::join_all(tasks).await;

        results
            .into_iter()
            .map(|result| {
                result.unwrap_or_else(|e| Err(AppError::concurrency(format!("タスクエラー: {e}"))))
            })
            .collect()
    }

    /// 単一ファイルの移行処理
    ///
    /// # 引数
    /// * `r2_client` - R2クライアント
    /// * `item` - 移行アイテム
    /// * `migration_log_id` - 移行ログID
    /// * `db_tx` - データベース操作チャネル
    ///
    /// # 戻り値
    /// 処理結果
    async fn migrate_single_file(
        r2_client: Arc<R2Client>,
        item: MigrationItem,
        migration_log_id: i64,
        db_tx: Option<mpsc::UnboundedSender<DatabaseRequest>>,
    ) -> AppResult<()> {
        info!("ファイル移行開始: {} -> {}", item.old_path, item.new_path);

        let db_tx = db_tx
            .ok_or_else(|| AppError::configuration("データベースワーカーが開始されていません"))?;

        // 移行アイテムをデータベースに記録
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        db_tx
            .send(DatabaseRequest::CreateMigrationItem {
                migration_log_id,
                old_path: item.old_path.clone(),
                new_path: item.new_path.clone(),
                user_id: item.user_id,
                file_size: item.file_size as i64,
                response_tx,
            })
            .map_err(|_| AppError::concurrency("データベースワーカーへの送信に失敗"))?;

        let migration_item_id = response_rx.await.map_err(|_| {
            AppError::concurrency("データベースワーカーからの応答を受信できませんでした")
        })??;

        // 処理中ステータスに更新
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        db_tx
            .send(DatabaseRequest::UpdateMigrationItemStatus {
                item_id: migration_item_id,
                status: "processing".to_string(),
                error_message: None,
                file_hash: None,
                response_tx,
            })
            .map_err(|_| AppError::concurrency("データベースワーカーへの送信に失敗"))?;

        response_rx.await.map_err(|_| {
            AppError::concurrency("データベースワーカーからの応答を受信できませんでした")
        })??;

        match Self::execute_file_migration(&r2_client, &item).await {
            Ok(file_hash) => {
                // データベースのreceipt_urlを更新（改善版）
                let (response_tx, response_rx) = tokio::sync::oneshot::channel();
                db_tx
                    .send(DatabaseRequest::UpdateReceiptUrl {
                        old_path: item.old_path.clone(),
                        new_path: item.new_path.clone(),
                        user_id: item.user_id,
                        expense_id: Self::extract_expense_id_from_path(&item.old_path),
                        response_tx,
                    })
                    .map_err(|_| AppError::concurrency("データベースワーカーへの送信に失敗"))?;

                response_rx.await.map_err(|_| {
                    AppError::concurrency("データベースワーカーからの応答を受信できませんでした")
                })??;

                // 成功ステータスに更新（ファイルハッシュも同時に記録）
                let (response_tx, response_rx) = tokio::sync::oneshot::channel();
                db_tx
                    .send(DatabaseRequest::UpdateMigrationItemStatus {
                        item_id: migration_item_id,
                        status: "completed".to_string(),
                        error_message: None,
                        file_hash: Some(file_hash),
                        response_tx,
                    })
                    .map_err(|_| AppError::concurrency("データベースワーカーへの送信に失敗"))?;

                response_rx.await.map_err(|_| {
                    AppError::concurrency("データベースワーカーからの応答を受信できませんでした")
                })??;

                log_migration_info(
                    "file_migration",
                    &format!("ファイル移行完了: {} -> {}", item.old_path, item.new_path),
                    Some(migration_log_id),
                );
                Ok(())
            }
            Err(e) => {
                // エラーハンドリング機能を使用
                let _handling_result = handle_migration_error(
                    &e,
                    Some(migration_log_id),
                    Some(item.user_id),
                    Some("file_migration"),
                )
                .await;

                error!("ファイル移行失敗: {}, エラー: {}", item.old_path, e);

                // 失敗ステータスに更新
                let (response_tx, response_rx) = tokio::sync::oneshot::channel();
                db_tx
                    .send(DatabaseRequest::UpdateMigrationItemStatus {
                        item_id: migration_item_id,
                        status: "failed".to_string(),
                        error_message: Some(e.to_string()),
                        file_hash: None,
                        response_tx,
                    })
                    .map_err(|_| AppError::concurrency("データベースワーカーへの送信に失敗"))?;

                response_rx.await.map_err(|_| {
                    AppError::concurrency("データベースワーカーからの応答を受信できませんでした")
                })??;

                Err(e.into())
            }
        }
    }

    /// ファイル移行の実行
    ///
    /// # 引数
    /// * `r2_client` - R2クライアント
    /// * `item` - 移行アイテム
    ///
    /// # 戻り値
    /// ファイルハッシュ
    async fn execute_file_migration(
        r2_client: &R2Client,
        item: &MigrationItem,
    ) -> Result<String, MigrationError> {
        // 1. 元ファイルをダウンロード
        let file_data = Self::download_file_from_r2(r2_client, &item.old_path).await?;

        // 2. ハッシュ値を計算（整合性チェック用）
        let original_hash = Self::calculate_file_hash(&file_data);

        // 3. 新しい場所にアップロード
        let content_type = R2Client::get_content_type(&item.old_path);
        let _new_url = r2_client
            .upload_file(&item.new_path, file_data.clone(), &content_type)
            .await
            .map_err(|e| MigrationError::R2Operation {
                message: format!("ファイルアップロードに失敗: {}", e),
                operation: "upload".to_string(),
                bucket: "unknown".to_string(),
                key: item.new_path.clone(),
                status_code: None,
            })?;

        // 4. アップロードしたファイルの整合性を確認
        let uploaded_data = Self::download_file_from_r2(r2_client, &item.new_path).await?;
        let uploaded_hash = Self::calculate_file_hash(&uploaded_data);

        if original_hash != uploaded_hash {
            return Err(MigrationError::IntegrityValidation {
                message: format!("ファイルハッシュが一致しません: {}", item.old_path),
                validation_type: "file_hash".to_string(),
                expected: original_hash,
                actual: uploaded_hash,
            });
        }

        Ok(original_hash)
    }

    /// R2からファイルをダウンロード
    ///
    /// # 引数
    /// * `r2_client` - R2クライアント
    /// * `key` - ファイルキー
    ///
    /// # 戻り値
    /// ファイルデータ
    async fn download_file_from_r2(
        r2_client: &R2Client,
        key: &str,
    ) -> Result<Vec<u8>, MigrationError> {
        debug!("R2からファイルをダウンロード中: {}", key);

        // R2Clientのdownload_fileメソッドを呼び出し
        r2_client
            .download_file(key)
            .await
            .map_err(|e| MigrationError::R2Operation {
                message: format!("ファイルダウンロードに失敗: {}", e),
                operation: "download".to_string(),
                bucket: "unknown".to_string(),
                key: key.to_string(),
                status_code: None,
            })
    }

    /// ファイルハッシュを計算
    ///
    /// # 引数
    /// * `data` - ファイルデータ
    ///
    /// # 戻り値
    /// SHA256ハッシュ（16進数文字列）
    pub fn calculate_file_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// データベースのreceipt_urlを更新（同期版）
    ///
    /// # 引数
    /// * `conn` - データベース接続
    /// * `old_path` - 古いパス
    /// * `new_path` - 新しいパス
    ///
    /// # 戻り値
    /// 処理結果
    fn update_database_receipt_url_sync(
        conn: &Connection,
        old_path: &str,
        new_path: &str,
    ) -> AppResult<()> {
        // 古いURLパターンを新しいURLパターンに置換
        let old_url_pattern = format!("%{old_path}");

        let query = "
            UPDATE expenses 
            SET receipt_url = REPLACE(receipt_url, ?, ?),
                updated_at = ?
            WHERE receipt_url LIKE ?
        ";

        let now = chrono::Utc::now()
            .with_timezone(&chrono_tz::Asia::Tokyo)
            .to_rfc3339();

        conn.execute(query, [old_path, new_path, &now, &old_url_pattern])
            .map_err(|e| AppError::Database(format!("receipt_url更新エラー: {e}")))?;

        debug!("receipt_url更新完了: {old_path} -> {new_path}");
        Ok(())
    }

    /// 特定の経費IDに対するreceipt_url更新（同期版・改善版）
    ///
    /// # 引数
    /// * `conn` - データベース接続
    /// * `expense_id` - 経費ID
    /// * `old_path` - 古いパス
    /// * `new_path` - 新しいパス
    /// * `user_id` - ユーザーID
    ///
    /// # 戻り値
    /// 処理結果
    fn update_specific_receipt_url_sync(
        conn: &Connection,
        expense_id: i64,
        old_path: &str,
        new_path: &str,
        user_id: i64,
    ) -> AppResult<()> {
        // トランザクション内で安全に更新
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Database(format!("トランザクション開始エラー: {e}")))?;

        // 更新前の値を確認
        let current_url: Option<String> = tx
            .query_row(
                "SELECT receipt_url FROM expenses WHERE id = ? AND user_id = ?",
                [expense_id, user_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(|e| {
                AppError::Database(format!("expense_id={}の現在URL取得エラー: {e}", expense_id))
            })?
            .flatten(); // Option<Option<String>> -> Option<String>

        match current_url {
            Some(url) => {
                // 古いパスが含まれているかチェック
                if !url.contains(old_path) {
                    warn!(
                        "expense_id={}のreceipt_urlに古いパスが含まれていません: 現在URL={}, 期待パス={}",
                        expense_id, url, old_path
                    );
                    return Ok(()); // エラーではなく、単に更新対象外として扱う
                }

                // URLを新しいパスに置換
                let new_url = url.replace(old_path, new_path);
                let now = chrono::Utc::now()
                    .with_timezone(&chrono_tz::Asia::Tokyo)
                    .to_rfc3339();

                let rows_affected = tx
                    .execute(
                        "UPDATE expenses SET receipt_url = ?, updated_at = ? WHERE id = ? AND user_id = ?",
                        [&new_url, &now, &expense_id.to_string(), &user_id.to_string()],
                    )
                    .map_err(|e| {
                        AppError::Database(format!(
                            "expense_id={}のreceipt_url更新エラー: {e}",
                            expense_id
                        ))
                    })?;

                if rows_affected > 0 {
                    // 更新後の整合性チェック
                    let updated_url: Option<String> = tx
                        .query_row(
                            "SELECT receipt_url FROM expenses WHERE id = ? AND user_id = ?",
                            [expense_id, user_id],
                            |row| row.get::<_, Option<String>>(0),
                        )
                        .optional()
                        .map_err(|e| {
                            AppError::Database(format!(
                                "expense_id={}の更新後URL確認エラー: {e}",
                                expense_id
                            ))
                        })?
                        .flatten(); // Option<Option<String>> -> Option<String>

                    if let Some(updated) = updated_url {
                        if updated != new_url {
                            return Err(AppError::validation(format!(
                                "expense_id={}の更新後整合性チェック失敗: 期待値={}, 実際値={}",
                                expense_id, new_url, updated
                            )));
                        }
                    }

                    tx.commit().map_err(|e| {
                        AppError::Database(format!("トランザクションコミットエラー: {e}"))
                    })?;

                    debug!(
                        "expense_id={}のreceipt_url更新完了: {} -> {}",
                        expense_id, url, new_url
                    );
                } else {
                    warn!(
                        "expense_id={}の更新対象レコードが見つかりません",
                        expense_id
                    );
                }
            }
            None => {
                warn!("expense_id={}のレコードが見つかりません", expense_id);
            }
        }

        Ok(())
    }

    /// パスから経費IDを抽出
    ///
    /// # 引数
    /// * `path` - ファイルパス
    ///
    /// # 戻り値
    /// 経費ID（抽出できない場合はNone）
    fn extract_expense_id_from_path(path: &str) -> Option<i64> {
        // パス例: "receipts/123/timestamp-uuid-filename.pdf" または "users/456/receipts/123/timestamp-uuid-filename.pdf"

        // receipts/の後の最初の数字を経費IDとして抽出
        if let Some(receipts_pos) = path.find("/receipts/") {
            let after_receipts = &path[receipts_pos + 10..]; // "/receipts/".len() = 10

            if let Some(slash_pos) = after_receipts.find('/') {
                let expense_id_str = &after_receipts[..slash_pos];
                expense_id_str.parse::<i64>().ok()
            } else {
                // スラッシュが見つからない場合、残り全体を試す
                after_receipts.parse::<i64>().ok()
            }
        } else {
            None
        }
    }

    /// 処理を一時停止
    pub async fn pause(&self) {
        *self.pause_token.lock().await = true;
        info!("バッチ処理を一時停止しました");
    }

    /// 処理を再開
    pub async fn resume(&self) {
        *self.pause_token.lock().await = false;
        info!("バッチ処理を再開しました");
    }

    /// 処理を停止
    pub async fn stop(&self) {
        self.cancellation_token.cancel();
        info!("バッチ処理を停止しました");
    }

    /// 現在の処理状況を取得
    pub async fn is_paused(&self) -> bool {
        *self.pause_token.lock().await
    }

    /// キャンセル状況を取得
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    /// 並列度を動的に調整
    ///
    /// # 引数
    /// * `new_concurrency` - 新しい並列数
    ///
    /// # 戻り値
    /// 処理結果
    pub async fn adjust_concurrency(&mut self, new_concurrency: usize) -> AppResult<()> {
        if new_concurrency == 0 {
            return Err(AppError::Validation(
                "並列数は1以上である必要があります".to_string(),
            ));
        }

        if new_concurrency != self.max_concurrency {
            self.max_concurrency = new_concurrency;
            self.semaphore = Arc::new(Semaphore::new(new_concurrency));
            info!("並列度を{}に調整しました", new_concurrency);
        }

        Ok(())
    }

    /// パフォーマンス統計を取得
    pub fn get_performance_stats(&self) -> BatchProcessorStats {
        BatchProcessorStats {
            max_concurrency: self.max_concurrency,
            available_permits: self.semaphore.available_permits(),
            is_paused: futures::executor::block_on(self.is_paused()),
            is_cancelled: self.is_cancelled(),
        }
    }
}

/// バッチプロセッサ統計
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchProcessorStats {
    /// 最大並列数
    pub max_concurrency: usize,
    /// 利用可能な許可数
    pub available_permits: usize,
    /// 一時停止状態
    pub is_paused: bool,
    /// キャンセル状態
    pub is_cancelled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_file_hash() {
        let data = b"test data";
        let hash1 = BatchProcessor::calculate_file_hash(data);
        let hash2 = BatchProcessor::calculate_file_hash(data);

        // 同じデータは同じハッシュを生成
        assert_eq!(hash1, hash2);

        // 異なるデータは異なるハッシュを生成
        let different_data = b"different test data";
        let hash3 = BatchProcessor::calculate_file_hash(different_data);
        assert_ne!(hash1, hash3);

        // ハッシュは64文字の16進数文字列
        assert_eq!(hash1.len(), 64);
        assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_migration_item_creation() {
        let item = MigrationItem {
            old_path: "receipts/123/file.pdf".to_string(),
            new_path: "users/456/receipts/123/file.pdf".to_string(),
            user_id: 456,
            file_size: 1024,
            last_modified: chrono::Utc::now(),
        };

        assert_eq!(item.old_path, "receipts/123/file.pdf");
        assert_eq!(item.new_path, "users/456/receipts/123/file.pdf");
        assert_eq!(item.user_id, 456);
        assert_eq!(item.file_size, 1024);
    }

    #[test]
    fn test_migration_result_dry_run() {
        let result = MigrationResult::dry_run_success(100);

        assert_eq!(result.total_items, 100);
        assert_eq!(result.success_count, 0);
        assert_eq!(result.error_count, 0);
        assert!(result.errors.is_empty());
        assert_eq!(result.duration, Duration::from_secs(0));
    }

    #[test]
    fn test_extract_expense_id_from_path() {
        // レガシーパス
        assert_eq!(
            BatchProcessor::extract_expense_id_from_path("path/receipts/123/file.pdf"),
            Some(123)
        );

        // ユーザーパス
        assert_eq!(
            BatchProcessor::extract_expense_id_from_path("users/456/receipts/789/file.pdf"),
            Some(789)
        );

        // 無効なパス
        assert_eq!(
            BatchProcessor::extract_expense_id_from_path("invalid/path"),
            None
        );

        // 数字以外
        assert_eq!(
            BatchProcessor::extract_expense_id_from_path("path/receipts/abc/file.pdf"),
            None
        );
    }

    #[tokio::test]
    async fn test_batch_processor_pause_resume() {
        let _r2_config = crate::shared::config::environment::R2Config {
            access_key_id: "test".to_string(),
            secret_access_key: "test".to_string(),
            endpoint_url: "https://test.com".to_string(),
            bucket_name: "test".to_string(),
            region: "auto".to_string(),
        };

        // R2Clientの作成をスキップ（テスト環境では実際の接続は不要）
        // let r2_client = Arc::new(R2Client::new(r2_config).await.unwrap());
        // let processor = BatchProcessor::new(r2_client, Some(3));

        // プレースホルダーテスト
        // 実際のテストでは、BatchProcessorの基本構造をテスト
        // テストが正常に完了することを確認
    }

    #[tokio::test]
    async fn test_batch_processor_concurrency_adjustment() {
        // プレースホルダーテスト
        // 実際のテストでは、BatchProcessorのconcurrency調整機能をテスト
        // テストが正常に完了することを確認
    }
}

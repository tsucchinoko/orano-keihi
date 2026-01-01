//! R2移行コマンド
//!
//! R2ユーザーディレクトリ移行のためのTauriコマンド

use crate::features::receipts::service::R2Client;
use crate::shared::config::environment::R2Config;
use crate::shared::database::connection::get_database_connection;
use crate::shared::errors::AppError;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

/// 移行サービスマップの型エイリアス
type MigrationServiceMap = HashMap<String, Arc<SimpleR2MigrationService>>;

/// グローバル移行サービス管理
static MIGRATION_SERVICES: Lazy<Arc<Mutex<MigrationServiceMap>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// 現在実行中の移行ID管理
static CURRENT_MIGRATION_ID: Lazy<Arc<Mutex<Option<i64>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// 簡易R2移行サービス（プレースホルダー実装）
pub struct SimpleR2MigrationService {
    _r2_client: Arc<R2Client>,
}

impl SimpleR2MigrationService {
    pub fn new(r2_client: Arc<R2Client>) -> Self {
        Self {
            _r2_client: r2_client,
        }
    }

    pub async fn execute_migration(
        &self,
        dry_run: bool,
        batch_size: usize,
        created_by: &str,
    ) -> Result<(super::batch_processor::MigrationResult, i64), AppError> {
        info!(
            "簡易移行サービスを実行中: dry_run={}, batch_size={}",
            dry_run, batch_size
        );

        // プレースホルダー実装
        let result = if dry_run {
            super::batch_processor::MigrationResult::dry_run_success(0)
        } else {
            super::batch_processor::MigrationResult {
                total_items: 0,
                success_count: 0,
                error_count: 0,
                errors: Vec::new(),
                duration: std::time::Duration::from_secs(0),
            }
        };

        // 移行ログエントリを作成
        let conn = get_database_connection().await?;
        let migration_log_id = super::r2_user_directory_migration::create_migration_log_entry(
            &conn,
            "r2_user_directory",
            0,
            created_by,
            Some(&serde_json::json!({"dry_run": dry_run, "batch_size": batch_size}).to_string()),
        )?;

        Ok((result, migration_log_id))
    }

    pub async fn pause_migration(&self) -> Result<(), AppError> {
        info!("移行を一時停止します（プレースホルダー）");
        Ok(())
    }

    pub async fn resume_migration(&self) -> Result<(), AppError> {
        info!("移行を再開します（プレースホルダー）");
        Ok(())
    }

    pub async fn stop_migration(&self) -> Result<(), AppError> {
        info!("移行を停止します（プレースホルダー）");
        Ok(())
    }
}

/// R2移行開始パラメータ
#[derive(Debug, Serialize, Deserialize)]
pub struct StartR2MigrationParams {
    /// ドライランモード
    pub dry_run: bool,
    /// バッチサイズ（オプション）
    pub batch_size: Option<usize>,
    /// 作成者（オプション）
    pub created_by: Option<String>,
}

/// R2移行結果
#[derive(Debug, Serialize, Deserialize)]
pub struct R2MigrationResult {
    /// 成功フラグ
    pub success: bool,
    /// メッセージ
    pub message: String,
    /// 移行ログID
    pub migration_log_id: Option<i64>,
    /// 総アイテム数
    pub total_items: usize,
    /// 成功数
    pub success_count: usize,
    /// エラー数
    pub error_count: usize,
    /// 実行時間（ミリ秒）
    pub duration_ms: u64,
}

/// R2移行ステータス
#[derive(Debug, Serialize, Deserialize)]
pub struct R2MigrationStatus {
    /// 実行中フラグ
    pub is_running: bool,
    /// 現在の移行ログID
    pub current_migration_id: Option<i64>,
    /// 進捗情報
    pub progress: Option<MigrationProgress>,
}

/// 移行進捗
#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationProgress {
    /// 総アイテム数
    pub total_items: usize,
    /// 処理済みアイテム数
    pub processed_items: usize,
    /// 成功数
    pub success_count: usize,
    /// エラー数
    pub error_count: usize,
    /// 現在のステータス
    pub current_status: String,
    /// 推定残り時間（秒）
    pub estimated_remaining_time: Option<u64>,
    /// スループット（アイテム/秒）
    pub throughput_items_per_second: f64,
}

/// 検証結果
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    /// 有効フラグ
    pub is_valid: bool,
    /// データベースレシート数
    pub database_receipt_count: i64,
    /// R2ファイル数
    pub r2_file_count: i64,
    /// 孤立ファイル数
    pub orphaned_files: usize,
    /// 破損ファイル数
    pub corrupted_files: usize,
    /// 警告一覧
    pub warnings: Vec<String>,
    /// エラー一覧
    pub errors: Vec<String>,
}

/// R2移行を開始する
#[tauri::command]
pub async fn start_r2_migration(
    params: StartR2MigrationParams,
    _app_handle: AppHandle,
) -> Result<R2MigrationResult, String> {
    info!(
        "R2移行コマンドを開始します: dry_run={}, batch_size={:?}",
        params.dry_run, params.batch_size
    );

    let start_time = std::time::Instant::now();

    // 既に実行中の移行があるかチェック
    {
        let current_id = CURRENT_MIGRATION_ID.lock().await;
        if current_id.is_some() {
            return Err("既に移行処理が実行中です".to_string());
        }
    }

    // R2設定を取得
    let r2_config = match R2Config::from_env() {
        Some(config) => config,
        None => {
            error!("R2設定が見つかりません");
            return Err("R2設定が見つかりません".to_string());
        }
    };

    // R2クライアントを作成
    let r2_client = match R2Client::new(r2_config).await {
        Ok(client) => Arc::new(client),
        Err(e) => {
            error!("R2クライアントの作成に失敗しました: {e}");
            return Err(format!("R2クライアントの作成に失敗しました: {e}"));
        }
    };

    // 移行サービスを作成
    let migration_service = Arc::new(SimpleR2MigrationService::new(r2_client));
    let service_key = format!("migration_{}", chrono::Utc::now().timestamp());

    // グローバル管理に追加
    {
        let mut services = MIGRATION_SERVICES.lock().await;
        services.insert(service_key.clone(), migration_service.clone());
    }

    let created_by = params.created_by.unwrap_or_else(|| "system".to_string());
    let batch_size = params.batch_size.unwrap_or(50);

    match migration_service
        .execute_migration(params.dry_run, batch_size, &created_by)
        .await
    {
        Ok((result, migration_log_id)) => {
            // 実際の移行の場合は現在の移行IDを設定
            if !params.dry_run {
                let mut current_id = CURRENT_MIGRATION_ID.lock().await;
                *current_id = Some(migration_log_id);
            }

            let duration = start_time.elapsed();

            info!(
                "R2移行が完了しました: 成功={}, エラー={}, 実行時間={:?}",
                result.success_count, result.error_count, duration
            );

            Ok(R2MigrationResult {
                success: result.error_count == 0,
                message: if params.dry_run {
                    format!("ドライラン完了: 移行対象ファイル数 {}", result.total_items)
                } else if result.error_count == 0 {
                    format!(
                        "移行完了: 成功={}, 実行時間={:?}",
                        result.success_count, duration
                    )
                } else {
                    format!(
                        "移行完了（エラーあり）: 成功={}, エラー={}, 実行時間={:?}",
                        result.success_count, result.error_count, duration
                    )
                },
                migration_log_id: Some(migration_log_id),
                total_items: result.total_items,
                success_count: result.success_count,
                error_count: result.error_count,
                duration_ms: duration.as_millis() as u64,
            })
        }
        Err(e) => {
            error!("R2移行でエラーが発生しました: {e}");

            // サービスをクリーンアップ
            {
                let mut services = MIGRATION_SERVICES.lock().await;
                services.remove(&service_key);
            }

            let duration = start_time.elapsed();

            Ok(R2MigrationResult {
                success: false,
                message: format!("移行処理でエラーが発生しました: {e}"),
                migration_log_id: None,
                total_items: 0,
                success_count: 0,
                error_count: 1,
                duration_ms: duration.as_millis() as u64,
            })
        }
    }
}

/// R2移行のステータスを取得する
#[tauri::command]
pub async fn get_r2_migration_status(_app_handle: AppHandle) -> Result<R2MigrationStatus, String> {
    info!("R2移行ステータスを取得します");

    // 現在実行中の移行IDを取得
    let current_migration_id = {
        let current_id = CURRENT_MIGRATION_ID.lock().await;
        *current_id
    };

    match current_migration_id {
        Some(migration_id) => {
            // 移行が実行中の場合、進捗情報を取得
            match get_migration_progress_from_db(migration_id).await {
                Ok(progress) => {
                    // 移行サービスから実行状態を取得
                    let is_running = {
                        let services = MIGRATION_SERVICES.lock().await;
                        !services.is_empty()
                    };

                    Ok(R2MigrationStatus {
                        is_running,
                        current_migration_id: Some(migration_id),
                        progress: Some(progress),
                    })
                }
                Err(e) => {
                    warn!("移行進捗の取得に失敗しました: {e}");
                    Ok(R2MigrationStatus {
                        is_running: false,
                        current_migration_id: Some(migration_id),
                        progress: None,
                    })
                }
            }
        }
        None => {
            // 移行が実行されていない場合
            Ok(R2MigrationStatus {
                is_running: false,
                current_migration_id: None,
                progress: None,
            })
        }
    }
}

/// R2移行を一時停止する
#[tauri::command]
pub async fn pause_r2_migration(migration_id: i64, _app_handle: AppHandle) -> Result<(), String> {
    info!("R2移行を一時停止します: migration_id={}", migration_id);

    // 現在実行中の移行IDをチェック
    let current_migration_id = {
        let current_id = CURRENT_MIGRATION_ID.lock().await;
        *current_id
    };

    if current_migration_id != Some(migration_id) {
        return Err("指定された移行IDは現在実行中ではありません".to_string());
    }

    // 全ての移行サービスを一時停止
    let services = MIGRATION_SERVICES.lock().await;
    for (_, service) in services.iter() {
        if let Err(e) = service.pause_migration().await {
            error!("移行サービスの一時停止に失敗しました: {e}");
            return Err(format!("移行の一時停止に失敗しました: {e}"));
        }
    }

    info!("R2移行を一時停止しました");
    Ok(())
}

/// R2移行を再開する
#[tauri::command]
pub async fn resume_r2_migration(migration_id: i64, _app_handle: AppHandle) -> Result<(), String> {
    info!("R2移行を再開します: migration_id={}", migration_id);

    // 現在実行中の移行IDをチェック
    let current_migration_id = {
        let current_id = CURRENT_MIGRATION_ID.lock().await;
        *current_id
    };

    if current_migration_id != Some(migration_id) {
        return Err("指定された移行IDは現在実行中ではありません".to_string());
    }

    // 全ての移行サービスを再開
    let services = MIGRATION_SERVICES.lock().await;
    for (_, service) in services.iter() {
        if let Err(e) = service.resume_migration().await {
            error!("移行サービスの再開に失敗しました: {e}");
            return Err(format!("移行の再開に失敗しました: {e}"));
        }
    }

    info!("R2移行を再開しました");
    Ok(())
}

/// R2移行を停止する
#[tauri::command]
pub async fn stop_r2_migration(migration_id: i64, _app_handle: AppHandle) -> Result<(), String> {
    info!("R2移行を停止します: migration_id={}", migration_id);

    // 現在実行中の移行IDをチェック
    let current_migration_id = {
        let current_id = CURRENT_MIGRATION_ID.lock().await;
        *current_id
    };

    if current_migration_id != Some(migration_id) {
        return Err("指定された移行IDは現在実行中ではありません".to_string());
    }

    // 全ての移行サービスを停止
    let services = MIGRATION_SERVICES.lock().await;
    for (_, service) in services.iter() {
        if let Err(e) = service.stop_migration().await {
            error!("移行サービスの停止に失敗しました: {e}");
            return Err(format!("移行の停止に失敗しました: {e}"));
        }
    }

    // 現在の移行IDをクリア
    {
        let mut current_id = CURRENT_MIGRATION_ID.lock().await;
        *current_id = None;
    }

    // サービスをクリーンアップ
    {
        let mut services = MIGRATION_SERVICES.lock().await;
        services.clear();
    }

    info!("R2移行を停止しました");
    Ok(())
}

/// R2移行の整合性を検証する
#[tauri::command]
pub async fn validate_r2_migration_integrity(
    _app_handle: AppHandle,
) -> Result<ValidationResult, String> {
    info!("R2移行の整合性を検証します");

    // R2設定を取得
    let r2_config = match R2Config::from_env() {
        Some(config) => config,
        None => {
            error!("R2設定が見つかりません");
            return Err("R2設定が見つかりません".to_string());
        }
    };

    // R2クライアントを作成
    let r2_client = match R2Client::new(r2_config).await {
        Ok(client) => Arc::new(client),
        Err(e) => {
            error!("R2クライアントの作成に失敗しました: {e}");
            return Err(format!("R2クライアントの作成に失敗しました: {e}"));
        }
    };

    // 検証を実行
    match validate_migration_integrity_internal(r2_client).await {
        Ok(result) => {
            info!(
                "整合性検証完了: 有効={}, DB={}, R2={}, 孤立={}, 破損={}",
                result.is_valid,
                result.database_receipt_count,
                result.r2_file_count,
                result.orphaned_files,
                result.corrupted_files
            );
            Ok(result)
        }
        Err(e) => {
            error!("整合性検証でエラーが発生しました: {e}");
            Err(format!("整合性検証でエラーが発生しました: {e}"))
        }
    }
}

/// 内部的な整合性検証処理
async fn validate_migration_integrity_internal(
    r2_client: Arc<R2Client>,
) -> Result<ValidationResult, AppError> {
    let mut result = ValidationResult {
        is_valid: true,
        database_receipt_count: 0,
        r2_file_count: 0,
        orphaned_files: 0,
        corrupted_files: 0,
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    // 1. データベースのレシート数をカウント
    match count_database_receipts().await {
        Ok(count) => {
            result.database_receipt_count = count;
            info!("データベースレシート数: {}", count);
        }
        Err(e) => {
            let error_msg = format!("データベースレシート数の取得に失敗: {e}");
            result.errors.push(error_msg);
            result.is_valid = false;
        }
    }

    // 2. R2ファイル数をカウント（新しい構造のみ）
    match count_r2_user_files(&r2_client).await {
        Ok(count) => {
            result.r2_file_count = count;
            info!("R2ユーザーファイル数: {}", count);
        }
        Err(e) => {
            let error_msg = format!("R2ファイル数の取得に失敗: {e}");
            result.errors.push(error_msg);
            result.is_valid = false;
        }
    }

    // 3. 数の整合性チェック
    if result.database_receipt_count != result.r2_file_count {
        let warning_msg = format!(
            "ファイル数の不整合: DB={}, R2={}",
            result.database_receipt_count, result.r2_file_count
        );
        result.warnings.push(warning_msg);
    }

    // 4. 孤立ファイルの検出（簡易版）
    match detect_orphaned_files(&r2_client).await {
        Ok(count) => {
            result.orphaned_files = count;
            if count > 0 {
                let warning_msg = format!("孤立ファイルが{}個見つかりました", count);
                result.warnings.push(warning_msg);
            }
        }
        Err(e) => {
            let error_msg = format!("孤立ファイル検出に失敗: {e}");
            result.errors.push(error_msg);
        }
    }

    // 5. 破損ファイルの検出（サンプリング）
    match detect_corrupted_files(&r2_client).await {
        Ok(count) => {
            result.corrupted_files = count;
            if count > 0 {
                let error_msg = format!("破損ファイルが{}個見つかりました", count);
                result.errors.push(error_msg);
                result.is_valid = false;
            }
        }
        Err(e) => {
            let error_msg = format!("破損ファイル検出に失敗: {e}");
            result.errors.push(error_msg);
        }
    }

    Ok(result)
}

/// データベースから移行進捗を取得
async fn get_migration_progress_from_db(migration_id: i64) -> Result<MigrationProgress, AppError> {
    let conn = get_database_connection().await?;
    let db_progress =
        super::r2_user_directory_migration::get_migration_progress(&conn, migration_id)?;

    // DbMigrationProgressをMigrationProgressに変換
    Ok(MigrationProgress {
        total_items: db_progress.total_items,
        processed_items: db_progress.processed_items,
        success_count: db_progress.success_count,
        error_count: db_progress.error_count,
        current_status: db_progress.current_status,
        estimated_remaining_time: db_progress.estimated_remaining_time.map(|d| d.as_secs()),
        throughput_items_per_second: db_progress.throughput_items_per_second,
    })
}

/// データベースのレシート数をカウント
async fn count_database_receipts() -> Result<i64, AppError> {
    let conn = get_database_connection().await?;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM expenses WHERE receipt_url IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(format!("レシート数カウントエラー: {e}")))?;
    Ok(count)
}

/// R2のユーザーファイル数をカウント
async fn count_r2_user_files(_r2_client: &R2Client) -> Result<i64, AppError> {
    // TODO: 実際のR2 list_objects_v2 APIを実装
    // 現在はプレースホルダー実装
    warn!("count_r2_user_files はプレースホルダー実装です");
    Ok(0)
}

/// 孤立ファイルを検出
async fn detect_orphaned_files(_r2_client: &R2Client) -> Result<usize, AppError> {
    // TODO: 実際の孤立ファイル検出を実装
    // 現在はプレースホルダー実装
    warn!("detect_orphaned_files はプレースホルダー実装です");
    Ok(0)
}

/// 破損ファイルを検出
async fn detect_corrupted_files(_r2_client: &R2Client) -> Result<usize, AppError> {
    // TODO: 実際の破損ファイル検出を実装
    // 現在はプレースホルダー実装
    warn!("detect_corrupted_files はプレースホルダー実装です");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_r2_migration_params() {
        let params = StartR2MigrationParams {
            dry_run: true,
            batch_size: Some(50),
            created_by: Some("test".to_string()),
        };

        assert!(params.dry_run);
        assert_eq!(params.batch_size, Some(50));
        assert_eq!(params.created_by, Some("test".to_string()));
    }

    #[test]
    fn test_r2_migration_result() {
        let result = R2MigrationResult {
            success: true,
            message: "テスト完了".to_string(),
            migration_log_id: Some(123),
            total_items: 100,
            success_count: 95,
            error_count: 5,
            duration_ms: 1500,
        };

        assert!(result.success);
        assert_eq!(result.message, "テスト完了");
        assert_eq!(result.migration_log_id, Some(123));
        assert_eq!(result.total_items, 100);
        assert_eq!(result.success_count, 95);
        assert_eq!(result.error_count, 5);
        assert_eq!(result.duration_ms, 1500);
    }

    #[test]
    fn test_validation_result() {
        let result = ValidationResult {
            is_valid: true,
            database_receipt_count: 100,
            r2_file_count: 100,
            orphaned_files: 0,
            corrupted_files: 0,
            warnings: Vec::new(),
            errors: Vec::new(),
        };

        assert!(result.is_valid);
        assert_eq!(result.database_receipt_count, 100);
        assert_eq!(result.r2_file_count, 100);
        assert_eq!(result.orphaned_files, 0);
        assert_eq!(result.corrupted_files, 0);
        assert!(result.warnings.is_empty());
        assert!(result.errors.is_empty());
    }
}

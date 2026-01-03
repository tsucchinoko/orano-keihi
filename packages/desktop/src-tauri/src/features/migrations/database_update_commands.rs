//! データベース更新コマンド
//!
//! R2ユーザーディレクトリ移行に伴うデータベース更新処理のTauriコマンド

use super::database_updater::{
    DatabaseStatistics, DatabaseUpdateResult, DatabaseUpdater, UrlUpdateItem,
};
use log::{info, warn};
use serde::{Deserialize, Serialize};

/// データベース更新パラメータ
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseUpdateParams {
    /// バッチサイズ（オプション、デフォルト: 100）
    pub batch_size: Option<usize>,
    /// ドライランモード
    pub dry_run: bool,
}

/// レガシーURL検出結果
#[derive(Debug, Serialize, Deserialize)]
pub struct LegacyUrlDetectionResult {
    /// 検出されたレガシーURL数
    pub legacy_count: usize,
    /// サンプルアイテム（最初の10件）
    pub sample_items: Vec<UrlUpdateItem>,
    /// 統計情報
    pub statistics: DatabaseStatistics,
}

/// レガシーURL検出コマンド
///
/// # 戻り値
/// レガシーURL検出結果
#[tauri::command]
pub async fn detect_legacy_receipt_urls() -> Result<LegacyUrlDetectionResult, String> {
    info!("レガシーreceipt_url検出コマンドを開始します");

    let legacy_items = DatabaseUpdater::detect_legacy_urls().await.map_err(|e| {
        let error_msg = format!("レガシーURL検出エラー: {e}");
        warn!("{}", error_msg);
        error_msg
    })?;

    let statistics = DatabaseUpdater::get_database_statistics()
        .await
        .map_err(|e| {
            let error_msg = format!("データベース統計取得エラー: {e}");
            warn!("{}", error_msg);
            error_msg
        })?;

    // サンプルとして最初の10件を返す
    let sample_items = legacy_items.into_iter().take(10).collect();
    let legacy_count = statistics.legacy_urls;

    let result = LegacyUrlDetectionResult {
        legacy_count,
        sample_items,
        statistics,
    };

    info!("レガシーreceipt_url検出完了: {}件検出", result.legacy_count);

    Ok(result)
}

/// データベース更新実行コマンド
///
/// # 引数
/// * `params` - 更新パラメータ
///
/// # 戻り値
/// データベース更新結果
#[tauri::command]
pub async fn execute_database_update(
    params: DatabaseUpdateParams,
) -> Result<DatabaseUpdateResult, String> {
    info!(
        "データベース更新コマンドを開始します (dry_run: {}, batch_size: {:?})",
        params.dry_run, params.batch_size
    );

    if params.dry_run {
        // ドライランモード: 検出のみ実行
        let legacy_items = DatabaseUpdater::detect_legacy_urls().await.map_err(|e| {
            let error_msg = format!("レガシーURL検出エラー: {e}");
            warn!("{}", error_msg);
            error_msg
        })?;

        let result = DatabaseUpdateResult {
            total_records: legacy_items.len(),
            updated_count: 0,
            failed_count: 0,
            verified_count: 0,
            errors: Vec::new(),
            duration_ms: 0,
        };

        info!(
            "ドライランモード完了: {}件の更新対象を検出",
            result.total_records
        );
        return Ok(result);
    }

    // 実際の更新処理
    let legacy_items = DatabaseUpdater::detect_legacy_urls().await.map_err(|e| {
        let error_msg = format!("レガシーURL検出エラー: {e}");
        warn!("{}", error_msg);
        error_msg
    })?;

    if legacy_items.is_empty() {
        info!("更新対象のレガシーURLが見つかりませんでした");
        return Ok(DatabaseUpdateResult {
            total_records: 0,
            updated_count: 0,
            failed_count: 0,
            verified_count: 0,
            errors: Vec::new(),
            duration_ms: 0,
        });
    }

    let result = DatabaseUpdater::update_receipt_urls_batch(legacy_items, params.batch_size)
        .await
        .map_err(|e| {
            let error_msg = format!("データベース更新エラー: {e}");
            warn!("{}", error_msg);
            error_msg
        })?;

    info!(
        "データベース更新完了: 成功={}, 失敗={}, 検証成功={}",
        result.updated_count, result.failed_count, result.verified_count
    );

    Ok(result)
}

/// データベース統計取得コマンド
///
/// # 戻り値
/// データベース統計情報
#[tauri::command]
pub async fn get_database_statistics() -> Result<DatabaseStatistics, String> {
    info!("データベース統計取得コマンドを開始します");

    let statistics = DatabaseUpdater::get_database_statistics()
        .await
        .map_err(|e| {
            let error_msg = format!("データベース統計取得エラー: {e}");
            warn!("{}", error_msg);
            error_msg
        })?;

    info!("データベース統計取得完了: {statistics:?}");
    Ok(statistics)
}

/// 特定のreceipt_url更新コマンド
///
/// # 引数
/// * `update_items` - 更新対象アイテム一覧
///
/// # 戻り値
/// データベース更新結果
#[tauri::command]
pub async fn update_specific_receipt_urls(
    update_items: Vec<UrlUpdateItem>,
) -> Result<DatabaseUpdateResult, String> {
    info!(
        "特定receipt_url更新コマンドを開始します: {}件",
        update_items.len()
    );

    if update_items.is_empty() {
        warn!("更新対象アイテムが空です");
        return Ok(DatabaseUpdateResult {
            total_records: 0,
            updated_count: 0,
            failed_count: 0,
            verified_count: 0,
            errors: Vec::new(),
            duration_ms: 0,
        });
    }

    let result = DatabaseUpdater::update_receipt_urls_batch(update_items, Some(50))
        .await
        .map_err(|e| {
            let error_msg = format!("特定receipt_url更新エラー: {e}");
            warn!("{}", error_msg);
            error_msg
        })?;

    info!(
        "特定receipt_url更新完了: 成功={}, 失敗={}",
        result.updated_count, result.failed_count
    );

    Ok(result)
}

/// データベース整合性チェックコマンド
///
/// # 戻り値
/// 整合性チェック結果
#[tauri::command]
pub async fn check_database_url_integrity() -> Result<DatabaseIntegrityResult, String> {
    info!("データベースURL整合性チェックを開始します");

    let statistics = DatabaseUpdater::get_database_statistics()
        .await
        .map_err(|e| {
            let error_msg = format!("データベース統計取得エラー: {e}");
            warn!("{}", error_msg);
            error_msg
        })?;

    // 整合性チェック
    let mut issues = Vec::new();

    // レガシーURLが残っているかチェック
    if statistics.legacy_urls > 0 {
        issues.push(format!(
            "{}件のレガシーURLが残っています",
            statistics.legacy_urls
        ));
    }

    // receipt_urlを持つレコード数の整合性チェック
    if statistics.legacy_urls + statistics.user_urls != statistics.with_receipt_url {
        issues.push(format!(
            "URL分類の整合性エラー: レガシー({}) + ユーザー({}) != 総URL数({})",
            statistics.legacy_urls, statistics.user_urls, statistics.with_receipt_url
        ));
    }

    let is_consistent = issues.is_empty();

    let issues_len = issues.len();
    let result = DatabaseIntegrityResult {
        is_consistent,
        statistics,
        issues,
    };

    if is_consistent {
        info!("データベースURL整合性チェック完了: 問題なし");
    } else {
        warn!(
            "データベースURL整合性チェック完了: {}件の問題を検出",
            issues_len
        );
    }

    Ok(result)
}

/// データベース整合性チェック結果
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseIntegrityResult {
    /// 整合性が保たれているか
    pub is_consistent: bool,
    /// 統計情報
    pub statistics: DatabaseStatistics,
    /// 検出された問題一覧
    pub issues: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_update_params() {
        let params = DatabaseUpdateParams {
            batch_size: Some(50),
            dry_run: true,
        };

        assert_eq!(params.batch_size, Some(50));
        assert!(params.dry_run);
    }

    #[test]
    fn test_legacy_url_detection_result() {
        let result = LegacyUrlDetectionResult {
            legacy_count: 100,
            sample_items: vec![],
            statistics: DatabaseStatistics {
                total_expenses: 1000,
                with_receipt_url: 800,
                legacy_urls: 100,
                user_urls: 700,
            },
        };

        assert_eq!(result.legacy_count, 100);
        assert_eq!(result.sample_items.len(), 0);
        assert_eq!(result.statistics.total_expenses, 1000);
    }

    #[test]
    fn test_database_integrity_result() {
        let result = DatabaseIntegrityResult {
            is_consistent: false,
            statistics: DatabaseStatistics {
                total_expenses: 1000,
                with_receipt_url: 800,
                legacy_urls: 100,
                user_urls: 700,
            },
            issues: vec!["テスト問題".to_string()],
        };

        assert!(!result.is_consistent);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.statistics.legacy_urls, 100);
    }

    #[tokio::test]
    async fn test_database_update_params_dry_run() {
        let params = DatabaseUpdateParams {
            batch_size: None,
            dry_run: true,
        };

        // ドライランモードでは実際の更新は行われない
        assert!(params.dry_run);
        assert_eq!(params.batch_size, None);
    }
}

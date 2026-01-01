# 設計書

## 概要

既存のCloudflare R2レシート保存システムを、現在の`/receipts/{expense_id}/`構造から`/users/{user_id}/receipts/`構造に移行する機能。この変更により、ユーザー別のデータ分離、セキュリティ強化、スケーラビリティの向上を実現する。

### 技術スタック

- **既存技術**: SvelteKit 5、Tauri 2、Rust、SQLite、Cloudflare R2
- **新規追加**: 
  - `tokio-stream` - ストリーミング処理
  - `sha2` - ファイルハッシュ計算
  - `indicatif` - プログレスバー表示
  - バッチ処理フレームワーク

## アーキテクチャ

### システム構成

```
┌─────────────────────────────────────────┐
│         SvelteKit Frontend              │
│  ┌─────────────────────────────────┐   │
│  │  Migration UI Components        │   │
│  │  - MigrationStatus (新規)       │   │
│  │  - ProgressMonitor (新規)       │   │
│  │  - MigrationControl (新規)      │   │
│  └─────────────────────────────────┘   │
│              ↕                          │
│  ┌─────────────────────────────────┐   │
│  │  Updated Tauri Commands         │   │
│  │  - start_r2_migration           │   │
│  │  - get_migration_status         │   │
│  │  - upload_receipt_to_r2 (更新)  │   │
│  │  - get_receipt_from_r2 (更新)   │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
              ↕
┌─────────────────────────────────────────┐
│         Rust Backend (Tauri)            │
│  ┌─────────────────────────────────┐   │
│  │  Migration Service Layer        │   │
│  │  - r2_migration_service.rs      │   │
│  │  - migration_validator.rs       │   │
│  │  - batch_processor.rs           │   │
│  └─────────────────────────────────┘   │
│              ↕                          │
│  ┌─────────────────────────────────┐   │
│  │  Updated R2 Service Layer       │   │
│  │  - r2_client.rs (更新)          │   │
│  │  - user_path_manager.rs (新規)  │   │
│  │  - migration_tracker.rs (新規)  │   │
│  └─────────────────────────────────┘   │
│              ↕                          │
│  ┌─────────────────────────────────┐   │
│  │  Database Layer (更新)          │   │
│  │  - migration_log table (新規)   │   │
│  │  - receipt_url updates          │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
              ↕
┌─────────────────────────────────────────┐
│         External Services               │
│  ┌─────────────────────────────────┐   │
│  │  Cloudflare R2                  │   │
│  │  - Legacy: /receipts/           │   │
│  │  - New: /users/{user_id}/       │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

### ディレクトリ構造の変更

```
src-tauri/
├── src/
│   ├── features/
│   │   ├── receipts/
│   │   │   ├── service.rs              # 更新: ユーザーパス対応
│   │   │   ├── user_path_manager.rs    # 新規: ユーザーパス管理
│   │   │   └── ...
│   │   ├── migration/                  # 新規: 移行機能
│   │   │   ├── mod.rs
│   │   │   ├── r2_migration_service.rs # R2移行サービス
│   │   │   ├── migration_validator.rs  # 移行検証
│   │   │   ├── batch_processor.rs      # バッチ処理
│   │   │   ├── migration_tracker.rs    # 移行追跡
│   │   │   └── commands.rs             # 移行コマンド
│   │   └── ...
│   └── ...
```

## コンポーネントとインターフェース

### 1. ユーザーパス管理サービス

```rust
// src-tauri/src/features/receipts/user_path_manager.rs

use crate::features::auth::models::User;
use crate::shared::errors::AppResult;

pub struct UserPathManager;

impl UserPathManager {
    /// 新しいユーザー別ファイルパスを生成
    pub fn generate_user_receipt_path(
        user_id: i64,
        expense_id: i64,
        filename: &str,
    ) -> String {
        let timestamp = chrono::Utc::now().timestamp();
        let uuid = uuid::Uuid::new_v4();
        format!("users/{user_id}/receipts/{expense_id}/{timestamp}-{uuid}-{filename}")
    }

    /// 既存のレガシーパスからユーザーパスに変換
    pub fn convert_legacy_to_user_path(
        legacy_path: &str,
        user_id: i64,
    ) -> AppResult<String> {
        // "receipts/123/timestamp-uuid-filename.pdf" -> "users/456/receipts/123/timestamp-uuid-filename.pdf"
        if let Some(stripped) = legacy_path.strip_prefix("receipts/") {
            Ok(format!("users/{user_id}/receipts/{stripped}"))
        } else {
            Err(AppError::Validation(format!("無効なレガシーパス: {legacy_path}")))
        }
    }

    /// パスからユーザーIDを抽出
    pub fn extract_user_id_from_path(path: &str) -> AppResult<i64> {
        if let Some(captures) = regex::Regex::new(r"^users/(\d+)/receipts/")
            .unwrap()
            .captures(path)
        {
            captures[1].parse::<i64>()
                .map_err(|_| AppError::Validation("ユーザーIDの解析に失敗".to_string()))
        } else {
            Err(AppError::Validation("ユーザーパス形式が無効".to_string()))
        }
    }

    /// ユーザーがパスにアクセス権限を持つかチェック
    pub fn validate_user_access(user_id: i64, path: &str) -> AppResult<()> {
        let path_user_id = Self::extract_user_id_from_path(path)?;
        if user_id == path_user_id {
            Ok(())
        } else {
            Err(AppError::Authorization("アクセス権限がありません".to_string()))
        }
    }

    /// レガシーパスかどうかを判定
    pub fn is_legacy_path(path: &str) -> bool {
        path.starts_with("receipts/") && !path.starts_with("receipts/users/")
    }

    /// ユーザーパスかどうかを判定
    pub fn is_user_path(path: &str) -> bool {
        regex::Regex::new(r"^users/\d+/receipts/")
            .unwrap()
            .is_match(path)
    }
}
```

### 2. R2移行サービス

```rust
// src-tauri/src/features/migration/r2_migration_service.rs

use super::migration_tracker::MigrationTracker;
use super::migration_validator::MigrationValidator;
use super::batch_processor::BatchProcessor;
use crate::features::receipts::service::R2Client;
use crate::features::receipts::user_path_manager::UserPathManager;
use crate::shared::errors::AppResult;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct R2MigrationService {
    r2_client: Arc<R2Client>,
    migration_tracker: Arc<Mutex<MigrationTracker>>,
    validator: MigrationValidator,
    batch_processor: BatchProcessor,
}

impl R2MigrationService {
    pub fn new(r2_client: Arc<R2Client>) -> Self {
        Self {
            r2_client: r2_client.clone(),
            migration_tracker: Arc::new(Mutex::new(MigrationTracker::new())),
            validator: MigrationValidator::new(r2_client.clone()),
            batch_processor: BatchProcessor::new(r2_client),
        }
    }

    /// 移行プロセス全体を実行
    pub async fn execute_migration(
        &self,
        dry_run: bool,
        batch_size: usize,
    ) -> AppResult<MigrationResult> {
        info!("R2移行プロセスを開始します (dry_run: {dry_run}, batch_size: {batch_size})");

        // 1. 事前検証
        self.pre_migration_validation().await?;

        // 2. 移行対象ファイルの特定
        let migration_items = self.identify_migration_targets().await?;
        info!("移行対象ファイル数: {}", migration_items.len());

        if dry_run {
            return Ok(MigrationResult::dry_run_success(migration_items.len()));
        }

        // 3. データベースバックアップ
        self.create_database_backup().await?;

        // 4. バッチ処理で移行実行
        let result = self.batch_processor
            .process_migration_batches(migration_items, batch_size)
            .await?;

        // 5. 移行後検証
        self.post_migration_validation().await?;

        // 6. クリーンアップ（オプション）
        if result.success_count > 0 {
            self.cleanup_legacy_files().await?;
        }

        Ok(result)
    }

    /// 移行対象ファイルを特定
    async fn identify_migration_targets(&self) -> AppResult<Vec<MigrationItem>> {
        let mut items = Vec::new();

        // R2からレガシーパスのファイルをリストアップ
        let legacy_files = self.list_legacy_files().await?;

        for file in legacy_files {
            // データベースから対応するユーザーIDを取得
            if let Some(user_id) = self.get_user_id_for_file(&file.key).await? {
                let new_path = UserPathManager::convert_legacy_to_user_path(&file.key, user_id)?;
                
                items.push(MigrationItem {
                    old_path: file.key,
                    new_path,
                    user_id,
                    file_size: file.size,
                    last_modified: file.last_modified,
                });
            } else {
                warn!("ユーザーIDが見つからないファイル: {}", file.key);
            }
        }

        Ok(items)
    }

    /// レガシーファイルをR2からリストアップ
    async fn list_legacy_files(&self) -> AppResult<Vec<R2FileInfo>> {
        // R2 list_objects_v2 APIを使用してreceipts/プレフィックスのファイルを取得
        let mut files = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let response = self.r2_client.client
                .list_objects_v2()
                .bucket(&self.r2_client.bucket_name)
                .prefix("receipts/")
                .set_continuation_token(continuation_token)
                .send()
                .await
                .map_err(|e| AppError::ExternalService(format!("R2リスト取得エラー: {e}")))?;

            if let Some(contents) = response.contents() {
                for object in contents {
                    if let Some(key) = object.key() {
                        // users/で始まるパスは除外（既に移行済み）
                        if !key.starts_with("receipts/users/") {
                            files.push(R2FileInfo {
                                key: key.to_string(),
                                size: object.size().unwrap_or(0) as u64,
                                last_modified: object.last_modified()
                                    .map(|dt| dt.to_chrono_utc().unwrap())
                                    .unwrap_or_else(chrono::Utc::now),
                            });
                        }
                    }
                }
            }

            continuation_token = response.next_continuation_token().map(|s| s.to_string());
            if continuation_token.is_none() {
                break;
            }
        }

        Ok(files)
    }

    /// ファイルパスからユーザーIDを取得
    async fn get_user_id_for_file(&self, file_path: &str) -> AppResult<Option<i64>> {
        // データベースからreceipt_urlでユーザーIDを検索
        // receipt_urlは完全なHTTPS URLなので、パスを含むURLを検索
        let db = self.get_database_connection().await?;
        
        let query = "
            SELECT e.user_id 
            FROM expenses e 
            WHERE e.receipt_url LIKE ?
            LIMIT 1
        ";
        
        let search_pattern = format!("%{file_path}%");
        
        let user_id: Option<i64> = db.query_row(
            query,
            [search_pattern],
            |row| row.get(0)
        ).optional()?;

        Ok(user_id)
    }

    /// 事前検証を実行
    async fn pre_migration_validation(&self) -> AppResult<()> {
        info!("移行前検証を実行中...");

        // R2接続テスト
        self.r2_client.test_connection().await?;

        // データベース接続テスト
        let _db = self.get_database_connection().await?;

        // 必要な権限チェック
        self.validate_permissions().await?;

        info!("移行前検証が完了しました");
        Ok(())
    }

    /// 移行後検証を実行
    async fn post_migration_validation(&self) -> AppResult<()> {
        info!("移行後検証を実行中...");

        // データ整合性チェック
        self.validator.validate_data_integrity().await?;

        // ファイルアクセステスト
        self.validator.validate_file_access().await?;

        info!("移行後検証が完了しました");
        Ok(())
    }

    /// データベースバックアップを作成
    async fn create_database_backup(&self) -> AppResult<()> {
        info!("データベースバックアップを作成中...");
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = format!("database_backup_migration_{timestamp}.db");
        
        // SQLiteデータベースファイルをコピー
        let source_path = self.get_database_path().await?;
        tokio::fs::copy(&source_path, &backup_path).await
            .map_err(|e| AppError::FileSystem(format!("バックアップ作成エラー: {e}")))?;

        info!("データベースバックアップが作成されました: {backup_path}");
        Ok(())
    }

    /// レガシーファイルのクリーンアップ
    async fn cleanup_legacy_files(&self) -> AppResult<()> {
        info!("レガシーファイルのクリーンアップを開始します");

        // 移行成功したファイルのリストを取得
        let tracker = self.migration_tracker.lock().await;
        let successful_migrations = tracker.get_successful_migrations();

        for item in successful_migrations {
            // 新しい場所にファイルが存在することを確認
            if self.validator.verify_file_exists(&item.new_path).await? {
                // レガシーファイルを削除
                self.r2_client.delete_file(&item.old_path).await?;
                info!("レガシーファイルを削除しました: {}", item.old_path);
            } else {
                warn!("新しいファイルが見つからないため、レガシーファイルを保持します: {}", item.old_path);
            }
        }

        info!("レガシーファイルのクリーンアップが完了しました");
        Ok(())
    }

    /// 移行進捗を取得
    pub async fn get_migration_progress(&self) -> MigrationProgress {
        let tracker = self.migration_tracker.lock().await;
        tracker.get_progress()
    }

    /// 移行を一時停止
    pub async fn pause_migration(&self) -> AppResult<()> {
        self.batch_processor.pause().await;
        info!("移行プロセスを一時停止しました");
        Ok(())
    }

    /// 移行を再開
    pub async fn resume_migration(&self) -> AppResult<()> {
        self.batch_processor.resume().await;
        info!("移行プロセスを再開しました");
        Ok(())
    }

    /// 移行を停止
    pub async fn stop_migration(&self) -> AppResult<()> {
        self.batch_processor.stop().await;
        info!("移行プロセスを停止しました");
        Ok(())
    }
}
```

### 3. バッチ処理システム

```rust
// src-tauri/src/features/migration/batch_processor.rs

use super::migration_tracker::MigrationTracker;
use crate::features::receipts::service::R2Client;
use crate::shared::errors::AppResult;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

pub struct BatchProcessor {
    r2_client: Arc<R2Client>,
    migration_tracker: Arc<Mutex<MigrationTracker>>,
    cancellation_token: CancellationToken,
    pause_token: Arc<Mutex<bool>>,
    semaphore: Arc<Semaphore>,
}

impl BatchProcessor {
    pub fn new(r2_client: Arc<R2Client>) -> Self {
        Self {
            r2_client,
            migration_tracker: Arc::new(Mutex::new(MigrationTracker::new())),
            cancellation_token: CancellationToken::new(),
            pause_token: Arc::new(Mutex::new(false)),
            semaphore: Arc::new(Semaphore::new(5)), // 最大5並列
        }
    }

    /// バッチ処理で移行を実行
    pub async fn process_migration_batches(
        &self,
        items: Vec<MigrationItem>,
        batch_size: usize,
    ) -> AppResult<MigrationResult> {
        let total_items = items.len();
        let mut success_count = 0;
        let mut error_count = 0;
        let mut errors = Vec::new();

        info!("バッチ処理開始: 総アイテム数={total_items}, バッチサイズ={batch_size}");

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
            }

            info!("バッチ {}/{} を処理中 (アイテム数: {})", batch_index + 1, total_batches, batch.len());

            // バッチ内のアイテムを並列処理
            let batch_results = self.process_batch_parallel(batch.to_vec()).await;

            // 結果を集計
            for result in batch_results {
                match result {
                    Ok(_) => success_count += 1,
                    Err(e) => {
                        error_count += 1;
                        errors.push(e);
                    }
                }
            }

            // 進捗を更新
            {
                let mut tracker = self.migration_tracker.lock().await;
                tracker.update_progress(success_count, error_count, total_items);
            }

            info!("バッチ {}/{} 完了 (成功: {}, エラー: {})", 
                  batch_index + 1, total_batches, success_count, error_count);
        }

        Ok(MigrationResult {
            total_items,
            success_count,
            error_count,
            errors,
            duration: std::time::Duration::from_secs(0), // 実際の処理時間を計算
        })
    }

    /// バッチ内のアイテムを並列処理
    async fn process_batch_parallel(&self, items: Vec<MigrationItem>) -> Vec<AppResult<()>> {
        let tasks: Vec<_> = items.into_iter().map(|item| {
            let r2_client = self.r2_client.clone();
            let semaphore = self.semaphore.clone();
            
            tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                Self::migrate_single_file(r2_client, item).await
            })
        }).collect();

        let results = futures::future::join_all(tasks).await;
        
        results.into_iter().map(|result| {
            result.unwrap_or_else(|e| Err(AppError::Internal(format!("タスクエラー: {e}"))))
        }).collect()
    }

    /// 単一ファイルの移行処理
    async fn migrate_single_file(
        r2_client: Arc<R2Client>,
        item: MigrationItem,
    ) -> AppResult<()> {
        info!("ファイル移行開始: {} -> {}", item.old_path, item.new_path);

        // 1. 元ファイルをダウンロード
        let file_data = r2_client.download_file(&item.old_path).await?;

        // 2. ハッシュ値を計算（整合性チェック用）
        let original_hash = Self::calculate_file_hash(&file_data);

        // 3. 新しい場所にアップロード
        let content_type = R2Client::get_content_type(&item.old_path);
        let _new_url = r2_client.upload_file(&item.new_path, file_data.clone(), &content_type).await?;

        // 4. アップロードしたファイルの整合性を確認
        let uploaded_data = r2_client.download_file(&item.new_path).await?;
        let uploaded_hash = Self::calculate_file_hash(&uploaded_data);

        if original_hash != uploaded_hash {
            return Err(AppError::DataIntegrity(
                format!("ファイルハッシュが一致しません: {}", item.old_path)
            ));
        }

        // 5. データベースのreceipt_urlを更新
        Self::update_database_receipt_url(&item).await?;

        info!("ファイル移行完了: {} -> {}", item.old_path, item.new_path);
        Ok(())
    }

    /// ファイルハッシュを計算
    fn calculate_file_hash(data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// データベースのreceipt_urlを更新
    async fn update_database_receipt_url(item: &MigrationItem) -> AppResult<()> {
        let db = get_database_connection().await?;
        
        // 古いURLパターンを新しいURLパターンに置換
        let old_url_pattern = format!("%{}", item.old_path);
        let new_url_pattern = item.new_path.clone();
        
        let query = "
            UPDATE expenses 
            SET receipt_url = REPLACE(receipt_url, ?, ?),
                updated_at = ?
            WHERE receipt_url LIKE ?
        ";
        
        let now = chrono::Utc::now().to_rfc3339();
        
        db.execute(query, [
            &item.old_path,
            &new_url_pattern,
            &now,
            &old_url_pattern,
        ])?;

        Ok(())
    }

    /// 処理を一時停止
    pub async fn pause(&self) {
        *self.pause_token.lock().await = true;
    }

    /// 処理を再開
    pub async fn resume(&self) {
        *self.pause_token.lock().await = false;
    }

    /// 処理を停止
    pub async fn stop(&self) {
        self.cancellation_token.cancel();
    }
}
```

### 4. 移行検証システム

```rust
// src-tauri/src/features/migration/migration_validator.rs

use crate::features::receipts::service::R2Client;
use crate::shared::errors::AppResult;
use std::sync::Arc;

pub struct MigrationValidator {
    r2_client: Arc<R2Client>,
}

impl MigrationValidator {
    pub fn new(r2_client: Arc<R2Client>) -> Self {
        Self { r2_client }
    }

    /// データ整合性を検証
    pub async fn validate_data_integrity(&self) -> AppResult<ValidationReport> {
        info!("データ整合性検証を開始します");

        let mut report = ValidationReport::new();

        // 1. データベースとR2の整合性チェック
        let db_receipt_count = self.count_database_receipts().await?;
        let r2_file_count = self.count_r2_files().await?;

        report.database_receipt_count = db_receipt_count;
        report.r2_file_count = r2_file_count;

        if db_receipt_count != r2_file_count {
            report.add_warning(format!(
                "ファイル数の不整合: DB={}, R2={}", 
                db_receipt_count, r2_file_count
            ));
        }

        // 2. 孤立ファイルのチェック
        let orphaned_files = self.find_orphaned_files().await?;
        report.orphaned_files = orphaned_files.len();

        if !orphaned_files.is_empty() {
            report.add_warning(format!("孤立ファイルが{}個見つかりました", orphaned_files.len()));
        }

        // 3. 破損ファイルのチェック
        let corrupted_files = self.find_corrupted_files().await?;
        report.corrupted_files = corrupted_files.len();

        if !corrupted_files.is_empty() {
            report.add_error(format!("破損ファイルが{}個見つかりました", corrupted_files.len()));
        }

        info!("データ整合性検証が完了しました: {report:?}");
        Ok(report)
    }

    /// ファイルアクセス可能性を検証
    pub async fn validate_file_access(&self) -> AppResult<()> {
        info!("ファイルアクセス検証を開始します");

        // データベースからランダムなreceipt_urlを取得してアクセステスト
        let sample_urls = self.get_sample_receipt_urls(10).await?;

        for url in sample_urls {
            match self.test_file_access(&url).await {
                Ok(_) => info!("アクセステスト成功: {url}"),
                Err(e) => {
                    error!("アクセステスト失敗: {url}, エラー: {e}");
                    return Err(e);
                }
            }
        }

        info!("ファイルアクセス検証が完了しました");
        Ok(())
    }

    /// ファイルが存在するかチェック
    pub async fn verify_file_exists(&self, path: &str) -> AppResult<bool> {
        match self.r2_client.client
            .head_object()
            .bucket(&self.r2_client.bucket_name)
            .key(path)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// データベースのレシート数をカウント
    async fn count_database_receipts(&self) -> AppResult<i64> {
        let db = get_database_connection().await?;
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM expenses WHERE receipt_url IS NOT NULL",
            [],
            |row| row.get(0)
        )?;
        Ok(count)
    }

    /// R2のファイル数をカウント
    async fn count_r2_files(&self) -> AppResult<i64> {
        let mut count = 0i64;
        let mut continuation_token: Option<String> = None;

        loop {
            let response = self.r2_client.client
                .list_objects_v2()
                .bucket(&self.r2_client.bucket_name)
                .prefix("users/")
                .set_continuation_token(continuation_token)
                .send()
                .await
                .map_err(|e| AppError::ExternalService(format!("R2リスト取得エラー: {e}")))?;

            if let Some(contents) = response.contents() {
                count += contents.len() as i64;
            }

            continuation_token = response.next_continuation_token().map(|s| s.to_string());
            if continuation_token.is_none() {
                break;
            }
        }

        Ok(count)
    }

    /// 孤立ファイルを検出
    async fn find_orphaned_files(&self) -> AppResult<Vec<String>> {
        let mut orphaned = Vec::new();

        // R2のすべてのファイルを取得
        let r2_files = self.list_all_r2_files().await?;

        // 各ファイルがデータベースに対応するレコードを持つかチェック
        for file_path in r2_files {
            if !self.file_has_database_record(&file_path).await? {
                orphaned.push(file_path);
            }
        }

        Ok(orphaned)
    }

    /// 破損ファイルを検出
    async fn find_corrupted_files(&self) -> AppResult<Vec<String>> {
        let mut corrupted = Vec::new();

        // サンプルファイルをダウンロードして整合性をチェック
        let sample_files = self.get_sample_file_paths(20).await?;

        for file_path in sample_files {
            match self.r2_client.download_file(&file_path).await {
                Ok(data) => {
                    // ファイルサイズが0または異常に小さい場合は破損とみなす
                    if data.is_empty() || data.len() < 100 {
                        corrupted.push(file_path);
                    }
                }
                Err(_) => {
                    corrupted.push(file_path);
                }
            }
        }

        Ok(corrupted)
    }

    /// サンプルのreceipt_urlを取得
    async fn get_sample_receipt_urls(&self, limit: i32) -> AppResult<Vec<String>> {
        let db = get_database_connection().await?;
        let mut stmt = db.prepare(
            "SELECT receipt_url FROM expenses WHERE receipt_url IS NOT NULL ORDER BY RANDOM() LIMIT ?"
        )?;

        let urls: Result<Vec<String>, _> = stmt.query_map([limit], |row| {
            Ok(row.get::<_, String>(0)?)
        })?.collect();

        Ok(urls?)
    }

    /// ファイルアクセステスト
    async fn test_file_access(&self, url: &str) -> AppResult<()> {
        // URLからパスを抽出
        let path = self.extract_path_from_url(url)?;
        
        // ファイルの存在確認
        if !self.verify_file_exists(&path).await? {
            return Err(AppError::NotFound(format!("ファイルが見つかりません: {path}")));
        }

        // 小さなデータをダウンロードしてアクセス可能性を確認
        let _data = self.r2_client.download_file_partial(&path, 0, 1024).await?;

        Ok(())
    }

    /// URLからパスを抽出
    fn extract_path_from_url(&self, url: &str) -> AppResult<String> {
        // "https://bucket.r2.cloudflarestorage.com/users/123/receipts/..." -> "users/123/receipts/..."
        if let Some(path_start) = url.find("/users/") {
            Ok(url[path_start + 1..].to_string())
        } else {
            Err(AppError::Validation(format!("無効なURL形式: {url}")))
        }
    }
}
```

## データモデル

### データベーススキーマの追加

#### 移行ログテーブル

```sql
-- Migration: 20241230_migration_log.sql

CREATE TABLE migration_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    migration_type TEXT NOT NULL, -- 'r2_user_directory'
    status TEXT NOT NULL, -- 'started', 'in_progress', 'completed', 'failed', 'paused'
    total_items INTEGER NOT NULL DEFAULT 0,
    processed_items INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,
    error_details TEXT, -- JSON形式のエラー詳細
    started_at TEXT NOT NULL,
    completed_at TEXT,
    created_by TEXT, -- システムまたはユーザーID
    metadata TEXT -- JSON形式の追加情報
);

CREATE INDEX idx_migration_log_type ON migration_log(migration_type);
CREATE INDEX idx_migration_log_status ON migration_log(status);
CREATE INDEX idx_migration_log_started_at ON migration_log(started_at);

-- 移行アイテム詳細テーブル
CREATE TABLE migration_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    migration_log_id INTEGER NOT NULL,
    old_path TEXT NOT NULL,
    new_path TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    file_size INTEGER NOT NULL,
    status TEXT NOT NULL, -- 'pending', 'processing', 'completed', 'failed'
    error_message TEXT,
    started_at TEXT,
    completed_at TEXT,
    file_hash TEXT, -- SHA256ハッシュ
    FOREIGN KEY (migration_log_id) REFERENCES migration_log(id)
);

CREATE INDEX idx_migration_items_log_id ON migration_items(migration_log_id);
CREATE INDEX idx_migration_items_status ON migration_items(status);
CREATE INDEX idx_migration_items_user_id ON migration_items(user_id);
```

### 更新されるRustモデル

```rust
// src-tauri/src/features/migration/models.rs

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MigrationItem {
    pub old_path: String,
    pub new_path: String,
    pub user_id: i64,
    pub file_size: u64,
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationResult {
    pub total_items: usize,
    pub success_count: usize,
    pub error_count: usize,
    pub errors: Vec<AppError>,
    pub duration: std::time::Duration,
}

impl MigrationResult {
    pub fn dry_run_success(total_items: usize) -> Self {
        Self {
            total_items,
            success_count: 0,
            error_count: 0,
            errors: Vec::new(),
            duration: std::time::Duration::from_secs(0),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationProgress {
    pub total_items: usize,
    pub processed_items: usize,
    pub success_count: usize,
    pub error_count: usize,
    pub current_status: String,
    pub estimated_remaining_time: Option<std::time::Duration>,
    pub throughput_items_per_second: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationReport {
    pub database_receipt_count: i64,
    pub r2_file_count: i64,
    pub orphaned_files: usize,
    pub corrupted_files: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            database_receipt_count: 0,
            r2_file_count: 0,
            orphaned_files: 0,
            corrupted_files: 0,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn add_warning(&mut self, message: String) {
        self.warnings.push(message);
    }

    pub fn add_error(&mut self, message: String) {
        self.errors.push(message);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct R2FileInfo {
    pub key: String,
    pub size: u64,
    pub last_modified: chrono::DateTime<chrono::Utc>,
}
```

### 更新されるTauriコマンド

```rust
// src-tauri/src/features/migration/commands.rs

use super::r2_migration_service::R2MigrationService;
use crate::shared::errors::AppResult;
use tauri::State;

#[tauri::command]
pub async fn start_r2_migration(
    dry_run: bool,
    batch_size: Option<usize>,
    state: State<'_, AppState>,
) -> Result<MigrationResult, String> {
    info!("R2移行コマンドを開始します (dry_run: {dry_run})");

    let migration_service = R2MigrationService::new(state.r2_client.clone());
    
    let result = migration_service
        .execute_migration(dry_run, batch_size.unwrap_or(50))
        .await
        .map_err(|e| {
            error!("R2移行エラー: {e}");
            e.to_string()
        })?;

    info!("R2移行コマンドが完了しました: {result:?}");
    Ok(result)
}

#[tauri::command]
pub async fn get_migration_status(
    state: State<'_, AppState>,
) -> Result<MigrationProgress, String> {
    let migration_service = R2MigrationService::new(state.r2_client.clone());
    Ok(migration_service.get_migration_progress().await)
}

#[tauri::command]
pub async fn pause_migration(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let migration_service = R2MigrationService::new(state.r2_client.clone());
    migration_service.pause_migration().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resume_migration(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let migration_service = R2MigrationService::new(state.r2_client.clone());
    migration_service.resume_migration().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_migration(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let migration_service = R2MigrationService::new(state.r2_client.clone());
    migration_service.stop_migration().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_migration_integrity(
    state: State<'_, AppState>,
) -> Result<ValidationReport, String> {
    let migration_service = R2MigrationService::new(state.r2_client.clone());
    let validator = MigrationValidator::new(state.r2_client.clone());
    
    validator.validate_data_integrity().await
        .map_err(|e| e.to_string())
}
```

### 更新されるレシートサービス

```rust
// src-tauri/src/features/receipts/service.rs への追加

impl R2Client {
    /// ユーザー別ファイルキーを生成（更新版）
    pub fn generate_user_receipt_path(
        user_id: i64,
        expense_id: i64,
        filename: &str,
    ) -> String {
        UserPathManager::generate_user_receipt_path(user_id, expense_id, filename)
    }

    /// ユーザー認証付きファイルアップロード
    pub async fn upload_user_receipt(
        &self,
        user_id: i64,
        expense_id: i64,
        filename: &str,
        file_data: Vec<u8>,
        content_type: &str,
    ) -> AppResult<String> {
        // ユーザー別パスを生成
        let file_path = Self::generate_user_receipt_path(user_id, expense_id, filename);
        
        // ファイルをアップロード
        self.upload_file(&file_path, file_data, content_type).await
    }

    /// ユーザー認証付きファイル取得
    pub async fn get_user_receipt(
        &self,
        user_id: i64,
        receipt_url: &str,
    ) -> AppResult<Vec<u8>> {
        // URLからパスを抽出
        let path = self.extract_path_from_url(receipt_url)?;
        
        // ユーザーアクセス権限をチェック
        UserPathManager::validate_user_access(user_id, &path)?;
        
        // ファイルをダウンロード
        self.download_file(&path).await
    }

    /// ユーザー認証付きファイル削除
    pub async fn delete_user_receipt(
        &self,
        user_id: i64,
        receipt_url: &str,
    ) -> AppResult<()> {
        // URLからパスを抽出
        let path = self.extract_path_from_url(receipt_url)?;
        
        // ユーザーアクセス権限をチェック
        UserPathManager::validate_user_access(user_id, &path)?;
        
        // ファイルを削除
        self.delete_file(&path).await
    }

    /// ファイルをダウンロード（新規メソッド）
    pub async fn download_file(&self, key: &str) -> AppResult<Vec<u8>> {
        let response = self.client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("R2ダウンロードエラー: {e}")))?;

        let data = response.body.collect().await
            .map_err(|e| AppError::ExternalService(format!("データ読み込みエラー: {e}")))?
            .into_bytes()
            .to_vec();

        Ok(data)
    }

    /// 部分ダウンロード（新規メソッド）
    pub async fn download_file_partial(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> AppResult<Vec<u8>> {
        let range = format!("bytes={start}-{end}");
        
        let response = self.client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .range(range)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("R2部分ダウンロードエラー: {e}")))?;

        let data = response.body.collect().await
            .map_err(|e| AppError::ExternalService(format!("データ読み込みエラー: {e}")))?
            .into_bytes()
            .to_vec();

        Ok(data)
    }

    /// URLからパスを抽出
    fn extract_path_from_url(&self, url: &str) -> AppResult<String> {
        if let Some(path_start) = url.find("/users/") {
            Ok(url[path_start + 1..].to_string())
        } else if let Some(path_start) = url.find("/receipts/") {
            // レガシーパスの場合
            Ok(url[path_start + 1..].to_string())
        } else {
            Err(AppError::Validation(format!("無効なURL形式: {url}")))
        }
    }
}
```

## エラーハンドリング

### 移行専用エラー型

```rust
// src-tauri/src/features/migration/errors.rs

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("移行前検証エラー: {0}")]
    PreValidationFailed(String),
    
    #[error("移行後検証エラー: {0}")]
    PostValidationFailed(String),
    
    #[error("バッチ処理エラー: {0}")]
    BatchProcessingFailed(String),
    
    #[error("データ整合性エラー: {0}")]
    DataIntegrityError(String),
    
    #[error("ファイル移行エラー: {0}")]
    FileMigrationError(String),
    
    #[error("データベース更新エラー: {0}")]
    DatabaseUpdateError(String),
    
    #[error("権限不足エラー: {0}")]
    InsufficientPermissions(String),
    
    #[error("移行がキャンセルされました")]
    MigrationCancelled,
    
    #[error("移行が一時停止中です")]
    MigrationPaused,
}

impl From<MigrationError> for AppError {
    fn from(error: MigrationError) -> Self {
        match error {
            MigrationError::DataIntegrityError(msg) => AppError::DataIntegrity(msg),
            MigrationError::InsufficientPermissions(msg) => AppError::Authorization(msg),
            MigrationError::DatabaseUpdateError(msg) => AppError::Database(msg),
            _ => AppError::Internal(error.to_string()),
        }
    }
}
```

## セキュリティ考慮事項

### アクセス制御の強化

```rust
// ユーザー認証とアクセス制御
pub async fn validate_user_receipt_access(
    user_id: i64,
    receipt_url: &str,
    db: &Connection,
) -> AppResult<()> {
    // 1. receipt_urlが該当ユーザーの経費に属するかチェック
    let query = "
        SELECT COUNT(*) 
        FROM expenses 
        WHERE user_id = ? AND receipt_url = ?
    ";
    
    let count: i64 = db.query_row(query, [user_id, receipt_url], |row| row.get(0))?;
    
    if count == 0 {
        return Err(AppError::Authorization("レシートへのアクセス権限がありません".to_string()));
    }

    // 2. パス形式の検証
    let path = extract_path_from_url(receipt_url)?;
    UserPathManager::validate_user_access(user_id, &path)?;

    Ok(())
}
```

### 監査ログ

```rust
// セキュリティ監査ログ
pub async fn log_migration_security_event(
    event_type: &str,
    user_id: Option<i64>,
    details: &str,
) -> AppResult<()> {
    let security_config = SecurityConfig {
        encryption_key: "default_key_32_bytes_long_enough".to_string(),
        max_token_age_hours: 24,
        enable_audit_logging: true,
    };
    
    let security_manager = SecurityManager::new(security_config)?;
    
    security_manager.log_security_event(
        event_type,
        user_id.map(|id| id.to_string()).as_deref(),
        details,
    ).await?;

    Ok(())
}
```

## パフォーマンス最適化

### 並列処理の最適化

```rust
// 動的並列度調整
pub struct AdaptiveBatchProcessor {
    base_concurrency: usize,
    max_concurrency: usize,
    current_concurrency: Arc<Mutex<usize>>,
    performance_monitor: Arc<Mutex<PerformanceMonitor>>,
}

impl AdaptiveBatchProcessor {
    /// パフォーマンスに基づいて並列度を調整
    pub async fn adjust_concurrency(&self) {
        let monitor = self.performance_monitor.lock().await;
        let current_throughput = monitor.get_current_throughput();
        let target_throughput = monitor.get_target_throughput();
        
        let mut concurrency = self.current_concurrency.lock().await;
        
        if current_throughput < target_throughput * 0.8 {
            // スループットが低い場合は並列度を上げる
            *concurrency = (*concurrency + 1).min(self.max_concurrency);
        } else if current_throughput > target_throughput * 1.2 {
            // スループットが高すぎる場合は並列度を下げる
            *concurrency = (*concurrency - 1).max(1);
        }
        
        info!("並列度を調整しました: {concurrency}");
    }
}
```

### メモリ使用量の最適化

```rust
// ストリーミング処理でメモリ使用量を制限
pub async fn process_large_file_streaming(
    &self,
    source_key: &str,
    dest_key: &str,
) -> AppResult<()> {
    const CHUNK_SIZE: usize = 1024 * 1024; // 1MB chunks
    
    let file_size = self.get_file_size(source_key).await?;
    let mut offset = 0;
    
    // マルチパートアップロードを開始
    let upload_id = self.start_multipart_upload(dest_key).await?;
    let mut parts = Vec::new();
    let mut part_number = 1;
    
    while offset < file_size {
        let chunk_size = CHUNK_SIZE.min((file_size - offset) as usize);
        
        // チャンクをダウンロード
        let chunk_data = self.download_file_partial(source_key, offset, offset + chunk_size as u64 - 1).await?;
        
        // チャンクをアップロード
        let part = self.upload_part(dest_key, &upload_id, part_number, chunk_data).await?;
        parts.push(part);
        
        offset += chunk_size as u64;
        part_number += 1;
    }
    
    // マルチパートアップロードを完了
    self.complete_multipart_upload(dest_key, &upload_id, parts).await?;
    
    Ok(())
}
```

## 正確性プロパティ

*プロパティは、システムのすべての有効な実行において真であるべき特性や動作です。これらは人間が読める仕様と機械で検証可能な正確性保証の橋渡しとなります。*

プロパティ1: レガシーファイル完全検出
*任意の*R2バケット状態について、レガシーパス（`/receipts/`配下）のファイルがすべて検出され、リストアップされる
**検証: 要件 1.1**

プロパティ2: ファイル・ユーザーマッピング正確性
*任意の*レシートファイルについて、データベースから対応する正しいユーザーIDが特定される
**検証: 要件 1.2**

プロパティ3: 移行プロセス順序保証
*任意の*移行操作について、ファイルコピーが成功した場合にのみ元ファイルの削除が実行される
**検証: 要件 1.4, 10.2**

プロパティ4: 移行失敗時ロールバック
*任意の*移行失敗シナリオについて、適切なエラーハンドリングとロールバックが実行される
**検証: 要件 1.5, 3.4**

プロパティ5: 新構造パス生成
*任意の*ユーザーIDとファイルについて、新しいレシートアップロード時に`/users/{user_id}/receipts/`構造が使用される
**検証: 要件 2.1**

プロパティ6: ファイル名一意性
*任意の*ファイル名生成要求について、ユーザーID、タイムスタンプ、UUIDを含む一意で予測困難な名前が作成される
**検証: 要件 2.2**

プロパティ7: ユーザーアクセス制御
*任意の*ユーザーとレシートの組み合わせについて、認証されたユーザーが自分のレシートのみアクセス可能である
**検証: 要件 2.5, 8.2**

プロパティ8: レガシーURL検出完全性
*任意の*データベース状態について、`/receipts/`パスを含むすべてのreceipt_urlが検出される
**検証: 要件 3.1**

プロパティ9: URL変換正確性
*任意の*レガシーreceipt_urlについて、対応するユーザーIDを使用して正しい新構造URLに変換される
**検証: 要件 3.2**

プロパティ10: データベース更新トランザクション整合性
*任意の*URL更新操作について、トランザクション処理によりACID特性が保たれる
**検証: 要件 3.3**

プロパティ11: 更新後検証完全性
*任意の*データベース更新操作について、変更内容が正しく適用されたことが検証される
**検証: 要件 3.5**

プロパティ12: 移行進捗表示正確性
*任意の*移行プロセスについて、開始時の対象ファイル数と進行中の処理済み/総数が正確に表示される
**検証: 要件 4.1, 4.2**

プロパティ13: 移行完了レポート完全性
*任意の*移行完了時について、成功・失敗の詳細レポートが正確に提供される
**検証: 要件 4.3**

プロパティ14: エラー情報記録完全性
*任意の*移行エラー発生時について、詳細なエラー情報とスタックトレースが記録される
**検証: 要件 4.4**

プロパティ15: 移行制御機能
*任意の*移行状態について、一時停止・再開・停止機能が正しく動作する
**検証: 要件 4.5, 7.5**

プロパティ16: ファイルハッシュ計算・記録
*任意の*移行対象ファイルについて、移行前後でハッシュ値が計算され、一致することが確認される
**検証: 要件 5.1, 5.2**

プロパティ17: データベース・R2整合性検証
*任意の*システム状態について、データベース内のreceipt_url数と実際のR2ファイル数の整合性が検証される
**検証: 要件 5.3**

プロパティ18: 移行後アクセス可能性
*任意の*移行完了状態について、すべてのレシートファイルがアクセス可能であることがテストされる
**検証: 要件 5.4**

プロパティ19: 不整合レポート提供
*任意の*整合性チェック失敗時について、詳細な不整合レポートが提供される
**検証: 要件 5.5**

プロパティ20: 移行後機能継続性
*任意の*移行完了状態について、既存のレシート表示・ダウンロード・削除機能が正常に動作する
**検証: 要件 6.1, 6.2, 6.3**

プロパティ21: 新機能正常動作
*任意の*移行完了状態について、新しいレシートアップロード機能が新構造で正常動作する
**検証: 要件 6.4**

プロパティ22: バックアップ作成完全性
*任意の*移行開始時について、データベースの完全バックアップとR2重要ファイルリストが作成・記録される
**検証: 要件 7.1, 7.2**

プロパティ23: ドライラン機能
*任意の*移行設定について、実際の変更なしでドライランテストが実行される
**検証: 要件 7.3**

プロパティ24: バッチ処理制御
*任意の*バッチサイズ指定について、指定されたサイズで段階的処理が実行される
**検証: 要件 7.4**

プロパティ25: 認証・権限検証
*任意の*レシートアクセス要求について、ユーザーID検証、管理者権限判定、アクセス制御違反検出が正しく実行される
**検証: 要件 8.1, 8.3, 8.4, 8.5**

プロパティ26: パフォーマンス最適化
*任意の*移行処理について、並列処理、メモリ制限、帯域幅制御、非阻害性、タイムアウト設定が適切に適用される
**検証: 要件 9.1, 9.2, 9.3, 9.4, 9.5**

プロパティ27: 自動クリーンアップ
*任意の*移行完了状態について、レガシーファイルの自動削除と削除対象リスト表示が実行される
**検証: 要件 10.1, 10.3**

プロパティ28: クリーンアップエラー処理
*任意の*削除処理エラーについて、詳細ログが記録され、完了後にストレージ使用量削減が報告される
**検証: 要件 10.4, 10.5**

## テスト戦略

### 単体テスト

**Rust（バックエンド）**
- ユーザーパス管理機能（パス生成、変換、検証）
- R2移行サービスの各操作（ファイル検出、移行、検証）
- バッチ処理システム（並列処理、エラーハンドリング）
- 移行検証システム（整合性チェック、アクセステスト）
- データベース操作（URL更新、トランザクション処理）

**TypeScript（フロントエンド）**
- 移行UI コンポーネント
- 進捗表示機能
- エラー表示とユーザーフィードバック
- 移行制御機能（開始、停止、一時停止）

### プロパティベーステスト

プロパティベーステストには**QuickCheck for Rust**を使用し、各テストは最小100回の反復実行を行います。

各プロパティベーステストは、設計書の対応する正確性プロパティを実装し、以下の形式でコメントタグを付けます：
`**Feature: r2-user-directory-migration, Property {番号}: {プロパティテキスト}**`

### 統合テスト

**R2移行統合**
- 実際のR2サービスとの移行処理
- 大量ファイルの移行テスト
- ネットワーク障害シミュレーション
- 並列処理の負荷テスト

**データベース統合**
- 移行ログテーブルの操作
- トランザクション処理の検証
- 整合性制約の確認

### 手動テストチェックリスト

- [ ] 各種ファイル形式の移行
- [ ] 大容量ファイルの移行
- [ ] 移行プロセスの一時停止・再開
- [ ] エラーハンドリングとロールバック
- [ ] ドライランモードの動作確認
- [ ] セキュリティ・アクセス制御の検証
- [ ] パフォーマンス測定
- [ ] 移行後の機能継続性確認

## 依存関係

### Cargo.toml追加項目

```toml
[dependencies]
# 既存の依存関係...

# ストリーミング処理
tokio-stream = "0.1"

# ハッシュ計算
sha2 = "0.10"

# プログレスバー
indicatif = "0.17"

# 正規表現
regex = "1.0"

# 並列処理制御
tokio-util = "0.7"
futures = "0.3"

# テスト用
[dev-dependencies]
quickcheck = "1.0"
quickcheck_macros = "1.0"
tempfile = "3.0"
tokio-test = "0.4"
```

## デプロイメント考慮事項

### 移行実行手順

```bash
# 1. 事前準備
# データベースバックアップの確認
# R2接続テストの実行

# 2. ドライラン実行
cargo run --bin migration -- --dry-run --batch-size 10

# 3. 本番移行実行（段階的）
cargo run --bin migration -- --batch-size 50 --max-concurrent 3

# 4. 移行後検証
cargo run --bin migration -- --validate-integrity

# 5. クリーンアップ（オプション）
cargo run --bin migration -- --cleanup-legacy-files
```

### 監視とアラート

- 移行進捗の監視
- エラー率の監視
- R2 API使用量の監視
- データベース負荷の監視
- メモリ使用量の監視

### ロールバック計画

1. **移行中断**: 進行中の移行を安全に停止
2. **データベース復元**: バックアップからの復元
3. **R2ファイル復元**: 必要に応じて新しいファイルを削除
4. **整合性確認**: 復元後の整合性チェック

## 将来の拡張

- 他のクラウドストレージプロバイダーへの対応
- 増分移行機能（新しいファイルのみ移行）
- 移行スケジューリング機能
- 移行統計とレポート機能
- 自動移行品質チェック
- 移行パフォーマンス最適化

<function_calls>
<invoke name="prework">
<parameter name="featureName">r2-user-directory-migration
//! R2ユーザーディレクトリ移行機能
//!
//! このモジュールは、R2バケット内のレシートファイルを
//! `/receipts/`構造から`/users/{user_id}/receipts/`構造に移行する
//! 機能を提供します。

use crate::features::migrations::auto_migration::executor::MigrationExecutorTrait;
use crate::features::migrations::auto_migration::models::{
    ExecutableMigrationDefinition, MigrationDefinition,
};
use crate::shared::errors::AppResult;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

/// R2移行用データベーススキーママイグレーション実行器
///
/// migration_logテーブルとmigration_itemsテーブルを作成し、
/// R2移行プロセスの追跡に必要なデータベース構造を提供します。
pub struct R2MigrationSchemaMigration;

impl MigrationExecutorTrait for R2MigrationSchemaMigration {
    /// マイグレーション実行器の名前を取得
    fn name(&self) -> &str {
        "r2_migration_schema"
    }

    /// R2移行用テーブルを作成する
    ///
    /// # 引数
    /// * `conn` - データベース接続
    ///
    /// # 戻り値
    /// 成功時はOk(())、失敗時はエラーメッセージ
    fn execute(&self, conn: &Connection) -> Result<(), String> {
        log::info!("R2移行用データベーススキーマを作成します");

        // migration_logテーブルを作成
        self.create_migration_log_table(conn)?;

        // migration_itemsテーブルを作成
        self.create_migration_items_table(conn)?;

        // インデックスを作成
        self.create_indexes(conn)?;

        log::info!("R2移行用データベーススキーマの作成が完了しました");
        Ok(())
    }
}

impl R2MigrationSchemaMigration {
    /// migration_logテーブルを作成する
    ///
    /// 移行プロセス全体の状態を追跡するテーブルです。
    /// 要件4.1, 4.2に対応します。
    fn create_migration_log_table(&self, conn: &Connection) -> Result<(), String> {
        let sql = "
            CREATE TABLE IF NOT EXISTS migration_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                migration_type TEXT NOT NULL, -- 'r2_user_directory'
                status TEXT NOT NULL CHECK(status IN ('started', 'in_progress', 'completed', 'failed', 'paused')),
                total_items INTEGER NOT NULL DEFAULT 0,
                processed_items INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                error_count INTEGER NOT NULL DEFAULT 0,
                error_details TEXT, -- JSON形式のエラー詳細
                started_at TEXT NOT NULL,
                completed_at TEXT,
                created_by TEXT, -- システムまたはユーザーID
                metadata TEXT -- JSON形式の追加情報
            )
        ";

        conn.execute(sql, [])
            .map_err(|e| format!("migration_logテーブル作成エラー: {e}"))?;

        log::info!("migration_logテーブルを作成しました");
        Ok(())
    }

    /// migration_itemsテーブルを作成する
    ///
    /// 個別ファイルの移行状態を追跡するテーブルです。
    /// 要件4.1, 4.2, 4.3に対応します。
    fn create_migration_items_table(&self, conn: &Connection) -> Result<(), String> {
        let sql = "
            CREATE TABLE IF NOT EXISTS migration_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                migration_log_id INTEGER NOT NULL,
                old_path TEXT NOT NULL,
                new_path TEXT NOT NULL,
                user_id INTEGER NOT NULL,
                file_size INTEGER NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('pending', 'processing', 'completed', 'failed')),
                error_message TEXT,
                started_at TEXT,
                completed_at TEXT,
                file_hash TEXT, -- SHA256ハッシュ
                FOREIGN KEY (migration_log_id) REFERENCES migration_log(id)
            )
        ";

        conn.execute(sql, [])
            .map_err(|e| format!("migration_itemsテーブル作成エラー: {e}"))?;

        log::info!("migration_itemsテーブルを作成しました");
        Ok(())
    }

    /// インデックスを作成する
    ///
    /// パフォーマンス向上のためのインデックスを作成します。
    fn create_indexes(&self, conn: &Connection) -> Result<(), String> {
        let indexes = [
            // migration_logテーブルのインデックス
            "CREATE INDEX IF NOT EXISTS idx_migration_log_type ON migration_log(migration_type)",
            "CREATE INDEX IF NOT EXISTS idx_migration_log_status ON migration_log(status)",
            "CREATE INDEX IF NOT EXISTS idx_migration_log_started_at ON migration_log(started_at)",
            // migration_itemsテーブルのインデックス
            "CREATE INDEX IF NOT EXISTS idx_migration_items_log_id ON migration_items(migration_log_id)",
            "CREATE INDEX IF NOT EXISTS idx_migration_items_status ON migration_items(status)",
            "CREATE INDEX IF NOT EXISTS idx_migration_items_user_id ON migration_items(user_id)",
            "CREATE INDEX IF NOT EXISTS idx_migration_items_old_path ON migration_items(old_path)",
            "CREATE INDEX IF NOT EXISTS idx_migration_items_new_path ON migration_items(new_path)",
        ];

        for (i, index_sql) in indexes.iter().enumerate() {
            conn.execute(index_sql, [])
                .map_err(|e| format!("インデックス作成エラー ({}): {e}", i + 1))?;
        }

        log::info!("R2移行用インデックスを作成しました");
        Ok(())
    }
}

/// R2移行用マイグレーション定義を取得する
///
/// # 戻り値
/// 実行可能なマイグレーション定義
pub fn get_r2_migration_schema_definition() -> ExecutableMigrationDefinition {
    // マイグレーション内容のチェックサムを計算
    let migration_content = include_str!("r2_migration_schema.sql");
    let mut hasher = Sha256::new();
    hasher.update(migration_content.as_bytes());
    let checksum = format!("{:x}", hasher.finalize());

    let definition = MigrationDefinition::new(
        "r2_migration_schema".to_string(),
        "1.0.0".to_string(),
        "R2ユーザーディレクトリ移行用データベーススキーマ".to_string(),
        checksum,
    );

    let executor = Box::new(R2MigrationSchemaMigration);

    ExecutableMigrationDefinition::new(definition, executor)
}

/// R2移行ログエントリを作成する
///
/// # 引数
/// * `conn` - データベース接続
/// * `migration_type` - 移行タイプ
/// * `total_items` - 総アイテム数
/// * `created_by` - 作成者
/// * `metadata` - メタデータ（JSON形式）
///
/// # 戻り値
/// 作成されたログエントリのID
pub fn create_migration_log_entry(
    conn: &Connection,
    migration_type: &str,
    total_items: i64,
    created_by: &str,
    metadata: Option<&str>,
) -> AppResult<i64> {
    let now = chrono::Utc::now()
        .with_timezone(&chrono_tz::Asia::Tokyo)
        .to_rfc3339();

    let sql = "
        INSERT INTO migration_log (
            migration_type, status, total_items, started_at, created_by, metadata
        ) VALUES (?, 'started', ?, ?, ?, ?)
    ";

    conn.execute(
        sql,
        [
            migration_type,
            &total_items.to_string(),
            &now,
            created_by,
            metadata.unwrap_or("null"),
        ],
    )
    .map_err(|e| {
        crate::shared::errors::AppError::Database(format!("移行ログエントリ作成エラー: {e}"))
    })?;

    let log_id = conn.last_insert_rowid();
    log::info!("移行ログエントリを作成しました: ID={log_id}");

    Ok(log_id)
}

/// R2移行ログエントリのステータスを更新する
///
/// # 引数
/// * `conn` - データベース接続
/// * `log_id` - ログエントリID
/// * `status` - 新しいステータス
/// * `processed_items` - 処理済みアイテム数
/// * `success_count` - 成功数
/// * `error_count` - エラー数
/// * `error_details` - エラー詳細（JSON形式）
pub fn update_migration_log_status(
    conn: &Connection,
    log_id: i64,
    status: &str,
    processed_items: i64,
    success_count: i64,
    error_count: i64,
    error_details: Option<&str>,
) -> AppResult<()> {
    let completed_at = if status == "completed" || status == "failed" {
        Some(
            chrono::Utc::now()
                .with_timezone(&chrono_tz::Asia::Tokyo)
                .to_rfc3339(),
        )
    } else {
        None
    };

    let sql = "
        UPDATE migration_log 
        SET status = ?, processed_items = ?, success_count = ?, error_count = ?, 
            error_details = ?, completed_at = ?
        WHERE id = ?
    ";

    conn.execute(
        sql,
        [
            status,
            &processed_items.to_string(),
            &success_count.to_string(),
            &error_count.to_string(),
            error_details.unwrap_or("null"),
            completed_at.as_deref().unwrap_or("null"),
            &log_id.to_string(),
        ],
    )
    .map_err(|e| crate::shared::errors::AppError::Database(format!("移行ログ更新エラー: {e}")))?;

    log::info!("移行ログを更新しました: ID={log_id}, status={status}");
    Ok(())
}

/// R2移行アイテムを作成する
///
/// # 引数
/// * `conn` - データベース接続
/// * `migration_log_id` - 移行ログID
/// * `old_path` - 古いパス
/// * `new_path` - 新しいパス
/// * `user_id` - ユーザーID
/// * `file_size` - ファイルサイズ
///
/// # 戻り値
/// 作成されたアイテムのID
pub fn create_migration_item(
    conn: &Connection,
    migration_log_id: i64,
    old_path: &str,
    new_path: &str,
    user_id: i64,
    file_size: i64,
) -> AppResult<i64> {
    let sql = "
        INSERT INTO migration_items (
            migration_log_id, old_path, new_path, user_id, file_size, status
        ) VALUES (?, ?, ?, ?, ?, 'pending')
    ";

    conn.execute(
        sql,
        [
            &migration_log_id.to_string(),
            old_path,
            new_path,
            &user_id.to_string(),
            &file_size.to_string(),
        ],
    )
    .map_err(|e| {
        crate::shared::errors::AppError::Database(format!("移行アイテム作成エラー: {e}"))
    })?;

    let item_id = conn.last_insert_rowid();
    log::debug!("移行アイテムを作成しました: ID={item_id}, path={old_path} -> {new_path}");

    Ok(item_id)
}

/// R2移行アイテムのステータスを更新する
///
/// # 引数
/// * `conn` - データベース接続
/// * `item_id` - アイテムID
/// * `status` - 新しいステータス
/// * `error_message` - エラーメッセージ（オプション）
/// * `file_hash` - ファイルハッシュ（オプション）
pub fn update_migration_item_status(
    conn: &Connection,
    item_id: i64,
    status: &str,
    error_message: Option<&str>,
    file_hash: Option<&str>,
) -> AppResult<()> {
    let now = chrono::Utc::now()
        .with_timezone(&chrono_tz::Asia::Tokyo)
        .to_rfc3339();

    let (started_at, completed_at) = match status {
        "processing" => (Some(now.as_str()), None),
        "completed" | "failed" => (None, Some(now.as_str())),
        _ => (None, None),
    };

    let sql = if started_at.is_some() {
        "UPDATE migration_items SET status = ?, started_at = ? WHERE id = ?"
    } else if completed_at.is_some() {
        "UPDATE migration_items SET status = ?, completed_at = ?, error_message = ?, file_hash = ? WHERE id = ?"
    } else {
        "UPDATE migration_items SET status = ? WHERE id = ?"
    };

    if let Some(started_time) = started_at {
        conn.execute(sql, [status, started_time, &item_id.to_string()])
    } else if let Some(completed_time) = completed_at {
        conn.execute(
            sql,
            [
                status,
                completed_time,
                error_message.unwrap_or("null"),
                file_hash.unwrap_or("null"),
                &item_id.to_string(),
            ],
        )
    } else {
        conn.execute(sql, [status, &item_id.to_string()])
    }
    .map_err(|e| {
        crate::shared::errors::AppError::Database(format!("移行アイテム更新エラー: {e}"))
    })?;

    log::debug!("移行アイテムを更新しました: ID={item_id}, status={status}");
    Ok(())
}

/// 移行進捗を取得する
///
/// # 引数
/// * `conn` - データベース接続
/// * `migration_log_id` - 移行ログID
///
/// # 戻り値
/// 移行進捗情報
pub fn get_migration_progress(
    conn: &Connection,
    migration_log_id: i64,
) -> AppResult<MigrationProgress> {
    // 移行ログの基本情報を取得
    let log_sql = "
        SELECT total_items, processed_items, success_count, error_count, status
        FROM migration_log WHERE id = ?
    ";

    let (total_items, processed_items, success_count, error_count, status): (
        i64,
        i64,
        i64,
        i64,
        String,
    ) = conn
        .query_row(log_sql, [migration_log_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(|e| {
            crate::shared::errors::AppError::Database(format!("移行進捗取得エラー: {e}"))
        })?;

    // スループット計算（簡易版）
    let throughput_items_per_second = if processed_items > 0 {
        // 実際の実装では開始時刻からの経過時間を使用
        processed_items as f64 / 60.0 // 仮の値
    } else {
        0.0
    };

    // 残り時間推定（簡易版）
    let estimated_remaining_time = if throughput_items_per_second > 0.0 {
        let remaining_items = total_items - processed_items;
        Some(std::time::Duration::from_secs(
            (remaining_items as f64 / throughput_items_per_second) as u64,
        ))
    } else {
        None
    };

    Ok(MigrationProgress {
        total_items: total_items as usize,
        processed_items: processed_items as usize,
        success_count: success_count as usize,
        error_count: error_count as usize,
        current_status: status,
        estimated_remaining_time,
        throughput_items_per_second,
    })
}

/// 移行進捗情報
#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
    /// 推定残り時間
    pub estimated_remaining_time: Option<std::time::Duration>,
    /// スループット（アイテム/秒）
    pub throughput_items_per_second: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_r2_migration_schema_creation() {
        let conn = Connection::open_in_memory().unwrap();
        let migration = R2MigrationSchemaMigration;

        // マイグレーション実行
        let result = migration.execute(&conn);
        assert!(result.is_ok(), "マイグレーション実行に失敗: {:?}", result);

        // テーブルが作成されていることを確認
        let tables = ["migration_log", "migration_items"];
        for table in &tables {
            let count: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{table}'"
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "テーブル {table} が作成されていません");
        }

        // インデックスが作成されていることを確認
        let index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name LIKE 'idx_migration_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            index_count >= 8,
            "期待されるインデックス数が作成されていません"
        );
    }

    #[test]
    fn test_migration_log_operations() {
        let conn = Connection::open_in_memory().unwrap();
        let migration = R2MigrationSchemaMigration;
        migration.execute(&conn).unwrap();

        // 移行ログエントリを作成
        let log_id = create_migration_log_entry(
            &conn,
            "r2_user_directory",
            100,
            "system",
            Some(r#"{"batch_size": 50}"#),
        )
        .unwrap();

        assert!(log_id > 0, "移行ログエントリが作成されませんでした");

        // ステータスを更新
        let result = update_migration_log_status(
            &conn,
            log_id,
            "in_progress",
            50,
            45,
            5,
            Some(r#"{"errors": ["file1", "file2"]}"#),
        );
        assert!(result.is_ok(), "移行ログ更新に失敗: {:?}", result);

        // 進捗を取得
        let progress = get_migration_progress(&conn, log_id).unwrap();
        assert_eq!(progress.total_items, 100);
        assert_eq!(progress.processed_items, 50);
        assert_eq!(progress.success_count, 45);
        assert_eq!(progress.error_count, 5);
        assert_eq!(progress.current_status, "in_progress");
    }

    #[test]
    fn test_migration_item_operations() {
        let conn = Connection::open_in_memory().unwrap();
        let migration = R2MigrationSchemaMigration;
        migration.execute(&conn).unwrap();

        // 移行ログを作成
        let log_id =
            create_migration_log_entry(&conn, "r2_user_directory", 1, "system", None).unwrap();

        // 移行アイテムを作成
        let item_id = create_migration_item(
            &conn,
            log_id,
            "receipts/123/file.pdf",
            "users/456/receipts/123/file.pdf",
            456,
            1024,
        )
        .unwrap();

        assert!(item_id > 0, "移行アイテムが作成されませんでした");

        // ステータスを更新（処理開始）
        let result = update_migration_item_status(&conn, item_id, "processing", None, None);
        assert!(result.is_ok(), "移行アイテム更新に失敗: {:?}", result);

        // ステータスを更新（完了）
        let result =
            update_migration_item_status(&conn, item_id, "completed", None, Some("abc123def456"));
        assert!(result.is_ok(), "移行アイテム完了更新に失敗: {:?}", result);

        // アイテムが正しく更新されていることを確認
        let (status, file_hash): (String, Option<String>) = conn
            .query_row(
                "SELECT status, file_hash FROM migration_items WHERE id = ?",
                [item_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(status, "completed");
        assert_eq!(file_hash, Some("abc123def456".to_string()));
    }
}

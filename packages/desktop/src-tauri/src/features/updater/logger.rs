use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use super::errors::UpdateError;

/// アップデートログエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLogEntry {
    /// ログのタイムスタンプ（Unix timestamp）
    pub timestamp: u64,
    /// ログレベル
    pub level: LogLevel,
    /// ログメッセージ
    pub message: String,
    /// 追加のコンテキスト情報
    pub context: Option<String>,
}

/// ログレベル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    /// 情報
    Info,
    /// 警告
    Warning,
    /// エラー
    Error,
    /// デバッグ
    Debug,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Debug => write!(f, "DEBUG"),
        }
    }
}

/// アップデートロガー
#[derive(Clone)]
pub struct UpdateLogger {
    log_file_path: PathBuf,
}

impl UpdateLogger {
    /// 新しいアップデートロガーを作成
    ///
    /// # 引数
    /// * `app_handle` - Tauriアプリケーションハンドル
    ///
    /// # 戻り値
    /// アップデートロガー、または作成に失敗した場合はErr
    pub fn new(app_handle: &AppHandle) -> Result<Self, UpdateError> {
        let app_data_dir = app_handle.path().app_data_dir().map_err(|e| {
            UpdateError::file_system(format!("アプリデータディレクトリの取得に失敗: {e}"))
        })?;

        // ログディレクトリを作成
        let log_dir = app_data_dir.join("logs");
        if !log_dir.exists() {
            fs::create_dir_all(&log_dir).map_err(|e| {
                UpdateError::file_system(format!("ログディレクトリの作成に失敗: {e}"))
            })?;
            debug!("ログディレクトリを作成しました: {}", log_dir.display());
        }

        let log_file_path = log_dir.join("updater.log");

        Ok(Self { log_file_path })
    }

    /// ログエントリを書き込み
    ///
    /// # 引数
    /// * `entry` - ログエントリ
    fn write_log_entry(&self, entry: &UpdateLogEntry) -> Result<(), UpdateError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)
            .map_err(|e| UpdateError::file_system(format!("ログファイルのオープンに失敗: {e}")))?;

        let timestamp = chrono::DateTime::from_timestamp(entry.timestamp as i64, 0)
            .unwrap_or_else(|| Utc::now())
            .with_timezone(&Tokyo);

        let log_line = if let Some(context) = &entry.context {
            format!(
                "[{}] [{}] {} - {}\n",
                timestamp.format("%Y-%m-%d %H:%M:%S JST"),
                entry.level,
                entry.message,
                context
            )
        } else {
            format!(
                "[{}] [{}] {}\n",
                timestamp.format("%Y-%m-%d %H:%M:%S JST"),
                entry.level,
                entry.message
            )
        };

        file.write_all(log_line.as_bytes())
            .map_err(|e| UpdateError::file_system(format!("ログの書き込みに失敗: {e}")))?;

        Ok(())
    }

    /// アップデートチェック開始をログ
    ///
    /// # 引数
    /// * `version` - 現在のバージョン
    pub fn log_check_start(&self, version: &str) {
        info!("アップデートチェックを開始: 現在のバージョン {version}");

        let entry = UpdateLogEntry {
            timestamp: Utc::now().with_timezone(&Tokyo).timestamp() as u64,
            level: LogLevel::Info,
            message: format!("アップデートチェックを開始: 現在のバージョン {version}"),
            context: None,
        };

        if let Err(e) = self.write_log_entry(&entry) {
            error!("ログの書き込みに失敗: {e}");
        }
    }

    /// アップデートチェック結果をログ
    ///
    /// # 引数
    /// * `available` - アップデートが利用可能かどうか
    /// * `latest_version` - 最新バージョン
    pub fn log_check_result(&self, available: bool, latest_version: Option<&str>) {
        let message = if available {
            if let Some(version) = latest_version {
                format!("アップデートが利用可能: バージョン {version}")
            } else {
                "アップデートが利用可能".to_string()
            }
        } else {
            "アップデートは利用できません".to_string()
        };

        info!("{message}");

        let entry = UpdateLogEntry {
            timestamp: Utc::now().with_timezone(&Tokyo).timestamp() as u64,
            level: LogLevel::Info,
            message,
            context: None,
        };

        if let Err(e) = self.write_log_entry(&entry) {
            error!("ログの書き込みに失敗: {e}");
        }
    }

    /// ダウンロード開始をログ
    ///
    /// # 引数
    /// * `version` - ダウンロードするバージョン
    /// * `size` - ファイルサイズ（バイト）
    pub fn log_download_start(&self, version: &str, size: Option<u64>) {
        let message = if let Some(bytes) = size {
            format!("ダウンロードを開始: バージョン {version} ({bytes} bytes)")
        } else {
            format!("ダウンロードを開始: バージョン {version}")
        };

        info!("{message}");

        let entry = UpdateLogEntry {
            timestamp: Utc::now().with_timezone(&Tokyo).timestamp() as u64,
            level: LogLevel::Info,
            message,
            context: None,
        };

        if let Err(e) = self.write_log_entry(&entry) {
            error!("ログの書き込みに失敗: {e}");
        }
    }

    /// ダウンロード進捗をログ
    ///
    /// # 引数
    /// * `downloaded` - ダウンロード済みバイト数
    /// * `total` - 総バイト数
    pub fn log_download_progress(&self, downloaded: u64, total: u64) {
        let progress = (downloaded as f64 / total as f64 * 100.0) as u32;
        debug!("ダウンロード進捗: {progress}% ({downloaded}/{total} bytes)");

        let entry = UpdateLogEntry {
            timestamp: Utc::now().with_timezone(&Tokyo).timestamp() as u64,
            level: LogLevel::Debug,
            message: format!("ダウンロード進捗: {progress}%"),
            context: Some(format!("{downloaded}/{total} bytes")),
        };

        if let Err(e) = self.write_log_entry(&entry) {
            error!("ログの書き込みに失敗: {e}");
        }
    }

    /// ダウンロード完了をログ
    ///
    /// # 引数
    /// * `version` - ダウンロードしたバージョン
    pub fn log_download_complete(&self, version: &str) {
        info!("ダウンロード完了: バージョン {version}");

        let entry = UpdateLogEntry {
            timestamp: Utc::now().with_timezone(&Tokyo).timestamp() as u64,
            level: LogLevel::Info,
            message: format!("ダウンロード完了: バージョン {version}"),
            context: None,
        };

        if let Err(e) = self.write_log_entry(&entry) {
            error!("ログの書き込みに失敗: {e}");
        }
    }

    /// インストール開始をログ
    ///
    /// # 引数
    /// * `version` - インストールするバージョン
    pub fn log_install_start(&self, version: &str) {
        info!("インストールを開始: バージョン {version}");

        let entry = UpdateLogEntry {
            timestamp: Utc::now().with_timezone(&Tokyo).timestamp() as u64,
            level: LogLevel::Info,
            message: format!("インストールを開始: バージョン {version}"),
            context: None,
        };

        if let Err(e) = self.write_log_entry(&entry) {
            error!("ログの書き込みに失敗: {e}");
        }
    }

    /// インストール完了をログ
    ///
    /// # 引数
    /// * `version` - インストールしたバージョン
    pub fn log_install_complete(&self, version: &str) {
        info!("インストール完了: バージョン {version}");

        let entry = UpdateLogEntry {
            timestamp: Utc::now().with_timezone(&Tokyo).timestamp() as u64,
            level: LogLevel::Info,
            message: format!("インストール完了: バージョン {version}"),
            context: None,
        };

        if let Err(e) = self.write_log_entry(&entry) {
            error!("ログの書き込みに失敗: {e}");
        }
    }

    /// エラーをログ
    ///
    /// # 引数
    /// * `error` - エラー情報
    pub fn log_error(&self, error: &UpdateError) {
        error!("アップデートエラー: {error}");

        let entry = UpdateLogEntry {
            timestamp: Utc::now().with_timezone(&Tokyo).timestamp() as u64,
            level: LogLevel::Error,
            message: format!("アップデートエラー: {}", error.message()),
            context: Some(format!("エラータイプ: {}", error.error_type())),
        };

        if let Err(e) = self.write_log_entry(&entry) {
            error!("ログの書き込みに失敗: {e}");
        }
    }

    /// 警告をログ
    ///
    /// # 引数
    /// * `message` - 警告メッセージ
    pub fn log_warning(&self, message: &str) {
        log::warn!("{message}");

        let entry = UpdateLogEntry {
            timestamp: Utc::now().with_timezone(&Tokyo).timestamp() as u64,
            level: LogLevel::Warning,
            message: message.to_string(),
            context: None,
        };

        if let Err(e) = self.write_log_entry(&entry) {
            error!("ログの書き込みに失敗: {e}");
        }
    }

    /// ログファイルのパスを取得
    pub fn get_log_file_path(&self) -> &PathBuf {
        &self.log_file_path
    }

    /// ログファイルをクリア
    pub fn clear_log(&self) -> Result<(), UpdateError> {
        fs::write(&self.log_file_path, "")
            .map_err(|e| UpdateError::file_system(format!("ログファイルのクリアに失敗: {e}")))?;

        info!("ログファイルをクリアしました");
        Ok(())
    }

    /// ログファイルの内容を読み込み
    ///
    /// # 引数
    /// * `max_lines` - 読み込む最大行数（Noneの場合は全行）
    ///
    /// # 戻り値
    /// ログファイルの内容
    pub fn read_log(&self, max_lines: Option<usize>) -> Result<Vec<String>, UpdateError> {
        let content = fs::read_to_string(&self.log_file_path)
            .map_err(|e| UpdateError::file_system(format!("ログファイルの読み込みに失敗: {e}")))?;

        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        if let Some(max) = max_lines {
            let start = if lines.len() > max {
                lines.len() - max
            } else {
                0
            };
            Ok(lines[start..].to_vec())
        } else {
            Ok(lines)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_display() {
        assert_eq!(format!("{}", LogLevel::Info), "INFO");
        assert_eq!(format!("{}", LogLevel::Warning), "WARN");
        assert_eq!(format!("{}", LogLevel::Error), "ERROR");
        assert_eq!(format!("{}", LogLevel::Debug), "DEBUG");
    }

    #[test]
    fn test_update_log_entry_creation() {
        let entry = UpdateLogEntry {
            timestamp: 1000,
            level: LogLevel::Info,
            message: "テストメッセージ".to_string(),
            context: Some("コンテキスト".to_string()),
        };

        assert_eq!(entry.timestamp, 1000);
        assert_eq!(entry.message, "テストメッセージ");
        assert_eq!(entry.context, Some("コンテキスト".to_string()));
    }
}

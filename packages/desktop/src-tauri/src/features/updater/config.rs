use chrono::{DateTime, Utc};
use chrono_tz::Asia::Tokyo;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// アップデーター設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdaterConfig {
    /// 自動アップデートチェックの有効/無効
    pub auto_check_enabled: bool,
    /// アップデートチェックの頻度（時間単位）
    pub check_interval_hours: u64,
    /// ベータ版アップデートの受信可否
    pub include_prereleases: bool,
    /// スキップされたバージョンのリスト
    pub skipped_versions: Vec<String>,
    /// 最後にチェックした時刻（Unix timestamp）
    pub last_check_time: Option<u64>,
}

impl Default for UpdaterConfig {
    fn default() -> Self {
        Self {
            auto_check_enabled: true,
            check_interval_hours: 24, // デフォルトは24時間
            include_prereleases: false,
            skipped_versions: Vec::new(),
            last_check_time: None,
        }
    }
}

impl UpdaterConfig {
    /// 設定ファイルのパスを取得
    ///
    /// # 引数
    /// * `app_handle` - Tauriアプリケーションハンドル
    ///
    /// # 戻り値
    /// 設定ファイルのパス
    fn get_config_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("アプリデータディレクトリの取得に失敗: {e}"))?;

        // アプリデータディレクトリが存在しない場合は作成
        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir)
                .map_err(|e| format!("アプリデータディレクトリの作成に失敗: {e}"))?;
            debug!(
                "アプリデータディレクトリを作成しました: {}",
                app_data_dir.display()
            );
        }

        Ok(app_data_dir.join("updater_config.json"))
    }

    /// 設定をファイルから読み込み
    ///
    /// # 引数
    /// * `app_handle` - Tauriアプリケーションハンドル
    ///
    /// # 戻り値
    /// 読み込まれた設定、またはデフォルト設定
    pub fn load(app_handle: &AppHandle) -> Self {
        let config_path = match Self::get_config_path(app_handle) {
            Ok(path) => path,
            Err(e) => {
                error!("設定ファイルパスの取得に失敗: {e}");
                return Self::default();
            }
        };

        debug!("設定ファイルを読み込み中: {}", config_path.display());

        match fs::read_to_string(&config_path) {
            Ok(content) => match serde_json::from_str::<UpdaterConfig>(&content) {
                Ok(config) => {
                    info!("アップデーター設定を読み込みました");
                    debug!("読み込まれた設定: {config:?}");
                    config
                }
                Err(e) => {
                    warn!("設定ファイルの解析に失敗、デフォルト設定を使用: {e}");
                    Self::default()
                }
            },
            Err(e) => {
                debug!("設定ファイルが見つからないか読み込みに失敗、デフォルト設定を使用: {e}");
                Self::default()
            }
        }
    }

    /// 設定をファイルに保存
    ///
    /// # 引数
    /// * `app_handle` - Tauriアプリケーションハンドル
    ///
    /// # 戻り値
    /// 保存に成功した場合はOk(())、失敗した場合はErr
    pub fn save(&self, app_handle: &AppHandle) -> Result<(), String> {
        let config_path = Self::get_config_path(app_handle)?;

        debug!("設定ファイルを保存中: {}", config_path.display());
        debug!("保存する設定: {self:?}");

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("設定のシリアライズに失敗: {e}"))?;

        fs::write(&config_path, content)
            .map_err(|e| format!("設定ファイルの書き込みに失敗: {e}"))?;

        info!(
            "アップデーター設定を保存しました: {}",
            config_path.display()
        );
        Ok(())
    }

    /// 特定のバージョンをスキップリストに追加
    ///
    /// # 引数
    /// * `version` - スキップするバージョン
    pub fn skip_version(&mut self, version: String) {
        if !self.skipped_versions.contains(&version) {
            self.skipped_versions.push(version.clone());
            info!("バージョン {version} をスキップリストに追加しました");
        }
    }

    /// バージョンがスキップされているかチェック
    ///
    /// # 引数
    /// * `version` - チェックするバージョン
    ///
    /// # 戻り値
    /// スキップされている場合はtrue
    pub fn is_version_skipped(&self, version: &str) -> bool {
        self.skipped_versions.contains(&version.to_string())
    }

    /// 最後のチェック時刻を更新
    pub fn update_last_check_time(&mut self) {
        let now = Utc::now().with_timezone(&Tokyo);
        self.last_check_time = Some(now.timestamp() as u64);
        debug!(
            "最後のチェック時刻を更新: {}",
            now.format("%Y-%m-%d %H:%M:%S JST")
        );
    }

    /// 次回チェック時刻を取得
    ///
    /// # 戻り値
    /// 次回チェック時刻（Unix timestamp）、または最初のチェックの場合はNone
    pub fn get_next_check_time(&self) -> Option<u64> {
        self.last_check_time
            .map(|last_check| last_check + (self.check_interval_hours * 3600))
    }

    /// チェックが必要かどうかを判定
    ///
    /// # 戻り値
    /// チェックが必要な場合はtrue
    pub fn should_check_now(&self) -> bool {
        if !self.auto_check_enabled {
            return false;
        }

        match self.get_next_check_time() {
            Some(next_check) => {
                let now = Utc::now().with_timezone(&Tokyo).timestamp() as u64;
                now >= next_check
            }
            None => true, // 初回チェック
        }
    }

    /// 設定の妥当性を検証
    ///
    /// # 戻り値
    /// 設定が有効な場合はOk(())、無効な場合はErr
    pub fn validate(&self) -> Result<(), String> {
        if self.check_interval_hours == 0 {
            return Err("チェック間隔は0より大きい値である必要があります".to_string());
        }

        if self.check_interval_hours > 24 * 7 {
            return Err("チェック間隔は1週間（168時間）以下である必要があります".to_string());
        }

        Ok(())
    }

    /// デバッグ情報を取得
    ///
    /// # 戻り値
    /// デバッグ情報のマップ
    pub fn get_debug_info(&self) -> std::collections::HashMap<String, String> {
        let mut info = std::collections::HashMap::new();
        info.insert(
            "auto_check_enabled".to_string(),
            self.auto_check_enabled.to_string(),
        );
        info.insert(
            "check_interval_hours".to_string(),
            self.check_interval_hours.to_string(),
        );
        info.insert(
            "include_prereleases".to_string(),
            self.include_prereleases.to_string(),
        );
        info.insert(
            "skipped_versions_count".to_string(),
            self.skipped_versions.len().to_string(),
        );

        if let Some(last_check) = self.last_check_time {
            let dt = DateTime::from_timestamp(last_check as i64, 0)
                .unwrap_or_else(Utc::now)
                .with_timezone(&Tokyo);
            info.insert(
                "last_check_time".to_string(),
                dt.format("%Y-%m-%d %H:%M:%S JST").to_string(),
            );
        } else {
            info.insert("last_check_time".to_string(), "未実行".to_string());
        }

        if let Some(next_check) = self.get_next_check_time() {
            let dt = DateTime::from_timestamp(next_check as i64, 0)
                .unwrap_or_else(Utc::now)
                .with_timezone(&Tokyo);
            info.insert(
                "next_check_time".to_string(),
                dt.format("%Y-%m-%d %H:%M:%S JST").to_string(),
            );
        } else {
            info.insert("next_check_time".to_string(), "初回チェック".to_string());
        }

        info.insert(
            "should_check_now".to_string(),
            self.should_check_now().to_string(),
        );

        info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_updater_config_default() {
        let config = UpdaterConfig::default();

        assert!(config.auto_check_enabled);
        assert_eq!(config.check_interval_hours, 24);
        assert!(!config.include_prereleases);
        assert!(config.skipped_versions.is_empty());
        assert!(config.last_check_time.is_none());
    }

    #[test]
    fn test_skip_version() {
        let mut config = UpdaterConfig::default();

        config.skip_version("1.0.0".to_string());
        assert!(config.is_version_skipped("1.0.0"));
        assert!(!config.is_version_skipped("1.0.1"));

        // 同じバージョンを再度スキップしても重複しない
        config.skip_version("1.0.0".to_string());
        assert_eq!(config.skipped_versions.len(), 1);
    }

    #[test]
    fn test_update_last_check_time() {
        let mut config = UpdaterConfig::default();

        assert!(config.last_check_time.is_none());

        config.update_last_check_time();
        assert!(config.last_check_time.is_some());

        let first_check = config.last_check_time.unwrap();

        // 少し待ってから再度更新
        std::thread::sleep(std::time::Duration::from_millis(10));
        config.update_last_check_time();

        let second_check = config.last_check_time.unwrap();
        assert!(second_check >= first_check);
    }

    #[test]
    fn test_should_check_now() {
        // 自動チェックが無効の場合
        let mut config = UpdaterConfig {
            auto_check_enabled: false,
            ..Default::default()
        };
        assert!(!config.should_check_now());

        // 自動チェックが有効で初回の場合
        config.auto_check_enabled = true;
        assert!(config.should_check_now());

        // 最後のチェック時刻を現在時刻に設定
        config.update_last_check_time();
        assert!(!config.should_check_now()); // まだ間隔が経過していない

        // 過去の時刻を設定（チェックが必要）
        config.last_check_time = Some(0); // 1970年1月1日
        assert!(config.should_check_now());
    }

    #[test]
    fn test_get_next_check_time() {
        let mut config = UpdaterConfig::default();

        // 初回の場合
        assert!(config.get_next_check_time().is_none());

        // 最後のチェック時刻を設定
        config.last_check_time = Some(1000);
        let next_check = config.get_next_check_time().unwrap();
        assert_eq!(next_check, 1000 + (24 * 3600)); // 24時間後
    }

    #[test]
    fn test_validate() {
        let mut config = UpdaterConfig::default();

        // デフォルト設定は有効
        assert!(config.validate().is_ok());

        // チェック間隔が0の場合は無効
        config.check_interval_hours = 0;
        assert!(config.validate().is_err());

        // チェック間隔が1週間を超える場合は無効
        config.check_interval_hours = 24 * 7 + 1;
        assert!(config.validate().is_err());

        // 有効な範囲内
        config.check_interval_hours = 12;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_get_debug_info() {
        let mut config = UpdaterConfig::default();
        config.update_last_check_time();
        config.skip_version("1.0.0".to_string());

        let debug_info = config.get_debug_info();

        assert_eq!(
            debug_info.get("auto_check_enabled"),
            Some(&"true".to_string())
        );
        assert_eq!(
            debug_info.get("check_interval_hours"),
            Some(&"24".to_string())
        );
        assert_eq!(
            debug_info.get("include_prereleases"),
            Some(&"false".to_string())
        );
        assert_eq!(
            debug_info.get("skipped_versions_count"),
            Some(&"1".to_string())
        );
        assert!(debug_info.contains_key("last_check_time"));
        assert!(debug_info.contains_key("next_check_time"));
        assert!(debug_info.contains_key("should_check_now"));
    }
}

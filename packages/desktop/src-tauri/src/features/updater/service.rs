use super::config::UpdaterConfig;
use super::errors::UpdateError;
use super::logger::UpdateLogger;
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

/// アップデート情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// 利用可能なアップデートがあるかどうか
    pub available: bool,
    /// 現在のバージョン
    pub current_version: String,
    /// 最新バージョン
    pub latest_version: Option<String>,
    /// アップデートの詳細情報
    pub release_notes: Option<String>,
    /// アップデートのサイズ（バイト）
    pub content_length: Option<u64>,
    /// 最後にチェックした時刻（Unix timestamp）
    pub last_checked: u64,
    /// ダウンロードURL
    pub download_url: Option<String>,
    /// 署名情報
    pub signature: Option<String>,
}

/// アップデートサービス
pub struct UpdaterService {
    app_handle: AppHandle,
    config: UpdaterConfig,
    logger: UpdateLogger,
}

impl UpdaterService {
    /// 新しいアップデートサービスを作成
    pub fn new(app_handle: AppHandle) -> Self {
        let config = UpdaterConfig::load(&app_handle);
        let logger = UpdateLogger::new(&app_handle).unwrap_or_else(|e| {
            error!("UpdateLoggerの作成に失敗: {e}");
            // フォールバック: ログ機能なしで続行
            panic!("UpdateLoggerの作成に失敗しました");
        });

        Self {
            app_handle,
            config,
            logger,
        }
    }

    /// 設定を取得
    pub async fn get_config(&self) -> UpdaterConfig {
        self.config.clone()
    }

    /// 設定を更新
    pub async fn update_config(&mut self, new_config: UpdaterConfig) -> Result<(), UpdateError> {
        // 設定の妥当性を検証
        new_config.validate().map_err(UpdateError::configuration)?;

        // 設定を保存
        new_config
            .save(&self.app_handle)
            .map_err(UpdateError::configuration)?;

        // メモリ上の設定を更新
        self.config = new_config;

        info!("アップデーター設定を更新しました");
        self.logger
            .log_warning("アップデーター設定が更新されました");
        Ok(())
    }

    /// アップデートをチェック
    pub async fn check_for_updates(&mut self) -> Result<UpdateInfo, UpdateError> {
        let current_version = self.app_handle.package_info().version.to_string();

        // ログ: チェック開始
        self.logger.log_check_start(&current_version);
        info!("アップデートをチェック中...");

        let now = Utc::now().with_timezone(&Tokyo);
        let now_timestamp = now.timestamp() as u64;

        // 設定を更新（最後のチェック時刻を記録）
        self.config.update_last_check_time();
        if let Err(e) = self.config.save(&self.app_handle) {
            warn!("設定の保存に失敗: {e}");
            self.logger.log_warning(&format!("設定の保存に失敗: {e}"));
        }

        match self.app_handle.updater() {
            Ok(updater) => {
                match updater.check().await {
                    Ok(Some(update)) => {
                        info!("アップデートが利用可能: {}", update.version);

                        // スキップされたバージョンかチェック
                        if self.config.is_version_skipped(&update.version) {
                            info!("バージョン {} はスキップされています", update.version);
                            self.logger.log_check_result(false, Some(&update.version));

                            return Ok(UpdateInfo {
                                available: false,
                                current_version,
                                latest_version: Some(update.version),
                                release_notes: update.body.clone(),
                                content_length: None,
                                last_checked: now_timestamp,
                                download_url: None,
                                signature: None,
                            });
                        }

                        // ログ: チェック結果
                        self.logger.log_check_result(true, Some(&update.version));

                        Ok(UpdateInfo {
                            available: true,
                            current_version,
                            latest_version: Some(update.version.clone()),
                            release_notes: update.body.clone(),
                            content_length: None, // content_lengthフィールドが利用できないため
                            last_checked: now_timestamp,
                            download_url: None, // Tauri updaterでは直接取得できない
                            signature: None,    // Tauri updaterでは直接取得できない
                        })
                    }
                    Ok(None) => {
                        info!("アップデートは利用できません");
                        self.logger.log_check_result(false, None);

                        Ok(UpdateInfo {
                            available: false,
                            current_version,
                            latest_version: None,
                            release_notes: None,
                            content_length: None,
                            last_checked: now_timestamp,
                            download_url: None,
                            signature: None,
                        })
                    }
                    Err(e) => {
                        let error = UpdateError::network(format!(
                            "アップデートチェックに失敗しました: {e}"
                        ));
                        self.logger.log_error(&error);
                        Err(error)
                    }
                }
            }
            Err(e) => {
                let error = UpdateError::initialization_error(format!(
                    "アップデーター機能が利用できません: {e}"
                ));
                self.logger.log_error(&error);
                Err(error)
            }
        }
    }

    /// アップデートをダウンロードしてインストール
    pub async fn download_and_install(&self) -> Result<(), UpdateError> {
        info!("アップデートのダウンロードとインストールを開始...");

        match self.app_handle.updater() {
            Ok(updater) => {
                match updater.check().await {
                    Ok(Some(update)) => {
                        let version = update.version.clone();

                        // ログ: ダウンロード開始
                        self.logger.log_download_start(&version, None);
                        info!("アップデートをダウンロード中: {version}");

                        let logger = self.logger.clone();
                        let app_handle = self.app_handle.clone();

                        match update
                            .download_and_install(
                                move |chunk_length, content_length| {
                                    // プログレスコールバック
                                    if let Some(total) = content_length {
                                        let progress = (chunk_length as f64 / total as f64 * 100.0) as u32;
                                        debug!("ダウンロード進捗: {progress}% ({chunk_length}/{total} bytes)");
                                        
                                        // ログ: ダウンロード進捗
                                        logger.log_download_progress(chunk_length as u64, total as u64);
                                        
                                        // フロントエンドに進捗を通知
                                        if let Err(e) = app_handle.emit("download-progress", progress) {
                                            warn!("ダウンロード進捗の通知に失敗: {e}");
                                        }
                                    } else {
                                        debug!("ダウンロード中: {chunk_length} bytes");
                                    }
                                },
                                || {
                                    // 完了コールバック
                                    info!("ダウンロード完了");
                                    if let Err(e) = self.app_handle.emit("download-complete", ()) {
                                        warn!("ダウンロード完了の通知に失敗: {e}");
                                    }
                                },
                            )
                            .await
                        {
                            Ok(_) => {
                                // ログ: ダウンロード完了
                                self.logger.log_download_complete(&version);
                                
                                // ログ: インストール開始
                                self.logger.log_install_start(&version);
                                
                                // ログ: インストール完了
                                self.logger.log_install_complete(&version);
                                
                                info!("アップデートのインストールが完了しました");
                                Ok(())
                            }
                            Err(e) => {
                                let error = UpdateError::installation(format!("アップデートのインストールに失敗しました: {e}"));
                                self.logger.log_error(&error);
                                Err(error)
                            }
                        }
                    }
                    Ok(None) => {
                        let error =
                            UpdateError::general("インストール可能なアップデートがありません");
                        self.logger
                            .log_warning("インストール可能なアップデートがありません");
                        Err(error)
                    }
                    Err(e) => {
                        let error = UpdateError::network(format!(
                            "アップデートチェックに失敗しました: {e}"
                        ));
                        self.logger.log_error(&error);
                        Err(error)
                    }
                }
            }
            Err(e) => {
                let error = UpdateError::initialization_error(format!(
                    "アップデーター機能が利用できません: {e}"
                ));
                self.logger.log_error(&error);
                Err(error)
            }
        }
    }

    /// バージョンをスキップ
    pub async fn skip_version(&mut self, version: String) -> Result<(), UpdateError> {
        info!("バージョン {version} をスキップします");

        self.config.skip_version(version);
        self.config
            .save(&self.app_handle)
            .map_err(UpdateError::configuration)?;

        Ok(())
    }

    /// 自動アップデートチェックを開始（バックグラウンドで定期実行）
    pub fn start_auto_check(&self) {
        if !self.config.auto_check_enabled {
            info!("自動アップデートチェックは無効になっています");
            return;
        }

        let app_handle = self.app_handle.clone();
        let interval_hours = self.config.check_interval_hours;
        let interval = Duration::from_secs(interval_hours * 3600);

        info!(
            "自動アップデートチェックを開始します（{}時間間隔）",
            interval_hours
        );

        tauri::async_runtime::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                let mut service = UpdaterService::new(app_handle.clone());

                // 設定をリロードして最新の状態を確認
                service.config = UpdaterConfig::load(&app_handle);

                if !service.config.auto_check_enabled {
                    info!("自動アップデートチェックが無効化されました。タスクを終了します。");
                    break;
                }

                if !service.config.should_check_now() {
                    debug!("まだチェック時刻ではありません。スキップします。");
                    continue;
                }

                match service.check_for_updates().await {
                    Ok(update_info) => {
                        if update_info.available {
                            info!("自動チェック: アップデートが利用可能です");

                            // フロントエンドにアップデート通知を送信
                            if let Err(e) = app_handle.emit("update-available", &update_info) {
                                error!("アップデート通知の送信に失敗: {e}");
                            }
                        } else {
                            debug!("自動チェック: アップデートはありません");
                        }
                    }
                    Err(e) => {
                        error!("自動アップデートチェックエラー: {e}");

                        // エラー情報をフロントエンドに送信
                        if let Err(emit_error) =
                            app_handle.emit("update-check-error", e.user_friendly_message())
                        {
                            error!("エラー通知の送信に失敗: {emit_error}");
                        }
                    }
                }
            }
        });
    }

    /// 自動アップデートチェックを停止
    pub fn stop_auto_check(&mut self) {
        info!("自動アップデートチェックを停止します");
        self.config.auto_check_enabled = false;

        if let Err(e) = self.config.save(&self.app_handle) {
            error!("設定の保存に失敗: {e}");
        }
    }
}

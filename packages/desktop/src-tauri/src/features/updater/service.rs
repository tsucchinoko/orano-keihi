use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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
}

/// アップデートサービス
pub struct UpdaterService {
    app_handle: AppHandle,
}

impl UpdaterService {
    /// 新しいアップデートサービスを作成
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// アップデートをチェック
    pub async fn check_for_updates(&self) -> Result<UpdateInfo, String> {
        info!("アップデートをチェック中...");

        let current_version = self.app_handle.package_info().version.to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        match self.app_handle.updater() {
            Ok(updater) => {
                match updater.check().await {
                    Ok(Some(update)) => {
                        info!("アップデートが利用可能: {}", update.version);

                        Ok(UpdateInfo {
                            available: true,
                            current_version,
                            latest_version: Some(update.version.clone()),
                            release_notes: update.body.clone(),
                            content_length: None, // content_lengthフィールドが利用できないため
                            last_checked: now,
                        })
                    }
                    Ok(None) => {
                        info!("アップデートは利用できません");
                        Ok(UpdateInfo {
                            available: false,
                            current_version,
                            latest_version: None,
                            release_notes: None,
                            content_length: None,
                            last_checked: now,
                        })
                    }
                    Err(e) => {
                        error!("アップデートチェックエラー: {}", e);
                        Err(format!("アップデートチェックに失敗しました: {}", e))
                    }
                }
            }
            Err(e) => {
                error!("アップデーター初期化エラー: {}", e);
                Err(format!("アップデーター機能が利用できません: {}", e))
            }
        }
    }

    /// アップデートをダウンロードしてインストール
    pub async fn download_and_install(&self) -> Result<(), String> {
        info!("アップデートのダウンロードとインストールを開始...");

        match self.app_handle.updater() {
            Ok(updater) => {
                match updater.check().await {
                    Ok(Some(update)) => {
                        info!("アップデートをダウンロード中: {}", update.version);

                        match update
                            .download_and_install(
                                |_chunk_length, _content_length| {
                                    // プログレスコールバック（簡略化）
                                    info!("ダウンロード中...");
                                },
                                || {
                                    // 完了コールバック
                                    info!("ダウンロード完了");
                                },
                            )
                            .await
                        {
                            Ok(_) => {
                                info!("アップデートのインストールが完了しました");
                                Ok(())
                            }
                            Err(e) => {
                                error!("アップデートのインストールエラー: {}", e);
                                Err(format!("アップデートのインストールに失敗しました: {}", e))
                            }
                        }
                    }
                    Ok(None) => {
                        warn!("インストール可能なアップデートがありません");
                        Err("インストール可能なアップデートがありません".to_string())
                    }
                    Err(e) => {
                        error!("アップデートチェックエラー: {}", e);
                        Err(format!("アップデートチェックに失敗しました: {}", e))
                    }
                }
            }
            Err(e) => {
                error!("アップデーター初期化エラー: {}", e);
                Err(format!("アップデーター機能が利用できません: {}", e))
            }
        }
    }

    /// 自動アップデートチェックを開始（バックグラウンドで定期実行）
    pub fn start_auto_check(&self, interval_hours: u64) {
        let app_handle = self.app_handle.clone();
        let interval = Duration::from_secs(interval_hours * 3600);

        tauri::async_runtime::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                let service = UpdaterService::new(app_handle.clone());
                match service.check_for_updates().await {
                    Ok(update_info) => {
                        if update_info.available {
                            info!("自動チェック: アップデートが利用可能です");

                            // フロントエンドにアップデート通知を送信
                            if let Err(e) = app_handle.emit("update-available", &update_info) {
                                error!("アップデート通知の送信に失敗: {}", e);
                            }
                        } else {
                            info!("自動チェック: アップデートはありません");
                        }
                    }
                    Err(e) => {
                        error!("自動アップデートチェックエラー: {}", e);
                    }
                }
            }
        });
    }
}

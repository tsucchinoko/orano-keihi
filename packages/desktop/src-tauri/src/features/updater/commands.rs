use super::service::{UpdateInfo, UpdaterService};
use log::{error, info};
use tauri::{AppHandle, State};

/// アップデートをチェックするコマンド
#[tauri::command]
pub async fn check_for_updates(app_handle: AppHandle) -> Result<UpdateInfo, String> {
    info!("アップデートチェックコマンドが呼び出されました");

    let service = UpdaterService::new(app_handle);
    service.check_for_updates().await
}

/// アップデートをダウンロードしてインストールするコマンド
#[tauri::command]
pub async fn download_and_install_update(app_handle: AppHandle) -> Result<(), String> {
    info!("アップデートインストールコマンドが呼び出されました");

    let service = UpdaterService::new(app_handle);
    service.download_and_install().await
}

/// 現在のアプリケーションバージョンを取得するコマンド
#[tauri::command]
pub fn get_app_version(app_handle: AppHandle) -> String {
    app_handle.package_info().version.to_string()
}

/// 自動アップデートチェックを開始するコマンド
#[tauri::command]
pub fn start_auto_update_check(
    app_handle: AppHandle,
    interval_hours: Option<u64>,
) -> Result<(), String> {
    let interval = interval_hours.unwrap_or(24); // デフォルトは24時間

    info!(
        "自動アップデートチェックを開始します（{}時間間隔）",
        interval
    );

    let service = UpdaterService::new(app_handle);
    service.start_auto_check(interval);

    Ok(())
}

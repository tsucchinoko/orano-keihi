use super::config::UpdaterConfig;
use super::service::{UpdateInfo, UpdaterService};
use log::info;
use tauri::AppHandle;

/// アップデートをチェックするコマンド
#[tauri::command]
pub async fn check_for_updates(app_handle: AppHandle) -> Result<UpdateInfo, String> {
    info!("アップデートチェックコマンドが呼び出されました");

    let mut service = UpdaterService::new(app_handle);
    service.check_for_updates().await.map_err(|e| e.to_string())
}

/// アップデートを強制的にチェックするコマンド（スキップされたバージョンも含む）
#[tauri::command]
pub async fn check_for_updates_force(app_handle: AppHandle) -> Result<UpdateInfo, String> {
    info!("アップデート強制チェックコマンドが呼び出されました");

    let mut service = UpdaterService::new(app_handle);
    service
        .check_for_updates_force()
        .await
        .map_err(|e| e.to_string())
}

/// アップデートをダウンロードしてインストールするコマンド
#[tauri::command]
pub async fn download_and_install_update(app_handle: AppHandle) -> Result<(), String> {
    info!("アップデートインストールコマンドが呼び出されました");

    let service = UpdaterService::new(app_handle);
    service
        .download_and_install()
        .await
        .map_err(|e| e.to_string())
}

/// 現在のアプリケーションバージョンを取得するコマンド
#[tauri::command]
pub fn get_app_version(app_handle: AppHandle) -> String {
    app_handle.package_info().version.to_string()
}

/// アップデーター設定を取得するコマンド
#[tauri::command]
pub async fn get_updater_config(app_handle: AppHandle) -> Result<UpdaterConfig, String> {
    info!("アップデーター設定取得コマンドが呼び出されました");

    let service = UpdaterService::new(app_handle);
    Ok(service.get_config().await)
}

/// アップデーター設定を更新するコマンド
#[tauri::command]
pub async fn update_updater_config(
    app_handle: AppHandle,
    config: UpdaterConfig,
) -> Result<(), String> {
    info!("アップデーター設定更新コマンドが呼び出されました");

    let mut service = UpdaterService::new(app_handle);
    service
        .update_config(config)
        .await
        .map_err(|e| e.to_string())
}

/// バージョンをスキップするコマンド
#[tauri::command]
pub async fn skip_version(app_handle: AppHandle, version: String) -> Result<(), String> {
    info!("バージョンスキップコマンドが呼び出されました: {version}");

    let mut service = UpdaterService::new(app_handle);
    service
        .skip_version(version)
        .await
        .map_err(|e| e.to_string())
}

/// 自動アップデートチェックを開始するコマンド
#[tauri::command]
pub fn start_auto_update_check(app_handle: AppHandle) -> Result<(), String> {
    info!("自動アップデートチェック開始コマンドが呼び出されました");

    let service = UpdaterService::new(app_handle);
    service.start_auto_check();

    Ok(())
}

/// 自動アップデートチェックを停止するコマンド
#[tauri::command]
pub fn stop_auto_update_check(app_handle: AppHandle) -> Result<(), String> {
    info!("自動アップデートチェック停止コマンドが呼び出されました");

    let mut service = UpdaterService::new(app_handle);
    service.stop_auto_check();

    Ok(())
}

/// アプリケーションを再起動してアップデートをインストールするコマンド
#[tauri::command]
pub fn restart_application(app_handle: AppHandle) -> Result<(), String> {
    info!("アプリケーション再起動コマンドが呼び出されました");

    // アプリケーションを再起動
    // 注: restart()は実行されるとプロセスが終了するため、この後のコードは実行されない
    app_handle.restart();
}

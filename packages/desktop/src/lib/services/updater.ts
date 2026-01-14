import type { UpdateInfo, UpdaterConfig } from '$lib/types/updater';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/**
 * アップデートサービス
 * Tauriのアップデート機能を管理します
 */
export class UpdaterService {
  /**
   * アップデートをチェック
   */
  static async checkForUpdates(): Promise<UpdateInfo> {
    try {
      return await invoke<UpdateInfo>('check_for_updates');
    } catch (error) {
      console.error('アップデートチェックエラー:', error);
      throw new Error(`アップデートチェックに失敗しました: ${String(error)}`);
    }
  }

  /**
   * アップデートを強制的にチェック（スキップされたバージョンも含む）
   */
  static async checkForUpdatesForce(): Promise<UpdateInfo> {
    try {
      return await invoke<UpdateInfo>('check_for_updates_force');
    } catch (error) {
      console.error('アップデート強制チェックエラー:', error);
      throw new Error(`アップデートチェックに失敗しました: ${String(error)}`);
    }
  }

  /**
   * アップデートをダウンロードしてインストール
   */
  static async downloadAndInstall(): Promise<void> {
    try {
      await invoke<void>('download_and_install_update');
    } catch (error) {
      console.error('アップデートインストールエラー:', error);
      throw new Error(
        `アップデートのインストールに失敗しました: ${String(error)}`
      );
    }
  }

  /**
   * 現在のアプリケーションバージョンを取得
   */
  static async getAppVersion(): Promise<string> {
    try {
      return await invoke<string>('get_app_version');
    } catch (error) {
      console.error('バージョン取得エラー:', error);
      throw new Error(`バージョンの取得に失敗しました: ${String(error)}`);
    }
  }

  /**
   * アップデーター設定を取得
   */
  static async getConfig(): Promise<UpdaterConfig> {
    try {
      return await invoke<UpdaterConfig>('get_updater_config');
    } catch (error) {
      console.error('設定取得エラー:', error);
      throw new Error(`設定の取得に失敗しました: ${String(error)}`);
    }
  }

  /**
   * アップデーター設定を更新
   * @param config 新しい設定
   */
  static async updateConfig(config: UpdaterConfig): Promise<void> {
    try {
      await invoke<void>('update_updater_config', { config });
    } catch (error) {
      console.error('設定更新エラー:', error);
      throw new Error(`設定の更新に失敗しました: ${String(error)}`);
    }
  }

  /**
   * 特定のバージョンをスキップ
   * @param version スキップするバージョン
   */
  static async skipVersion(version: string): Promise<void> {
    try {
      await invoke<void>('skip_version', { version });
    } catch (error) {
      console.error('バージョンスキップエラー:', error);
      throw new Error(`バージョンのスキップに失敗しました: ${String(error)}`);
    }
  }

  /**
   * 自動アップデートチェックを開始
   */
  static async startAutoUpdateCheck(): Promise<void> {
    try {
      await invoke<void>('start_auto_update_check');
    } catch (error) {
      console.error('自動アップデートチェック開始エラー:', error);
      throw new Error(
        `自動アップデートチェックの開始に失敗しました: ${String(error)}`
      );
    }
  }

  /**
   * 自動アップデートチェックを停止
   */
  static async stopAutoUpdateCheck(): Promise<void> {
    try {
      await invoke<void>('stop_auto_update_check');
    } catch (error) {
      console.error('自動アップデートチェック停止エラー:', error);
      throw new Error(
        `自動アップデートチェックの停止に失敗しました: ${String(error)}`
      );
    }
  }

  /**
   * アプリケーションを再起動してアップデートをインストール
   */
  static async restartApplication(): Promise<void> {
    try {
      await invoke<void>('restart_application');
    } catch (error) {
      console.error('アプリケーション再起動エラー:', error);
      throw new Error(
        `アプリケーションの再起動に失敗しました: ${String(error)}`
      );
    }
  }

  /**
   * アップデート通知イベントをリッスン
   * @param callback アップデートが利用可能になったときのコールバック
   */
  static async listenForUpdates(
    callback: (updateInfo: UpdateInfo) => void
  ): Promise<() => void> {
    try {
      const unlisten = await listen<UpdateInfo>('update-available', (event) => {
        console.info('アップデートが利用可能です:', event.payload);
        callback(event.payload);
      });
      return unlisten;
    } catch (error) {
      console.error('アップデート通知リスナー設定エラー:', error);
      throw new Error(`アップデート通知の設定に失敗しました: ${String(error)}`);
    }
  }

  /**
   * 再起動必要通知イベントをリッスン
   * @param callback 再起動が必要になったときのコールバック
   */
  static async listenForRestartRequired(
    callback: () => void
  ): Promise<() => void> {
    try {
      const unlisten = await listen('restart-required', () => {
        console.info('アプリケーションの再起動が必要です');
        callback();
      });
      return unlisten;
    } catch (error) {
      console.error('再起動通知リスナー設定エラー:', error);
      throw new Error(`再起動通知の設定に失敗しました: ${String(error)}`);
    }
  }

  /**
   * ダウンロード完了通知イベントをリッスン
   * @param callback ダウンロードが完了したときのコールバック
   */
  static async listenForDownloadComplete(
    callback: () => void
  ): Promise<() => void> {
    try {
      const unlisten = await listen('download-complete', () => {
        console.info('ダウンロードが完了しました');
        callback();
      });
      return unlisten;
    } catch (error) {
      console.error('ダウンロード完了通知リスナー設定エラー:', error);
      throw new Error(
        `ダウンロード完了通知の設定に失敗しました: ${String(error)}`
      );
    }
  }

  /**
   * ダウンロード進捗イベントをリッスン
   * @param callback ダウンロード進捗が更新されたときのコールバック
   */
  static async listenForDownloadProgress(
    callback: (progress: number) => void
  ): Promise<() => void> {
    try {
      const unlisten = await listen<number>('download-progress', (event) => {
        callback(event.payload);
      });
      return unlisten;
    } catch (error) {
      console.error('ダウンロード進捗リスナー設定エラー:', error);
      throw new Error(
        `ダウンロード進捗通知の設定に失敗しました: ${String(error)}`
      );
    }
  }

  /**
   * アップデート不要通知イベントをリッスン
   * @param callback アップデートが不要なときのコールバック
   */
  static async listenForNoUpdates(callback: () => void): Promise<() => void> {
    try {
      const unlisten = await listen('update-not-available', () => {
        console.info('最新バージョンです');
        callback();
      });
      return unlisten;
    } catch (error) {
      console.error('アップデート不要通知リスナー設定エラー:', error);
      throw new Error(
        `アップデート不要通知の設定に失敗しました: ${String(error)}`
      );
    }
  }

  /**
   * アップデートチェックエラーイベントをリッスン
   * @param callback エラーが発生したときのコールバック
   */
  static async listenForUpdateErrors(
    callback: (error: string) => void
  ): Promise<() => void> {
    try {
      const unlisten = await listen<string>('update-check-error', (event) => {
        console.error('アップデートチェックエラー:', event.payload);
        callback(event.payload);
      });
      return unlisten;
    } catch (error) {
      console.error('アップデートエラーリスナー設定エラー:', error);
      throw new Error(
        `アップデートエラー通知の設定に失敗しました: ${String(error)}`
      );
    }
  }

  /**
   * バージョン文字列を比較（改善版）
   * セマンティックバージョニング（major.minor.patch）に対応
   * @param version1 バージョン1
   * @param version2 バージョン2
   * @returns version1 > version2 なら 1、version1 < version2 なら -1、同じなら 0
   */
  static compareVersions(version1: string, version2: string): number {
    // "v"プレフィックスを削除
    const v1 = version1.replace(/^v/, '');
    const v2 = version2.replace(/^v/, '');

    // プレリリース情報を分離（例: 1.0.0-beta.1）
    const [v1Base, v1Pre] = v1.split('-');
    const [v2Base, v2Pre] = v2.split('-');

    // ベースバージョンを比較
    const v1Parts = v1Base.split('.').map(Number);
    const v2Parts = v2Base.split('.').map(Number);

    const maxLength = Math.max(v1Parts.length, v2Parts.length);

    for (let i = 0; i < maxLength; i++) {
      const v1Part = v1Parts[i] || 0;
      const v2Part = v2Parts[i] || 0;

      if (v1Part > v2Part) return 1;
      if (v1Part < v2Part) return -1;
    }

    // ベースバージョンが同じ場合、プレリリース情報を比較
    // プレリリースがない方が新しい（1.0.0 > 1.0.0-beta）
    if (!v1Pre && v2Pre) return 1;
    if (v1Pre && !v2Pre) return -1;
    if (v1Pre && v2Pre) {
      return v1Pre.localeCompare(v2Pre);
    }

    return 0;
  }

  /**
   * ファイルサイズを人間が読みやすい形式に変換
   * @param bytes バイト数
   * @returns フォーマットされたサイズ文字列
   */
  static formatFileSize(bytes: number): string {
    const units = ['B', 'KB', 'MB', 'GB'];
    let size = bytes;
    let unitIndex = 0;

    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }

    return `${size.toFixed(1)} ${units[unitIndex]}`;
  }

  /**
   * 日時をフォーマット（JST）
   * @param timestamp Unix timestamp
   * @returns フォーマットされた日時文字列
   */
  static formatTimestamp(timestamp: number): string {
    const date = new Date(timestamp * 1000);
    return date.toLocaleString('ja-JP', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      timeZone: 'Asia/Tokyo',
    });
  }
}

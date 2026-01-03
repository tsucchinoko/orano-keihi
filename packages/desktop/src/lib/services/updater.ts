import type { UpdateInfo } from '$lib/types/updater';
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
      throw new Error(`アップデートチェックに失敗しました: ${error}`);
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
      throw new Error(`アップデートのインストールに失敗しました: ${error}`);
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
      throw new Error(`バージョンの取得に失敗しました: ${error}`);
    }
  }

  /**
   * 自動アップデートチェックを開始
   * @param intervalHours チェック間隔（時間）、デフォルトは24時間
   */
  static async startAutoUpdateCheck(intervalHours: number = 24): Promise<void> {
    try {
      await invoke<void>('start_auto_update_check', { intervalHours });
    } catch (error) {
      console.error('自動アップデートチェック開始エラー:', error);
      throw new Error(`自動アップデートチェックの開始に失敗しました: ${error}`);
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
        console.log('アップデートが利用可能です:', event.payload);
        callback(event.payload);
      });
      return unlisten;
    } catch (error) {
      console.error('アップデート通知リスナー設定エラー:', error);
      throw new Error(`アップデート通知の設定に失敗しました: ${error}`);
    }
  }

  /**
   * バージョン文字列を比較
   * @param version1 バージョン1
   * @param version2 バージョン2
   * @returns version1 > version2 なら 1、version1 < version2 なら -1、同じなら 0
   */
  static compareVersions(version1: string, version2: string): number {
    const v1Parts = version1.split('.').map(Number);
    const v2Parts = version2.split('.').map(Number);

    const maxLength = Math.max(v1Parts.length, v2Parts.length);

    for (let i = 0; i < maxLength; i++) {
      const v1Part = v1Parts[i] || 0;
      const v2Part = v2Parts[i] || 0;

      if (v1Part > v2Part) return 1;
      if (v1Part < v2Part) return -1;
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
   * 日時をフォーマット
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
    });
  }
}

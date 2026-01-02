/**
 * APIサーバー状態管理ストア
 */

import type { HealthCheckResult, SyncResult } from '$lib/types/api-client';
import {
  checkApiServerHealthDetailed,
  syncFallbackFiles,
  getFallbackFileCount,
} from '$lib/types/api-client';

export interface ApiServerState {
  isHealthy: boolean;
  isChecking: boolean;
  lastCheckTime: Date | null;
  healthResult: HealthCheckResult | null;
  fallbackFileCount: number;
  isSyncing: boolean;
  lastSyncResult: SyncResult | null;
  autoCheckEnabled: boolean;
}

class ApiServerStore {
  private state = $state<ApiServerState>({
    isHealthy: false,
    isChecking: false,
    lastCheckTime: null,
    healthResult: null,
    fallbackFileCount: 0,
    isSyncing: false,
    lastSyncResult: null,
    autoCheckEnabled: true,
  });

  private checkInterval: number | null = null;

  constructor() {
    // 初回チェック
    this.checkHealth();

    // 定期的なヘルスチェック（30秒間隔）
    this.startAutoCheck();
  }

  /**
   * 現在の状態を取得
   */
  get currentState() {
    return this.state;
  }

  /**
   * APIサーバーのヘルスチェックを実行
   */
  async checkHealth(): Promise<void> {
    if (this.state.isChecking) return;

    this.state.isChecking = true;

    try {
      const healthResult = await checkApiServerHealthDetailed();

      this.state.healthResult = healthResult;
      this.state.isHealthy = healthResult.is_healthy;
      this.state.lastCheckTime = new Date();

      // ヘルスチェック成功時にフォールバックファイル数も更新
      if (healthResult.is_healthy) {
        await this.updateFallbackFileCount();
      }

      console.info('APIサーバーヘルスチェック完了:', {
        isHealthy: healthResult.is_healthy,
        responseTime: healthResult.response_time_ms,
        statusCode: healthResult.status_code,
      });
    } catch (error) {
      console.error('APIサーバーヘルスチェックエラー:', error);

      this.state.isHealthy = false;
      this.state.healthResult = {
        is_healthy: false,
        response_time_ms: 0,
        status_code: 0,
        error_message: String(error),
      };
      this.state.lastCheckTime = new Date();
    } finally {
      this.state.isChecking = false;
    }
  }

  /**
   * フォールバック状態のファイル数を更新
   */
  async updateFallbackFileCount(): Promise<void> {
    try {
      const count = await getFallbackFileCount();
      this.state.fallbackFileCount = count;
    } catch (error) {
      console.error('フォールバックファイル数取得エラー:', error);
      this.state.fallbackFileCount = 0;
    }
  }

  /**
   * フォールバック状態のファイルを同期
   */
  async syncFallbackFiles(): Promise<SyncResult | null> {
    if (this.state.isSyncing) return null;

    this.state.isSyncing = true;

    try {
      const syncResult = await syncFallbackFiles();

      this.state.lastSyncResult = syncResult;

      // 同期後にフォールバックファイル数を更新
      await this.updateFallbackFileCount();

      console.info('フォールバックファイル同期完了:', {
        totalFiles: syncResult.total_files,
        successfulSyncs: syncResult.successful_syncs,
        failedSyncs: syncResult.failed_syncs,
      });

      return syncResult;
    } catch (error) {
      console.error('フォールバックファイル同期エラー:', error);
      return null;
    } finally {
      this.state.isSyncing = false;
    }
  }

  /**
   * 自動ヘルスチェックを開始
   */
  startAutoCheck(): void {
    if (this.checkInterval) return;

    this.state.autoCheckEnabled = true;
    this.checkInterval = window.setInterval(() => {
      if (this.state.autoCheckEnabled) {
        this.checkHealth();
      }
    }, 30000); // 30秒間隔
  }

  /**
   * 自動ヘルスチェックを停止
   */
  stopAutoCheck(): void {
    if (this.checkInterval) {
      clearInterval(this.checkInterval);
      this.checkInterval = null;
    }
    this.state.autoCheckEnabled = false;
  }

  /**
   * 自動チェックの有効/無効を切り替え
   */
  toggleAutoCheck(): void {
    if (this.state.autoCheckEnabled) {
      this.stopAutoCheck();
    } else {
      this.startAutoCheck();
    }
  }

  /**
   * APIサーバーが利用可能かどうか
   */
  get isAvailable(): boolean {
    return this.state.isHealthy;
  }

  /**
   * フォールバック状態のファイルがあるかどうか
   */
  get hasFallbackFiles(): boolean {
    return this.state.fallbackFileCount > 0;
  }

  /**
   * 同期が推奨される状態かどうか
   */
  get shouldSync(): boolean {
    return this.state.isHealthy && this.state.fallbackFileCount > 0;
  }

  /**
   * ストアを破棄
   */
  destroy(): void {
    this.stopAutoCheck();
  }
}

// シングルトンインスタンスをエクスポート
export const apiServerStore = new ApiServerStore();

// ページ離脱時にクリーンアップ
if (typeof window !== 'undefined') {
  window.addEventListener('beforeunload', () => {
    apiServerStore.destroy();
  });
}

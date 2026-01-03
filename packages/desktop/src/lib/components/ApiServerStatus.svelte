<script lang="ts">
/**
 * APIサーバー状態表示コンポーネント
 * APIサーバーの接続状態とフォールバック機能を管理
 */

import { apiServerStore } from '$lib/stores/api-server.svelte';
import { toastStore } from '$lib/stores/toast.svelte';

const state = $derived(apiServerStore.currentState);

// 状態に応じたスタイルクラス
const statusClass = $derived(() => {
  if (state.isHealthy) {
    return 'bg-green-100 text-green-800 border-green-200';
  } else if (state.isChecking) {
    return 'bg-yellow-100 text-yellow-800 border-yellow-200';
  } else {
    return 'bg-red-100 text-red-800 border-red-200';
  }
});

// 状態アイコン
const statusIcon = $derived(() => {
  if (state.isHealthy) {
    return '✓';
  } else if (state.isChecking) {
    return '⟳';
  } else {
    return '✕';
  }
});

// 状態テキスト
const statusText = $derived(() => {
  if (state.isChecking) {
    return 'チェック中...';
  } else if (state.isHealthy) {
    return 'オンライン';
  } else {
    return 'オフライン';
  }
});

// 同期ボタンの処理
async function handleSync() {
  try {
    const result = await apiServerStore.syncFallbackFiles();
    
    if (result) {
      if (result.successful_syncs > 0) {
        toastStore.success(
          `${result.successful_syncs}個のファイルを同期しました`
        );
      } else if (result.total_files === 0) {
        toastStore.info('同期が必要なファイルはありません');
      } else {
        toastStore.warning(
          `同期に失敗しました（${result.failed_syncs}個のファイル）`
        );
      }
    } else {
      toastStore.error('同期処理に失敗しました');
    }
  } catch (error) {
    toastStore.error(`同期エラー: ${error}`);
  }
}

// ヘルスチェックボタンの処理
async function handleHealthCheck() {
  try {
    await apiServerStore.checkHealth();
    
    if (state.isHealthy) {
      toastStore.success('APIサーバーは正常に動作しています');
    } else {
      toastStore.warning(
        `APIサーバーに問題があります: ${state.healthResult?.error_message || '不明なエラー'}`
      );
    }
  } catch (error) {
    toastStore.error(`ヘルスチェックエラー: ${error}`);
  }
}

// 自動チェック切り替えの処理
function handleToggleAutoCheck() {
  apiServerStore.toggleAutoCheck();
  
  if (state.autoCheckEnabled) {
    toastStore.info('自動ヘルスチェックを有効にしました');
  } else {
    toastStore.info('自動ヘルスチェックを無効にしました');
  }
}
</script>

<div class="bg-white rounded-lg shadow-sm border p-4 space-y-4">
  <!-- APIサーバー状態表示 -->
  <div class="flex items-center justify-between">
    <div class="flex items-center space-x-3">
      <div class="flex items-center space-x-2">
        <div class="w-3 h-3 rounded-full {state.isHealthy ? 'bg-green-500' : 'bg-red-500'}"></div>
        <span class="font-medium text-gray-900">APIサーバー</span>
      </div>
      
      <div class="px-2 py-1 rounded-full text-xs font-medium border {statusClass}">
        <span class="mr-1">{statusIcon}</span>
        {statusText}
      </div>
    </div>

    <div class="flex items-center space-x-2">
      <!-- ヘルスチェックボタン -->
      <button
        onclick={handleHealthCheck}
        disabled={state.isChecking}
        class="px-3 py-1 text-xs bg-blue-50 text-blue-700 rounded-md hover:bg-blue-100 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {state.isChecking ? 'チェック中...' : 'チェック'}
      </button>

      <!-- 自動チェック切り替えボタン -->
      <button
        onclick={handleToggleAutoCheck}
        class="px-3 py-1 text-xs rounded-md transition-colors {state.autoCheckEnabled 
          ? 'bg-green-50 text-green-700 hover:bg-green-100' 
          : 'bg-gray-50 text-gray-700 hover:bg-gray-100'}"
      >
        自動チェック: {state.autoCheckEnabled ? 'ON' : 'OFF'}
      </button>
    </div>
  </div>

  <!-- 詳細情報 -->
  {#if state.healthResult}
    <div class="text-xs text-gray-600 space-y-1">
      {#if state.isHealthy}
        <div class="flex justify-between">
          <span>応答時間:</span>
          <span class="font-mono">{state.healthResult.response_time_ms}ms</span>
        </div>
        <div class="flex justify-between">
          <span>ステータス:</span>
          <span class="font-mono">HTTP {state.healthResult.status_code}</span>
        </div>
      {:else}
        <div class="text-red-600">
          <span>エラー: {state.healthResult.error_message || '不明なエラー'}</span>
        </div>
      {/if}
      
      {#if state.lastCheckTime}
        <div class="flex justify-between">
          <span>最終チェック:</span>
          <span class="font-mono">{state.lastCheckTime.toLocaleTimeString('ja-JP')}</span>
        </div>
      {/if}
    </div>
  {/if}

  <!-- フォールバック状態のファイル情報 -->
  {#if state.fallbackFileCount > 0}
    <div class="border-t pt-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center space-x-2">
          <div class="w-2 h-2 bg-orange-500 rounded-full"></div>
          <span class="text-sm font-medium text-gray-900">
            未同期ファイル: {state.fallbackFileCount}個
          </span>
        </div>

        <button
          onclick={handleSync}
          disabled={state.isSyncing || !state.isHealthy}
          class="px-3 py-1 text-xs bg-orange-50 text-orange-700 rounded-md hover:bg-orange-100 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {state.isSyncing ? '同期中...' : '同期'}
        </button>
      </div>

      {#if !state.isHealthy}
        <p class="text-xs text-gray-500 mt-2">
          APIサーバーがオンラインになったら同期ボタンを押してください
        </p>
      {/if}
    </div>
  {/if}

  <!-- 最後の同期結果 -->
  {#if state.lastSyncResult}
    <div class="border-t pt-4">
      <div class="text-xs text-gray-600">
        <div class="font-medium mb-1">最後の同期結果:</div>
        <div class="space-y-1">
          <div class="flex justify-between">
            <span>総ファイル数:</span>
            <span>{state.lastSyncResult.total_files}</span>
          </div>
          <div class="flex justify-between">
            <span>成功:</span>
            <span class="text-green-600">{state.lastSyncResult.successful_syncs}</span>
          </div>
          {#if state.lastSyncResult.failed_syncs > 0}
            <div class="flex justify-between">
              <span>失敗:</span>
              <span class="text-red-600">{state.lastSyncResult.failed_syncs}</span>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
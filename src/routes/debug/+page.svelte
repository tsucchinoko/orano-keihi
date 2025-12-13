<script lang="ts">
	import { onMount } from 'svelte';
	import { 
		testR2ConnectionDetailed, 
		getR2UsageMonitoring, 
		getR2DebugInfo,
		getR2PerformanceStats
	} from '$lib/utils/tauri';
	import type { 
		R2ConnectionTestResult, 
		R2UsageInfo, 
		R2DebugInfo,
		PerformanceStats
	} from '$lib/types';

	let connectionTestResult: R2ConnectionTestResult | null = null;
	let usageInfo: R2UsageInfo | null = null;
	let debugInfo: R2DebugInfo | null = null;
	let performanceStats: PerformanceStats | null = null;
	let isLoading = false;
	let error: string | null = null;

	async function runConnectionTest() {
		isLoading = true;
		error = null;
		
		try {
			const result = await testR2ConnectionDetailed();
			if (result.error) {
				error = result.error;
			} else {
				connectionTestResult = result.data || null;
			}
		} catch (e) {
			error = `接続テストエラー: ${e}`;
		} finally {
			isLoading = false;
		}
	}

	async function loadUsageInfo() {
		isLoading = true;
		error = null;
		
		try {
			const result = await getR2UsageMonitoring();
			if (result.error) {
				error = result.error;
			} else {
				usageInfo = result.data || null;
			}
		} catch (e) {
			error = `使用量情報取得エラー: ${e}`;
		} finally {
			isLoading = false;
		}
	}

	async function loadDebugInfo() {
		isLoading = true;
		error = null;
		
		try {
			const result = await getR2DebugInfo();
			if (result.error) {
				error = result.error;
			} else {
				debugInfo = result.data || null;
			}
		} catch (e) {
			error = `デバッグ情報取得エラー: ${e}`;
		} finally {
			isLoading = false;
		}
	}

	async function loadPerformanceStats() {
		isLoading = true;
		error = null;
		
		try {
			const result = await getR2PerformanceStats();
			if (result.error) {
				error = result.error;
			} else {
				performanceStats = result.data || null;
			}
		} catch (e) {
			error = `パフォーマンス統計取得エラー: ${e}`;
		} finally {
			isLoading = false;
		}
	}

	function formatBytes(bytes: number): string {
		if (bytes === 0) return '0 Bytes';
		const k = 1024;
		const sizes = ['Bytes', 'KB', 'MB', 'GB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
	}

	function formatDuration(ms: number): string {
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}

	onMount(() => {
		// ページ読み込み時にパフォーマンス統計を自動取得
		loadPerformanceStats();
	});
</script>

<svelte:head>
	<title>R2デバッグ・統合テスト</title>
</svelte:head>

<div class="container mx-auto p-6 max-w-6xl">
	<h1 class="text-3xl font-bold mb-6">R2デバッグ・統合テスト</h1>

	{#if error}
		<div class="alert alert-error mb-6">
			<svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
			</svg>
			<span>{error}</span>
		</div>
	{/if}

	<!-- アクションボタン -->
	<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
		<button 
			class="btn btn-primary" 
			class:loading={isLoading}
			on:click={runConnectionTest}
			disabled={isLoading}
		>
			接続テスト実行
		</button>
		
		<button 
			class="btn btn-secondary" 
			class:loading={isLoading}
			on:click={loadUsageInfo}
			disabled={isLoading}
		>
			使用量情報取得
		</button>
		
		<button 
			class="btn btn-accent" 
			class:loading={isLoading}
			on:click={loadDebugInfo}
			disabled={isLoading}
		>
			デバッグ情報取得
		</button>
		
		<button 
			class="btn btn-info" 
			class:loading={isLoading}
			on:click={loadPerformanceStats}
			disabled={isLoading}
		>
			パフォーマンス統計
		</button>
	</div>

	<!-- パフォーマンス統計 -->
	{#if performanceStats}
		<div class="card bg-base-100 shadow-xl mb-6">
			<div class="card-body">
				<h2 class="card-title">パフォーマンス統計</h2>
				<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
					<div class="stat">
						<div class="stat-title">レイテンシ</div>
						<div class="stat-value text-primary">{performanceStats.latency_ms}ms</div>
					</div>
					<div class="stat">
						<div class="stat-title">スループット</div>
						<div class="stat-value text-secondary">{formatBytes(performanceStats.throughput_bps)}/s</div>
					</div>
					<div class="stat">
						<div class="stat-title">接続状態</div>
						<div class="stat-value text-accent">{performanceStats.connection_status}</div>
					</div>
				</div>
				<div class="text-sm text-gray-500 mt-2">
					最終測定: {new Date(performanceStats.last_measured).toLocaleString('ja-JP')}
				</div>
			</div>
		</div>
	{/if}

	<!-- 接続テスト結果 -->
	{#if connectionTestResult}
		<div class="card bg-base-100 shadow-xl mb-6">
			<div class="card-body">
				<h2 class="card-title">
					R2接続詳細テスト結果
					<div class="badge" class:badge-success={connectionTestResult.overall_success} class:badge-error={!connectionTestResult.overall_success}>
						{connectionTestResult.overall_success ? '成功' : '失敗'}
					</div>
				</h2>
				
				<div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
					<!-- 設定検証 -->
					<div class="card bg-base-200">
						<div class="card-body p-4">
							<h3 class="font-semibold flex items-center gap-2">
								設定検証
								<div class="badge" class:badge-success={connectionTestResult.config_validation.success} class:badge-error={!connectionTestResult.config_validation.success}>
									{connectionTestResult.config_validation.success ? '成功' : '失敗'}
								</div>
							</h3>
							<p class="text-sm">{connectionTestResult.config_validation.message}</p>
							<p class="text-xs text-gray-500">{formatDuration(connectionTestResult.config_validation.duration_ms)}</p>
							{#if connectionTestResult.config_validation.details}
								<p class="text-xs mt-1">{connectionTestResult.config_validation.details}</p>
							{/if}
						</div>
					</div>

					<!-- クライアント初期化 -->
					<div class="card bg-base-200">
						<div class="card-body p-4">
							<h3 class="font-semibold flex items-center gap-2">
								クライアント初期化
								<div class="badge" class:badge-success={connectionTestResult.client_initialization.success} class:badge-error={!connectionTestResult.client_initialization.success}>
									{connectionTestResult.client_initialization.success ? '成功' : '失敗'}
								</div>
							</h3>
							<p class="text-sm">{connectionTestResult.client_initialization.message}</p>
							<p class="text-xs text-gray-500">{formatDuration(connectionTestResult.client_initialization.duration_ms)}</p>
						</div>
					</div>

					<!-- バケットアクセス -->
					<div class="card bg-base-200">
						<div class="card-body p-4">
							<h3 class="font-semibold flex items-center gap-2">
								バケットアクセス
								<div class="badge" class:badge-success={connectionTestResult.bucket_access.success} class:badge-error={!connectionTestResult.bucket_access.success}>
									{connectionTestResult.bucket_access.success ? '成功' : '失敗'}
								</div>
							</h3>
							<p class="text-sm">{connectionTestResult.bucket_access.message}</p>
							<p class="text-xs text-gray-500">{formatDuration(connectionTestResult.bucket_access.duration_ms)}</p>
						</div>
					</div>

					<!-- アップロードテスト -->
					<div class="card bg-base-200">
						<div class="card-body p-4">
							<h3 class="font-semibold flex items-center gap-2">
								アップロードテスト
								<div class="badge" class:badge-success={connectionTestResult.upload_test.success} class:badge-error={!connectionTestResult.upload_test.success}>
									{connectionTestResult.upload_test.success ? '成功' : '失敗'}
								</div>
							</h3>
							<p class="text-sm">{connectionTestResult.upload_test.message}</p>
							<p class="text-xs text-gray-500">{formatDuration(connectionTestResult.upload_test.duration_ms)}</p>
						</div>
					</div>

					<!-- ダウンロードテスト -->
					<div class="card bg-base-200">
						<div class="card-body p-4">
							<h3 class="font-semibold flex items-center gap-2">
								ダウンロードテスト
								<div class="badge" class:badge-success={connectionTestResult.download_test.success} class:badge-error={!connectionTestResult.download_test.success}>
									{connectionTestResult.download_test.success ? '成功' : '失敗'}
								</div>
							</h3>
							<p class="text-sm">{connectionTestResult.download_test.message}</p>
							<p class="text-xs text-gray-500">{formatDuration(connectionTestResult.download_test.duration_ms)}</p>
						</div>
					</div>

					<!-- 削除テスト -->
					<div class="card bg-base-200">
						<div class="card-body p-4">
							<h3 class="font-semibold flex items-center gap-2">
								削除テスト
								<div class="badge" class:badge-success={connectionTestResult.delete_test.success} class:badge-error={!connectionTestResult.delete_test.success}>
									{connectionTestResult.delete_test.success ? '成功' : '失敗'}
								</div>
							</h3>
							<p class="text-sm">{connectionTestResult.delete_test.message}</p>
							<p class="text-xs text-gray-500">{formatDuration(connectionTestResult.delete_test.duration_ms)}</p>
						</div>
					</div>
				</div>

				<div class="mt-4 text-sm text-gray-500">
					総実行時間: {formatDuration(connectionTestResult.total_duration_ms)} | 環境: {connectionTestResult.environment}
				</div>
			</div>
		</div>
	{/if}

	<!-- 使用量情報 -->
	{#if usageInfo}
		<div class="card bg-base-100 shadow-xl mb-6">
			<div class="card-body">
				<h2 class="card-title">R2使用量監視情報</h2>
				
				<div class="grid grid-cols-1 md:grid-cols-4 gap-4">
					<div class="stat">
						<div class="stat-title">総ファイル数</div>
						<div class="stat-value text-primary">{usageInfo.total_files.toLocaleString()}</div>
					</div>
					<div class="stat">
						<div class="stat-title">推定ストレージ</div>
						<div class="stat-value text-secondary">{formatBytes(usageInfo.estimated_storage_mb * 1024 * 1024)}</div>
					</div>
					<div class="stat">
						<div class="stat-title">今月のアップロード</div>
						<div class="stat-value text-accent">{usageInfo.monthly_uploads}</div>
					</div>
					<div class="stat">
						<div class="stat-title">今日のアップロード</div>
						<div class="stat-value text-info">{usageInfo.daily_uploads}</div>
					</div>
				</div>

				<div class="mt-4">
					<div class="flex justify-between items-center mb-2">
						<span class="font-semibold">推定月額コスト</span>
						<span class="text-lg font-bold">${usageInfo.cost_estimate_usd.toFixed(4)} USD</span>
					</div>
					<div class="text-sm text-gray-500">
						バケット: {usageInfo.bucket_name} | リージョン: {usageInfo.region}
					</div>
					<div class="text-xs text-gray-500 mt-1">
						最終更新: {new Date(usageInfo.last_updated).toLocaleString('ja-JP')}
					</div>
				</div>

				{#if usageInfo.cache_stats}
					<div class="divider">キャッシュ統計</div>
					<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
						<div class="stat">
							<div class="stat-title">キャッシュファイル数</div>
							<div class="stat-value text-sm">{usageInfo.cache_stats.total_files}</div>
						</div>
						<div class="stat">
							<div class="stat-title">キャッシュサイズ</div>
							<div class="stat-value text-sm">{formatBytes(usageInfo.cache_stats.total_size_bytes)}</div>
						</div>
						<div class="stat">
							<div class="stat-title">最大サイズ</div>
							<div class="stat-value text-sm">{formatBytes(usageInfo.cache_stats.max_size_bytes)}</div>
						</div>
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<!-- デバッグ情報 -->
	{#if debugInfo}
		<div class="card bg-base-100 shadow-xl mb-6">
			<div class="card-body">
				<h2 class="card-title">R2デバッグ情報</h2>
				
				<div class="tabs tabs-boxed mb-4">
					<input type="radio" name="debug_tabs" class="tab" aria-label="環境変数" checked />
					<div class="tab-content bg-base-100 border-base-300 rounded-box p-6">
						<h3 class="font-semibold mb-2">環境変数</h3>
						<div class="overflow-x-auto">
							<table class="table table-sm">
								<tbody>
									{#each Object.entries(debugInfo.environment_variables) as [key, value]}
										<tr>
											<td class="font-mono text-sm">{key}</td>
											<td class="font-mono text-sm">{value}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					</div>

					<input type="radio" name="debug_tabs" class="tab" aria-label="システム情報" />
					<div class="tab-content bg-base-100 border-base-300 rounded-box p-6">
						<h3 class="font-semibold mb-2">システム情報</h3>
						<div class="overflow-x-auto">
							<table class="table table-sm">
								<tbody>
									{#each Object.entries(debugInfo.system_info) as [key, value]}
										<tr>
											<td class="font-mono text-sm">{key}</td>
											<td class="font-mono text-sm">{value}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					</div>

					<input type="radio" name="debug_tabs" class="tab" aria-label="DB統計" />
					<div class="tab-content bg-base-100 border-base-300 rounded-box p-6">
						<h3 class="font-semibold mb-2">データベース統計</h3>
						<div class="overflow-x-auto">
							<table class="table table-sm">
								<tbody>
									{#each Object.entries(debugInfo.database_stats) as [key, value]}
										<tr>
											<td class="font-mono text-sm">{key}</td>
											<td class="font-mono text-sm">{value}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					</div>

					<input type="radio" name="debug_tabs" class="tab" aria-label="エラーログ" />
					<div class="tab-content bg-base-100 border-base-300 rounded-box p-6">
						<h3 class="font-semibold mb-2">最近のエラーログ</h3>
						<div class="space-y-2">
							{#each debugInfo.recent_errors as error}
								<div class="alert alert-warning alert-sm">
									<span class="font-mono text-xs">{error}</span>
								</div>
							{/each}
						</div>
					</div>
				</div>

				<div class="text-xs text-gray-500 mt-4">
					生成時刻: {new Date(debugInfo.timestamp).toLocaleString('ja-JP')}
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.tab-content {
		display: none;
	}
	
	.tab:checked + .tab-content {
		display: block;
	}
</style>
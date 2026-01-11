<script lang="ts">
	import { onMount } from 'svelte';
	import { UpdaterService } from '$lib/services/updater';
	import type { UpdateInfo, UpdateNotificationState } from '$lib/types/updater';

	// アップデート通知の状態管理
	let updateState = $state<UpdateNotificationState>({
		show: false,
		downloading: false,
		progress: 0
	});

	// アップデート通知を表示する関数
	function showUpdateNotification(updateInfo: UpdateInfo) {
		updateState = {
			...updateState,
			show: true,
			updateInfo,
			error: undefined
		};
	}

	// アップデート通知を非表示にする関数
	function hideUpdateNotification() {
		updateState = {
			...updateState,
			show: false,
			downloading: false,
			progress: 0,
			error: undefined
		};
	}

	// アップデートをダウンロードしてインストールする関数
	async function handleUpdateInstall() {
		if (!updateState.updateInfo) return;

		updateState = {
			...updateState,
			downloading: true,
			progress: 0,
			error: undefined
		};

		try {
			// ダウンロード進捗のシミュレーション（実際のTauri updaterは進捗イベントを提供）
			const progressInterval = setInterval(() => {
				updateState = {
					...updateState,
					progress: Math.min(updateState.progress + 10, 90)
				};
			}, 500);

			await UpdaterService.downloadAndInstall();
			
			clearInterval(progressInterval);
			updateState = {
				...updateState,
				progress: 100
			};
			
			// インストール成功後はアプリケーションが再起動されるため、ここには到達しない
		} catch (error) {
			console.error('アップデートインストールエラー:', error);
			updateState = {
				...updateState,
				downloading: false,
				progress: 0,
				error: error instanceof Error ? error.message : 'アップデートに失敗しました'
			};
		}
	}

	// 後でアップデートする（通知を非表示にする）
	function handleUpdateLater() {
		hideUpdateNotification();
	}

	// このバージョンをスキップする
	async function handleSkipVersion() {
		if (!updateState.updateInfo?.latest_version) return;

		try {
			await UpdaterService.skipVersion(updateState.updateInfo.latest_version);
			hideUpdateNotification();
		} catch (error) {
			console.error('バージョンスキップエラー:', error);
			updateState = {
				...updateState,
				error: error instanceof Error ? error.message : 'バージョンのスキップに失敗しました'
			};
		}
	}

	// エラーをクリアして再試行
	function handleRetry() {
		updateState = {
			...updateState,
			error: undefined,
			downloading: false,
			progress: 0
		};
	}

	// コンポーネントマウント時の処理
	onMount(() => {
		let unlisten: (() => void) | undefined;

		const initializeUpdater = async () => {
			try {
				// アップデート通知イベントをリッスン
				unlisten = await UpdaterService.listenForUpdates(showUpdateNotification);

				// 自動アップデートチェックを開始
				await UpdaterService.startAutoUpdateCheck();

				// 初回アップデートチェック
				const updateInfo = await UpdaterService.checkForUpdates();
				if (updateInfo.available) {
					showUpdateNotification(updateInfo);
				}
			} catch (error) {
				console.error('アップデート機能の初期化エラー:', error);
			}
		};

		initializeUpdater();

		// クリーンアップ関数を返す
		return () => {
			if (unlisten) {
				unlisten();
			}
		};
	});
</script>

<!-- アップデート通知モーダル -->
{#if updateState.show && updateState.updateInfo}
	<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
		<div class="bg-white rounded-lg shadow-xl max-w-md w-full mx-4 p-6">
			<!-- ヘッダー -->
			<div class="flex items-center mb-4">
				<div class="w-12 h-12 bg-blue-100 rounded-full flex items-center justify-center mr-4">
					<svg class="w-6 h-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>
					</svg>
				</div>
				<div>
					<h3 class="text-lg font-semibold text-gray-900">アップデートが利用可能です</h3>
					<p class="text-sm text-gray-600">
						バージョン {updateState.updateInfo.latest_version} が利用可能です
					</p>
				</div>
			</div>

			<!-- バージョン情報 -->
			<div class="mb-4 p-3 bg-gray-50 rounded-lg">
				<div class="flex justify-between text-sm">
					<span class="text-gray-600">現在のバージョン:</span>
					<span class="font-medium">{updateState.updateInfo.current_version}</span>
				</div>
				<div class="flex justify-between text-sm mt-1">
					<span class="text-gray-600">最新バージョン:</span>
					<span class="font-medium text-blue-600">{updateState.updateInfo.latest_version}</span>
				</div>
				{#if updateState.updateInfo.content_length}
					<div class="flex justify-between text-sm mt-1">
						<span class="text-gray-600">ダウンロードサイズ:</span>
						<span class="font-medium">{UpdaterService.formatFileSize(updateState.updateInfo.content_length)}</span>
					</div>
				{/if}
			</div>

			<!-- リリースノート -->
			{#if updateState.updateInfo.release_notes}
				<div class="mb-4">
					<h4 class="text-sm font-medium text-gray-900 mb-2">更新内容:</h4>
					<div class="text-sm text-gray-700 bg-gray-50 p-3 rounded-lg max-h-32 overflow-y-auto whitespace-pre-wrap">
						{updateState.updateInfo.release_notes}
					</div>
				</div>
			{/if}

			<!-- エラーメッセージ -->
			{#if updateState.error}
				<div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg">
					<div class="flex items-start">
						<svg class="w-5 h-5 text-red-600 mr-2 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
						</svg>
						<div class="flex-1">
							<p class="text-sm text-red-700 font-medium mb-1">エラーが発生しました</p>
							<p class="text-sm text-red-600">{updateState.error}</p>
						</div>
					</div>
					<button
						onclick={handleRetry}
						class="mt-2 w-full bg-red-100 text-red-700 px-3 py-1.5 rounded text-sm hover:bg-red-200 transition-colors font-medium"
					>
						再試行
					</button>
				</div>
			{/if}

			<!-- ダウンロード進捗 -->
			{#if updateState.downloading}
				<div class="mb-4">
					<div class="flex justify-between text-sm mb-2">
						<span class="text-gray-600">ダウンロード中...</span>
						<span class="font-medium">{updateState.progress.toFixed(1)}%</span>
					</div>
					<div class="w-full bg-gray-200 rounded-full h-2.5 overflow-hidden">
						<div 
							class="bg-blue-600 h-2.5 rounded-full transition-all duration-300 ease-out"
							style="width: {updateState.progress}%"
						></div>
					</div>
					<p class="text-xs text-gray-500 mt-1.5">
						ダウンロード完了後、アプリケーションが自動的に再起動されます
					</p>
				</div>
			{/if}

			<!-- アクションボタン -->
			<div class="flex flex-col space-y-2">
				{#if !updateState.downloading}
					<button
						onclick={handleUpdateInstall}
						disabled={!!updateState.error}
						class="w-full bg-blue-600 text-white px-4 py-2.5 rounded-lg hover:bg-blue-700 transition-colors font-medium disabled:bg-gray-400 disabled:cursor-not-allowed"
					>
						今すぐアップデート
					</button>
					<div class="flex space-x-2">
						<button
							onclick={handleUpdateLater}
							class="flex-1 bg-gray-200 text-gray-800 px-4 py-2 rounded-lg hover:bg-gray-300 transition-colors font-medium"
						>
							後で
						</button>
						<button
							onclick={handleSkipVersion}
							class="flex-1 bg-gray-100 text-gray-600 px-4 py-2 rounded-lg hover:bg-gray-200 transition-colors font-medium border border-gray-300"
						>
							スキップ
						</button>
					</div>
				{:else}
					<button
						disabled
						class="w-full bg-gray-400 text-white px-4 py-2.5 rounded-lg cursor-not-allowed font-medium"
					>
						ダウンロード中...
					</button>
				{/if}
			</div>

			<!-- 最終チェック時刻 -->
			<div class="mt-3 text-xs text-gray-500 text-center">
				最終チェック: {UpdaterService.formatTimestamp(updateState.updateInfo.last_checked)}
			</div>
		</div>
	</div>
{/if}
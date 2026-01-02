<script lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import { getReceiptFromR2, getReceiptOffline } from "$lib/utils/tauri";
import { toastStore } from "$lib/stores/toast.svelte";

// Props
interface Props {
	receiptUrl?: string;
	receiptPath?: string; // 後方互換性のため
	onClose: () => void;
}

let { receiptUrl, receiptPath, onClose }: Props = $props();

// ズームレベル
let zoomLevel = $state(100);

// ローディング状態
let isLoading = $state(false);
let loadError = $state<string | null>(null);

// ファイルデータ（Base64）
let fileData = $state<string | null>(null);

// オフラインモード状態
let isOfflineMode = $state(false);

// ファイルタイプ判定
const isPdf = $derived(() => {
	if (receiptUrl) {
		return receiptUrl.toLowerCase().includes(".pdf");
	}
	if (receiptPath) {
		return receiptPath.toLowerCase().endsWith(".pdf");
	}
	return false;
});

const isImage = $derived(() => {
	if (receiptUrl) {
		return /\.(png|jpg|jpeg)/i.test(receiptUrl);
	}
	if (receiptPath) {
		return /\.(png|jpg|jpeg)$/i.test(receiptPath);
	}
	return false;
});

// ファイルURL（ローカルファイル用）
const localFileUrl = $derived.by(() => {
	if (receiptPath) {
		return convertFileSrc(receiptPath);
	}
	return null;
});

// Base64データURL（R2ファイル用）
const dataUrl = $derived.by(() => {
	if (fileData && isImage()) {
		const mimeType = receiptUrl?.toLowerCase().includes(".png")
			? "image/png"
			: "image/jpeg";
		return `data:${mimeType};base64,${fileData}`;
	}
	return null;
});

// ネットワーク接続状態を確認する関数
async function checkNetworkConnection(): Promise<boolean> {
	try {
		// 簡単なネットワークチェック（実際のプロジェクトではより堅牢な方法を使用）
		return navigator.onLine;
	} catch {
		return false;
	}
}

// R2からファイルを取得（オンライン時）
async function loadFromR2() {
	if (!receiptUrl) return;

	isLoading = true;
	loadError = null;
	isOfflineMode = false;

	try {
		// ネットワーク接続を確認
		const isOnline = await checkNetworkConnection();
		if (!isOnline) {
			throw new Error("ネットワークに接続されていません");
		}

		const result = await getReceiptFromR2(receiptUrl);
		if (result.error) {
			throw new Error(result.error);
		}
		fileData = result.data || null;

		// 成功時にキャッシュ情報を表示
		if (fileData) {
			toastStore.success("領収書を読み込みました");
		}
	} catch (error) {
		console.error("R2からの領収書取得に失敗しました:", error);
		// オンライン取得に失敗した場合、オフラインキャッシュを試行
		await tryLoadFromCache();
	} finally {
		isLoading = false;
	}
}

// キャッシュからファイルを取得（オフライン時）
async function tryLoadFromCache() {
	if (!receiptUrl) return;

	try {
		const result = await getReceiptOffline(receiptUrl);
		if (result.error) {
			throw new Error(result.error);
		}
		fileData = result.data || null;
		isOfflineMode = true;
		loadError = null;

		// オフラインモードでの読み込み成功を通知
		if (fileData) {
			toastStore.info("オフラインキャッシュから領収書を表示しています");
		}
	} catch (error) {
		console.error("キャッシュからの領収書取得に失敗しました:", error);
		const errorMessage =
			error instanceof Error ? error.message : "不明なエラー";
		loadError = `領収書の取得に失敗しました: ${errorMessage}`;
		isOfflineMode = false;
	}
}

// 手動でオフラインモードを試行
async function tryOfflineMode() {
	if (!receiptUrl) return;

	isLoading = true;
	loadError = null;

	await tryLoadFromCache();

	isLoading = false;
}

// コンポーネント初期化時にR2からファイルを取得
$effect(() => {
	if (receiptUrl) {
		loadFromR2();
	}
});

// ズームイン
function zoomIn() {
	if (zoomLevel < 200) {
		zoomLevel += 25;
	}
}

// ズームアウト
function zoomOut() {
	if (zoomLevel > 50) {
		zoomLevel -= 25;
	}
}

// リセット
function resetZoom() {
	zoomLevel = 100;
}

// キーボードショートカット
function handleKeydown(event: KeyboardEvent) {
	if (event.key === "Escape") {
		onClose();
	} else if (event.key === "+" || event.key === "=") {
		event.preventDefault();
		zoomIn();
	} else if (event.key === "-") {
		event.preventDefault();
		zoomOut();
	} else if (event.key === "0") {
		event.preventDefault();
		resetZoom();
	} else if (event.key === "r" || event.key === "R") {
		event.preventDefault();
		if (receiptUrl) {
			loadFromR2();
		}
	} else if (event.key === "o" || event.key === "O") {
		event.preventDefault();
		if (receiptUrl && !isOfflineMode) {
			tryOfflineMode();
		}
	}
}

// 背景クリックで閉じる
function handleBackdropClick(event: MouseEvent) {
	if (event.target === event.currentTarget) {
		onClose();
	}
}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- モーダルオーバーレイ -->
<div
	class="fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-50 p-4"
	onclick={handleBackdropClick}
	onkeydown={(e) => e.key === 'Enter' && handleBackdropClick(e as any)}
	role="dialog"
	aria-modal="true"
	tabindex="-1"
>
	<!-- モーダルコンテンツ -->
	<div class="relative max-w-6xl max-h-[90vh] w-full bg-white rounded-lg shadow-2xl overflow-hidden">
		<!-- ヘッダー -->
		<div class="flex items-center justify-between p-4 border-b border-gray-200 bg-gradient-to-r from-purple-50 to-pink-50">
			<div class="flex items-center gap-3">
				<h3 class="text-xl font-bold">領収書</h3>
				{#if isOfflineMode}
					<span class="px-2 py-1 text-xs font-semibold bg-orange-100 text-orange-800 rounded-full">
						📱 オフライン
					</span>
				{/if}
				{#if isLoading}
					<span class="px-2 py-1 text-xs font-semibold bg-blue-100 text-blue-800 rounded-full">
						🔄 読み込み中
					</span>
				{/if}
			</div>
			
			<!-- キーボードショートカットヒント -->
			<div class="hidden md:block text-xs text-gray-500 mr-4">
				ESC: 閉じる | +/-: ズーム | 0: リセット | R: 再読み込み
			</div>
			
			<!-- コントロールボタン -->
			<div class="flex items-center gap-2">
				{#if isImage()}
					<div class="flex items-center gap-2 mr-4">
						<button
							type="button"
							onclick={zoomOut}
							class="btn btn-info text-sm px-3 py-1"
							disabled={zoomLevel <= 50}
							title="ズームアウト"
						>
							🔍−
						</button>
						<span class="text-sm font-semibold min-w-16 text-center">
							{zoomLevel}%
						</span>
						<button
							type="button"
							onclick={zoomIn}
							class="btn btn-info text-sm px-3 py-1"
							disabled={zoomLevel >= 200}
							title="ズームイン"
						>
							🔍+
						</button>
						<button
							type="button"
							onclick={resetZoom}
							class="btn bg-gray-300 text-gray-700 text-sm px-3 py-1"
							title="リセット"
						>
							リセット
						</button>
					</div>
				{/if}
				
				<button
					type="button"
					onclick={onClose}
					class="btn bg-red-500 hover:bg-red-600 text-white px-4 py-2"
					title="閉じる"
				>
					✕ 閉じる
				</button>
			</div>
		</div>

		<!-- コンテンツエリア -->
		<div class="overflow-auto max-h-[calc(90vh-80px)] p-4 bg-gray-100">
			{#if isLoading}
				<!-- ローディング表示 -->
				<div class="flex items-center justify-center min-h-[400px]">
					<div class="text-center">
						<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-purple-600 mx-auto mb-4"></div>
						<p class="text-gray-600">領収書を読み込み中...</p>
					</div>
				</div>
			{:else if loadError}
				<!-- エラー表示 -->
				<div class="bg-white rounded-lg p-6 text-center max-w-md mx-auto">
					<div class="text-6xl mb-4 text-red-500">⚠️</div>
					<p class="text-lg font-semibold mb-2 text-red-600">読み込みエラー</p>
					<p class="text-gray-600 mb-6 text-sm leading-relaxed">{loadError}</p>
					
					<!-- エラー対処のヒント -->
					<div class="bg-gray-50 rounded-lg p-4 mb-4 text-left">
						<p class="text-sm font-semibold text-gray-700 mb-2">💡 対処方法:</p>
						<ul class="text-xs text-gray-600 space-y-1">
							<li>• ネットワーク接続を確認してください</li>
							<li>• しばらく時間をおいて再試行してください</li>
							<li>• オフラインキャッシュがある場合は「オフラインで表示」をお試しください</li>
						</ul>
					</div>
					
					<div class="flex gap-2 justify-center">
						<button
							type="button"
							onclick={() => receiptUrl && loadFromR2()}
							class="btn btn-primary text-sm px-4 py-2"
						>
							🔄 再試行
						</button>
						{#if receiptUrl && !isOfflineMode}
							<button
								type="button"
								onclick={tryOfflineMode}
								class="btn bg-orange-500 hover:bg-orange-600 text-white text-sm px-4 py-2"
							>
								📱 オフラインで表示
							</button>
						{/if}
					</div>
				</div>
			{:else if isImage()}
				<!-- 画像表示 -->
				<div class="flex items-center justify-center min-h-[400px]">
					{#if dataUrl}
						<!-- R2からの画像 -->
						<img
							src={dataUrl}
							alt="領収書"
							class="max-w-full h-auto transition-transform duration-200"
							style="transform: scale({zoomLevel / 100}); transform-origin: center;"
						/>
					{:else if localFileUrl}
						<!-- ローカルファイルの画像 -->
						<img
							src={localFileUrl}
							alt="領収書"
							class="max-w-full h-auto transition-transform duration-200"
							style="transform: scale({zoomLevel / 100}); transform-origin: center;"
						/>
					{:else}
						<div class="text-center">
							<div class="text-6xl mb-4">🖼️</div>
							<p class="text-gray-600">画像を読み込めませんでした</p>
						</div>
					{/if}
				</div>
			{:else if isPdf()}
				<!-- PDF表示 -->
				<div class="bg-white rounded-lg p-6 text-center max-w-md mx-auto">
					<div class="text-6xl mb-4">📄</div>
					<p class="text-lg font-semibold mb-2">PDFファイル</p>
					<p class="text-gray-600 mb-4 text-sm">
						{receiptUrl ? receiptUrl.split('/').pop() : receiptPath?.split('/').pop()}
					</p>
					
					{#if receiptUrl}
						<div class="bg-blue-50 rounded-lg p-4 mb-4">
							<p class="text-sm text-blue-700 mb-2">
								📋 PDFファイルはクラウドに保存されています
							</p>
							<p class="text-xs text-blue-600">
								現在のバージョンではPDFの直接表示に対応していません。<br />
								将来のアップデートで対応予定です。
							</p>
						</div>
						
						<!-- 将来の機能として外部アプリで開くボタンを追加可能 -->
						<div class="text-xs text-gray-500">
							💡 ヒント: 外部PDFビューアーでの表示機能を検討中です
						</div>
					{:else}
						<div class="bg-gray-50 rounded-lg p-4">
							<p class="text-sm text-gray-600">
								PDFファイルはブラウザでは直接表示できません。<br />
								ファイルマネージャーで開いてください。
							</p>
						</div>
					{/if}
				</div>
			{:else}
				<!-- その他のファイル -->
				<div class="bg-white rounded-lg p-4 text-center">
					<div class="text-6xl mb-4">📎</div>
					<p class="text-lg font-semibold mb-2">ファイル</p>
					<p class="text-gray-600">
						{receiptUrl ? receiptUrl.split('/').pop() : receiptPath?.split('/').pop()}
					</p>
				</div>
			{/if}
		</div>
	</div>
</div>

<style>
	/* モーダルアニメーション */
	@keyframes fadeIn {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes slideUp {
		from {
			transform: translateY(20px);
			opacity: 0;
		}
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}

	div[role="dialog"] {
		animation: fadeIn 0.2s ease-out;
	}

	div[role="dialog"] > div {
		animation: slideUp 0.3s ease-out;
	}

	/* スクロールバーのカスタマイズ */
	.overflow-auto::-webkit-scrollbar {
		width: 8px;
		height: 8px;
	}

	.overflow-auto::-webkit-scrollbar-track {
		background: #f1f1f1;
		border-radius: 4px;
	}

	.overflow-auto::-webkit-scrollbar-thumb {
		background: #888;
		border-radius: 4px;
	}

	.overflow-auto::-webkit-scrollbar-thumb:hover {
		background: #555;
	}

	/* ボタンの無効化スタイル */
	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	button:disabled:hover {
		transform: none;
	}
</style>

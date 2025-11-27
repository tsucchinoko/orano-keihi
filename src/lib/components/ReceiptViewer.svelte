<script lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";

// Props
interface Props {
	receiptPath: string;
	onClose: () => void;
}

let { receiptPath, onClose }: Props = $props();

// Tauriのファイルパスを変換
const fileUrl = $derived(convertFileSrc(receiptPath));

// ズームレベル
let zoomLevel = $state(100);

// ファイルタイプ判定
const isPdf = $derived(() => {
	return receiptPath.toLowerCase().endsWith(".pdf");
});

const isImage = $derived(() => {
	return /\.(png|jpg|jpeg)$/i.test(receiptPath);
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

// ESCキーで閉じる
function handleKeydown(event: KeyboardEvent) {
	if (event.key === "Escape") {
		onClose();
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
	role="dialog"
	aria-modal="true"
>
	<!-- モーダルコンテンツ -->
	<div class="relative max-w-6xl max-h-[90vh] w-full bg-white rounded-lg shadow-2xl overflow-hidden">
		<!-- ヘッダー -->
		<div class="flex items-center justify-between p-4 border-b border-gray-200 bg-gradient-to-r from-purple-50 to-pink-50">
			<h3 class="text-xl font-bold">領収書</h3>
			
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
			{#if isImage()}
				<!-- 画像表示 -->
				<div class="flex items-center justify-center min-h-[400px]">
					<img
						src={fileUrl}
						alt="領収書"
						class="max-w-full h-auto transition-transform duration-200"
						style="transform: scale({zoomLevel / 100}); transform-origin: center;"
					/>
				</div>
			{:else if isPdf()}
				<!-- PDF表示 -->
				<div class="bg-white rounded-lg p-4 text-center">
					<div class="text-6xl mb-4">📄</div>
					<p class="text-lg font-semibold mb-2">PDFファイル</p>
					<p class="text-gray-600 mb-4">{receiptPath.split('/').pop()}</p>
					<p class="text-sm text-gray-500">
						PDFファイルはブラウザでは直接表示できません。<br />
						ファイルマネージャーで開いてください。
					</p>
				</div>
			{:else}
				<!-- その他のファイル -->
				<div class="bg-white rounded-lg p-4 text-center">
					<div class="text-6xl mb-4">📎</div>
					<p class="text-lg font-semibold mb-2">ファイル</p>
					<p class="text-gray-600">{receiptPath.split('/').pop()}</p>
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

<script lang="ts">
/**
 * エラーバウンダリコンポーネント
 * 予期しないエラーをキャッチして表示する
 */

interface Props {
	children: any;
}

let { children }: Props = $props();

// エラー状態
let error = $state<Error | null>(null);
let errorInfo = $state<string>("");

// エラーをリセット
function resetError() {
	error = null;
	errorInfo = "";
}

// エラーハンドラー（グローバルエラーをキャッチ）
if (typeof window !== "undefined") {
	window.addEventListener("error", (event) => {
		error = event.error;
		errorInfo = event.message;
		event.preventDefault();
	});

	window.addEventListener("unhandledrejection", (event) => {
		error = new Error(event.reason);
		errorInfo = "未処理のPromise拒否が発生しました";
		event.preventDefault();
	});
}
</script>

{#if error}
	<!-- エラー表示UI -->
	<div class="error-boundary">
		<div class="error-card">
			<div class="error-icon">⚠️</div>
			<h2 class="error-title">予期しないエラーが発生しました</h2>
			<p class="error-message">{errorInfo || error.message}</p>
			
			<details class="error-details">
				<summary class="error-details-summary">詳細情報</summary>
				<pre class="error-stack">{error.stack || "スタックトレースがありません"}</pre>
			</details>

			<div class="error-actions">
				<button onclick={resetError} class="btn btn-primary">
					再試行
				</button>
				<button onclick={() => window.location.reload()} class="btn bg-gray-300 text-gray-700">
					ページをリロード
				</button>
			</div>
		</div>
	</div>
{:else}
	<!-- 通常のコンテンツ -->
	{@render children()}
{/if}

<style>
	.error-boundary {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 2rem;
		background: linear-gradient(to bottom right, #fef2f2, #fee2e2);
	}

	.error-card {
		background: white;
		border-radius: 16px;
		padding: 3rem;
		max-width: 600px;
		width: 100%;
		box-shadow: 0 10px 25px rgba(0, 0, 0, 0.1);
		text-align: center;
	}

	.error-icon {
		font-size: 4rem;
		margin-bottom: 1rem;
	}

	.error-title {
		font-size: 1.5rem;
		font-weight: 700;
		color: #dc2626;
		margin-bottom: 1rem;
	}

	.error-message {
		color: #6b7280;
		margin-bottom: 2rem;
		line-height: 1.6;
	}

	.error-details {
		text-align: left;
		margin-bottom: 2rem;
		background: #f9fafb;
		border-radius: 8px;
		padding: 1rem;
	}

	.error-details-summary {
		cursor: pointer;
		font-weight: 600;
		color: #4b5563;
		user-select: none;
	}

	.error-details-summary:hover {
		color: #1f2937;
	}

	.error-stack {
		margin-top: 1rem;
		padding: 1rem;
		background: #1f2937;
		color: #f3f4f6;
		border-radius: 4px;
		overflow-x: auto;
		font-size: 0.875rem;
		line-height: 1.5;
	}

	.error-actions {
		display: flex;
		gap: 1rem;
		justify-content: center;
	}

	/* ボタンスタイル */
	.btn {
		padding: 0.75rem 1.5rem;
		border-radius: 8px;
		font-weight: 600;
		border: none;
		cursor: pointer;
		transition: all 0.2s ease-in-out;
	}

	.btn-primary {
		background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
		color: white;
	}

	.btn-primary:hover {
		transform: translateY(-2px);
		box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
	}

	.bg-gray-300 {
		background: #d1d5db;
	}

	.text-gray-700 {
		color: #374151;
	}

	.bg-gray-300:hover {
		background: #9ca3af;
	}
</style>

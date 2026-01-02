<script lang="ts">
import { onMount } from "svelte";
import { goto } from "$app/navigation";
import { page } from "$app/state";
import { authStore } from "$lib/stores";

// ローディング状態とエラー状態を取得
let isLoading = $derived(authStore.isLoading);
let error = $derived(authStore.error);
let isAuthenticated = $derived(authStore.isAuthenticated);

// 認証状態の初期化
onMount(async () => {
	// 認証状態を初期化
	await authStore.initialize();

	// 既に認証済みの場合はメインページにリダイレクト
	if (isAuthenticated) {
		goto("/");
		return;
	}
});

// 認証状態の変化を監視してリダイレクト
$effect(() => {
	if (isAuthenticated) {
		console.log("🔘 認証成功を検出しました。メインページにリダイレクトします");
		goto("/");
	}
});

// Googleログインボタンクリック処理
async function handleGoogleLogin() {
	console.log("🔘 Googleログインボタンがクリックされました");
	try {
		await authStore.login();
		console.log("🔘 authStore.login()が完了しました");
	} catch (error) {
		console.error("🔘 ログイン処理でエラーが発生しました:", error);
	}
}

// エラークリア処理
function clearError() {
	authStore.clearError();
}
</script>

<svelte:head>
	<title>ログイン - オラの経費だゾ</title>
</svelte:head>

<div class="login-container">
	<div class="login-card">
		<!-- ロゴとタイトル -->
		<div class="login-header">
			<h1 class="login-title">オラの経費だゾ</h1>
			<p class="login-subtitle">Googleアカウントでログインしてください</p>
		</div>

		<!-- エラーメッセージ -->
		{#if error}
			<div class="error-message">
				<div class="error-content">
					<svg class="error-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
					</svg>
					<span>{error}</span>
				</div>
				<button type="button" class="error-close" onclick={clearError} aria-label="エラーメッセージを閉じる">
					<svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
					</svg>
				</button>
			</div>
		{/if}

		<!-- ログインボタン -->
		<div class="login-actions">
			<button 
				type="button"
				class="google-login-button"
				onclick={handleGoogleLogin}
				disabled={isLoading}
			>
				{#if isLoading}
					<div class="loading-spinner"></div>
					<span>ログイン中...</span>
				{:else}
					<svg class="google-icon" viewBox="0 0 24 24">
						<path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/>
						<path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/>
						<path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/>
						<path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/>
					</svg>
					<span>Googleでログイン</span>
				{/if}
			</button>
		</div>

		<!-- 説明文 -->
		<div class="login-description">
			<p>
				Googleアカウントでログインすることで、あなたの経費とサブスクリプションデータを安全に管理できます。
			</p>
		</div>
	</div>
</div>

<style>
	.login-container {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
		padding: 1rem;
	}

	.login-card {
		background: white;
		border-radius: 16px;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
		padding: 2rem;
		width: 100%;
		max-width: 400px;
	}

	.login-header {
		text-align: center;
		margin-bottom: 2rem;
	}

	.login-title {
		font-size: 2rem;
		font-weight: 700;
		background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		margin: 0 0 0.5rem 0;
	}

	.login-subtitle {
		color: #6b7280;
		font-size: 1rem;
		margin: 0;
	}

	.error-message {
		background: #fef2f2;
		border: 1px solid #fecaca;
		border-radius: 8px;
		padding: 1rem;
		margin-bottom: 1.5rem;
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
	}

	.error-content {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex: 1;
	}

	.error-icon {
		width: 1.25rem;
		height: 1.25rem;
		color: #dc2626;
		flex-shrink: 0;
	}

	.error-message span {
		color: #dc2626;
		font-size: 0.875rem;
		line-height: 1.4;
	}

	.error-close {
		background: none;
		border: none;
		color: #dc2626;
		cursor: pointer;
		padding: 0;
		width: 1.25rem;
		height: 1.25rem;
		flex-shrink: 0;
	}

	.error-close:hover {
		color: #991b1b;
	}

	.error-close svg {
		width: 100%;
		height: 100%;
	}

	.login-actions {
		margin-bottom: 2rem;
	}

	.google-login-button {
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 0.875rem 1.5rem;
		background: white;
		border: 2px solid #e5e7eb;
		border-radius: 8px;
		font-size: 1rem;
		font-weight: 600;
		color: #374151;
		cursor: pointer;
		transition: all 0.2s ease-in-out;
	}

	.google-login-button:hover:not(:disabled) {
		border-color: #d1d5db;
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
		transform: translateY(-1px);
	}

	.google-login-button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.google-icon {
		width: 1.25rem;
		height: 1.25rem;
		flex-shrink: 0;
	}

	.loading-spinner {
		width: 1.25rem;
		height: 1.25rem;
		border: 2px solid #e5e7eb;
		border-top: 2px solid #3b82f6;
		border-radius: 50%;
		animation: spin 1s linear infinite;
		flex-shrink: 0;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.login-description {
		text-align: center;
	}

	.login-description p {
		color: #6b7280;
		font-size: 0.875rem;
		line-height: 1.5;
		margin: 0;
	}

	/* レスポンシブデザイン */
	@media (max-width: 480px) {
		.login-card {
			padding: 1.5rem;
		}

		.login-title {
			font-size: 1.75rem;
		}
	}
</style>
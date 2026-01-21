<script lang="ts">
import "../app.css";
// import ErrorBoundary from "$lib/components/ErrorBoundary.svelte";
// import ToastContainer from "$lib/components/ToastContainer.svelte";
import UpdateNotification from "$lib/components/UpdateNotification.svelte";
import { goto } from "$app/navigation";
import { page } from "$app/state";
import { authStore } from "$lib/stores";
import { onMount } from "svelte";
import { UpdaterService } from "$lib/services/updater";
import { listen } from "@tauri-apps/api/event";
import { confirm, message } from "@tauri-apps/plugin-dialog";
import type { UpdateInfo } from "$lib/types/updater";

interface Props {
	children: import('svelte').Snippet;
}

let { children }: Props = $props();

// 現在のパスを取得
let currentPath = $derived(page.url.pathname);

// 認証状態を取得
let isAuthenticated = $derived(authStore.isAuthenticated);
let user = $derived(authStore.user);
let isLoading = $derived(authStore.isLoading);

// アプリケーション初期化
onMount(() => {
	// 認証状態の初期化（非同期で実行）
	authStore.initialize().then(() => {
		console.info("🔧 レイアウト: 認証状態の初期化が完了しました");
	}).catch((error) => {
		console.error("🔧 レイアウト: 認証状態の初期化でエラーが発生しました:", error);
	});

	// メニューからのアップデート通知をリッスン（ダイアログ表示用）
	let unlistenShowDialog: (() => void) | undefined;
	let unlistenNoUpdate: (() => void) | undefined;
	let unlistenError: (() => void) | undefined;

	listen<UpdateInfo>('show-update-dialog', async (event) => {
		const updateInfo = event.payload;

		
		const shouldUpdate = await confirm(
			`新しいバージョン ${updateInfo.latest_version} が利用可能です。\nアップデートをインストールしますか？`,
			{
				title: 'アップデート利用可能',
				kind: 'info',
				okLabel: 'インストール',
				cancelLabel: 'キャンセル'
			}
		);

		if (shouldUpdate) {
			try {
				// アップデートをダウンロード＆インストール
				await UpdaterService.downloadAndInstall();
			} catch (error) {
				console.error('アップデートインストールエラー:', error);
				await message(`アップデートのインストールに失敗しました: ${error}`, {
					title: 'エラー',
					kind: 'error'
				});
			}
		}
	}).then((unlisten) => {
		unlistenShowDialog = unlisten;
	});

	listen('show-no-update-dialog', async () => {
		await message('最新バージョンを使用しています。', {
			title: 'アップデート確認',
			kind: 'info'
		});
	}).then((unlisten) => {
		unlistenNoUpdate = unlisten;
	});

	listen<string>('show-update-error-dialog', async (event) => {
		console.error('アップデートエラーダイアログを表示:', event.payload);
		await message(`アップデートチェックに失敗しました:\n${event.payload}`, {
			title: 'エラー',
			kind: 'error'
		});
	}).then((unlisten) => {
		unlistenError = unlisten;
	});

	// クリーンアップ
	return () => {
		unlistenShowDialog?.();
		unlistenNoUpdate?.();
		unlistenError?.();
	};
});

// プログラム的なナビゲーション関数
function navigateTo(path: string) {
	console.info(`Navigating to: ${path}`);
	goto(path);
}

// アクティブなナビゲーションリンクかどうかを判定
function isActive(path: string): boolean {
	return currentPath === path;
}

// ログアウト処理
async function handleLogout() {
	const confirmed = await confirm("ログアウトしますか？", {
		title: 'ログアウト確認',
		kind: 'warning',
		okLabel: 'ログアウト',
		cancelLabel: 'キャンセル'
	});
	if (confirmed) {
		await authStore.logout();
		goto("/login");
	}
}

// ログインページかどうかを判定
let isLoginPage = $derived(currentPath.startsWith("/login"));
</script>

<!-- 簡素化されたレイアウト（ErrorBoundaryとToastContainerは一時的にコメントアウト） -->
<!-- <ErrorBoundary> -->
	<!-- {#snippet children()} -->
		<!-- グローバルレイアウト: グラデーション背景とナビゲーション構造 -->
		<div class="app-container">
			<!-- ナビゲーションヘッダー（ログインページ以外で表示） -->
			{#if !isLoginPage}
				<header class="header">
					<nav class="nav-container">
						<div class="nav-brand">
							<button 
								type="button"
								class="brand-link brand-button" 
								onclick={() => navigateTo('/')}
							>
								<h1 class="brand-title">オラの経費だゾ</h1>
							</button>
						</div>
						
						{#if isAuthenticated}
							<!-- 認証済みユーザー向けナビゲーション -->
							<div class="nav-links">
								<button 
									type="button"
									class:active={isActive('/expenses')}
									class="nav-link nav-button" 
									onclick={() => navigateTo('/expenses')}
								>
									経費一覧
								</button>
								<button 
									type="button"
									class:active={isActive('/subscriptions')}
									class="nav-link nav-button" 
									onclick={() => navigateTo('/subscriptions')}
								>
									サブスクリプション
								</button>
								{#if import.meta.env.DEV}
									<button 
										type="button"
										class:active={isActive('/debug')}
										class="nav-link nav-button debug-link" 
										onclick={() => navigateTo('/debug')}
									>
										デバッグ
									</button>
									<button 
										type="button"
										class:active={isActive('/migration-status')}
										class="nav-link nav-button debug-link" 
										onclick={() => navigateTo('/migration-status')}
									>
										マイグレーション状態
									</button>
								{/if}
							</div>
							
							<!-- ユーザー情報とログアウト -->
							<div class="user-section">
								{#if user}
									<div class="user-info">
										{#if user.picture_url}
											<img 
												src={user.picture_url} 
												alt={user.name}
												class="user-avatar"
											/>
										{/if}
									</div>
								{/if}
								<button 
									type="button"
									class="logout-button"
									onclick={handleLogout}
									disabled={isLoading}
								>
									ログアウト
								</button>
							</div>
						{:else if !isLoading}
							<!-- 未認証ユーザー向けナビゲーション -->
							<div class="nav-links">
								<button 
									type="button"
									class="nav-link nav-button login-button" 
									onclick={() => navigateTo('/login')}
								>
									ログイン
								</button>
							</div>
						{/if}
					</nav>
				</header>
			{/if}

			<!-- メインコンテンツエリア -->
			<main class="main-content" class:no-header={isLoginPage}>
				<!-- デバッグ情報（開発環境のみ） -->
				{#if import.meta.env.DEV && !isLoginPage}
					<div class="debug-info">
						現在のパス: {currentPath} | 認証状態: {isAuthenticated ? '認証済み' : '未認証'}
					</div>
				{/if}
				{@render children()}
			</main>

			<!-- トースト通知コンテナ -->
			<!-- <ToastContainer /> -->

			<!-- アップデート通知 -->
			<UpdateNotification />
		</div>
	<!-- {/snippet} -->
<!-- </ErrorBoundary> -->

<style>
	/* アプリケーション全体のコンテナ */
	.app-container {
		min-height: 100vh;
		background: #f9fafb;
		font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
		position: relative;
		overflow-x: hidden;
	}

	/* ヘッダースタイル */
	.header {
		background: white;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
		position: sticky;
		top: 0;
		z-index: 50;
	}

	.nav-container {
		max-width: 1200px;
		margin: 0 auto;
		padding: 1rem 2rem;
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
	}

	.nav-brand {
		display: flex;
		align-items: center;
	}

	.brand-link {
		text-decoration: none;
		transition: transform 0.2s ease-in-out;
		display: inline-block;
	}

	.brand-link:hover {
		transform: scale(1.05);
	}

	.brand-button {
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
	}

	.brand-title {
		font-size: 1.5rem;
		font-weight: 700;
		background: var(--gradient-primary);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		margin: 0;
		cursor: pointer;
	}

	.nav-links {
		display: flex;
		gap: 2rem;
	}

	.nav-link {
		font-weight: 600;
		color: #4b5563;
		text-decoration: none;
		padding: 0.5rem 1rem;
		border-radius: 8px;
		transition: all 0.2s ease-in-out;
	}

	.nav-link:hover {
		background: var(--gradient-primary);
		color: white;
		transform: translateY(-2px);
	}

	.debug-link {
		background: #f3f4f6;
		color: #6b7280;
		font-size: 0.875rem;
	}

	.debug-link:hover {
		background: #ef4444;
		color: white;
	}

	.nav-button {
		background: none;
		border: none;
		cursor: pointer;
		font-family: inherit;
		font-size: inherit;
	}

	.nav-link.active {
		background: var(--gradient-primary);
		color: white;
		transform: translateY(-2px);
	}

	.login-button {
		background: var(--gradient-primary);
		color: white;
	}

	.login-button:hover {
		background: var(--gradient-primary);
		opacity: 0.9;
	}

	/* ユーザーセクション */
	.user-section {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.user-info {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.user-avatar {
		width: 2rem;
		height: 2rem;
		border-radius: 50%;
		object-fit: cover;
	}

	.user-name {
		font-weight: 600;
		color: #374151;
		font-size: 0.875rem;
	}

	.logout-button {
		background: #f3f4f6;
		color: #6b7280;
		border: none;
		padding: 0.5rem 1rem;
		border-radius: 6px;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s ease-in-out;
	}

	.logout-button:hover:not(:disabled) {
		background: #ef4444;
		color: white;
		transform: translateY(-1px);
	}

	.logout-button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	/* メインコンテンツエリア */
	.main-content {
		max-width: 1200px;
		margin: 0 auto;
		padding: 2rem;
	}

	.main-content.no-header {
		padding: 0;
		max-width: none;
	}

	/* デバッグ情報 */
	.debug-info {
		position: fixed;
		bottom: 10px;
		right: 10px;
		background: rgba(0, 0, 0, 0.8);
		color: white;
		padding: 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
		z-index: 1000;
	}

	/* レスポンシブデザイン */
	@media (max-width: 768px) {
		.nav-container {
			flex-direction: column;
			gap: 1rem;
			padding: 1rem;
		}

		.nav-links {
			gap: 1rem;
		}

		.user-section {
			flex-direction: column;
			gap: 0.5rem;
		}

		.main-content {
			padding: 1rem;
		}

		.main-content.no-header {
			padding: 0;
		}
	}
</style>

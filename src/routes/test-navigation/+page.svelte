<script lang="ts">
import { goto } from "$app/navigation";
import { page } from "$app/stores";

let currentPath = $derived($page.url.pathname);

function testNavigation() {
	console.log("Testing navigation...");
	goto("/expenses");
}

function testNavigationWithReplace() {
	console.log("Testing navigation with replace...");
	goto("/subscriptions", { replaceState: true });
}
</script>

<div class="test-page">
	<h1>ナビゲーションテストページ</h1>
	<p>現在のパス: {currentPath}</p>
	
	<div class="test-buttons">
		<button onclick={testNavigation} class="btn btn-primary">
			経費ページへ移動（goto）
		</button>
		
		<button onclick={testNavigationWithReplace} class="btn btn-info">
			サブスクリプションページへ移動（replace）
		</button>
		
		<a href="/debug" class="btn bg-gray-500 text-white">
			デバッグページへ（通常リンク）
		</a>
		
		<a href="/" onclick={(e) => { e.preventDefault(); goto('/'); }} class="btn bg-green-500 text-white">
			ホームへ（preventDefault + goto）
		</a>
	</div>
	
	<div class="navigation-info">
		<h2>ナビゲーション情報</h2>
		<p>SvelteKit バージョン: {import.meta.env.VITE_SVELTEKIT_VERSION || '不明'}</p>
		<p>ブラウザ履歴API: {typeof window !== 'undefined' && window.history ? '利用可能' : '利用不可'}</p>
	</div>
</div>

<style>
	.test-page {
		padding: 2rem;
		max-width: 800px;
		margin: 0 auto;
	}
	
	.test-buttons {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		margin: 2rem 0;
	}
	
	.navigation-info {
		margin-top: 2rem;
		padding: 1rem;
		background: #f3f4f6;
		border-radius: 8px;
	}
</style>
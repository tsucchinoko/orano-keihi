<script lang="ts">
import "../app.css";
</script>

<!-- グローバルレイアウト: グラデーション背景とナビゲーション構造 -->
<div class="app-container">
	<!-- ナビゲーションヘッダー -->
	<header class="header">
		<nav class="nav-container">
			<div class="nav-brand">
				<h1 class="brand-title">経費管理</h1>
			</div>
			<div class="nav-links">
				<a href="/" class="nav-link">ダッシュボード</a>
				<a href="/expenses" class="nav-link">経費一覧</a>
			</div>
		</nav>
	</header>

	<!-- メインコンテンツエリア -->
	<main class="main-content">
		<slot />
	</main>
</div>

<style>
	/* アプリケーション全体のコンテナ */
	.app-container {
		min-height: 100vh;
		background: var(--bg-gradient-light);
		font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
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
	}

	.nav-brand {
		display: flex;
		align-items: center;
	}

	.brand-title {
		font-size: 1.5rem;
		font-weight: 700;
		background: var(--gradient-primary);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		margin: 0;
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

	/* メインコンテンツエリア */
	.main-content {
		max-width: 1200px;
		margin: 0 auto;
		padding: 2rem;
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

		.main-content {
			padding: 1rem;
		}
	}
</style>

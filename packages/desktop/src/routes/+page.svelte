<script lang="ts">
import { onMount } from "svelte";
import { goto } from "$app/navigation";
// 一時的にコンポーネントのインポートをコメントアウト
// import { SubscriptionForm, SubscriptionList } from "$features/subscriptions";
// import { ExpenseForm } from "$features/expenses";
import type { Expense, Subscription } from "$lib/types";
import {
	getExpenses,
} from "$lib/utils/tauri";
import {
	fetchSubscriptions,
	fetchMonthlySubscriptionTotal,
} from "$lib/utils/api-client";
import { authStore } from "$lib/stores";

// 状態管理
let expenses = $state<Expense[]>([]);
let subscriptions = $state<Subscription[]>([]);
let monthlySubscriptionTotal = $state<number>(0);
let loading = $state(true);
let error = $state<string | null>(null);

// 認証状態のリアクティブな値
let isAuthenticated = $derived(authStore.isAuthenticated);
let isLoading = $derived(authStore.isLoading);
let authError = $derived(authStore.error);

// モーダル表示状態（サブスクリプション編集）
let showEditModal = $state(false);
let editingSubscription = $state<Subscription | undefined>(undefined);

// モーダル表示状態（経費追加）
let showExpenseModal = $state(false);

// 今月の経費サマリー
let currentMonth = $derived(new Date().toISOString().slice(0, 7)); // YYYY-MM形式
let monthlyExpenses = $derived(
	expenses.filter((expense) => expense.date.startsWith(currentMonth)),
);

// カテゴリ別集計
let categoryTotals = $derived(() => {
	const totals = new Map<string, number>();
	for (const expense of monthlyExpenses) {
		const current = totals.get(expense.category) || 0;
		totals.set(expense.category, current + expense.amount);
	}
	return Array.from(totals.entries()).map(([category, total]) => ({
		category,
		total,
	}));
});

// 今月の合計金額
let monthlyTotal = $derived(
	monthlyExpenses.reduce((sum, expense) => sum + expense.amount, 0),
);

// データ読み込み
async function loadData() {
	loading = true;
	error = null;

	try {
		// 今月の経費を取得
		const expensesResult = await getExpenses(currentMonth);
		if (expensesResult.error) {
			throw new Error(expensesResult.error);
		}
		expenses = expensesResult.data || [];

		// 認証されている場合のみサブスクリプションデータを取得
		if (isAuthenticated) {
			try {
				// サブスクリプションを取得（APIサーバー経由）
				const subscriptionsResponse = await fetchSubscriptions(true);
				subscriptions = subscriptionsResponse.subscriptions || [];
				monthlySubscriptionTotal = subscriptionsResponse.monthlyTotal || 0;
			} catch (apiError) {
				console.warn("APIサーバー経由でのサブスクリプション取得に失敗:", apiError);
				// フォールバック: 空のデータを設定
				subscriptions = [];
				monthlySubscriptionTotal = 0;
			}
		} else {
			// 未認証の場合は空のデータを設定
			subscriptions = [];
			monthlySubscriptionTotal = 0;
		}
	} catch (e) {
		error = e instanceof Error ? e.message : "不明なエラーが発生しました";
		console.error("データ読み込みエラー:", e);
	} finally {
		loading = false;
	}
}

// 経費追加ハンドラー
function handleAddExpense() {
	showExpenseModal = true;
}

// 経費フォーム成功時
function handleExpenseFormSuccess() {
	showExpenseModal = false;
	// データを再読み込み
	loadData();
}

// 経費フォームキャンセル時
function handleExpenseFormCancel() {
	showExpenseModal = false;
}

// サブスクリプション編集ハンドラー
function handleEditSubscription(subscription: Subscription) {
	editingSubscription = subscription;
	showEditModal = true;
}

// サブスクリプションフォーム成功時
function handleSubscriptionFormSuccess() {
	showEditModal = false;
	editingSubscription = undefined;
	// データを再読み込み
	loadData();
}

// サブスクリプションフォームキャンセル時
function handleSubscriptionFormCancel() {
	showEditModal = false;
	editingSubscription = undefined;
}

onMount(() => {
	loadData();
});

// 認証状態が変更されたときにデータを再読み込み
$effect(() => {
	if (isAuthenticated) {
		loadData();
	}
});

// 金額フォーマット
function formatCurrency(amount: number): string {
	return new Intl.NumberFormat("ja-JP", {
		style: "currency",
		currency: "JPY",
	}).format(amount);
}

// カテゴリカラー取得
function getCategoryColor(category: string): string {
	const colorMap: Record<string, string> = {
		交通費: "var(--color-category-transport)",
		飲食費: "var(--color-category-meals)",
		通信費: "var(--color-category-communication)",
		消耗品費: "var(--color-category-supplies)",
		接待交際費: "var(--color-category-entertainment)",
		その他: "var(--color-category-other)",
	};
	return colorMap[category] || "var(--color-category-other)";
}
</script>

<!-- ダッシュボードページ -->
<div class="dashboard">
	<div class="dashboard-header">
		<h1 class="page-title">ダッシュボード</h1>
		<p class="page-subtitle">今月の経費とサブスクリプションの概要</p>
	</div>

	{#if loading}
		<div class="loading-container">
			<p>読み込み中...</p>
		</div>
	{:else if error}
		<div class="error-container">
			<p class="error-message">エラー: {error}</p>
			<button class="btn btn-primary" onclick={loadData}>再読み込み</button>
		</div>
	{:else}
		<!-- クイックアクションボタン -->
		<div class="quick-actions">
			<button
				type="button"
				onclick={handleAddExpense}
				class="action-card gradient-primary"
			>
				<div class="action-icon">💰</div>
				<h3 class="action-title">経費を追加</h3>
				<p class="action-description">新しい経費を記録する</p>
			</button>
			<button 
				type="button"
				class="action-card gradient-info" 
				onclick={() => goto('/expenses')}
			>
				<div class="action-icon">📊</div>
				<h3 class="action-title">経費一覧</h3>
				<p class="action-description">経費を確認・編集する</p>
			</button>
			<button 
				type="button"
				class="action-card gradient-warning" 
				onclick={() => goto('/subscriptions')}
			>
				<div class="action-icon">💳</div>
				<h3 class="action-title">サブスクリプション</h3>
				<p class="action-description">定期支払いを管理する</p>
			</button>
		</div>

		<!-- 今月の経費サマリー -->
		<div class="summary-section">
			<div class="card summary-card">
				<h2 class="section-title">今月の経費サマリー</h2>
				<div class="summary-total">
					<span class="total-label">合計</span>
					<span class="total-amount">{formatCurrency(monthlyTotal)}</span>
				</div>

				{#if categoryTotals().length > 0}
					<div class="category-breakdown">
						<h3 class="breakdown-title">カテゴリ別内訳</h3>
						<div class="category-list">
							{#each categoryTotals() as { category, total }}
								<div class="category-item">
									<div class="category-info">
										<span
											class="category-dot"
											style="background-color: {getCategoryColor(category)}"
										></span>
										<span class="category-name">{category}</span>
									</div>
									<span class="category-amount">{formatCurrency(total)}</span>
								</div>
							{/each}
						</div>
					</div>
				{:else}
					<p class="empty-message">今月の経費はまだありません</p>
				{/if}
			</div>
		</div>

		<!-- サブスクリプション一覧 -->
		<div class="subscription-section">
			<div class="card">
				<div class="section-header">
					<h2 class="section-title">サブスクリプション</h2>
					<div class="subscription-total">
						<span class="total-label">月額合計</span>
						<span class="total-amount">{formatCurrency(monthlySubscriptionTotal)}</span>
					</div>
				</div>
				<!-- サブスクリプション一覧を表示 -->
				{#if subscriptions.length > 0}
					<div class="subscription-list">
						{#each subscriptions.slice(0, 3) as subscription}
							<div class="subscription-item">
								<div class="subscription-info">
									<span class="subscription-name">{subscription.name}</span>
									<span class="subscription-category">{subscription.category}</span>
								</div>
								<div class="subscription-amount">
									{formatCurrency(subscription.billing_cycle === 'monthly' ? subscription.amount : Math.round(subscription.amount / 12))}
									<span class="billing-cycle">
										{subscription.billing_cycle === 'monthly' ? '/月' : '/月 (年額)'}
									</span>
								</div>
							</div>
						{/each}
						{#if subscriptions.length > 3}
							<div class="subscription-more">
								<button 
									type="button"
									onclick={() => goto('/subscriptions')}
									class="btn-link"
								>
									他 {subscriptions.length - 3} 件を表示
								</button>
							</div>
						{/if}
					</div>
				{:else if isAuthenticated}
					<div class="empty-state">
						<p>サブスクリプションが登録されていません</p>
						<button 
							type="button"
							onclick={() => goto('/subscriptions')}
							class="btn btn-secondary"
						>
							サブスクリプションを追加
						</button>
					</div>
				{:else}
					<div class="empty-state">
						<p>ログインしてサブスクリプションを表示</p>
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<!-- 経費追加モーダル -->
	{#if showExpenseModal}
		<div 
			class="modal-overlay" 
			role="dialog" 
			aria-modal="true"
			onclick={handleExpenseFormCancel}
			onkeydown={(e) => e.key === 'Escape' && handleExpenseFormCancel()}
		>
			<div 
				class="modal-content" 
				role="document"
				onclick={(e) => e.stopPropagation()}
			>
				<!-- 一時的にコンポーネントをコメントアウト -->
				<p>経費フォーム（開発中）</p>
				<!-- <ExpenseForm
					onSuccess={handleExpenseFormSuccess}
					onCancel={handleExpenseFormCancel}
				/> -->
			</div>
		</div>
	{/if}

	<!-- サブスクリプション編集モーダル -->
	{#if showEditModal}
		<div 
			class="modal-overlay" 
			role="dialog" 
			aria-modal="true"
			onclick={handleSubscriptionFormCancel}
			onkeydown={(e) => e.key === 'Escape' && handleSubscriptionFormCancel()}
		>
			<div 
				class="modal-content" 
				role="document"
				onclick={(e) => e.stopPropagation()}
			>
				<!-- 一時的にコンポーネントをコメントアウト -->
				<p>サブスクリプションフォーム（開発中）</p>
				<!-- <SubscriptionForm
					subscription={editingSubscription}
					onSuccess={handleSubscriptionFormSuccess}
					onCancel={handleSubscriptionFormCancel}
				/> -->
			</div>
		</div>
	{/if}
</div>

<style>
	/* ダッシュボードコンテナ */
	.dashboard {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	/* ヘッダー */
	.dashboard-header {
		text-align: center;
		margin-bottom: 1rem;
	}

	.page-title {
		font-size: 2.5rem;
		font-weight: 700;
		background: var(--gradient-primary);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		margin: 0;
	}

	.page-subtitle {
		color: #6b7280;
		font-size: 1.125rem;
		margin-top: 0.5rem;
	}

	/* ローディング・エラー */
	.loading-container,
	.error-container {
		text-align: center;
		padding: 3rem;
	}

	.error-message {
		color: #ef4444;
		margin-bottom: 1rem;
	}

	/* クイックアクション */
	.quick-actions {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 1.5rem;
	}

	.action-card {
		padding: 2rem;
		border-radius: 12px;
		text-decoration: none;
		color: white;
		text-align: center;
		transition: all 0.3s ease-in-out;
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
		border: none;
		cursor: pointer;
		width: 100%;
		font-family: inherit;
	}

	.action-card:hover {
		transform: translateY(-4px);
		box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.2);
	}

	.action-icon {
		font-size: 3rem;
		margin-bottom: 1rem;
	}

	.action-title {
		font-size: 1.25rem;
		font-weight: 700;
		margin: 0.5rem 0;
	}

	.action-description {
		font-size: 0.875rem;
		opacity: 0.9;
		margin: 0;
	}

	/* サマリーセクション */
	.summary-section {
		margin-top: 1rem;
	}

	.summary-card {
		padding: 2rem;
	}

	.section-title {
		font-size: 1.5rem;
		font-weight: 700;
		color: #1f2937;
		margin: 0 0 1.5rem 0;
	}

	.summary-total {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		background: var(--gradient-primary);
		border-radius: 12px;
		color: white;
		margin-bottom: 1.5rem;
	}

	.total-label {
		font-size: 1rem;
		font-weight: 600;
	}

	.total-amount {
		font-size: 2rem;
		font-weight: 700;
	}

	/* カテゴリ内訳 */
	.category-breakdown {
		margin-top: 1.5rem;
	}

	.breakdown-title {
		font-size: 1.125rem;
		font-weight: 600;
		color: #4b5563;
		margin-bottom: 1rem;
	}

	.category-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.category-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		background: #f9fafb;
		border-radius: 8px;
		transition: all 0.2s ease-in-out;
	}

	.category-item:hover {
		background: #f3f4f6;
		transform: translateX(4px);
	}

	.category-info {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.category-dot {
		width: 12px;
		height: 12px;
		border-radius: 50%;
	}

	.category-name {
		font-weight: 600;
		color: #374151;
	}

	.category-amount {
		font-weight: 700;
		color: #1f2937;
	}

	.empty-message {
		text-align: center;
		color: #9ca3af;
		padding: 2rem;
	}

	/* サブスクリプションセクション */
	.subscription-section {
		margin-top: 1rem;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	.subscription-total {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.75rem 1.5rem;
		background: var(--gradient-info);
		border-radius: 8px;
		color: white;
	}

	.subscription-total .total-label {
		font-size: 0.875rem;
		font-weight: 600;
	}

	.subscription-total .total-amount {
		font-size: 1.25rem;
		font-weight: 700;
	}

	/* サブスクリプション一覧 */
	.subscription-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.subscription-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		background: #f9fafb;
		border-radius: 8px;
		transition: all 0.2s ease-in-out;
	}

	.subscription-item:hover {
		background: #f3f4f6;
		transform: translateX(4px);
	}

	.subscription-info {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.subscription-name {
		font-weight: 600;
		color: #374151;
		font-size: 0.875rem;
	}

	.subscription-category {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.subscription-amount {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 0.125rem;
		font-weight: 700;
		color: #1f2937;
		font-size: 0.875rem;
	}

	.billing-cycle {
		font-size: 0.625rem;
		color: #6b7280;
		font-weight: 400;
	}

	.subscription-more {
		text-align: center;
		padding: 0.75rem;
	}

	.btn-link {
		background: none;
		border: none;
		color: var(--color-primary);
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		text-decoration: underline;
		transition: color 0.2s ease-in-out;
	}

	.btn-link:hover {
		color: var(--color-primary-dark);
	}

	.empty-state {
		text-align: center;
		padding: 2rem 1rem;
		color: #6b7280;
	}

	.empty-state p {
		margin-bottom: 1rem;
		font-size: 0.875rem;
	}

	.btn-secondary {
		background: #f3f4f6;
		color: #374151;
		border: 1px solid #d1d5db;
		padding: 0.5rem 1rem;
		border-radius: 6px;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s ease-in-out;
	}

	.btn-secondary:hover {
		background: #e5e7eb;
		border-color: #9ca3af;
	}

	/* モーダル */
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		padding: 1rem;
		backdrop-filter: blur(4px);
		animation: fadeIn 0.2s ease-out;
	}

	.modal-content {
		background: white;
		border-radius: 16px;
		padding: 2rem;
		max-width: 600px;
		width: 100%;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
		animation: modalSlideIn 0.3s ease-out;
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes modalSlideIn {
		from {
			opacity: 0;
			transform: translateY(-20px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	/* レスポンシブデザイン */
	@media (max-width: 768px) {
		.page-title {
			font-size: 2rem;
		}

		.quick-actions {
			grid-template-columns: 1fr;
		}

		.section-header {
			flex-direction: column;
			align-items: flex-start;
			gap: 1rem;
		}

		.summary-total {
			flex-direction: column;
			gap: 0.5rem;
			text-align: center;
		}

		.total-amount {
			font-size: 1.5rem;
		}

		.modal-content {
			padding: 1.5rem;
			max-height: 95vh;
		}
	}
</style>

<script lang="ts">
import { onMount } from "svelte";
import MonthSelector from "$lib/components/MonthSelector.svelte";
import CategoryFilter from "$lib/components/CategoryFilter.svelte";
import ExpenseList from "$lib/components/ExpenseList.svelte";
import ExpenseForm from "$lib/components/ExpenseForm.svelte";
import type { Expense } from "$lib/types";
import { getExpenses, deleteExpense } from "$lib/utils/tauri";

// 状態管理
let expenses = $state<Expense[]>([]);
let selectedMonth = $state<string>(new Date().toISOString().slice(0, 7)); // YYYY-MM形式
let selectedCategories = $state<string[]>([]);
let loading = $state(true);
let error = $state<string | null>(null);

// モーダル状態
let showExpenseForm = $state(false);
let editingExpense = $state<Expense | undefined>(undefined);

// フィルタリングされた経費
let filteredExpenses = $derived(() => {
	let result = expenses;

	// 月でフィルタリング
	if (selectedMonth) {
		result = result.filter((expense) => expense.date.startsWith(selectedMonth));
	}

	// カテゴリでフィルタリング
	if (selectedCategories.length > 0) {
		result = result.filter((expense) =>
			selectedCategories.includes(expense.category),
		);
	}

	return result;
});

// 経費データの読み込み
async function loadExpenses() {
	loading = true;
	error = null;

	try {
		const result = await getExpenses(selectedMonth);
		if (result.error) {
			throw new Error(result.error);
		}
		expenses = result.data || [];
	} catch (e) {
		error = e instanceof Error ? e.message : "不明なエラーが発生しました";
		console.error("経費読み込みエラー:", e);
	} finally {
		loading = false;
	}
}

// 月変更ハンドラー
function handleMonthChange(month: string) {
	selectedMonth = month;
	loadExpenses();
}

// カテゴリフィルター変更ハンドラー
function handleCategoryFilterChange(categories: string[]) {
	selectedCategories = categories;
}

// 経費追加ボタンクリック
function handleAddExpense() {
	editingExpense = undefined;
	showExpenseForm = true;
}

// 経費編集ハンドラー
function handleEditExpense(expense: Expense) {
	editingExpense = expense;
	showExpenseForm = true;
}

// 経費削除ハンドラー
async function handleDeleteExpense(expenseId: number) {
	if (!confirm("この経費を削除してもよろしいですか？")) {
		return;
	}

	try {
		const result = await deleteExpense(expenseId);
		if (result.error) {
			throw new Error(result.error);
		}

		// 削除成功後、リストを更新
		await loadExpenses();
	} catch (e) {
		const errorMessage = e instanceof Error ? e.message : "削除に失敗しました";
		alert(`エラー: ${errorMessage}`);
		console.error("経費削除エラー:", e);
	}
}

// フォーム保存ハンドラー
async function handleFormSave(expenseData: any, receiptFile?: string) {
	// TODO: 実際の保存処理を実装
	// createExpense または updateExpense を呼び出す
	showExpenseForm = false;
	editingExpense = undefined;
	await loadExpenses();
}

// フォームキャンセルハンドラー
function handleFormCancel() {
	showExpenseForm = false;
	editingExpense = undefined;
}

// 初期データ読み込み
onMount(() => {
	loadExpenses();
});
</script>

<!-- 経費一覧ページ -->
<div class="expenses-page">
	<div class="page-header">
		<h1 class="page-title">経費一覧</h1>
		<p class="page-subtitle">月別・カテゴリ別に経費を確認できます</p>
	</div>

	<!-- フィルターセクション -->
	<div class="filters-section">
		<div class="filter-row">
			<div class="filter-item">
				<label class="filter-label">月を選択</label>
				<MonthSelector {selectedMonth} onMonthChange={handleMonthChange} />
			</div>
			<div class="filter-item">
				<label class="filter-label">カテゴリでフィルター</label>
				<CategoryFilter {selectedCategories} onFilterChange={handleCategoryFilterChange} />
			</div>
		</div>
	</div>

	<!-- 経費リスト -->
	<div class="expenses-section">
		{#if loading}
			<div class="loading-container">
				<p>読み込み中...</p>
			</div>
		{:else if error}
			<div class="error-container">
				<p class="error-message">エラー: {error}</p>
				<button class="btn btn-primary" onclick={loadExpenses}>再読み込み</button>
			</div>
		{:else}
			<ExpenseList
				onEdit={handleEditExpense}
			/>
		{/if}
	</div>

	<!-- フローティングアクションボタン -->
	<button class="fab gradient-primary" onclick={handleAddExpense} aria-label="経費を追加">
		<span class="fab-icon">+</span>
	</button>

	<!-- 経費フォームモーダル -->
	{#if showExpenseForm}
		<div class="modal-overlay" onclick={handleFormCancel}>
			<div class="modal-content" onclick={(e) => e.stopPropagation()}>
				<div class="modal-header">
					<h2 class="modal-title">{editingExpense ? '経費を編集' : '経費を追加'}</h2>
					<button class="modal-close" onclick={handleFormCancel} aria-label="閉じる">×</button>
				</div>
				<div class="modal-body">
					<ExpenseForm expense={editingExpense} onSuccess={handleFormCancel} onCancel={handleFormCancel} />
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	/* ページコンテナ */
	.expenses-page {
		display: flex;
		flex-direction: column;
		gap: 2rem;
		position: relative;
		min-height: calc(100vh - 200px);
	}

	/* ページヘッダー */
	.page-header {
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

	/* フィルターセクション */
	.filters-section {
		background: white;
		border-radius: 12px;
		padding: 1.5rem;
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
	}

	.filter-row {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 1.5rem;
	}

	.filter-item {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.filter-label {
		font-weight: 600;
		color: #374151;
		font-size: 0.875rem;
	}

	/* 経費セクション */
	.expenses-section {
		flex: 1;
	}

	/* ローディング・エラー */
	.loading-container,
	.error-container {
		text-align: center;
		padding: 3rem;
		background: white;
		border-radius: 12px;
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
	}

	.error-message {
		color: #ef4444;
		margin-bottom: 1rem;
	}

	/* フローティングアクションボタン */
	.fab {
		position: fixed;
		bottom: 2rem;
		right: 2rem;
		width: 64px;
		height: 64px;
		border-radius: 50%;
		border: none;
		color: white;
		font-size: 2rem;
		cursor: pointer;
		box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
		transition: all 0.3s ease-in-out;
		z-index: 40;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.fab:hover {
		transform: scale(1.1) rotate(90deg);
		box-shadow: 0 15px 35px rgba(0, 0, 0, 0.3);
	}

	.fab:active {
		transform: scale(0.95) rotate(90deg);
	}

	.fab-icon {
		line-height: 1;
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
		z-index: 50;
		padding: 1rem;
	}

	.modal-content {
		background: white;
		border-radius: 12px;
		max-width: 600px;
		width: 100%;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.modal-title {
		font-size: 1.5rem;
		font-weight: 700;
		color: #1f2937;
		margin: 0;
	}

	.modal-close {
		background: none;
		border: none;
		font-size: 2rem;
		color: #9ca3af;
		cursor: pointer;
		padding: 0;
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
		transition: all 0.2s ease-in-out;
	}

	.modal-close:hover {
		background: #f3f4f6;
		color: #374151;
	}

	.modal-body {
		padding: 1.5rem;
	}

	/* レスポンシブデザイン */
	@media (max-width: 768px) {
		.page-title {
			font-size: 2rem;
		}

		.filter-row {
			grid-template-columns: 1fr;
		}

		.fab {
			bottom: 1rem;
			right: 1rem;
			width: 56px;
			height: 56px;
			font-size: 1.75rem;
		}

		.modal-content {
			max-height: 95vh;
		}

		.modal-header,
		.modal-body {
			padding: 1rem;
		}
	}
</style>

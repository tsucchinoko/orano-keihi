<script lang="ts">
import { onMount } from "svelte";
import {
	MonthSelector,
	CategoryFilter,
	ExpenseList,
	ExpenseForm,
} from "$features/expenses";
import { ReceiptViewer } from "$features/receipts";
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

// 領収書表示状態
let showReceiptViewer = $state(false);
let currentReceiptUrl = $state<string | undefined>(undefined);
let currentReceiptPath = $state<string | undefined>(undefined);

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

// 領収書表示ハンドラー
function handleViewReceipt(receiptUrl?: string, receiptPath?: string) {
	currentReceiptUrl = receiptUrl;
	currentReceiptPath = receiptPath;
	showReceiptViewer = true;
}

// 領収書表示を閉じる
function handleCloseReceiptViewer() {
	showReceiptViewer = false;
	currentReceiptUrl = undefined;
	currentReceiptPath = undefined;
}

// 初期データ読み込み
onMount(() => {
	loadExpenses();
});
</script>

<!-- 経費一覧ページ -->
<div class="expenses-page">
	<div class="page-header">
		<div>
			<h1 class="page-title">経費一覧</h1>
			<p class="page-subtitle">月別・カテゴリ別に経費を確認できます</p>
		</div>
		<button
			type="button"
			onclick={handleAddExpense}
			class="btn btn-primary"
		>
			➕ 新規追加
		</button>
	</div>

	<!-- フィルターセクション -->
	<div class="filters-section">
		<div class="filter-row">
			<div class="filter-item">
				<span class="filter-label">月を選択</span>
				<MonthSelector {selectedMonth} onMonthChange={handleMonthChange} />
			</div>
			<div class="filter-item">
				<span class="filter-label">カテゴリでフィルター</span>
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
				onViewReceipt={handleViewReceipt}
			/>
		{/if}
	</div>

	<!-- 経費フォームモーダル -->
	{#if showExpenseForm}
		<div 
			class="modal-overlay" 
			onclick={handleFormCancel}
			onkeydown={(e) => e.key === 'Enter' && handleFormCancel()}
			role="button"
			tabindex="0"
			aria-label="モーダルを閉じる"
		>
			<div 
				class="modal-content" 
				onclick={(e) => e.stopPropagation()}
				onkeydown={(e) => e.stopPropagation()}
				role="dialog"
				aria-modal="true"
				tabindex="-1"
			>
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

	<!-- 領収書表示モーダル -->
	{#if showReceiptViewer && (currentReceiptUrl || currentReceiptPath)}
		<ReceiptViewer
			receiptUrl={currentReceiptUrl}
			receiptPath={currentReceiptPath}
			onClose={handleCloseReceiptViewer}
		/>
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
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 2rem;
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
		.page-header {
			flex-direction: column;
			align-items: flex-start;
		}

		.page-title {
			font-size: 2rem;
		}

		.filter-row {
			grid-template-columns: 1fr;
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

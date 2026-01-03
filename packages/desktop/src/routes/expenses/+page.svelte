<script lang="ts">
import {
	MonthSelector,
	CategoryFilter,
	ExpenseList,
	ExpenseForm,
} from "$features/expenses";
import { ReceiptViewer } from "$features/receipts";
import { expenseStore } from "$lib/stores/expenses.svelte";
import type { Expense } from "$lib/types";

// モーダル状態
let showExpenseForm = $state(false);
let editingExpense = $state<Expense | undefined>(undefined);

// 領収書表示状態
let showReceiptViewer = $state(false);
let currentReceiptUrl = $state<string | undefined>(undefined);
let currentReceiptPath = $state<string | undefined>(undefined);

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

// フォーム成功ハンドラー
function handleFormSuccess() {
	showExpenseForm = false;
	editingExpense = undefined;
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
				<MonthSelector />
			</div>
			<div class="filter-item">
				<span class="filter-label">カテゴリでフィルター</span>
				<CategoryFilter />
			</div>
		</div>
	</div>

	<!-- 経費リスト -->
	<div class="expenses-section">
		<ExpenseList
			onEdit={handleEditExpense}
			onViewReceipt={handleViewReceipt}
		/>
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
					<ExpenseForm expense={editingExpense} onSuccess={handleFormSuccess} onCancel={handleFormCancel} />
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

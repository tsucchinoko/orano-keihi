<script lang="ts">
	import type { Expense } from '$lib/types';

	// Props
	interface Props {
		expense: Expense;
		onEdit: (expense: Expense) => void;
		onDelete: (id: number) => void;
		onViewReceipt?: (receiptPath: string) => void;
	}

	let { expense, onEdit, onDelete, onViewReceipt }: Props = $props();

	// 削除確認ダイアログの状態
	let showDeleteConfirm = $state(false);

	// カテゴリごとのアイコンとカラー
	const categoryConfig: Record<string, { icon: string; colorClass: string }> = {
		'交通費': { icon: '🚗', colorClass: 'bg-category-transport' },
		'飲食費': { icon: '🍽️', colorClass: 'bg-category-meals' },
		'通信費': { icon: '📱', colorClass: 'bg-category-communication' },
		'消耗品費': { icon: '📦', colorClass: 'bg-category-supplies' },
		'接待交際費': { icon: '🤝', colorClass: 'bg-category-entertainment' },
		'その他': { icon: '📋', colorClass: 'bg-category-other' }
	};

	// 日付フォーマット
	function formatDate(dateStr: string): string {
		const date = new Date(dateStr);
		return date.toLocaleDateString('ja-JP', {
			year: 'numeric',
			month: 'long',
			day: 'numeric'
		});
	}

	// 金額フォーマット
	function formatAmount(amount: number): string {
		return new Intl.NumberFormat('ja-JP', {
			style: 'currency',
			currency: 'JPY'
		}).format(amount);
	}

	// 削除確認
	function confirmDelete() {
		showDeleteConfirm = true;
	}

	// 削除実行
	function handleDelete() {
		onDelete(expense.id);
		showDeleteConfirm = false;
	}

	// 削除キャンセル
	function cancelDelete() {
		showDeleteConfirm = false;
	}

	// 領収書表示
	function handleViewReceipt() {
		if (expense.receipt_path && onViewReceipt) {
			onViewReceipt(expense.receipt_path);
		}
	}
</script>

<div class="card hover:shadow-lg transition-shadow duration-200 relative overflow-hidden">
	<!-- カテゴリカラーバー -->
	<div
		class="absolute top-0 left-0 w-1 h-full {categoryConfig[expense.category]?.colorClass || 'bg-category-other'}"
	></div>

	<div class="pl-4">
		<div class="flex items-start justify-between gap-4">
			<!-- 左側：経費情報 -->
			<div class="flex-1">
				<div class="flex items-center gap-2 mb-2">
					<span class="text-2xl">{categoryConfig[expense.category]?.icon || '📋'}</span>
					<span class="font-semibold text-gray-700">{expense.category}</span>
					<span class="text-sm text-gray-500">{formatDate(expense.date)}</span>
				</div>

				<div class="text-2xl font-bold bg-gradient-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent mb-2">
					{formatAmount(expense.amount)}
				</div>

				{#if expense.description}
					<p class="text-gray-600 text-sm mb-2">{expense.description}</p>
				{/if}

				<!-- 領収書サムネイル -->
				{#if expense.receipt_path}
					<button
						type="button"
						onclick={handleViewReceipt}
						class="inline-flex items-center gap-2 text-sm text-blue-600 hover:text-blue-800 transition-colors"
					>
						📎 領収書を表示
					</button>
				{/if}
			</div>

			<!-- 右側：アクションボタン -->
			<div class="flex flex-col gap-2">
				<button
					type="button"
					onclick={() => onEdit(expense)}
					class="btn btn-info text-sm px-3 py-1"
					title="編集"
				>
					✏️ 編集
				</button>
				<button
					type="button"
					onclick={confirmDelete}
					class="btn bg-red-500 hover:bg-red-600 text-white text-sm px-3 py-1"
					title="削除"
				>
					🗑️ 削除
				</button>
			</div>
		</div>
	</div>
</div>

<!-- 削除確認ダイアログ -->
{#if showDeleteConfirm}
	<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
		<div class="card max-w-md mx-4">
			<h3 class="text-xl font-bold mb-4">削除の確認</h3>
			<p class="text-gray-700 mb-6">
				この経費を削除してもよろしいですか？<br />
				この操作は取り消せません。
			</p>
			<div class="flex gap-3">
				<button
					type="button"
					onclick={handleDelete}
					class="btn bg-red-500 hover:bg-red-600 text-white flex-1"
				>
					削除する
				</button>
				<button
					type="button"
					onclick={cancelDelete}
					class="btn bg-gray-300 text-gray-700 flex-1"
				>
					キャンセル
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	/* グラデーションホバー効果 */
	.card:hover::before {
		content: '';
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: linear-gradient(135deg, rgba(102, 126, 234, 0.05) 0%, rgba(118, 75, 162, 0.05) 100%);
		pointer-events: none;
	}
</style>

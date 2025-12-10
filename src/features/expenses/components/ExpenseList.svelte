<script lang="ts">
import type { Expense } from "$lib/types";
import { expenseStore } from "$lib/stores/expenses.svelte";
import { toastStore } from "$lib/stores/toast.svelte";
import ExpenseItem from "./ExpenseItem.svelte";

// Props
interface Props {
	onEdit: (expense: Expense) => void;
	onViewReceipt?: (receiptUrl?: string, receiptPath?: string) => void;
}

let { onEdit, onViewReceipt }: Props = $props();

// ストアから経費データを取得
const expenses = $derived(expenseStore.filteredExpenses);
const selectedMonth = $derived(expenseStore.selectedMonth);

// コンポーネントマウント時に経費を読み込む
$effect(() => {
	expenseStore.loadExpenses();
});

// 日付でグループ化された経費
const groupedExpenses = $derived.by(() => {
	const groups: Record<string, Expense[]> = {};

	for (const expense of expenses) {
		const dateKey = expense.date.split("T")[0];
		if (!groups[dateKey]) {
			groups[dateKey] = [];
		}
		groups[dateKey].push(expense);
	}

	// 日付の降順でソート
	const sortedDates = Object.keys(groups).sort((a, b) => b.localeCompare(a));
	const result: Record<string, Expense[]> = {};
	for (const date of sortedDates) {
		result[date] = groups[date];
	}

	return result;
});

// カテゴリ別合計（ストアから取得）
const categoryTotals = $derived(
	Object.entries(expenseStore.categoryTotals)
		.map(([category, total]) => ({ category, total }))
		.sort((a, b) => b.total - a.total),
);

// 総合計（ストアから取得）
const grandTotal = $derived(expenseStore.monthlyTotal);

// 削除処理
async function handleDelete(id: number): Promise<void> {
	const success = await expenseStore.removeExpense(id);
	if (success) {
		toastStore.success("経費を削除しました");
	} else {
		toastStore.error(expenseStore.error || "経費の削除に失敗しました");
	}
}

// 日付フォーマット
function formatDate(dateStr: string): string {
	const date = new Date(dateStr);
	return date.toLocaleDateString("ja-JP", {
		year: "numeric",
		month: "long",
		day: "numeric",
		weekday: "short",
	});
}

// 金額フォーマット
function formatAmount(amount: number): string {
	return new Intl.NumberFormat("ja-JP", {
		style: "currency",
		currency: "JPY",
	}).format(amount);
}

// カテゴリアイコン
const categoryIcons: Record<string, string> = {
	交通費: "🚗",
	飲食費: "🍽️",
	通信費: "📱",
	消耗品費: "📦",
	接待交際費: "🤝",
	その他: "📋",
};
</script>

<div class="space-y-6">
	<!-- サマリーカード -->
	<div class="card bg-gradient-to-br from-purple-50 to-pink-50">
		<h3 class="text-lg font-bold mb-4">
			{selectedMonth ? `${selectedMonth}の` : ''}経費サマリー
		</h3>

		<!-- カテゴリ別合計 -->
		<div class="space-y-2 mb-4">
			{#each categoryTotals as { category, total }}
				<div class="flex items-center justify-between">
					<span class="text-sm">
						{categoryIcons[category] || '📋'} {category}
					</span>
					<span class="font-semibold">{formatAmount(total)}</span>
				</div>
			{/each}
		</div>

		<!-- 総合計 -->
		<div class="border-t-2 border-purple-200 pt-3 mt-3">
			<div class="flex items-center justify-between">
				<span class="text-lg font-bold">合計</span>
				<span class="text-2xl font-bold bg-gradient-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent">
					{formatAmount(grandTotal)}
				</span>
			</div>
		</div>
	</div>

	<!-- 経費一覧 -->
	{#if expenses.length === 0}
		<div class="card text-center py-12">
			<div class="text-6xl mb-4">📝</div>
			<p class="text-gray-500 text-lg">経費データがありません</p>
			<p class="text-gray-400 text-sm mt-2">新しい経費を追加してください</p>
		</div>
	{:else}
		{#each Object.entries(groupedExpenses) as [date, dayExpenses]}
			<div class="space-y-3">
				<!-- 日付ヘッダー -->
				<div class="flex items-center gap-3">
					<h4 class="text-lg font-bold text-gray-700">{formatDate(date)}</h4>
					<div class="flex-1 h-px bg-gradient-to-r from-purple-300 to-transparent"></div>
					<span class="text-sm font-semibold text-purple-600">
						{formatAmount(dayExpenses.reduce((sum, e) => sum + e.amount, 0))}
					</span>
				</div>

				<!-- その日の経費一覧 -->
				<div class="space-y-3">
					{#each dayExpenses as expense (expense.id)}
						<div class="transition-all duration-200">
							<ExpenseItem
								{expense}
								{onEdit}
								onDelete={handleDelete}
								{onViewReceipt}
							/>
						</div>
					{/each}
				</div>
			</div>
		{/each}
	{/if}
</div>

<style>
	/* スムーズなリスト更新アニメーション */
	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(-10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.space-y-3 > div {
		animation: fadeIn 0.3s ease-out;
	}
</style>

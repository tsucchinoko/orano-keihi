<script lang="ts">
import { expenseStore } from "$lib/stores/expenses.svelte";

// ストアから選択された月を取得
const selectedMonth = $derived(expenseStore.selectedMonth);

// 現在の年月
const currentYear = new Date().getFullYear();
const currentMonth = new Date().getMonth() + 1;

// 選択可能な年のリスト（過去5年から現在まで）
const years = Array.from(
	{ length: 6 },
	(_, i) => currentYear - 5 + i,
).reverse();

// 月のリスト
const months = [
	{ value: 1, label: "1月" },
	{ value: 2, label: "2月" },
	{ value: 3, label: "3月" },
	{ value: 4, label: "4月" },
	{ value: 5, label: "5月" },
	{ value: 6, label: "6月" },
	{ value: 7, label: "7月" },
	{ value: 8, label: "8月" },
	{ value: 9, label: "9月" },
	{ value: 10, label: "10月" },
	{ value: 11, label: "11月" },
	{ value: 12, label: "12月" },
];

// 選択中の年と月を分解（リアクティブに）
const selectedYear = $derived(() => {
	const [year] = selectedMonth.split("-").map(Number);
	return year;
});

const selectedMonthNum = $derived(() => {
	const [, month] = selectedMonth.split("-").map(Number);
	return month;
});

// 年の変更
function handleYearChange(event: Event) {
	const target = event.target as HTMLSelectElement;
	const newYear = target.value;
	const newMonth = String(selectedMonthNum()).padStart(2, "0");
	expenseStore.setSelectedMonth(`${newYear}-${newMonth}`);
}

// 月の変更
function handleMonthChange(event: Event) {
	const target = event.target as HTMLSelectElement;
	const newMonth = String(target.value).padStart(2, "0");
	expenseStore.setSelectedMonth(`${selectedYear()}-${newMonth}`);
}

// 前月へ
function previousMonth() {
	let year = selectedYear();
	let month = selectedMonthNum() - 1;

	if (month < 1) {
		month = 12;
		year -= 1;
	}

	const newMonth = String(month).padStart(2, "0");
	expenseStore.setSelectedMonth(`${year}-${newMonth}`);
}

// 次月へ
function nextMonth() {
	let year = selectedYear();
	let month = selectedMonthNum() + 1;

	if (month > 12) {
		month = 1;
		year += 1;
	}

	const newMonth = String(month).padStart(2, "0");
	expenseStore.setSelectedMonth(`${year}-${newMonth}`);
}

// 今月へ
function goToCurrentMonth() {
	const now = new Date();
	const year = now.getFullYear();
	const month = String(now.getMonth() + 1).padStart(2, "0");
	expenseStore.setSelectedMonth(`${year}-${month}`);
}

// 次月ボタンの無効化判定（未来の月は選択不可）
const isNextDisabled = $derived(() => {
	return selectedYear() === currentYear && selectedMonthNum() >= currentMonth;
});

// 今月かどうか
const isCurrentMonth = $derived(() => {
	return selectedYear() === currentYear && selectedMonthNum() === currentMonth;
});
</script>

<div class="card">
	<div class="flex items-center justify-between gap-4">
		<!-- 前月ボタン -->
		<button
			type="button"
			onclick={previousMonth}
			class="btn btn-info px-3 py-2"
			title="前月"
		>
			◀
		</button>

		<!-- 年月選択 -->
		<div class="flex-1 flex items-center gap-2">
			<select
				value={selectedYear()}
				onchange={handleYearChange}
				class="input flex-1"
			>
				{#each years as year}
					<option value={year}>{year}年</option>
				{/each}
			</select>

			<select
				value={selectedMonthNum()}
				onchange={handleMonthChange}
				class="input flex-1"
			>
				{#each months as month}
					<option value={month.value}>{month.label}</option>
				{/each}
			</select>
		</div>

		<!-- 次月ボタン -->
		<button
			type="button"
			onclick={nextMonth}
			disabled={isNextDisabled()}
			class="btn btn-info px-3 py-2"
			title="次月"
		>
			▶
		</button>
	</div>

	<!-- 今月へ戻るボタン -->
	{#if !isCurrentMonth()}
		<div class="mt-3">
			<button
				type="button"
				onclick={goToCurrentMonth}
				class="btn btn-primary w-full text-sm"
			>
				📅 今月に戻る
			</button>
		</div>
	{/if}

	<!-- 選択中の月を大きく表示 -->
	<div class="mt-4 text-center">
		<p class="text-2xl font-bold bg-gradient-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent">
			{selectedYear()}年 {selectedMonthNum()}月
		</p>
	</div>
</div>

<style>
	/* セレクトボックスのカスタムスタイル */
	select.input {
		cursor: pointer;
		background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 20 20'%3E%3Cpath stroke='%236b7280' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.5' d='M6 8l4 4 4-4'/%3E%3C/svg%3E");
		background-position: right 0.5rem center;
		background-repeat: no-repeat;
		background-size: 1.5em 1.5em;
		padding-right: 2.5rem;
	}

	/* ボタンの無効化スタイル */
	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	button:disabled:hover {
		transform: none;
	}
</style>

<script lang="ts">
import { expenseStore } from "$lib/stores/expenses.svelte";

// ストアから選択されたカテゴリを取得
const selectedCategories = $derived(expenseStore.selectedCategories);

// カテゴリ定義
const categories = [
	{ name: "交通費", icon: "🚗", color: "bg-category-transport" },
	{ name: "飲食費", icon: "🍽️", color: "bg-category-meals" },
	{ name: "通信費", icon: "📱", color: "bg-category-communication" },
	{ name: "消耗品費", icon: "📦", color: "bg-category-supplies" },
	{ name: "接待交際費", icon: "🤝", color: "bg-category-entertainment" },
	{ name: "その他", icon: "📋", color: "bg-category-other" },
];

// チェックボックスの変更ハンドラ
function handleToggle(categoryName: string) {
	const newSelected = selectedCategories.includes(categoryName)
		? selectedCategories.filter((c) => c !== categoryName)
		: [...selectedCategories, categoryName];

	expenseStore.setSelectedCategories(newSelected);
}

// 全選択
function selectAll() {
	expenseStore.setSelectedCategories(categories.map((c) => c.name));
}

// 全解除
function clearAll() {
	expenseStore.setSelectedCategories([]);
}

// 全選択状態かどうか
const isAllSelected = $derived(() => {
	return selectedCategories.length === categories.length;
});
</script>

<div class="card">
	<div class="flex items-center justify-between mb-4">
		<h3 class="text-lg font-bold">カテゴリフィルター</h3>
		<div class="flex gap-2">
			<button
				type="button"
				onclick={selectAll}
				class="text-sm text-purple-600 hover:text-purple-800 font-semibold"
				disabled={isAllSelected()}
			>
				全選択
			</button>
			<span class="text-gray-300">|</span>
			<button
				type="button"
				onclick={clearAll}
				class="text-sm text-purple-600 hover:text-purple-800 font-semibold"
				disabled={selectedCategories.length === 0}
			>
				クリア
			</button>
		</div>
	</div>

	<div class="space-y-2">
		{#each categories as category}
			<label
				class="flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-all duration-200 hover:bg-gray-50 {selectedCategories.includes(category.name) ? 'bg-purple-50' : ''}"
			>
				<input
					type="checkbox"
					checked={selectedCategories.includes(category.name)}
					onchange={() => handleToggle(category.name)}
					class="w-5 h-5 rounded accent-purple-600"
				/>
				
				<div class="flex items-center gap-2 flex-1">
					<span class="text-2xl">{category.icon}</span>
					<span class="font-semibold">{category.name}</span>
				</div>

				<!-- カテゴリカラーインジケーター -->
				<div class="w-4 h-4 rounded-full {category.color}"></div>
			</label>
		{/each}
	</div>

	<!-- 選択中のカテゴリ数表示 -->
	<div class="mt-4 pt-4 border-t border-gray-200">
		<p class="text-sm text-gray-600">
			{#if selectedCategories.length === 0}
				すべてのカテゴリを表示
			{:else if selectedCategories.length === categories.length}
				すべてのカテゴリを選択中
			{:else}
				{selectedCategories.length}件のカテゴリを選択中
			{/if}
		</p>
	</div>
</div>

<style>
	/* チェックボックスのカスタムスタイル */
	input[type="checkbox"]:checked {
		accent-color: #667eea;
	}

	/* ホバー時のグラデーション効果 */
	label:hover {
		background: linear-gradient(135deg, rgba(102, 126, 234, 0.05) 0%, rgba(118, 75, 162, 0.05) 100%);
	}

	/* 選択中のラベル */
	label:has(input:checked) {
		background: linear-gradient(135deg, rgba(102, 126, 234, 0.1) 0%, rgba(118, 75, 162, 0.1) 100%);
		border-left: 3px solid #667eea;
	}

	/* ボタンの無効化スタイル */
	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>

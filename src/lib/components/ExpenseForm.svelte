<script lang="ts">
	import type { Expense, CreateExpenseDto } from '$lib/types';
	import { open } from '@tauri-apps/plugin-dialog';

	// Props
	interface Props {
		expense?: Expense;
		onSave: (expense: CreateExpenseDto, receiptFile?: string) => void;
		onCancel: () => void;
	}

	let { expense, onSave, onCancel }: Props = $props();

	// フォームの状態
	let date = $state(expense?.date.split('T')[0] || new Date().toISOString().split('T')[0]);
	let amount = $state(expense?.amount.toString() || '');
	let category = $state(expense?.category || '');
	let description = $state(expense?.description || '');
	let receiptFile = $state<string | undefined>(undefined);
	let receiptPreview = $state<string | undefined>(expense?.receipt_path);

	// バリデーションエラー
	let errors = $state<Record<string, string>>({});

	// カテゴリ一覧
	const categories = [
		{ name: '交通費', icon: '🚗' },
		{ name: '飲食費', icon: '🍽️' },
		{ name: '通信費', icon: '📱' },
		{ name: '消耗品費', icon: '📦' },
		{ name: '接待交際費', icon: '🤝' },
		{ name: 'その他', icon: '📋' }
	];

	// バリデーション関数
	function validate(): boolean {
		const newErrors: Record<string, string> = {};

		// 金額のバリデーション
		const amountNum = Number.parseFloat(amount);
		if (!amount || Number.isNaN(amountNum)) {
			newErrors.amount = '金額を入力してください';
		} else if (amountNum <= 0) {
			newErrors.amount = '金額は正の数値である必要があります';
		}

		// 日付のバリデーション
		if (!date) {
			newErrors.date = '日付を入力してください';
		} else {
			const selectedDate = new Date(date);
			const today = new Date();
			today.setHours(0, 0, 0, 0);
			if (selectedDate > today) {
				newErrors.date = '未来の日付は選択できません';
			}
		}

		// カテゴリのバリデーション
		if (!category) {
			newErrors.category = 'カテゴリを選択してください';
		}

		errors = newErrors;
		return Object.keys(newErrors).length === 0;
	}

	// 領収書ファイル選択
	async function selectReceipt() {
		try {
			const selected = await open({
				multiple: false,
				filters: [
					{
						name: 'Images',
						extensions: ['png', 'jpg', 'jpeg', 'pdf']
					}
				]
			});

			if (selected && typeof selected === 'string') {
				receiptFile = selected;
				// 画像プレビュー用（PDFの場合はプレビューなし）
				if (selected.match(/\.(png|jpg|jpeg)$/i)) {
					receiptPreview = `file://${selected}`;
				}
			}
		} catch (error) {
			console.error('領収書ファイルの選択に失敗しました:', error);
		}
	}

	// フォーム送信
	function handleSubmit(event: Event) {
		event.preventDefault();
		
		if (!validate()) {
			return;
		}

		const expenseData: CreateExpenseDto = {
			date: new Date(date).toISOString(),
			amount: Number.parseFloat(amount),
			category,
			description: description || undefined
		};

		onSave(expenseData, receiptFile);
	}
</script>

<div class="card max-w-2xl mx-auto">
	<h2 class="text-2xl font-bold mb-6 bg-gradient-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent">
		{expense ? '経費を編集' : '新しい経費を追加'}
	</h2>

	<form onsubmit={handleSubmit} class="space-y-4">
		<!-- 日付入力 -->
		<div>
			<label for="date" class="block text-sm font-semibold mb-2">
				日付 <span class="text-red-500">*</span>
			</label>
			<input
				id="date"
				type="date"
				bind:value={date}
				class="input {errors.date ? 'border-red-500' : ''}"
				max={new Date().toISOString().split('T')[0]}
			/>
			{#if errors.date}
				<p class="text-red-500 text-sm mt-1">{errors.date}</p>
			{/if}
		</div>

		<!-- 金額入力 -->
		<div>
			<label for="amount" class="block text-sm font-semibold mb-2">
				金額 <span class="text-red-500">*</span>
			</label>
			<div class="relative">
				<span class="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500">¥</span>
				<input
					id="amount"
					type="number"
					step="0.01"
					bind:value={amount}
					class="input pl-8 {errors.amount ? 'border-red-500' : ''}"
					placeholder="0"
				/>
			</div>
			{#if errors.amount}
				<p class="text-red-500 text-sm mt-1">{errors.amount}</p>
			{/if}
		</div>

		<!-- カテゴリ選択 -->
		<div>
			<label for="category" class="block text-sm font-semibold mb-2">
				カテゴリ <span class="text-red-500">*</span>
			</label>
			<select
				id="category"
				bind:value={category}
				class="input {errors.category ? 'border-red-500' : ''}"
			>
				<option value="">カテゴリを選択してください</option>
				{#each categories as cat}
					<option value={cat.name}>
						{cat.icon} {cat.name}
					</option>
				{/each}
			</select>
			{#if errors.category}
				<p class="text-red-500 text-sm mt-1">{errors.category}</p>
			{/if}
		</div>

		<!-- 説明入力 -->
		<div>
			<label for="description" class="block text-sm font-semibold mb-2">
				説明
			</label>
			<textarea
				id="description"
				bind:value={description}
				class="input min-h-24"
				placeholder="経費の詳細を入力してください（任意）"
				maxlength="500"
			></textarea>
			<p class="text-gray-500 text-xs mt-1">{description.length}/500文字</p>
		</div>

		<!-- 領収書アップロード -->
		<div>
			<label class="block text-sm font-semibold mb-2">
				領収書
			</label>
			<button
				type="button"
				onclick={selectReceipt}
				class="btn btn-info w-full"
			>
				📎 領収書を選択
			</button>
			{#if receiptPreview}
				<div class="mt-3">
					<p class="text-sm text-gray-600 mb-2">プレビュー:</p>
					<img
						src={receiptPreview}
						alt="領収書プレビュー"
						class="max-w-full h-auto max-h-48 rounded-lg border-2 border-gray-200"
					/>
				</div>
			{:else if receiptFile}
				<p class="text-sm text-gray-600 mt-2">📄 {receiptFile.split('/').pop()}</p>
			{/if}
		</div>

		<!-- ボタン -->
		<div class="flex gap-3 pt-4">
			<button
				type="submit"
				class="btn btn-primary flex-1"
			>
				💾 保存
			</button>
			<button
				type="button"
				onclick={onCancel}
				class="btn bg-gray-300 text-gray-700 flex-1"
			>
				キャンセル
			</button>
		</div>
	</form>
</div>

<style>
	/* グラデーションフォーカス効果 */
	.input:focus {
		border-image: linear-gradient(135deg, #667eea 0%, #764ba2 100%) 1;
	}
</style>

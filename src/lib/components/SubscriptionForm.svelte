<script lang="ts">
	import type { Subscription, CreateSubscriptionDto } from '$lib/types';

	// Props
	interface Props {
		subscription?: Subscription;
		onSave: (subscription: CreateSubscriptionDto) => void;
		onCancel: () => void;
	}

	let { subscription, onSave, onCancel }: Props = $props();

	// フォームの状態
	let name = $state(subscription?.name || '');
	let amount = $state(subscription?.amount.toString() || '');
	let billingCycle = $state<'monthly' | 'annual'>(subscription?.billing_cycle || 'monthly');
	let startDate = $state(subscription?.start_date.split('T')[0] || new Date().toISOString().split('T')[0]);
	let category = $state(subscription?.category || '');

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

		// サービス名のバリデーション
		if (!name.trim()) {
			newErrors.name = 'サービス名を入力してください';
		}

		// 金額のバリデーション
		const amountNum = Number.parseFloat(amount);
		if (!amount || Number.isNaN(amountNum)) {
			newErrors.amount = '金額を入力してください';
		} else if (amountNum <= 0) {
			newErrors.amount = '金額は正の数値である必要があります';
		}

		// 開始日のバリデーション
		if (!startDate) {
			newErrors.startDate = '開始日を入力してください';
		}

		// カテゴリのバリデーション
		if (!category) {
			newErrors.category = 'カテゴリを選択してください';
		}

		errors = newErrors;
		return Object.keys(newErrors).length === 0;
	}

	// フォーム送信
	function handleSubmit(event: Event) {
		event.preventDefault();
		
		if (!validate()) {
			return;
		}

		const subscriptionData: CreateSubscriptionDto = {
			name: name.trim(),
			amount: Number.parseFloat(amount),
			billing_cycle: billingCycle,
			start_date: new Date(startDate).toISOString(),
			category
		};

		onSave(subscriptionData);
	}

	// 月額換算表示
	const monthlyAmount = $derived(() => {
		const amountNum = Number.parseFloat(amount);
		if (Number.isNaN(amountNum)) return 0;
		return billingCycle === 'annual' ? amountNum / 12 : amountNum;
	});
</script>

<div class="card max-w-2xl mx-auto">
	<h2 class="text-2xl font-bold mb-6 bg-gradient-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent">
		{subscription ? 'サブスクリプションを編集' : '新しいサブスクリプションを追加'}
	</h2>

	<form onsubmit={handleSubmit} class="space-y-4">
		<!-- サービス名入力 -->
		<div>
			<label for="name" class="block text-sm font-semibold mb-2">
				サービス名 <span class="text-red-500">*</span>
			</label>
			<input
				id="name"
				type="text"
				bind:value={name}
				class="input {errors.name ? 'border-red-500' : ''}"
				placeholder="例: Netflix, Spotify"
			/>
			{#if errors.name}
				<p class="text-red-500 text-sm mt-1">{errors.name}</p>
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

		<!-- 支払いサイクル選択 -->
		<div>
			<label class="block text-sm font-semibold mb-2">
				支払いサイクル <span class="text-red-500">*</span>
			</label>
			<div class="flex gap-4">
				<label class="flex items-center gap-2 cursor-pointer">
					<input
						type="radio"
						bind:group={billingCycle}
						value="monthly"
						class="w-4 h-4"
					/>
					<span>月払い</span>
				</label>
				<label class="flex items-center gap-2 cursor-pointer">
					<input
						type="radio"
						bind:group={billingCycle}
						value="annual"
						class="w-4 h-4"
					/>
					<span>年払い</span>
				</label>
			</div>
			{#if billingCycle === 'annual' && monthlyAmount() > 0}
				<p class="text-sm text-gray-600 mt-2">
					月額換算: ¥{monthlyAmount().toLocaleString('ja-JP', { maximumFractionDigits: 0 })}
				</p>
			{/if}
		</div>

		<!-- 開始日選択 -->
		<div>
			<label for="startDate" class="block text-sm font-semibold mb-2">
				開始日 <span class="text-red-500">*</span>
			</label>
			<input
				id="startDate"
				type="date"
				bind:value={startDate}
				class="input {errors.startDate ? 'border-red-500' : ''}"
			/>
			{#if errors.startDate}
				<p class="text-red-500 text-sm mt-1">{errors.startDate}</p>
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

	/* ラジオボタンのカスタムスタイル */
	input[type="radio"]:checked {
		accent-color: #667eea;
	}
</style>

<script lang="ts">
import type { Subscription } from "$lib/types";
import { expenseStore } from "$lib/stores/expenses.svelte";
import { toastStore } from "$lib/stores/toast.svelte";
import { saveSubscriptionReceipt } from "$lib/utils/tauri";
import { open } from "@tauri-apps/plugin-dialog";

// Props
interface Props {
	subscription?: Subscription;
	onSuccess: () => void;
	onCancel: () => void;
}

let { subscription, onSuccess, onCancel }: Props = $props();

// フォームの状態
let name = $state(subscription?.name || "");
let amount = $state(subscription?.amount.toString() || "");
let billingCycle = $state<"monthly" | "annual">(
	subscription?.billing_cycle || "monthly",
);
let startDate = $state(
	subscription?.start_date.split("T")[0] ||
		new Date().toISOString().split("T")[0],
);
let category = $state(subscription?.category || "");
let receiptFile = $state<string | undefined>(undefined);
let receiptPreview = $state<string | undefined>(subscription?.receipt_path);

// バリデーションエラー
let errors = $state<Record<string, string>>({});

// カテゴリ一覧
const categories = [
	{ name: "交通費", icon: "🚗" },
	{ name: "飲食費", icon: "🍽️" },
	{ name: "通信費", icon: "📱" },
	{ name: "消耗品費", icon: "📦" },
	{ name: "接待交際費", icon: "🤝" },
	{ name: "その他", icon: "📋" },
];

// バリデーション関数
function validate(): boolean {
	const newErrors: Record<string, string> = {};

	// サービス名のバリデーション
	if (!name.trim()) {
		newErrors.name = "サービス名を入力してください";
	} else if (name.trim().length > 100) {
		newErrors.name = "サービス名は100文字以内で入力してください";
	}

	// 金額のバリデーション
	const amountNum = Number.parseFloat(amount);
	if (!amount || Number.isNaN(amountNum)) {
		newErrors.amount = "金額を入力してください";
	} else if (amountNum <= 0) {
		newErrors.amount = "金額は正の数値である必要があります";
	} else if (amountNum > 9999999999) {
		newErrors.amount = "金額は10桁以内で入力してください";
	}

	// 開始日のバリデーション
	if (!startDate) {
		newErrors.startDate = "開始日を入力してください";
	}

	// カテゴリのバリデーション
	if (!category) {
		newErrors.category = "カテゴリを選択してください";
	}

	errors = newErrors;
	return Object.keys(newErrors).length === 0;
}

// 送信中フラグ
let isSubmitting = $state(false);

// 領収書ファイル選択
async function selectReceipt() {
	try {
		const selected = await open({
			multiple: false,
			filters: [
				{
					name: "画像・PDF",
					extensions: ["png", "jpg", "jpeg", "pdf"],
				},
			],
		});

		if (selected) {
			receiptFile = selected;
			receiptPreview = selected;
		}
	} catch (error) {
		toastStore.error(`ファイル選択エラー: ${error}`);
	}
}

// フォーム送信
async function handleSubmit(event: Event) {
	event.preventDefault();

	if (!validate() || isSubmitting) {
		return;
	}

	isSubmitting = true;

	try {
		const subscriptionData = {
			name: name.trim(),
			amount: Number.parseFloat(amount),
			billing_cycle: billingCycle,
			start_date: new Date(startDate).toISOString(),
			category,
		};

		// サブスクリプションを作成または更新
		let success = false;
		let savedSubscriptionId: number | undefined;

		if (subscription) {
			// 更新
			success = await expenseStore.modifySubscription(
				subscription.id,
				subscriptionData,
			);
			savedSubscriptionId = subscription.id;
		} else {
			// 新規作成
			success = await expenseStore.addSubscription(subscriptionData);
			// 新規作成の場合、最後に追加されたサブスクリプションのIDを取得
			if (success && expenseStore.subscriptions.length > 0) {
				const lastSubscription =
					expenseStore.subscriptions[expenseStore.subscriptions.length - 1];
				savedSubscriptionId = lastSubscription.id;
			}
		}

		if (!success) {
			toastStore.error(
				expenseStore.error || "サブスクリプションの保存に失敗しました",
			);
			return;
		}

		// 領収書ファイルがある場合は保存
		if (receiptFile && savedSubscriptionId) {
			const receiptResult = await saveSubscriptionReceipt(
				savedSubscriptionId,
				receiptFile,
			);
			if (receiptResult.error) {
				toastStore.error(`領収書の保存に失敗しました: ${receiptResult.error}`);
			}
		}

		// 成功メッセージ
		toastStore.success(
			subscription
				? "サブスクリプションを更新しました"
				: "サブスクリプションを追加しました",
		);

		// 成功コールバック
		onSuccess();
	} catch (error) {
		toastStore.error(`エラーが発生しました: ${error}`);
	} finally {
		isSubmitting = false;
	}
}

// 月額換算表示
const monthlyAmount = $derived(() => {
	const amountNum = Number.parseFloat(amount);
	if (Number.isNaN(amountNum)) return 0;
	return billingCycle === "annual" ? amountNum / 12 : amountNum;
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
				maxlength="100"
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
			<fieldset>
				<legend class="block text-sm font-semibold mb-2">
					支払いサイクル <span class="text-red-500">*</span>
				</legend>
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
			</fieldset>
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

		<!-- 領収書アップロード -->
		<div>
			<label class="block text-sm font-semibold mb-2">
				領収書（オプション）
			</label>
			<button
				type="button"
				onclick={selectReceipt}
				class="btn bg-gray-200 text-gray-700 w-full"
			>
				📎 領収書を選択
			</button>
			{#if receiptPreview}
				<div class="mt-2 p-2 bg-gray-50 rounded border border-gray-200">
					<p class="text-sm text-gray-600 truncate">
						📄 {receiptPreview.split('/').pop() || receiptPreview.split('\\').pop()}
					</p>
				</div>
			{/if}
		</div>

		<!-- ボタン -->
		<div class="flex gap-3 pt-4">
			<button
				type="submit"
				class="btn btn-primary flex-1"
				disabled={isSubmitting}
			>
				{isSubmitting ? '保存中...' : '💾 保存'}
			</button>
			<button
				type="button"
				onclick={onCancel}
				class="btn bg-gray-300 text-gray-700 flex-1"
				disabled={isSubmitting}
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

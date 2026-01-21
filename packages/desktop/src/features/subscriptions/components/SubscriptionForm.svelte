<script lang="ts">
import type { Subscription } from "$lib/types";
import { expenseStore } from "$lib/stores/expenses.svelte";
import { toastStore } from "$lib/stores/toast.svelte";
import {
	deleteSubscriptionReceipt,
	getReceiptFromR2,
	uploadSubscriptionReceiptToR2,
	deleteSubscriptionReceiptFromR2,
} from "$lib/utils/tauri";
import { open } from "@tauri-apps/plugin-dialog";

// Props
interface Props {
	subscription?: Subscription;
	onSuccess: () => void;
	onCancel: () => void;
}

let { subscription, onSuccess, onCancel }: Props = $props();

// フォームの状態
let name = $state("");
let amount = $state("");
let billingCycle = $state<"monthly" | "annual">("monthly");
let startDate = $state("");
let category = $state("");
let receiptFile = $state<string | undefined>(undefined);
let receiptPreview = $state<string | undefined>(undefined);
let isLoadingPreview = $state(false);

// フォームの初期化と既存の領収書パス変換
$effect(() => {
	// フォームフィールドの初期化
	if (subscription) {
		name = subscription.name || "";
		amount = subscription.amount.toString() || "";
		billingCycle = subscription.billing_cycle || "monthly";
		startDate =
			subscription.start_date.split("T")[0] ||
			new Date().toISOString().split("T")[0];
		category = subscription.category || "";

		// 既存の領収書を表示
		if (subscription.receipt_path) {
			// HTTPS URLの場合はR2から取得、ローカルパスの場合は変換
			if (subscription.receipt_path.startsWith('https://')) {
				loadReceiptPreview(subscription.receipt_path);
			} else {
				// ローカルパスの場合は変換
				import("@tauri-apps/api/core").then(({ convertFileSrc }) => {
					if (subscription?.receipt_path && !subscription.receipt_path.startsWith('https://')) {
						receiptPreview = convertFileSrc(subscription.receipt_path);
					}
				});
			}
		} else {
			receiptPreview = undefined;
		}
	} else {
		// 新規作成時の初期値
		name = "";
		amount = "";
		billingCycle = "monthly";
		startDate = new Date().toISOString().split("T")[0];
		category = "";
		receiptPreview = undefined;
	}
});

// バリデーションエラー
let errors = $state<Record<string, string>>({});

// 領収書プレビューを読み込む関数
async function loadReceiptPreview(receiptUrl: string) {
	if (!receiptUrl) return;

	// HTTPS URLの場合はR2から取得
	if (receiptUrl.startsWith("https://")) {
		isLoadingPreview = true;
		try {
			const result = await getReceiptFromR2(receiptUrl);
			if (result.data && !result.error) {
				// Base64データをdata URLに変換
				receiptPreview = `data:image/jpeg;base64,${result.data}`;
			} else {
				console.error("領収書の取得に失敗しました:", result.error);
				toastStore.error("領収書の読み込みに失敗しました");
				receiptPreview = undefined;
			}
		} catch (error) {
			console.error("領収書プレビューの読み込みエラー:", error);
			toastStore.error("領収書の読み込み中にエラーが発生しました");
			receiptPreview = undefined;
		} finally {
			isLoadingPreview = false;
		}
	} else {
		// ローカルファイルパスの場合はそのまま設定
		receiptPreview = receiptUrl;
	}
}

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
	} else {
		// YYYY-MM-DD形式の確認
		const dateRegex = /^\d{4}-\d{2}-\d{2}$/;
		if (!dateRegex.test(startDate)) {
			newErrors.startDate = "開始日はYYYY-MM-DD形式で入力してください";
		} else {
			// 日付の妥当性チェック
			const dateObj = new Date(startDate + 'T00:00:00');
			if (isNaN(dateObj.getTime())) {
				newErrors.startDate = "有効な日付を入力してください";
			}
		}
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

		if (selected && typeof selected === "string") {
			receiptFile = selected;
			// 画像プレビュー用（PDFの場合はプレビューなし）
			if (selected.match(/\.(png|jpg|jpeg)$/i)) {
				// Tauriのファイルパスを変換してプレビュー表示
				const { convertFileSrc } = await import("@tauri-apps/api/core");
				receiptPreview = convertFileSrc(selected);
			} else {
				receiptPreview = undefined;
			}
		}
	} catch (error) {
		console.error("領収書ファイルの選択に失敗しました:", error);
		toastStore.error("領収書ファイルの選択に失敗しました");
	}
}

// 領収書削除
async function deleteReceipt() {
	if (!subscription?.id) {
		toastStore.error("サブスクリプションIDが見つかりません");
		return;
	}

	try {
		// R2から領収書を削除
		const r2DeleteResult = await deleteSubscriptionReceiptFromR2(subscription.id);
		if (r2DeleteResult.error) {
			toastStore.error(`R2からの領収書削除に失敗しました: ${r2DeleteResult.error}`);
			return;
		}

		// データベースからも領収書パスを削除
		const dbDeleteResult = await deleteSubscriptionReceipt(subscription.id);
		if (dbDeleteResult.error) {
			toastStore.error(`データベースからの領収書削除に失敗しました: ${dbDeleteResult.error}`);
			return;
		}

		// プレビューとファイル選択をクリア
		receiptPreview = undefined;
		receiptFile = undefined;

		// subscriptionオブジェクトを更新（リアクティブに反映）
		if (subscription) {
			subscription.receipt_path = undefined;
		}

		// ストアを更新して他のコンポーネントにも反映
		await expenseStore.loadSubscriptions();

		toastStore.success("領収書を削除しました");
	} catch (error) {
		console.error("領収書削除エラー:", error);
		toastStore.error("領収書の削除に失敗しました");
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
			start_date: startDate, // YYYY-MM-DD形式のまま送信
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

		// 領収書ファイルがある場合はR2にアップロード
		if (receiptFile && savedSubscriptionId) {
			const uploadResult = await uploadSubscriptionReceiptToR2(
				savedSubscriptionId,
				receiptFile,
			);
			if (uploadResult.error) {
				toastStore.error(`領収書のアップロードに失敗しました: ${uploadResult.error}`);
			} else if (uploadResult.data) {
				// アップロード成功時、サブスクリプションのreceipt_pathを更新
				const updateResult = await expenseStore.modifySubscription(
					savedSubscriptionId,
					{
						receipt_path: uploadResult.data,
					},
				);
				
				if (!updateResult) {
					toastStore.error(`領収書パスの保存に失敗しました`);
				} else {
					// 領収書アップロード成功時、subscriptionオブジェクトを更新
					if (subscription) {
						subscription.receipt_path = uploadResult.data;
					}
					// ストアを更新して最新データを反映
					await expenseStore.loadSubscriptions();
				}
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
	<h2 class="text-2xl font-bold mb-6 bg-linear-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent">
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
			<label for="receipt-upload" class="block text-sm font-semibold mb-2">
				領収書（オプション）
			</label>
			<div class="flex gap-2">
				<button
					id="receipt-upload"
					type="button"
					onclick={selectReceipt}
					class="btn bg-gray-200 text-gray-700 flex-1"
				>
					📎 領収書を選択
				</button>
				{#if (receiptPreview || receiptFile) && subscription}
					<button
						type="button"
						onclick={deleteReceipt}
						class="btn bg-red-500 text-white px-4"
						title="領収書を削除"
					>
						🗑️
					</button>
				{/if}
			</div>
			{#if isLoadingPreview}
				<div class="mt-3">
					<p class="text-sm text-gray-600 mb-2">プレビュー:</p>
					<div class="flex items-center justify-center h-48 bg-gray-100 rounded-lg border-2 border-gray-200">
						<div class="flex flex-col items-center gap-2">
							<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
							<p class="text-sm text-gray-500">領収書を読み込み中...</p>
						</div>
					</div>
				</div>
			{:else if receiptPreview}
				<div class="mt-3">
					<p class="text-sm text-gray-600 mb-2">プレビュー:</p>
					<img
						src={receiptPreview}
						alt="領収書プレビュー"
						class="max-w-full h-auto max-h-48 rounded-lg border-2 border-gray-200"
						onerror={() => {
							console.error('画像の読み込みに失敗しました');
							toastStore.error('画像の表示に失敗しました');
						}}
					/>
				</div>
			{:else if receiptFile}
				<div class="mt-2 p-2 bg-gray-50 rounded border border-gray-200">
					<p class="text-sm text-gray-600 truncate">
						📄 {receiptFile.split('/').pop() || receiptFile.split('\\').pop()}
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

<script lang="ts">
import type { Expense } from "$lib/types";
import { expenseStore } from "$lib/stores/expenses.svelte";
import { toastStore } from "$lib/stores/toast.svelte";
import { uploadReceiptToR2 } from "$lib/utils/tauri";
import { open } from "@tauri-apps/plugin-dialog";

// Props
interface Props {
	expense?: Expense;
	onSuccess: () => void;
	onCancel: () => void;
}

let { expense, onSuccess, onCancel }: Props = $props();

// フォームの状態
let date = $state("");
let amount = $state("");
let category = $state("");
let description = $state("");
let receiptFile = $state<string | undefined>(undefined);
let receiptPreview = $state<string | undefined>(undefined);

// フォームの初期化
$effect(() => {
	if (expense) {
		date = expense.date.split("T")[0] || new Date().toISOString().split("T")[0];
		amount = expense.amount.toString() || "";
		category = expense.category || "";
		description = expense.description || "";
		
		// 既存の領収書を表示
		if (expense.receipt_url) {
			receiptPreview = expense.receipt_url;
		} else if (expense.receipt_path) {
			// 後方互換性：ローカルパスの場合は変換
			import("@tauri-apps/api/core").then(({ convertFileSrc }) => {
				if (expense?.receipt_path) {
					receiptPreview = convertFileSrc(expense.receipt_path);
				}
			});
		}
	} else {
		// 新規作成時の初期値
		date = new Date().toISOString().split("T")[0];
		amount = "";
		category = "";
		description = "";
		receiptFile = undefined;
		receiptPreview = undefined;
	}
});

// バリデーションエラー
let errors = $state<Record<string, string>>({});

// 送信中フラグ
let isSubmitting = $state(false);
let isUploading = $state(false);

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

	// 金額のバリデーション
	const amountNum = Number.parseFloat(amount);
	if (!amount || Number.isNaN(amountNum)) {
		newErrors.amount = "金額を入力してください";
	} else if (amountNum <= 0) {
		newErrors.amount = "金額は正の数値である必要があります";
	} else if (amountNum > 9999999999) {
		newErrors.amount = "金額は10桁以内で入力してください";
	}

	// 日付のバリデーション
	if (!date) {
		newErrors.date = "日付を入力してください";
	} else {
		// YYYY-MM-DD形式の文字列を直接比較
		const today = new Date().toISOString().split("T")[0];
		if (date > today) {
			newErrors.date = "未来の日付は選択できません";
		}
	}

	// カテゴリのバリデーション
	if (!category) {
		newErrors.category = "カテゴリを選択してください";
	}

	// 説明のバリデーション（最大500文字）
	if (description && description.length > 500) {
		newErrors.description = "説明は500文字以内で入力してください";
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
					name: "領収書",
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

			toastStore.success("領収書ファイルを選択しました");
		}
	} catch (error) {
		console.error("領収書ファイルの選択に失敗しました:", error);
		toastStore.error("領収書ファイルの選択に失敗しました");
	}
}

// 領収書削除
function removeReceipt() {
	receiptFile = undefined;
	receiptPreview = undefined;
	toastStore.success("領収書を削除しました");
}

// フォーム送信
async function handleSubmit(event: Event) {
	event.preventDefault();

	if (!validate() || isSubmitting) {
		return;
	}

	isSubmitting = true;

	try {
		const expenseData = {
			date: date, // YYYY-MM-DD形式のまま送信
			amount: Number.parseFloat(amount),
			category,
			description: description || undefined,
		};

		// 経費を作成または更新
		let success = false;
		if (expense) {
			// 更新
			success = await expenseStore.modifyExpense(expense.id, expenseData);
		} else {
			// 新規作成
			success = await expenseStore.addExpense(expenseData);
		}

		if (!success) {
			throw new Error(expenseStore.error || "経費の保存に失敗しました");
		}

		// 領収書がある場合はR2にアップロード
		if (receiptFile && !expense) {
			// 新規作成の場合のみ領収書をアップロード
			// 最後に追加された経費のIDを取得
			const lastExpense = expenseStore.expenses[expenseStore.expenses.length - 1];
			if (lastExpense) {
				isUploading = true;
				try {
					const uploadResult = await uploadReceiptToR2(lastExpense.id, receiptFile);
					if (uploadResult.error) {
						console.warn("領収書アップロードエラー:", uploadResult.error);
						toastStore.warning("経費は保存されましたが、領収書のアップロードに失敗しました");
					} else {
						// 経費データを更新してreceipt_urlを設定
						await expenseStore.modifyExpense(lastExpense.id, {
							receipt_url: uploadResult.data,
						});
						toastStore.success("経費と領収書を保存しました");
					}
				} catch (uploadError) {
					console.warn("領収書アップロードエラー:", uploadError);
					toastStore.warning("経費は保存されましたが、領収書のアップロードに失敗しました");
				} finally {
					isUploading = false;
				}
			}
		} else {
			// 成功メッセージ
			toastStore.success(expense ? "経費を更新しました" : "経費を追加しました");
		}

		// 成功コールバック
		onSuccess();
	} catch (error) {
		console.error("経費保存エラー:", error);
		toastStore.error(error instanceof Error ? error.message : "経費の保存に失敗しました");
	} finally {
		isSubmitting = false;
	}
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
					class="input pl-6 {errors.amount ? 'border-red-500' : ''}"
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
				class="input min-h-24 {errors.description ? 'border-red-500' : ''}"
				placeholder="経費の詳細を入力してください（任意）"
				maxlength="500"
			></textarea>
			<div class="flex justify-between items-center mt-1">
				<p class="text-gray-500 text-xs">{description.length}/500文字</p>
				{#if errors.description}
					<p class="text-red-500 text-xs">{errors.description}</p>
				{/if}
			</div>
		</div>

		<!-- 領収書アップロード -->
		<div>
			<label for="receipt-upload" class="block text-sm font-semibold mb-2">
				領収書（オプション）
			</label>
			
			<div class="flex gap-2 mb-3">
				<button
					id="receipt-upload"
					type="button"
					onclick={selectReceipt}
					class="btn btn-info flex-1"
					disabled={isSubmitting || isUploading}
				>
					📎 領収書を選択
				</button>
				{#if receiptFile || receiptPreview}
					<button
						type="button"
						onclick={removeReceipt}
						class="btn bg-red-500 text-white px-4"
						title="領収書を削除"
						disabled={isSubmitting || isUploading}
					>
						🗑️
					</button>
				{/if}
			</div>

			<!-- プレビュー表示 -->
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
				disabled={isSubmitting || isUploading}
			>
				{#if isSubmitting}
					<span class="flex items-center gap-2">
						<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
						保存中...
					</span>
				{:else if isUploading}
					<span class="flex items-center gap-2">
						<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
						アップロード中...
					</span>
				{:else}
					💾 保存
				{/if}
			</button>
			<button
				type="button"
				onclick={onCancel}
				class="btn bg-gray-300 text-gray-700 flex-1"
				disabled={isSubmitting || isUploading}
			>
				キャンセル
			</button>
		</div>

		<!-- 操作中の注意事項 -->
		{#if isSubmitting || isUploading}
			<div class="mt-3 p-3 bg-yellow-50 rounded-lg border border-yellow-200">
				<div class="flex items-center gap-2">
					<span class="text-yellow-600">⚠️</span>
					<p class="text-sm text-yellow-700">
						{#if isUploading}
							領収書をアップロード中です。ページを閉じたり、ブラウザを更新しないでください。
						{:else}
							経費を保存中です。ページを閉じたり、ブラウザを更新しないでください。
						{/if}
					</p>
				</div>
			</div>
		{/if}
	</form>
</div>

<style>
	/* グラデーションフォーカス効果 */
	.input:focus {
		border-image: linear-gradient(135deg, #667eea 0%, #764ba2 100%) 1;
	}
</style>

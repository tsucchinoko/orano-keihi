<script lang="ts">
import type { Expense } from "$lib/types";
import { expenseStore } from "$lib/stores/expenses.svelte";
import { toastStore } from "$lib/stores/toast.svelte";
import { saveReceipt, deleteReceipt, uploadReceiptToR2, deleteReceiptFromR2, syncCacheOnOnline } from "$lib/utils/tauri";
import { open } from "@tauri-apps/plugin-dialog";

// Props
interface Props {
	expense?: Expense;
	onSuccess: () => void;
	onCancel: () => void;
}

let { expense, onSuccess, onCancel }: Props = $props();

// フォームの状態
let date = $state(
	expense?.date.split("T")[0] || new Date().toISOString().split("T")[0],
);
let amount = $state(expense?.amount.toString() || "");
let category = $state(expense?.category || "");
let description = $state(expense?.description || "");
let receiptFile = $state<string | undefined>(undefined);
let receiptPreview = $state<string | undefined>(undefined);

// 既存の領収書を表示（R2 URLまたはローカルパス）
$effect(() => {
	if (expense?.receipt_url) {
		// R2のHTTPS URLの場合はそのまま使用
		receiptPreview = expense.receipt_url;
	} else if (expense?.receipt_path) {
		// 後方互換性：ローカルパスの場合は変換
		import("@tauri-apps/api/core").then(({ convertFileSrc }) => {
			receiptPreview = convertFileSrc(expense.receipt_path!);
		});
	}
});

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
		}
	} catch (error) {
		console.error("領収書ファイルの選択に失敗しました:", error);
		toastStore.error("領収書ファイルの選択に失敗しました");
	}
}

// 領収書削除（R2対応）
async function deleteReceiptFile() {
	if (!expense?.id) {
		toastStore.error("経費IDが見つかりません");
		return;
	}

	try {
		let result;
		
		// R2 URLがある場合はR2から削除、そうでなければローカルから削除
		if (expense.receipt_url) {
			result = await deleteReceiptFromR2(expense.id);
		} else {
			result = await deleteReceipt(expense.id);
		}

		if (result.error) {
			toastStore.error(`領収書の削除に失敗しました: ${result.error}`);
			return;
		}

		// プレビューとファイル選択をクリア
		receiptPreview = undefined;
		receiptFile = undefined;

		toastStore.success("領収書を削除しました");
	} catch (error) {
		console.error("領収書削除エラー:", error);
		toastStore.error("領収書の削除に失敗しました");
	}
}

// アップロードキャンセル
function cancelUpload() {
	uploadCancelled = true;
	isUploading = false;
	uploadProgress = 0;
	toastStore.info("アップロードをキャンセルしました");
}

// プログレス表示付きR2アップロード
async function uploadReceiptWithProgress(expenseId: number, filePath: string) {
	isUploading = true;
	uploadProgress = 0;
	uploadCancelled = false;

	try {
		// プログレス表示のシミュレーション（実際のプログレスはバックエンドから取得）
		const progressInterval = setInterval(() => {
			if (uploadCancelled) {
				clearInterval(progressInterval);
				return;
			}
			
			if (uploadProgress < 90) {
				uploadProgress += Math.random() * 10;
			}
		}, 200);

		// R2にアップロード
		const result = await uploadReceiptToR2(expenseId, filePath);
		
		clearInterval(progressInterval);

		if (uploadCancelled) {
			return;
		}

		if (result.error) {
			toastStore.error(`領収書のアップロードに失敗しました: ${result.error}`);
			return;
		}

		uploadProgress = 100;
		
		// 経費データを更新してreceipt_urlを設定
		await expenseStore.modifyExpense(expenseId, {
			receipt_url: result.data,
		});

		toastStore.success("領収書をクラウドにアップロードしました");
	} catch (error) {
		console.error("アップロードエラー:", error);
		toastStore.error("領収書のアップロードに失敗しました");
	} finally {
		isUploading = false;
		uploadProgress = 0;
	}
}

// 送信中フラグ
let isSubmitting = $state(false);

// アップロード関連の状態
let isUploading = $state(false);
let uploadProgress = $state(0);
let uploadCancelled = $state(false);

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
			toastStore.error(expenseStore.error || "経費の保存に失敗しました");
			return;
		}

		// 領収書がある場合はR2にアップロード
		if (receiptFile && !expense) {
			// 新規作成の場合のみ領収書をアップロード
			// 最後に追加された経費のIDを取得
			const lastExpense =
				expenseStore.expenses[expenseStore.expenses.length - 1];
			if (lastExpense) {
				await uploadReceiptWithProgress(lastExpense.id, receiptFile);
			}
		}

		// キャッシュ同期を実行（バックグラウンドで）
		syncCacheOnOnline().then((result) => {
			if (result.error) {
				console.warn("キャッシュ同期エラー:", result.error);
			} else {
				console.log("キャッシュ同期完了:", result.data, "個のファイルを処理");
			}
		}).catch((error) => {
			console.warn("キャッシュ同期エラー:", error);
		});

		// 成功メッセージ
		toastStore.success(expense ? "経費を更新しました" : "経費を追加しました");

		// 成功コールバック
		onSuccess();
	} catch (error) {
		toastStore.error(`エラーが発生しました: ${error}`);
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
			<div class="flex gap-2">
				<button
					id="receipt-upload"
					type="button"
					onclick={selectReceipt}
					class="btn btn-info flex-1"
					disabled={isUploading}
				>
					📎 領収書を選択
				</button>
				{#if (receiptPreview || receiptFile) && expense}
					<button
						type="button"
						onclick={deleteReceiptFile}
						class="btn bg-red-500 text-white px-4"
						title="領収書を削除"
						disabled={isUploading}
					>
						🗑️
					</button>
				{/if}
			</div>

			<!-- アップロードプログレス表示 -->
			{#if isUploading}
				<div class="mt-3 p-3 bg-blue-50 rounded-lg border border-blue-200">
					<div class="flex justify-between items-center mb-2">
						<span class="text-sm font-medium text-blue-700">
							クラウドにアップロード中...
						</span>
						<button
							type="button"
							onclick={cancelUpload}
							class="text-xs text-red-600 hover:text-red-800"
						>
							キャンセル
						</button>
					</div>
					<div class="w-full bg-blue-200 rounded-full h-2">
						<div
							class="bg-blue-600 h-2 rounded-full transition-all duration-300"
							style="width: {uploadProgress}%"
						></div>
					</div>
					<div class="text-xs text-blue-600 mt-1">
						{Math.round(uploadProgress)}%
					</div>
				</div>
			{/if}

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
				{isSubmitting ? '保存中...' : isUploading ? 'アップロード中...' : '💾 保存'}
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
	</form>
</div>

<style>
	/* グラデーションフォーカス効果 */
	.input:focus {
		border-image: linear-gradient(135deg, #667eea 0%, #764ba2 100%) 1;
	}
</style>

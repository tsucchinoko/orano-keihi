<script lang="ts">
import type { 
	Expense, 
	UploadProgress, 
	MultipleFileUploadInput, 
	MultipleUploadResult,
	PerformanceStats,
	UserFriendlyError,
	OperationResult
} from "$lib/types";
import { expenseStore } from "$lib/stores/expenses.svelte";
import { toastStore } from "$lib/stores/toast.svelte";
import {
	uploadReceiptToR2,
	deleteReceiptFromR2,
	syncCacheOnOnline,
	uploadMultipleReceiptsToR2,
	getR2PerformanceStats,
} from "$lib/utils/tauri";
import { ErrorHandler, createErrorStore } from "$lib/utils/error-handler";
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

// フォームの初期化とプレビュー設定
$effect(() => {
	// フォームフィールドの初期化
	if (expense) {
		date = expense.date.split("T")[0] || new Date().toISOString().split("T")[0];
		amount = expense.amount.toString() || "";
		category = expense.category || "";
		description = expense.description || "";
		
		// 既存の領収書を表示（R2 URLまたはローカルパス）
		if (expense.receipt_url) {
			// R2のHTTPS URLの場合はそのまま使用
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
		receiptPreview = undefined;
	}
});

// バリデーションエラー
let errors = $state<Record<string, string>>({});

// 統一エラーハンドリング
const errorStore = createErrorStore();
let uploadError = $state<UserFriendlyError | null>(null);

// ヘルパー関数
function getFileType(filePath: string): string {
	const extension = filePath.split('.').pop()?.toLowerCase();
	switch (extension) {
		case 'png': return 'image/png';
		case 'jpg':
		case 'jpeg': return 'image/jpeg';
		case 'pdf': return 'application/pdf';
		default: return 'application/octet-stream';
	}
}

async function getFileSize(filePath: string): Promise<number> {
	try {
		// ファイル拡張子に基づいて推定サイズを返す（実際のプロジェクトではバックエンドAPIを使用）
		const extension = filePath.toLowerCase().split('.').pop();
		switch (extension) {
			case 'pdf':
				return 2 * 1024 * 1024; // 2MB
			case 'png':
			case 'jpg':
			case 'jpeg':
				return 1 * 1024 * 1024; // 1MB
			default:
				return 1024 * 1024; // 1MB
		}
	} catch (error) {
		console.error('ファイルサイズの推定に失敗しました:', error);
		return 1024 * 1024; // 1MB
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

// 領収書ファイル選択（統一エラーハンドリング版）
async function selectReceipt() {
	const result = await ErrorHandler.executeWithErrorHandling(async () => {
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
			// ファイルサイズを取得してFile風オブジェクトを作成
			const fileSize = await getFileSize(selected);
			const fileName = selected.split('/').pop() || selected.split('\\').pop() || 'unknown';
			const fileType = getFileType(selected);
			
			// ファイル検証用のオブジェクトを作成
			const fileObj = {
				name: fileName,
				size: fileSize,
				type: fileType
			} as File;

			// ファイル形式とサイズの検証
			const validation = ErrorHandler.validateFileFormat(fileObj);
			if (!validation.success && validation.error) {
				uploadError = validation.error;
				return;
			}

			receiptFile = selected;
			uploadError = null; // エラーをクリア
			errorStore.clearError();

			// 画像プレビュー用（PDFの場合はプレビューなし）
			if (selected.match(/\.(png|jpg|jpeg)$/i)) {
				// Tauriのファイルパスを変換してプレビュー表示
				const { convertFileSrc } = await import("@tauri-apps/api/core");
				receiptPreview = convertFileSrc(selected);
			} else {
				receiptPreview = undefined;
			}

			// ファイル選択成功を通知
			const sizeMB = (fileSize / (1024 * 1024)).toFixed(1);
			toastStore.success(`ファイルを選択しました: ${fileName} (${sizeMB}MB)`);
		}
	}, "領収書ファイルの選択");

	if (!result.success && result.error) {
		uploadError = result.error;
		toastStore.error(ErrorHandler.formatErrorForDisplay(result.error));
	}
}

// 複数領収書ファイル選択（並列アップロード用）
async function selectMultipleReceipts() {
	try {
		const selected = await open({
			multiple: true,
			filters: [
				{
					name: "領収書",
					extensions: ["png", "jpg", "jpeg", "pdf"],
				},
			],
		});

		if (selected && Array.isArray(selected) && selected.length > 0) {
			// 各ファイルを検証
			const validFiles: string[] = [];
			
			for (const filePath of selected) {
				// ファイル形式の事前検証
				const formatValidation = validateFileFormat(filePath);
				if (!formatValidation.valid) {
					const fileName = filePath.split('/').pop() || filePath.split('\\').pop();
					toastStore.error(`${fileName}: ${formatValidation.error}`);
					continue;
				}

				// ファイルサイズの事前検証
				const fileSize = await getFileSize(filePath);
				const sizeValidation = validateFileSize(fileSize);
				if (!sizeValidation.valid) {
					const fileName = filePath.split('/').pop() || filePath.split('\\').pop();
					toastStore.error(`${fileName}: ${sizeValidation.error}`);
					continue;
				}

				validFiles.push(filePath);
			}

			if (validFiles.length === 0) {
				toastStore.error("有効なファイルがありません");
				return;
			}

			multipleFiles = validFiles;
			multipleUploadResult = null; // 前回の結果をクリア

			toastStore.success(`${validFiles.length}個のファイルを選択しました`);
		}
	} catch (error) {
		console.error("複数ファイルの選択に失敗しました:", error);
		toastStore.error("複数ファイルの選択に失敗しました");
	}
}

// 領収書削除（統一エラーハンドリング版）
async function deleteReceiptFile() {
	if (!expense?.id) {
		const error: UserFriendlyError = {
			title: '削除エラー',
			message: '経費IDが見つかりません。',
			canRetry: false,
			severity: 'error'
		};
		uploadError = error;
		toastStore.error(ErrorHandler.formatErrorForDisplay(error));
		return;
	}

	const result = await ErrorHandler.handleFileDelete(async () => {
		// R2 URLがある場合はR2から削除、そうでなければエラー
		if (expense.receipt_url) {
			const tauriResult = await deleteReceiptFromR2(expense.id);
			
			if (tauriResult.error) {
				throw new Error(tauriResult.error);
			}

			return tauriResult.data || true;
		} else {
			throw new Error("削除対象の領収書が見つかりません");
		}
	}, "領収書");

	if (result.success) {
		// プレビューとファイル選択をクリア
		receiptPreview = undefined;
		receiptFile = undefined;
		uploadError = null;
		errorStore.clearError();

		toastStore.success("領収書を削除しました");
	} else if (result.error) {
		uploadError = result.error;
		errorStore.setError(result.error);
		toastStore.error(ErrorHandler.formatErrorForDisplay(result.error));
	}
}

// アップロードキャンセル
function cancelUpload() {
	uploadCancelled = true;
	isUploading = false;
	uploadProgress = { loaded: 0, total: 0, percentage: 0 };
	uploadError = null;
	toastStore.info("アップロードをキャンセルしました");
}



// ファイル形式を検証する関数
function validateFileFormat(filePath: string): { valid: boolean; error?: string } {
	const allowedExtensions = ['.png', '.jpg', '.jpeg', '.pdf'];
	const extension = filePath.toLowerCase().substring(filePath.lastIndexOf('.'));
	
	if (!allowedExtensions.includes(extension)) {
		return {
			valid: false,
			error: `対応していないファイル形式です。対応形式: ${allowedExtensions.join(', ')}`
		};
	}
	
	return { valid: true };
}

// ファイルサイズを検証する関数（10MB制限）
function validateFileSize(sizeBytes: number): { valid: boolean; error?: string } {
	const maxSizeBytes = 10 * 1024 * 1024; // 10MB
	
	if (sizeBytes > maxSizeBytes) {
		const sizeMB = (sizeBytes / (1024 * 1024)).toFixed(1);
		return {
			valid: false,
			error: `ファイルサイズが大きすぎます（${sizeMB}MB）。10MB以下のファイルを選択してください。`
		};
	}
	
	return { valid: true };
}

// プログレス表示付きR2アップロード（統一エラーハンドリング版）
async function uploadReceiptWithProgressUnified(expenseId: number, filePath: string): Promise<OperationResult<string>> {
	isUploading = true;
	uploadProgress = { loaded: 0, total: 0, percentage: 0 };
	uploadCancelled = false;

	const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'unknown';

	const result = await ErrorHandler.handleFileUpload(async () => {
		// ファイルサイズを取得
		const fileSize = await getFileSize(filePath);
		
		// プログレス表示の初期化
		uploadProgress = { loaded: 0, total: fileSize, percentage: 0 };

		// プログレス表示のシミュレーション（実際のプログレスはバックエンドから取得）
		const progressInterval = setInterval(() => {
			if (uploadCancelled) {
				clearInterval(progressInterval);
				return;
			}

			if (uploadProgress.percentage < 90) {
				const increment = Math.random() * 10;
				const newPercentage = Math.min(uploadProgress.percentage + increment, 90);
				const newLoaded = Math.floor((newPercentage / 100) * fileSize);
				
				uploadProgress = {
					loaded: newLoaded,
					total: fileSize,
					percentage: newPercentage
				};
			}
		}, 200);

		try {
			// R2にアップロード
			const tauriResult = await uploadReceiptToR2(expenseId, filePath);

			clearInterval(progressInterval);

			if (uploadCancelled) {
				throw new Error("アップロードがキャンセルされました");
			}

			if (tauriResult.error) {
				throw new Error(tauriResult.error);
			}

			// アップロード完了
			uploadProgress = { loaded: fileSize, total: fileSize, percentage: 100 };

			// 経費データを更新してreceipt_urlを設定
			const updateSuccess = await expenseStore.modifyExpense(expenseId, {
				receipt_url: tauriResult.data,
			});

			if (!updateSuccess) {
				throw new Error("経費データの更新に失敗しました");
			}

			toastStore.success("領収書をクラウドにアップロードしました");
			
			// プレビューを更新
			receiptPreview = tauriResult.data;
			
			return tauriResult.data || "";
		} finally {
			clearInterval(progressInterval);
		}
	}, fileName);

	isUploading = false;

	// プログレスは成功時は100%のまま、エラー時はリセット
	if (!result.success) {
		uploadProgress = { loaded: 0, total: 0, percentage: 0 };
	}

	return result;
}

// 従来のアップロード関数（後方互換性のため残す）
async function uploadReceiptWithProgress(expenseId: number, filePath: string) {
	const result = await uploadReceiptWithProgressUnified(expenseId, filePath);
	if (!result.success && result.error) {
		uploadError = result.error;
	}
}

// 送信中フラグ
let isSubmitting = $state(false);

// アップロード関連の状態
let isUploading = $state(false);
let uploadProgress = $state<UploadProgress>({ loaded: 0, total: 0, percentage: 0 });
let uploadCancelled = $state(false);

// 並列アップロード関連の状態
let isMultipleUploading = $state(false);
let multipleFiles = $state<string[]>([]);
let multipleUploadResult = $state<MultipleUploadResult | null>(null);
let showPerformanceStats = $state(false);
let performanceStats = $state<PerformanceStats | null>(null);

// フォーム送信（統一エラーハンドリング版）
async function handleSubmit(event: Event) {
	event.preventDefault();

	if (!validate() || isSubmitting) {
		return;
	}

	isSubmitting = true;
	errorStore.clearError();
	uploadError = null;

	const result = await ErrorHandler.executeWithErrorHandling(async () => {
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
				const uploadResult = await uploadReceiptWithProgressUnified(lastExpense.id, receiptFile);
				if (!uploadResult.success && uploadResult.error) {
					uploadError = uploadResult.error;
					// アップロードエラーは経費保存の成功を妨げない
					toastStore.warning("経費は保存されましたが、領収書のアップロードに失敗しました");
				}
			}
		}

		// キャッシュ同期を実行（バックグラウンドで、エラーは無視）
		syncCacheOnOnline()
			.then((result) => {
				if (result.error) {
					console.warn("キャッシュ同期エラー:", result.error);
				} else {
					console.log("キャッシュ同期完了:", result.data, "個のファイルを処理");
				}
			})
			.catch((error) => {
				console.warn("キャッシュ同期エラー:", error);
			});

		return true;
	}, expense ? "経費の更新" : "経費の追加");

	if (result.success) {
		// 成功メッセージ
		toastStore.success(expense ? "経費を更新しました" : "経費を追加しました");
		// 成功コールバック
		onSuccess();
	} else if (result.error) {
		errorStore.setError(result.error);
		toastStore.error(ErrorHandler.formatErrorForDisplay(result.error));
	}

	isSubmitting = false;
}

// 複数ファイルを並列アップロードする
async function uploadMultipleFiles() {
	if (multipleFiles.length === 0) {
		toastStore.error("アップロードするファイルがありません");
		return;
	}

	// 仮の経費IDを使用（実際の実装では、事前に経費を作成するか、一括作成機能を実装）
	const tempExpenseIds = Array.from({ length: multipleFiles.length }, (_, i) => i + 1000);

	const uploadInputs: MultipleFileUploadInput[] = multipleFiles.map((filePath, index) => ({
		expense_id: tempExpenseIds[index],
		file_path: filePath,
	}));

	isMultipleUploading = true;
	multipleUploadResult = null;

	try {
		const result = await uploadMultipleReceiptsToR2(uploadInputs, 3); // 最大3並列

		if (result.error) {
			toastStore.error(`並列アップロードに失敗しました: ${result.error}`);
			return;
		}

		multipleUploadResult = result.data!;

		const { successful_uploads, failed_uploads, total_duration_ms } = result.data!;
		
		toastStore.success(
			`並列アップロード完了: 成功=${successful_uploads}, 失敗=${failed_uploads}, 時間=${total_duration_ms}ms`
		);

	} catch (error) {
		console.error("並列アップロードエラー:", error);
		toastStore.error("並列アップロードに失敗しました");
	} finally {
		isMultipleUploading = false;
	}
}

// パフォーマンス統計を取得する
async function loadPerformanceStats() {
	try {
		const result = await getR2PerformanceStats();

		if (result.error) {
			toastStore.error(`パフォーマンス統計の取得に失敗しました: ${result.error}`);
			return;
		}

		performanceStats = result.data!;
		showPerformanceStats = true;

		toastStore.success("パフォーマンス統計を取得しました");

	} catch (error) {
		console.error("パフォーマンス統計取得エラー:", error);
		toastStore.error("パフォーマンス統計の取得に失敗しました");
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
			
			<!-- 単一ファイルアップロード -->
			<div class="flex gap-2 mb-3">
				<button
					id="receipt-upload"
					type="button"
					onclick={selectReceipt}
					class="btn btn-info flex-1"
					disabled={isUploading || isMultipleUploading}
				>
					📎 領収書を選択
				</button>
				{#if (receiptPreview || receiptFile) && expense}
					<button
						type="button"
						onclick={deleteReceiptFile}
						class="btn bg-red-500 text-white px-4"
						title="領収書を削除"
						disabled={isUploading || isMultipleUploading}
					>
						🗑️
					</button>
				{/if}
			</div>

			<!-- エラー表示 -->
			{#if uploadError}
				<div class="mt-3 p-3 rounded-lg border {ErrorHandler.getErrorCssClass(uploadError.severity)} bg-red-50 border-red-200">
					<div class="flex items-start gap-2">
						<div class="flex-shrink-0">
							{#if uploadError.severity === 'critical'}
								🚨
							{:else if uploadError.severity === 'error'}
								❌
							{:else if uploadError.severity === 'warning'}
								⚠️
							{:else}
								ℹ️
							{/if}
						</div>
						<div class="flex-1">
							<h4 class="font-semibold text-sm text-red-800">{uploadError.title}</h4>
							<p class="text-sm text-red-700 mt-1">{uploadError.message}</p>
							{#if uploadError.actions && uploadError.actions.length > 0}
								<div class="flex gap-2 mt-2">
									{#each uploadError.actions as action}
										<button
											type="button"
											onclick={action.action}
											class="text-xs px-2 py-1 rounded {action.primary ? 'bg-red-600 text-white' : 'bg-red-100 text-red-700'} hover:opacity-80"
										>
											{action.label}
										</button>
									{/each}
								</div>
							{/if}
						</div>
						<button
							type="button"
							onclick={() => { uploadError = null; errorStore.clearError(); }}
							class="flex-shrink-0 text-red-500 hover:text-red-700"
						>
							✕
						</button>
					</div>
				</div>
			{/if}

			<!-- 並列アップロード機能 -->
			<div class="border-t pt-3 mt-3">
				<h4 class="text-sm font-semibold mb-2 text-gray-700">
					🚀 高速並列アップロード（複数ファイル対応）
				</h4>
				
				<div class="flex gap-2 mb-2">
					<button
						type="button"
						onclick={selectMultipleReceipts}
						class="btn bg-purple-500 text-white flex-1"
						disabled={isUploading || isMultipleUploading}
					>
						📁 複数ファイル選択
					</button>
					<button
						type="button"
						onclick={uploadMultipleFiles}
						class="btn bg-green-500 text-white flex-1"
						disabled={isUploading || isMultipleUploading || multipleFiles.length === 0}
					>
						{#if isMultipleUploading}
							<span class="flex items-center gap-2">
								<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
								並列アップロード中...
							</span>
						{:else}
							⚡ 並列アップロード
						{/if}
					</button>
				</div>

				<!-- 選択されたファイル一覧 -->
				{#if multipleFiles.length > 0}
					<div class="bg-gray-50 rounded-lg p-3 mb-3">
						<p class="text-sm font-medium text-gray-700 mb-2">
							選択されたファイル ({multipleFiles.length}個):
						</p>
						<div class="space-y-1 max-h-32 overflow-y-auto">
							{#each multipleFiles as filePath, index}
								<div class="flex items-center justify-between text-xs bg-white rounded px-2 py-1">
									<span class="truncate flex-1">
										📄 {filePath.split('/').pop() || filePath.split('\\').pop()}
									</span>
									<button
										type="button"
										onclick={() => {
											multipleFiles = multipleFiles.filter((_, i) => i !== index);
										}}
										class="text-red-500 hover:text-red-700 ml-2"
										disabled={isMultipleUploading}
									>
										✕
									</button>
								</div>
							{/each}
						</div>
					</div>
				{/if}

				<!-- 並列アップロード結果 -->
				{#if multipleUploadResult}
					<div class="bg-blue-50 rounded-lg p-3 mb-3 border border-blue-200">
						<h5 class="text-sm font-semibold text-blue-800 mb-2">
							📊 アップロード結果
						</h5>
						<div class="grid grid-cols-2 gap-2 text-xs">
							<div class="bg-white rounded px-2 py-1">
								<span class="text-gray-600">総ファイル数:</span>
								<span class="font-medium">{multipleUploadResult.total_files}</span>
							</div>
							<div class="bg-white rounded px-2 py-1">
								<span class="text-gray-600">成功:</span>
								<span class="font-medium text-green-600">{multipleUploadResult.successful_uploads}</span>
							</div>
							<div class="bg-white rounded px-2 py-1">
								<span class="text-gray-600">失敗:</span>
								<span class="font-medium text-red-600">{multipleUploadResult.failed_uploads}</span>
							</div>
							<div class="bg-white rounded px-2 py-1">
								<span class="text-gray-600">処理時間:</span>
								<span class="font-medium">{multipleUploadResult.total_duration_ms}ms</span>
							</div>
						</div>
						
						<!-- 詳細結果 -->
						{#if multipleUploadResult.results.length > 0}
							<details class="mt-2">
								<summary class="text-xs text-blue-700 cursor-pointer hover:text-blue-900">
									詳細結果を表示
								</summary>
								<div class="mt-2 space-y-1 max-h-32 overflow-y-auto">
									{#each multipleUploadResult.results as result}
										<div class="text-xs bg-white rounded px-2 py-1 flex items-center justify-between">
											<span class="truncate flex-1">
												経費ID: {result.expense_id}
											</span>
											<span class="ml-2 {result.success ? 'text-green-600' : 'text-red-600'}">
												{result.success ? '✅' : '❌'}
											</span>
										</div>
									{/each}
								</div>
							</details>
						{/if}
					</div>
				{/if}

				<!-- パフォーマンス統計 -->
				<div class="flex gap-2">
					<button
						type="button"
						onclick={loadPerformanceStats}
						class="btn bg-indigo-500 text-white text-xs px-3 py-1"
						disabled={isUploading || isMultipleUploading}
					>
						📈 パフォーマンス統計
					</button>
					{#if showPerformanceStats}
						<button
							type="button"
							onclick={() => showPerformanceStats = false}
							class="btn bg-gray-400 text-white text-xs px-3 py-1"
						>
							統計を非表示
						</button>
					{/if}
				</div>

				<!-- パフォーマンス統計表示 -->
				{#if showPerformanceStats && performanceStats}
					<div class="bg-indigo-50 rounded-lg p-3 mt-2 border border-indigo-200">
						<h5 class="text-sm font-semibold text-indigo-800 mb-2">
							📈 R2パフォーマンス統計
						</h5>
						<div class="grid grid-cols-2 gap-2 text-xs">
							<div class="bg-white rounded px-2 py-1">
								<span class="text-gray-600">レイテンシ:</span>
								<span class="font-medium">{performanceStats.latency_ms}ms</span>
							</div>
							<div class="bg-white rounded px-2 py-1">
								<span class="text-gray-600">スループット:</span>
								<span class="font-medium">{(performanceStats.throughput_bps / 1024).toFixed(1)}KB/s</span>
							</div>
							<div class="bg-white rounded px-2 py-1">
								<span class="text-gray-600">接続状態:</span>
								<span class="font-medium text-green-600">{performanceStats.connection_status}</span>
							</div>
							<div class="bg-white rounded px-2 py-1">
								<span class="text-gray-600">測定時刻:</span>
								<span class="font-medium text-xs">{new Date(performanceStats.last_measured).toLocaleTimeString()}</span>
							</div>
						</div>
					</div>
				{/if}
			</div>

			<!-- アップロードプログレス表示 -->
			{#if isUploading}
				<div class="mt-3 p-3 bg-blue-50 rounded-lg border border-blue-200">
					<div class="flex justify-between items-center mb-2">
						<span class="text-sm font-medium text-blue-700">
							{#if errorStore.state.isRetrying}
								🔄 再試行中... ({errorStore.state.retryCount}/{errorStore.state.maxRetries})
							{:else}
								📤 クラウドにアップロード中...
							{/if}
						</span>
						<button
							type="button"
							onclick={cancelUpload}
							class="text-xs text-red-600 hover:text-red-800"
							disabled={errorStore.state.isRetrying}
						>
							キャンセル
						</button>
					</div>
					<div class="w-full bg-blue-200 rounded-full h-2">
						<div
							class="bg-blue-600 h-2 rounded-full transition-all duration-300 {errorStore.state.isRetrying ? 'animate-pulse' : ''}"
							style="width: {uploadProgress.percentage}%"
						></div>
					</div>
					<div class="flex justify-between items-center mt-1">
						<div class="text-xs text-blue-600">
							{Math.round(uploadProgress.percentage)}%
						</div>
						{#if uploadProgress.total > 0}
							<div class="text-xs text-blue-600">
								{(uploadProgress.loaded / (1024 * 1024)).toFixed(1)}MB / {(uploadProgress.total / (1024 * 1024)).toFixed(1)}MB
							</div>
						{/if}
					</div>
				</div>
			{/if}

			<!-- アップロードエラー表示 -->
			{#if uploadError}
				<div class="mt-3 p-3 bg-red-50 rounded-lg border border-red-200">
					<div class="flex items-start gap-2">
						<span class="text-red-500 text-sm">⚠️</span>
						<div class="flex-1">
							<p class="text-sm font-medium text-red-700 mb-1">
								アップロードエラー
							</p>
							<p class="text-sm text-red-600">
								{uploadError}
							</p>
						</div>
						<button
							type="button"
							onclick={() => uploadError = null}
							class="text-red-400 hover:text-red-600 text-sm"
						>
							✕
						</button>
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
						{isUploading ? 'ファイルをアップロード中です。' : '経費を保存中です。'}
						ページを閉じたり、ブラウザを更新しないでください。
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

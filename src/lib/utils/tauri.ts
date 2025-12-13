import { invoke } from "@tauri-apps/api/core";
import type {
	Expense,
	CreateExpenseDto,
	UpdateExpenseDto,
	Subscription,
	CreateSubscriptionDto,
	UpdateSubscriptionDto,
	TauriResult,
} from "../types";

/**
 * エラーメッセージをユーザーフレンドリーな形式にフォーマットする
 *
 * @param error - エラーオブジェクトまたはメッセージ
 * @returns フォーマットされたエラーメッセージ
 */
export function formatErrorMessage(error: unknown): string {
	if (typeof error === "string") {
		return error;
	}

	if (error instanceof Error) {
		return error.message;
	}

	// オブジェクトの場合、JSONとして表示
	if (typeof error === "object" && error !== null) {
		try {
			return JSON.stringify(error);
		} catch {
			return "不明なエラーが発生しました";
		}
	}

	return "不明なエラーが発生しました";
}

/**
 * Tauriコマンドのエラーハンドリングラッパー
 *
 * @param command - 実行するTauriコマンドのPromise
 * @returns データまたはエラーメッセージを含むオブジェクト
 */
export async function handleTauriCommand<T>(
	command: Promise<T>,
): Promise<TauriResult<T>> {
	try {
		const data = await command;
		return { data };
	} catch (error) {
		console.error("Tauriコマンドエラー:", error);
		const errorMessage = formatErrorMessage(error);
		return { error: errorMessage };
	}
}

// ========================================
// 経費関連のコマンド
// ========================================

/**
 * 新しい経費を作成する
 *
 * @param expense - 作成する経費データ
 * @returns 作成された経費データまたはエラー
 */
export async function createExpense(
	expense: CreateExpenseDto,
): Promise<TauriResult<Expense>> {
	return handleTauriCommand(
		invoke<Expense>("create_expense", { dto: expense }),
	);
}

/**
 * 経費一覧を取得する
 *
 * @param month - フィルタする月（オプション、YYYY-MM形式）
 * @param category - フィルタするカテゴリ（オプション）
 * @returns 経費一覧またはエラー
 */
export async function getExpenses(
	month?: string,
	category?: string,
): Promise<TauriResult<Expense[]>> {
	return handleTauriCommand(
		invoke<Expense[]>("get_expenses", { month, category }),
	);
}

/**
 * 経費を更新する
 *
 * @param id - 更新する経費のID
 * @param expense - 更新データ
 * @returns 更新された経費データまたはエラー
 */
export async function updateExpense(
	id: number,
	expense: UpdateExpenseDto,
): Promise<TauriResult<Expense>> {
	return handleTauriCommand(
		invoke<Expense>("update_expense", { id, dto: expense }),
	);
}

/**
 * 経費を削除する
 *
 * @param id - 削除する経費のID
 * @returns 成功またはエラー
 */
export async function deleteExpense(id: number): Promise<TauriResult<void>> {
	return handleTauriCommand(invoke<void>("delete_expense", { id }));
}

/**
 * 領収書ファイルを保存する
 *
 * @param expenseId - 経費ID
 * @param filePath - 保存するファイルのパス
 * @returns 保存されたファイルパスまたはエラー
 */
export async function saveReceipt(
	expenseId: number,
	filePath: string,
): Promise<TauriResult<string>> {
	return handleTauriCommand(
		invoke<string>("save_receipt", { expenseId, filePath }),
	);
}

// ========================================
// サブスクリプション関連のコマンド
// ========================================

/**
 * 新しいサブスクリプションを作成する
 *
 * @param subscription - 作成するサブスクリプションデータ
 * @returns 作成されたサブスクリプションデータまたはエラー
 */
export async function createSubscription(
	subscription: CreateSubscriptionDto,
): Promise<TauriResult<Subscription>> {
	return handleTauriCommand(
		invoke<Subscription>("create_subscription", { dto: subscription }),
	);
}

/**
 * サブスクリプション一覧を取得する
 *
 * @param activeOnly - アクティブなサブスクリプションのみ取得するか
 * @returns サブスクリプション一覧またはエラー
 */
export async function getSubscriptions(
	activeOnly: boolean = false,
): Promise<TauriResult<Subscription[]>> {
	return handleTauriCommand(
		invoke<Subscription[]>("get_subscriptions", { activeOnly }),
	);
}

/**
 * サブスクリプションを更新する
 *
 * @param id - 更新するサブスクリプションのID
 * @param subscription - 更新データ
 * @returns 更新されたサブスクリプションデータまたはエラー
 */
export async function updateSubscription(
	id: number,
	subscription: UpdateSubscriptionDto,
): Promise<TauriResult<Subscription>> {
	return handleTauriCommand(
		invoke<Subscription>("update_subscription", { id, dto: subscription }),
	);
}

/**
 * サブスクリプションのアクティブ状態を切り替える
 *
 * @param id - 切り替えるサブスクリプションのID
 * @returns 更新されたサブスクリプションデータまたはエラー
 */
export async function toggleSubscriptionStatus(
	id: number,
): Promise<TauriResult<Subscription>> {
	return handleTauriCommand(
		invoke<Subscription>("toggle_subscription_status", { id }),
	);
}

/**
 * 月額サブスクリプション合計を取得する
 *
 * @returns 月額合計金額またはエラー
 */
export async function getMonthlySubscriptionTotal(): Promise<
	TauriResult<number>
> {
	return handleTauriCommand(invoke<number>("get_monthly_subscription_total"));
}

/**
 * サブスクリプションの領収書ファイルを保存する
 *
 * @param subscriptionId - サブスクリプションID
 * @param filePath - 保存するファイルのパス
 * @returns 保存されたファイルパスまたはエラー
 */
export async function saveSubscriptionReceipt(
	subscriptionId: number,
	filePath: string,
): Promise<TauriResult<string>> {
	return handleTauriCommand(
		invoke<string>("save_subscription_receipt", { subscriptionId, filePath }),
	);
}
/**
 * 経費の領収書を削除する
 *
 * @param expenseId - 経費ID
 * @returns 成功またはエラー
 */
export async function deleteReceipt(
	expenseId: number,
): Promise<TauriResult<boolean>> {
	return handleTauriCommand(invoke<boolean>("delete_receipt", { expenseId }));
}

/**
 * サブスクリプションの領収書を削除する
 *
 * @param subscriptionId - サブスクリプションID
 * @returns 成功またはエラー
 */
export async function deleteSubscriptionReceipt(
	subscriptionId: number,
): Promise<TauriResult<boolean>> {
	return handleTauriCommand(
		invoke<boolean>("delete_subscription_receipt", { subscriptionId }),
	);
}

// ========================================
// R2領収書関連のコマンド
// ========================================

/**
 * 領収書ファイルをR2にアップロードする
 *
 * @param expenseId - 経費ID
 * @param filePath - アップロードするファイルのパス
 * @returns アップロードされたHTTPS URLまたはエラー
 */
export async function uploadReceiptToR2(
	expenseId: number,
	filePath: string,
): Promise<TauriResult<string>> {
	return handleTauriCommand(
		invoke<string>("upload_receipt_to_r2", { expenseId, filePath }),
	);
}

/**
 * R2から領収書を取得する
 *
 * @param receiptUrl - 領収書のHTTPS URL
 * @returns Base64エンコードされたファイルデータまたはエラー
 */
export async function getReceiptFromR2(
	receiptUrl: string,
): Promise<TauriResult<string>> {
	return handleTauriCommand(
		invoke<string>("get_receipt_from_r2", { receiptUrl }),
	);
}

/**
 * R2から領収書を削除する
 *
 * @param expenseId - 経費ID
 * @returns 成功またはエラー
 */
export async function deleteReceiptFromR2(
	expenseId: number,
): Promise<TauriResult<boolean>> {
	return handleTauriCommand(
		invoke<boolean>("delete_receipt_from_r2", { expenseId }),
	);
}

/**
 * R2接続をテストする
 *
 * @returns 接続成功またはエラー
 */
export async function testR2Connection(): Promise<TauriResult<boolean>> {
	return handleTauriCommand(invoke<boolean>("test_r2_connection"));
}

// ========================================
// キャッシュ関連のコマンド
// ========================================

/**
 * オフライン時に領収書をキャッシュから取得する
 *
 * @param receiptUrl - 領収書のHTTPS URL
 * @returns Base64エンコードされたキャッシュファイルデータまたはエラー
 */
export async function getReceiptOffline(
	receiptUrl: string,
): Promise<TauriResult<string>> {
	return handleTauriCommand(
		invoke<string>("get_receipt_offline", { receiptUrl }),
	);
}

/**
 * オンライン復帰時にキャッシュを同期する
 *
 * @returns 同期されたキャッシュ数またはエラー
 */
export async function syncCacheOnOnline(): Promise<TauriResult<number>> {
	return handleTauriCommand(invoke<number>("sync_cache_on_online"));
}

/**
 * キャッシュ統計情報を取得する
 *
 * @returns キャッシュ統計情報またはエラー
 */
export async function getCacheStats(): Promise<
	TauriResult<import("../types").CacheStats>
> {
	return handleTauriCommand(
		invoke<import("../types").CacheStats>("get_cache_stats"),
	);
}

// ========================================
// 並列処理とパフォーマンス関連のコマンド
// ========================================

/**
 * 複数ファイルを並列でR2にアップロードする
 *
 * @param files - アップロードするファイルのリスト
 * @param maxConcurrent - 最大同時実行数（オプション、デフォルト: 3）
 * @returns アップロード結果またはエラー
 */
export async function uploadMultipleReceiptsToR2(
	files: import("../types").MultipleFileUploadInput[],
	maxConcurrent?: number,
): Promise<TauriResult<import("../types").MultipleUploadResult>> {
	return handleTauriCommand(
		invoke<import("../types").MultipleUploadResult>(
			"upload_multiple_receipts_to_r2",
			{ files, maxConcurrent },
		),
	);
}

/**
 * アップロードをキャンセルする
 *
 * @param uploadId - アップロードID
 * @returns キャンセル成功またはエラー
 */
export async function cancelUpload(uploadId: string): Promise<TauriResult<boolean>> {
	return handleTauriCommand(invoke<boolean>("cancel_upload", { uploadId }));
}

/**
 * R2パフォーマンス統計を取得する
 *
 * @returns パフォーマンス統計またはエラー
 */
export async function getR2PerformanceStats(): Promise<
	TauriResult<import("../types").PerformanceStats>
> {
	return handleTauriCommand(
		invoke<import("../types").PerformanceStats>("get_r2_performance_stats"),
	);
}

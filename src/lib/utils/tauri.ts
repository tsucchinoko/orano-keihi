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

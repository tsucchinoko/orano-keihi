// 経費データモデル
export interface Expense {
	id: number;
	date: string; // ISO 8601形式
	amount: number;
	category: string;
	description?: string;
	receipt_path?: string;
	created_at: string;
	updated_at: string;
}

// 経費作成用DTO
export interface CreateExpenseDto {
	date: string;
	amount: number;
	category: string;
	description?: string;
}

// 経費更新用DTO
export interface UpdateExpenseDto {
	date?: string;
	amount?: number;
	category?: string;
	description?: string;
}

// サブスクリプションデータモデル
export interface Subscription {
	id: number;
	name: string;
	amount: number;
	billing_cycle: "monthly" | "annual";
	start_date: string;
	category: string;
	is_active: boolean;
	receipt_path?: string;
	created_at: string;
	updated_at: string;
}

// サブスクリプション作成用DTO
export interface CreateSubscriptionDto {
	name: string;
	amount: number;
	billing_cycle: "monthly" | "annual";
	start_date: string;
	category: string;
}

// サブスクリプション更新用DTO
export interface UpdateSubscriptionDto {
	name?: string;
	amount?: number;
	billing_cycle?: "monthly" | "annual";
	start_date?: string;
	category?: string;
}

// カテゴリデータモデル
export interface Category {
	id: number;
	name: string;
	color: string;
	icon?: string;
}

// 月別合計データモデル
export interface MonthlyTotal {
	category: string;
	total: number;
}

// Tauriコマンドのレスポンス型
export interface TauriResult<T> {
	data?: T;
	error?: string;
}

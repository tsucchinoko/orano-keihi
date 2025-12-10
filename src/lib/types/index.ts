// 経費データモデル
export interface Expense {
	id: number;
	date: string; // ISO 8601形式
	amount: number;
	category: string;
	description?: string;
	receipt_path?: string; // 後方互換性のため残す
	receipt_url?: string; // R2対応の新しいフィールド
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

// アップロードプログレス型
export interface UploadProgress {
	loaded: number;
	total: number;
	percentage: number;
}

// R2設定型
export interface R2Config {
	account_id: string;
	access_key: string;
	secret_key: string;
	bucket_name: string;
	endpoint?: string;
}

// 領収書キャッシュ型
export interface ReceiptCache {
	id: number;
	receipt_url: string;
	local_path: string;
	cached_at: string;
	file_size: number;
	last_accessed: string;
}

// キャッシュ統計情報型
export interface CacheStats {
	total_files: number;
	total_size_bytes: number;
	max_size_bytes: number;
	cache_hit_rate: number;
}

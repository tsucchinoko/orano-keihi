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

// ファイル検証結果型
export interface FileValidationResult {
	valid: boolean;
	error?: string;
}

// アップロード状態型
export interface UploadState {
	isUploading: boolean;
	progress: UploadProgress;
	error: string | null;
	cancelled: boolean;
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

// セキュリティ関連型
export interface SystemDiagnosticInfo {
	environment: string;
	debug_mode: string;
	log_level: string;
	credential_r2_account_id?: string;
	credential_r2_access_key?: string;
	credential_r2_bucket_name?: string;
	rust_version: string;
	target_arch: string;
	target_os: string;
}

// 環境情報型
export interface EnvironmentInfo {
	environment: string;
	debug_mode: string;
	log_level: string;
	is_production: string;
	is_development: string;
}

// R2診断情報型
export interface R2DiagnosticInfo {
	bucket_name: string;
	endpoint_url: string;
	region: string;
	config_account_id?: string;
	config_access_key?: string;
	config_bucket_name?: string;
}

// セキュリティイベント型
export interface SecurityEvent {
	event_type: string;
	details: string;
	timestamp?: string;
}

// 並列アップロード関連型
export interface MultipleFileUploadInput {
	expense_id: number;
	file_path: string;
}

export interface SingleUploadResult {
	expense_id: number;
	success: boolean;
	url?: string;
	error?: string;
	file_size: number;
	duration_ms: number;
}

export interface MultipleUploadResult {
	total_files: number;
	successful_uploads: number;
	failed_uploads: number;
	results: SingleUploadResult[];
	total_duration_ms: number;
}

export interface UploadProgressEvent {
	file_index: number;
	file_key: string;
	status: "Started" | "InProgress" | "Completed" | "Failed" | "Cancelled";
	bytes_uploaded: number;
	total_bytes: number;
	speed_bps: number;
}

export interface PerformanceStats {
	latency_ms: number;
	throughput_bps: number;
	connection_status: string;
	last_measured: string;
}

// 並列アップロード設定型
export interface ParallelUploadConfig {
	max_concurrent: number;
	enable_progress: boolean;
	enable_cancel: boolean;
}

// エラーハンドリング関連型
export interface AppError {
	type:
		| "R2ConnectionFailed"
		| "UploadFailed"
		| "DownloadFailed"
		| "FileNotFound"
		| "InvalidCredentials"
		| "NetworkError"
		| "FileOperationError"
		| "InvalidFileFormat"
		| "FileSizeError"
		| "DatabaseError"
		| "ConfigError"
		| "CacheError"
		| "InternalError";
	details: string;
	user_message: string;
	retry_possible: boolean;
	severity: "Low" | "Medium" | "High" | "Critical";
}

export interface ErrorState {
	hasError: boolean;
	error: AppError | null;
	isRetrying: boolean;
	retryCount: number;
	maxRetries: number;
}

export interface UserFriendlyError {
	title: string;
	message: string;
	canRetry: boolean;
	severity: "info" | "warning" | "error" | "critical";
	actions?: ErrorAction[];
}

export interface ErrorAction {
	label: string;
	action: () => void;
	primary?: boolean;
}

// 操作結果型
export interface OperationResult<T = void> {
	success: boolean;
	data?: T;
	error?: UserFriendlyError;
}

// リトライ設定型
export interface RetryConfig {
	maxRetries: number;
	baseDelay: number; // ミリ秒
	maxDelay: number; // ミリ秒
	exponentialBackoff: boolean;
}

// 統合テストとデバッグ機能関連型

// テストステップ結果型
export interface TestStepResult {
	success: boolean;
	message: string;
	duration_ms: number;
	details?: string;
}

// R2接続詳細テスト結果型
export interface R2ConnectionTestResult {
	overall_success: boolean;
	config_validation: TestStepResult;
	client_initialization: TestStepResult;
	bucket_access: TestStepResult;
	upload_test: TestStepResult;
	download_test: TestStepResult;
	delete_test: TestStepResult;
	performance_metrics?: PerformanceStats;
	total_duration_ms: number;
	environment: string;
}

// R2使用量監視情報型
export interface R2UsageInfo {
	total_files: number;
	estimated_storage_mb: number;
	monthly_uploads: number;
	daily_uploads: number;
	cache_stats?: CacheStats;
	bucket_name: string;
	region: string;
	last_updated: string;
	cost_estimate_usd: number;
}

// R2デバッグ情報型
export interface R2DebugInfo {
	environment_variables: Record<string, string>;
	r2_config?: Record<string, string>;
	system_info: Record<string, string>;
	database_stats: Record<string, string>;
	recent_errors: string[];
	timestamp: string;
}

// ========================================
// 認証関連型
// ========================================

// ユーザー情報型
export interface User {
	id: number;
	google_id: string;
	email: string;
	name: string;
	picture_url?: string;
	created_at: string;
	updated_at: string;
}

// 認証状態型
export interface AuthState {
	user: User | null;
	is_authenticated: boolean;
	is_loading: boolean;
}

// OAuth認証開始レスポンス型
export interface StartAuthResponse {
	auth_url: string;
	code_verifier: string;
	state: string;
}

// 認証コールバック処理リクエスト型
export interface HandleCallbackRequest {
	code: string;
	state: string;
	code_verifier: string;
}

// 認証コールバック処理レスポンス型
export interface HandleCallbackResponse {
	user: User;
	session_token: string;
}

// セッション検証レスポンス型
export interface ValidateSessionResponse {
	user: User;
	is_authenticated: boolean;
}

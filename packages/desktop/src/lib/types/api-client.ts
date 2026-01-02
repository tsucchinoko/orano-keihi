// APIサーバー経由でのファイルアップロード関連の型定義

export interface ApiUploadResponse {
  success: boolean;
  fileUrl?: string;
  fileKey: string;
  fileSize: number;
  contentType: string;
  uploadedAt: string;
  error?: string;
}

export interface ApiMultipleUploadResponse {
  totalFiles: number;
  successfulUploads: number;
  failedUploads: number;
  results: ApiUploadResponse[];
  totalDurationMs: number;
}

export interface ApiErrorResponse {
  error: {
    code: string;
    message: string;
    details?: any;
    timestamp: string;
    requestId: string;
  };
}

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

// フォールバック機能関連の型定義
export interface SyncResult {
  total_files: number;
  successful_syncs: number;
  failed_syncs: number;
  results: SyncFileResult[];
}

export interface SyncFileResult {
  expense_id: number;
  original_path: string;
  success: boolean;
  new_url?: string;
  error?: string;
  duration_ms: number;
}

export interface HealthCheckResult {
  is_healthy: boolean;
  response_time_ms: number;
  status_code: number;
  error_message?: string;
  details?: any;
}

// APIサーバー経由でのファイルアップロード関数（エラーハンドリング強化版）
export async function uploadReceiptViaApi(
  expenseId: number,
  filePath: string
): Promise<string> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('upload_receipt_via_api', {
    expenseId,
    filePath,
  });
}

// APIサーバー経由での複数ファイル並列アップロード関数
export async function uploadMultipleReceiptsViaApi(
  files: MultipleFileUploadInput[],
  maxConcurrent?: number
): Promise<MultipleUploadResult> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('upload_multiple_receipts_via_api', {
    files,
    maxConcurrent,
  });
}

// APIサーバーのヘルスチェック関数
export async function checkApiServerHealth(): Promise<boolean> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('check_api_server_health');
}

// APIサーバーの詳細ヘルスチェック関数
export async function checkApiServerHealthDetailed(): Promise<HealthCheckResult> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('check_api_server_health_detailed');
}

// フォールバック状態のファイルを同期する関数
export async function syncFallbackFiles(): Promise<SyncResult> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('sync_fallback_files');
}

// フォールバック状態のファイル数を取得する関数
export async function getFallbackFileCount(): Promise<number> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('get_fallback_file_count');
}

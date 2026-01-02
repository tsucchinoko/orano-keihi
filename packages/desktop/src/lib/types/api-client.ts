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

// APIサーバー経由でのファイルアップロード関数
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

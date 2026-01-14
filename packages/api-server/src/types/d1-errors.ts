/**
 * D1データベース用のエラー型定義
 */

/**
 * エラーコード
 */
export enum ErrorCode {
  // 認証エラー
  UNAUTHORIZED = "UNAUTHORIZED", // 401
  FORBIDDEN = "FORBIDDEN", // 403

  // データエラー
  NOT_FOUND = "NOT_FOUND", // 404
  VALIDATION_ERROR = "VALIDATION_ERROR", // 400
  CONFLICT = "CONFLICT", // 409

  // データベースエラー
  DATABASE_ERROR = "DATABASE_ERROR", // 500
  CONNECTION_ERROR = "CONNECTION_ERROR", // 503

  // その他
  INTERNAL_ERROR = "INTERNAL_ERROR", // 500
}

/**
 * エラーレスポンス型
 */
export interface ErrorResponse {
  error: {
    code: ErrorCode;
    message: string;
    details?: any;
    timestamp: string;
    requestId: string;
  };
}

/**
 * エラーレスポンスを作成するヘルパー関数
 */
export function createErrorResponse(
  code: ErrorCode,
  message: string,
  requestId: string,
  details?: any,
): ErrorResponse {
  return {
    error: {
      code,
      message,
      details,
      timestamp: new Date().toISOString(),
      requestId,
    },
  };
}

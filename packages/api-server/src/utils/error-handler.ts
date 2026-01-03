/**
 * エラーハンドリングシステム
 * 構造化エラーレスポンス、エラー分類、HTTPステータスコードマッピングを提供
 */

import type { Context } from "hono";
import { logger, alertSystem, AlertLevel } from "./logger.js";

/**
 * エラーコード定義
 */
export enum ErrorCode {
  // バリデーションエラー（400番台）
  BAD_REQUEST = "BAD_REQUEST",
  VALIDATION_ERROR = "VALIDATION_ERROR",
  MISSING_FILE = "MISSING_FILE",
  INVALID_EXPENSE_ID = "INVALID_EXPENSE_ID",
  INVALID_FILE_TYPE = "INVALID_FILE_TYPE",
  FILE_TOO_LARGE = "FILE_TOO_LARGE",
  INVALID_REQUEST_FORMAT = "INVALID_REQUEST_FORMAT",

  // 認証・認可エラー（401, 403）
  UNAUTHORIZED = "UNAUTHORIZED",
  INVALID_TOKEN = "INVALID_TOKEN",
  TOKEN_EXPIRED = "TOKEN_EXPIRED",
  FORBIDDEN = "FORBIDDEN",
  INSUFFICIENT_PERMISSIONS = "INSUFFICIENT_PERMISSIONS",

  // リソースエラー（404）
  NOT_FOUND = "NOT_FOUND",

  // レート制限エラー（429）
  RATE_LIMIT_EXCEEDED = "RATE_LIMIT_EXCEEDED",

  // サーバーエラー（500番台）
  INTERNAL_SERVER_ERROR = "INTERNAL_SERVER_ERROR",
  R2_CONNECTION_ERROR = "R2_CONNECTION_ERROR",
  R2_UPLOAD_ERROR = "R2_UPLOAD_ERROR",
  DATABASE_ERROR = "DATABASE_ERROR",
  SERVICE_UNAVAILABLE = "SERVICE_UNAVAILABLE",
  GATEWAY_TIMEOUT = "GATEWAY_TIMEOUT",

  // ファイル処理エラー
  UPLOAD_FAILED = "UPLOAD_FAILED",
  DELETE_FAILED = "DELETE_FAILED",
  FILE_NOT_FOUND = "FILE_NOT_FOUND",
  PRESIGNED_URL_ERROR = "PRESIGNED_URL_ERROR",
}

/**
 * エラー分類
 */
export enum ErrorCategory {
  VALIDATION = "validation",
  AUTHENTICATION = "authentication",
  AUTHORIZATION = "authorization",
  RATE_LIMIT = "rate_limit",
  SERVER = "server",
  EXTERNAL_SERVICE = "external_service",
  FILE_PROCESSING = "file_processing",
}

/**
 * HTTPステータスコードマッピング
 */
export const ERROR_STATUS_MAP: Record<ErrorCode, number> = {
  [ErrorCode.BAD_REQUEST]: 400,
  [ErrorCode.VALIDATION_ERROR]: 400,
  [ErrorCode.MISSING_FILE]: 400,
  [ErrorCode.INVALID_EXPENSE_ID]: 400,
  [ErrorCode.INVALID_FILE_TYPE]: 415,
  [ErrorCode.FILE_TOO_LARGE]: 413,
  [ErrorCode.INVALID_REQUEST_FORMAT]: 400,

  [ErrorCode.UNAUTHORIZED]: 401,
  [ErrorCode.INVALID_TOKEN]: 401,
  [ErrorCode.TOKEN_EXPIRED]: 401,
  [ErrorCode.FORBIDDEN]: 403,
  [ErrorCode.INSUFFICIENT_PERMISSIONS]: 403,

  [ErrorCode.NOT_FOUND]: 404,

  [ErrorCode.RATE_LIMIT_EXCEEDED]: 429,

  [ErrorCode.INTERNAL_SERVER_ERROR]: 500,
  [ErrorCode.R2_CONNECTION_ERROR]: 502,
  [ErrorCode.R2_UPLOAD_ERROR]: 502,
  [ErrorCode.DATABASE_ERROR]: 500,
  [ErrorCode.SERVICE_UNAVAILABLE]: 503,
  [ErrorCode.GATEWAY_TIMEOUT]: 504,

  [ErrorCode.UPLOAD_FAILED]: 400,
  [ErrorCode.DELETE_FAILED]: 500,
  [ErrorCode.FILE_NOT_FOUND]: 404,
  [ErrorCode.PRESIGNED_URL_ERROR]: 500,
};
/**
 * エラーカテゴリマッピング
 */
export const ERROR_CATEGORY_MAP: Record<ErrorCode, ErrorCategory> = {
  [ErrorCode.BAD_REQUEST]: ErrorCategory.VALIDATION,
  [ErrorCode.VALIDATION_ERROR]: ErrorCategory.VALIDATION,
  [ErrorCode.MISSING_FILE]: ErrorCategory.VALIDATION,
  [ErrorCode.INVALID_EXPENSE_ID]: ErrorCategory.VALIDATION,
  [ErrorCode.INVALID_FILE_TYPE]: ErrorCategory.VALIDATION,
  [ErrorCode.FILE_TOO_LARGE]: ErrorCategory.VALIDATION,
  [ErrorCode.INVALID_REQUEST_FORMAT]: ErrorCategory.VALIDATION,

  [ErrorCode.UNAUTHORIZED]: ErrorCategory.AUTHENTICATION,
  [ErrorCode.INVALID_TOKEN]: ErrorCategory.AUTHENTICATION,
  [ErrorCode.TOKEN_EXPIRED]: ErrorCategory.AUTHENTICATION,
  [ErrorCode.FORBIDDEN]: ErrorCategory.AUTHORIZATION,
  [ErrorCode.INSUFFICIENT_PERMISSIONS]: ErrorCategory.AUTHORIZATION,

  [ErrorCode.NOT_FOUND]: ErrorCategory.VALIDATION,

  [ErrorCode.RATE_LIMIT_EXCEEDED]: ErrorCategory.RATE_LIMIT,

  [ErrorCode.INTERNAL_SERVER_ERROR]: ErrorCategory.SERVER,
  [ErrorCode.R2_CONNECTION_ERROR]: ErrorCategory.EXTERNAL_SERVICE,
  [ErrorCode.R2_UPLOAD_ERROR]: ErrorCategory.EXTERNAL_SERVICE,
  [ErrorCode.DATABASE_ERROR]: ErrorCategory.SERVER,
  [ErrorCode.SERVICE_UNAVAILABLE]: ErrorCategory.SERVER,
  [ErrorCode.GATEWAY_TIMEOUT]: ErrorCategory.EXTERNAL_SERVICE,

  [ErrorCode.UPLOAD_FAILED]: ErrorCategory.FILE_PROCESSING,
  [ErrorCode.DELETE_FAILED]: ErrorCategory.FILE_PROCESSING,
  [ErrorCode.FILE_NOT_FOUND]: ErrorCategory.FILE_PROCESSING,
  [ErrorCode.PRESIGNED_URL_ERROR]: ErrorCategory.FILE_PROCESSING,
};

/**
 * 構造化エラーレスポンス
 */
export interface StructuredErrorResponse {
  error: {
    code: string;
    message: string;
    category: ErrorCategory;
    details?: any;
    timestamp: string;
    requestId: string;
    retryable?: boolean;
  };
}

/**
 * エラー詳細情報
 */
export interface ErrorDetails {
  field?: string;
  value?: any;
  constraint?: string;
  context?: Record<string, any>;
}

/**
 * アプリケーションエラークラス
 */
export class AppError extends Error {
  public readonly code: ErrorCode;
  public readonly category: ErrorCategory;
  public readonly statusCode: number;
  public readonly details?: ErrorDetails;
  public readonly retryable: boolean;

  constructor(
    code: ErrorCode,
    message: string,
    details?: ErrorDetails,
    retryable: boolean = false,
  ) {
    super(message);
    this.name = "AppError";
    this.code = code;
    this.category = ERROR_CATEGORY_MAP[code];
    this.statusCode = ERROR_STATUS_MAP[code];
    this.details = details;
    this.retryable = retryable;
  }
}

/**
 * エラーレスポンス生成
 */
export function createErrorResponse(
  c: Context,
  error: AppError | Error,
  context?: Record<string, any>,
): StructuredErrorResponse {
  const requestId = c.get("requestId") || crypto.randomUUID();
  const timestamp = new Date().toISOString();

  if (error instanceof AppError) {
    return {
      error: {
        code: error.code,
        message: error.message,
        category: error.category,
        details: error.details,
        timestamp,
        requestId,
        retryable: error.retryable,
      },
    };
  }

  // 一般的なエラーの場合
  return {
    error: {
      code: ErrorCode.INTERNAL_SERVER_ERROR,
      message: "内部サーバーエラーが発生しました",
      category: ErrorCategory.SERVER,
      details: {
        originalMessage: error.message,
        context,
      },
      timestamp,
      requestId,
      retryable: false,
    },
  };
}
/**
 * エラーハンドリングミドルウェア
 */
export function handleError(c: Context, error: AppError | Error, context?: Record<string, any>) {
  const errorResponse = createErrorResponse(c, error, context);
  const statusCode = error instanceof AppError ? error.statusCode : 500;

  // ログ出力
  if (statusCode >= 500) {
    logger.error("サーバーエラーが発生しました", {
      error: errorResponse.error,
      context,
      stack: error.stack,
    });

    // 重大エラーの場合はアラート生成
    if (statusCode >= 500) {
      generateAlert(error, context);
    }
  } else if (statusCode >= 400) {
    logger.warn("クライアントエラーが発生しました", {
      error: errorResponse.error,
      context,
    });
  }

  return c.json(errorResponse, statusCode as any);
}

/**
 * アラート生成
 */
function generateAlert(error: AppError | Error, context?: Record<string, any>) {
  let alertLevel: AlertLevel;
  let alertMessage: string;

  if (error instanceof AppError) {
    switch (error.category) {
      case ErrorCategory.EXTERNAL_SERVICE:
        alertLevel = AlertLevel.HIGH;
        alertMessage = `外部サービスエラー: ${error.message}`;
        break;
      case ErrorCategory.SERVER:
        alertLevel = AlertLevel.CRITICAL;
        alertMessage = `サーバーエラー: ${error.message}`;
        break;
      default:
        alertLevel = AlertLevel.MEDIUM;
        alertMessage = `アプリケーションエラー: ${error.message}`;
    }
  } else {
    alertLevel = AlertLevel.CRITICAL;
    alertMessage = `予期しないエラー: ${error.message}`;
  }

  alertSystem.generateAlert({
    level: alertLevel,
    title: alertMessage,
    message: alertMessage,
    source: "error-handler",
    timestamp: new Date().toISOString(),
    details: {
      error: {
        name: error.name,
        message: error.message,
        stack: error.stack,
      },
      context,
    },
  });
}

/**
 * バリデーションエラー生成ヘルパー
 */
export function createValidationError(
  message: string,
  field?: string,
  value?: any,
  constraint?: string,
): AppError {
  return new AppError(ErrorCode.BAD_REQUEST, message, { field, value, constraint });
}

/**
 * 認証エラー生成ヘルパー
 */
export function createAuthError(message: string = "認証が必要です"): AppError {
  return new AppError(ErrorCode.UNAUTHORIZED, message);
}

/**
 * 認可エラー生成ヘルパー
 */
export function createAuthorizationError(message: string = "権限が不足しています"): AppError {
  return new AppError(ErrorCode.FORBIDDEN, message);
}

/**
 * 404エラー生成ヘルパー
 */
export function createNotFoundError(message: string = "リソースが見つかりません"): AppError {
  return new AppError(ErrorCode.NOT_FOUND, message);
}

/**
 * ファイル関連エラー生成ヘルパー
 */
export function createFileError(
  code: ErrorCode.INVALID_FILE_TYPE | ErrorCode.FILE_TOO_LARGE | ErrorCode.UPLOAD_FAILED,
  message: string,
  details?: ErrorDetails,
): AppError {
  return new AppError(code, message, details);
}

/**
 * R2関連エラー生成ヘルパー
 */
export function createR2Error(
  code: ErrorCode.R2_CONNECTION_ERROR | ErrorCode.R2_UPLOAD_ERROR,
  message: string,
  retryable: boolean = true,
): AppError {
  return new AppError(code, message, undefined, retryable);
}

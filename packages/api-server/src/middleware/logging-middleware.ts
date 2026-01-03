/**
 * ログミドルウェア
 * リクエスト/レスポンストレーシングと構造化ログ出力
 */

import type { Context, Next } from "hono";
import { logger } from "../utils/logger.js";

/**
 * リクエスト情報の型定義
 */
interface RequestInfo {
  requestId: string;
  method: string;
  path: string;
  query: string;
  userAgent?: string;
  clientIP: string;
  contentType?: string;
  contentLength?: number;
  authorization?: boolean; // 認証ヘッダーの有無（値は記録しない）
}

/**
 * レスポンス情報の型定義
 */
interface ResponseInfo {
  requestId: string;
  statusCode: number;
  contentType?: string;
  contentLength?: number;
  duration: number;
}

/**
 * クライアントIPアドレスを取得
 * @param c Honoコンテキスト
 * @returns IPアドレス
 */
function getClientIP(c: Context): string {
  // プロキシ経由の場合のヘッダーをチェック
  const forwarded = c.req.header("x-forwarded-for");
  if (forwarded) {
    return forwarded.split(",")[0].trim();
  }

  const realIP = c.req.header("x-real-ip");
  if (realIP) {
    return realIP;
  }

  // Cloudflareの場合
  const cfConnectingIP = c.req.header("cf-connecting-ip");
  if (cfConnectingIP) {
    return cfConnectingIP;
  }

  // フォールバック（開発環境用）
  return "127.0.0.1";
}

/**
 * コンテンツ長を取得
 * @param contentLength Content-Lengthヘッダー値
 * @returns 数値またはundefined
 */
function parseContentLength(contentLength?: string): number | undefined {
  if (!contentLength) return undefined;
  const length = parseInt(contentLength, 10);
  return isNaN(length) ? undefined : length;
}

/**
 * ログレベルを決定
 * @param statusCode HTTPステータスコード
 * @param duration レスポンス時間（ミリ秒）
 * @returns ログレベル
 */
function determineLogLevel(statusCode: number, duration: number): "info" | "warn" | "error" {
  if (statusCode >= 500) {
    return "error";
  }
  if (statusCode >= 400 || duration > 5000) {
    return "warn";
  }
  return "info";
}

/**
 * ログミドルウェアを作成
 * @returns ミドルウェア関数
 */
export function createLoggingMiddleware() {
  return async (c: Context, next: Next) => {
    const startTime = Date.now();
    const requestId = crypto.randomUUID();

    // リクエスト情報を収集
    const requestInfo: RequestInfo = {
      requestId,
      method: c.req.method,
      path: c.req.path,
      query: c.req.url.includes("?") ? c.req.url.split("?")[1] : "",
      userAgent: c.req.header("user-agent"),
      clientIP: getClientIP(c),
      contentType: c.req.header("content-type"),
      contentLength: parseContentLength(c.req.header("content-length")),
      authorization: !!c.req.header("authorization"),
    };

    // リクエストIDをコンテキストに設定
    c.set("requestId", requestId);

    // リクエスト開始ログ
    logger.info("リクエスト開始", {
      type: "request_start",
      ...requestInfo,
      timestamp: new Date().toISOString(),
    });

    try {
      await next();
    } catch (error) {
      // エラーが発生した場合のログ
      logger.error("リクエスト処理中にエラーが発生しました", {
        type: "request_error",
        requestId,
        error: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined,
        method: c.req.method,
        path: c.req.path,
        clientIP: requestInfo.clientIP,
        duration: Date.now() - startTime,
        timestamp: new Date().toISOString(),
      });

      // エラーを再スロー
      throw error;
    } finally {
      // レスポンス情報を収集
      const endTime = Date.now();
      const duration = endTime - startTime;

      const responseInfo: ResponseInfo = {
        requestId,
        statusCode: c.res.status,
        contentType: c.res.headers.get("content-type") || undefined,
        contentLength: parseContentLength(c.res.headers.get("content-length") || undefined),
        duration,
      };

      // レスポンス完了ログ
      const logLevel = determineLogLevel(responseInfo.statusCode, duration);
      const logMessage = `リクエスト完了 ${requestInfo.method} ${requestInfo.path} ${responseInfo.statusCode} ${duration}ms`;

      logger[logLevel](logMessage, {
        type: "request_complete",
        request: requestInfo,
        response: responseInfo,
        timestamp: new Date().toISOString(),
      });

      // パフォーマンス警告
      if (duration > 1000) {
        logger.warn("レスポンス時間が長いリクエストを検出しました", {
          type: "performance_warning",
          requestId,
          duration,
          threshold: 1000,
          method: requestInfo.method,
          path: requestInfo.path,
          statusCode: responseInfo.statusCode,
          timestamp: new Date().toISOString(),
        });
      }

      // レスポンスヘッダーにリクエストIDを追加
      c.header("X-Request-ID", requestId);
    }
  };
}

/**
 * エラーログ用のヘルパー関数
 * @param c Honoコンテキスト
 * @param error エラーオブジェクト
 * @param additionalInfo 追加情報
 */
export function logError(c: Context, error: unknown, additionalInfo?: Record<string, any>) {
  const requestId = c.get("requestId") || crypto.randomUUID();

  logger.error("アプリケーションエラーが発生しました", {
    type: "application_error",
    requestId,
    error: error instanceof Error ? error.message : String(error),
    stack: error instanceof Error ? error.stack : undefined,
    method: c.req.method,
    path: c.req.path,
    clientIP: getClientIP(c),
    userAgent: c.req.header("user-agent"),
    timestamp: new Date().toISOString(),
    ...additionalInfo,
  });
}

/**
 * セキュリティイベントログ用のヘルパー関数
 * @param c Honoコンテキスト
 * @param event セキュリティイベント名
 * @param details イベント詳細
 */
export function logSecurityEvent(c: Context, event: string, details?: Record<string, any>) {
  const requestId = c.get("requestId") || crypto.randomUUID();

  logger.warn("セキュリティイベントが発生しました", {
    type: "security_event",
    event,
    requestId,
    method: c.req.method,
    path: c.req.path,
    clientIP: getClientIP(c),
    userAgent: c.req.header("user-agent"),
    timestamp: new Date().toISOString(),
    ...details,
  });
}

/**
 * ビジネスロジックログ用のヘルパー関数
 * @param c Honoコンテキスト
 * @param action アクション名
 * @param details アクション詳細
 * @param level ログレベル
 */
export function logBusinessEvent(
  c: Context,
  action: string,
  details?: Record<string, any>,
  level: "info" | "warn" | "error" = "info",
) {
  const requestId = c.get("requestId") || crypto.randomUUID();

  logger[level](`ビジネスイベント: ${action}`, {
    type: "business_event",
    action,
    requestId,
    method: c.req.method,
    path: c.req.path,
    timestamp: new Date().toISOString(),
    ...details,
  });
}

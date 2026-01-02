/**
 * Honoアプリケーションの設定
 * ミドルウェアとルートの設定を行う
 */

import { Hono } from "hono";
import { cors } from "hono/cors";
import { logger as honoLogger } from "hono/logger";
import { secureHeaders } from "hono/secure-headers";
import type { ApiServerConfig } from "./types/config.js";
import { logger } from "./utils/logger.js";

/**
 * Honoアプリケーションを作成
 */
export function createApp(config: ApiServerConfig): Hono {
  const app = new Hono();

  // ログミドルウェア
  app.use(
    "*",
    honoLogger((message) => {
      logger.info(message);
    }),
  );

  // セキュリティヘッダー
  app.use(
    "*",
    secureHeaders({
      xContentTypeOptions: "nosniff",
      xFrameOptions: "DENY",
      xXssProtection: "1; mode=block",
    }),
  );

  // CORS設定
  app.use(
    "*",
    cors({
      origin: config.cors.origin,
      allowMethods: config.cors.methods,
      allowHeaders: config.cors.headers,
      credentials: true,
    }),
  );

  // ヘルスチェックエンドポイント
  app.get("/api/v1/health", (c) => {
    return c.json({
      status: "ok",
      timestamp: new Date().toISOString(),
      version: "0.1.0",
      environment: config.nodeEnv,
    });
  });

  // 404ハンドラー
  app.notFound((c) => {
    logger.warn("存在しないエンドポイントへのアクセス", {
      path: c.req.path,
      method: c.req.method,
    });

    return c.json(
      {
        error: {
          code: "NOT_FOUND",
          message: "エンドポイントが見つかりません",
          timestamp: new Date().toISOString(),
          requestId: crypto.randomUUID(),
        },
      },
      404,
    );
  });

  // エラーハンドラー
  app.onError((error, c) => {
    logger.error("予期しないエラーが発生しました", {
      error: error.message,
      stack: error.stack,
      path: c.req.path,
      method: c.req.method,
    });

    return c.json(
      {
        error: {
          code: "INTERNAL_SERVER_ERROR",
          message: "内部サーバーエラーが発生しました",
          timestamp: new Date().toISOString(),
          requestId: crypto.randomUUID(),
        },
      },
      500,
    );
  });

  return app;
}

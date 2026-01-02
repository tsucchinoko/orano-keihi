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
import { createR2Client, createR2TestService, createAuthService } from "./services/index.js";
import { createAuthMiddleware, createPermissionMiddleware } from "./middleware/index.js";

/**
 * Honoアプリケーションを作成
 */
export function createApp(config: ApiServerConfig): Hono {
  const app = new Hono();

  // R2クライアントとテストサービスを初期化
  const r2Client = createR2Client(config.r2);
  const r2TestService = createR2TestService(r2Client);

  // 認証サービスを初期化
  const authService = createAuthService(config.auth);

  // 認証ミドルウェアを作成
  const authMiddleware = createAuthMiddleware(authService);
  const fileUploadPermissionMiddleware = createPermissionMiddleware(authService, "file_upload");

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

  // R2接続テストエンドポイント（認証が必要）
  app.get("/api/v1/system/r2/test", authMiddleware, async (c) => {
    try {
      const testResult = await r2TestService.runComprehensiveTest();

      if (testResult.success) {
        logger.info("R2接続テストが成功しました", testResult.details);
        return c.json({
          status: "success",
          message: testResult.message,
          details: testResult.details,
          timestamp: new Date().toISOString(),
        });
      } else {
        logger.warn("R2接続テストが失敗しました", { error: testResult.error });
        return c.json(
          {
            status: "error",
            message: testResult.message,
            error: testResult.error,
            timestamp: new Date().toISOString(),
          },
          503,
        );
      }
    } catch (error) {
      logger.error("R2接続テストでエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
      });

      return c.json(
        {
          error: {
            code: "R2_TEST_ERROR",
            message: "R2接続テストでエラーが発生しました",
            details: error instanceof Error ? error.message : String(error),
            timestamp: new Date().toISOString(),
            requestId: crypto.randomUUID(),
          },
        },
        500,
      );
    }
  });

  // R2簡単接続確認エンドポイント
  app.get("/api/v1/system/r2/ping", async (c) => {
    try {
      const isConnected = await r2TestService.quickConnectionTest();

      return c.json({
        status: isConnected ? "connected" : "disconnected",
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      logger.error("R2接続確認でエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
      });

      return c.json(
        {
          status: "error",
          error: error instanceof Error ? error.message : String(error),
          timestamp: new Date().toISOString(),
        },
        500,
      );
    }
  });

  // 認証テスト用エンドポイント
  app.get("/api/v1/auth/test", authMiddleware, (c) => {
    const user = c.get("user");

    return c.json({
      message: "認証が成功しました",
      user: {
        id: user.id,
        email: user.email,
        name: user.name,
      },
      timestamp: new Date().toISOString(),
    });
  });

  // 権限テスト用エンドポイント
  app.get("/api/v1/auth/permission-test", authMiddleware, fileUploadPermissionMiddleware, (c) => {
    const user = c.get("user");

    return c.json({
      message: "権限チェックが成功しました",
      user: {
        id: user.id,
        email: user.email,
        name: user.name,
      },
      resource: "file_upload",
      timestamp: new Date().toISOString(),
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

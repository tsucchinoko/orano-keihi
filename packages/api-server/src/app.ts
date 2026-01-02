/**
 * Honoアプリケーションの設定
 * ミドルウェアとルートの設定を行う
 */

import { Hono } from "hono";
import { cors } from "hono/cors";
import { secureHeaders } from "hono/secure-headers";
import type { ApiServerConfig } from "./types/config.js";
import { logger, enhancedLogger, alertSystem, AlertLevel } from "./utils/logger.js";
import { createR2Client, createR2TestService, createAuthService } from "./services/index.js";
import {
  createAuthMiddleware,
  createPermissionMiddleware,
  createRateLimitMiddleware,
  createLoggingMiddleware,
  logError,
  logSecurityEvent,
} from "./middleware/index.js";

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

  // レート制限ミドルウェアを作成
  const rateLimitMiddleware = createRateLimitMiddleware(config.rateLimit);

  // ログミドルウェアを作成
  const loggingMiddleware = createLoggingMiddleware();

  // ログミドルウェア（最初に適用）
  app.use("*", loggingMiddleware);

  // レート制限ミドルウェア
  app.use("*", rateLimitMiddleware);

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
        enhancedLogger.systemFailure("R2接続テストが失敗しました", {
          error: testResult.error,
          details: testResult.details,
        });
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
      logError(c, error, { context: "R2接続テスト" });

      return c.json(
        {
          error: {
            code: "R2_TEST_ERROR",
            message: "R2接続テストでエラーが発生しました",
            details: error instanceof Error ? error.message : String(error),
            timestamp: new Date().toISOString(),
            requestId: c.get("requestId") || crypto.randomUUID(),
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
      logError(c, error, { context: "R2接続確認" });

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

  // システム統計エンドポイント
  app.get("/api/v1/system/stats", authMiddleware, (c) => {
    const alertStats = alertSystem.getAlertStats();
    const recentAlerts = alertSystem.getRecentAlerts(10);

    return c.json({
      system: {
        uptime: process.uptime(),
        memory: process.memoryUsage(),
        version: "0.1.0",
        environment: config.nodeEnv,
      },
      alerts: {
        stats: alertStats,
        recent: recentAlerts,
      },
      timestamp: new Date().toISOString(),
    });
  });

  // アラート一覧エンドポイント
  app.get("/api/v1/system/alerts", authMiddleware, (c) => {
    const level = c.req.query("level") as keyof typeof AlertLevel;
    const limit = parseInt(c.req.query("limit") || "50", 10);

    let alerts;
    if (level && Object.values(AlertLevel).includes(level as AlertLevel)) {
      alerts = alertSystem.getAlertsByLevel(level as AlertLevel, limit);
    } else {
      alerts = alertSystem.getRecentAlerts(limit);
    }

    return c.json({
      alerts,
      total: alerts.length,
      timestamp: new Date().toISOString(),
    });
  });

  // 404ハンドラー
  app.notFound((c) => {
    logSecurityEvent(c, "NOT_FOUND_ACCESS", {
      path: c.req.path,
      method: c.req.method,
      userAgent: c.req.header("user-agent"),
    });

    return c.json(
      {
        error: {
          code: "NOT_FOUND",
          message: "エンドポイントが見つかりません",
          timestamp: new Date().toISOString(),
          requestId: c.get("requestId") || crypto.randomUUID(),
        },
      },
      404,
    );
  });

  // エラーハンドラー
  app.onError((error, c) => {
    logError(c, error, { context: "グローバルエラーハンドラー" });

    return c.json(
      {
        error: {
          code: "INTERNAL_SERVER_ERROR",
          message: "内部サーバーエラーが発生しました",
          timestamp: new Date().toISOString(),
          requestId: c.get("requestId") || crypto.randomUUID(),
        },
      },
      500,
    );
  });

  return app;
}

/**
 * Honoアプリケーションの設定
 * ミドルウェアとルートの設定を行う
 */

import { Hono } from "hono";
import { cors } from "hono/cors";
import { secureHeaders } from "hono/secure-headers";
import type { ApiServerConfig } from "./types/config.js";
import { logger, enhancedLogger, alertSystem, AlertLevel } from "./utils/logger.js";
import {
  AppError,
  ErrorCode,
  handleError,
  createValidationError,
  createAuthorizationError,
  createNotFoundError,
} from "./utils/error-handler.js";
import { retryStatsTracker } from "./utils/retry.js";
import {
  createEnvironmentAwareR2Client,
  createR2TestService,
  createAuthService,
  createFileUploadService,
  createTauriSubscriptionService,
} from "./services/index.js";
import {
  createAuthMiddleware,
  createPermissionMiddleware,
  createRateLimitMiddleware,
  createLoggingMiddleware,
  logError,
  logSecurityEvent,
} from "./middleware/index.js";
import { updaterApp } from "./routes/updater.js";
import { createReceiptsRouter } from "./routes/receipts.js";

/**
 * ファイルキーからContent-Typeを推定する
 * @param fileKey ファイルキー
 * @returns Content-Type
 */
function getContentTypeFromFileKey(fileKey: string): string {
  const extension = fileKey.toLowerCase().split(".").pop();

  switch (extension) {
    case "jpg":
    case "jpeg":
      return "image/jpeg";
    case "png":
      return "image/png";
    case "gif":
      return "image/gif";
    case "webp":
      return "image/webp";
    case "pdf":
      return "application/pdf";
    case "txt":
      return "text/plain";
    case "json":
      return "application/json";
    default:
      return "application/octet-stream";
  }
}

/**
 * Honoアプリケーションを作成
 * @param config API サーバー設定
 * @param r2Bucket Workers環境でのR2バケットバインディング（オプション）
 * @param accountId CloudflareアカウントID（Workers環境で必要）
 */
export function createApp(config: ApiServerConfig, r2Bucket?: R2Bucket, accountId?: string): Hono {
  const app = new Hono();

  // 環境に応じたR2クライアントを初期化
  const r2Client = createEnvironmentAwareR2Client(config.r2, r2Bucket, accountId);
  const r2TestService = createR2TestService(r2Client);

  // 認証サービスを初期化
  const authService = createAuthService(config.auth);

  // ファイルアップロードサービスを初期化
  const fileUploadService = createFileUploadService(r2Client, config.fileUpload);

  // サブスクリプションサービスを初期化（Tauri経由）
  const subscriptionService = createTauriSubscriptionService();

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
      credentials: false, // JWTトークンをヘッダーで送信するため、credentialsは不要
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

  // アップデーター関連エンドポイント
  app.route("/api/updater", updaterApp);

  // 領収書関連エンドポイント（認証が必要）
  const receiptsRouter = createReceiptsRouter(r2Client);
  app.use("/api/v1/receipts/*", authMiddleware);
  app.route("/api/v1/receipts", receiptsRouter);

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
    const retryStats = retryStatsTracker.getStats();

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
      retry: retryStats,
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

  // サブスクリプション関連エンドポイント

  // サブスクリプション一覧取得
  app.get("/api/v1/subscriptions", authMiddleware, async (c) => {
    try {
      const user = c.get("user");
      const activeOnly = c.req.query("activeOnly") === "true";

      const result = await subscriptionService.getSubscriptions(user.id, activeOnly);

      logger.info("サブスクリプション一覧を取得しました", {
        userId: user.id,
        activeOnly,
        total: result.total,
        activeCount: result.activeCount,
      });

      return c.json(result);
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプション一覧取得",
      });
    }
  });

  // 開発環境用：認証不要のサブスクリプション一覧取得
  if (config.nodeEnv === "development") {
    app.get("/api/v1/subscriptions/dev", async (c) => {
      try {
        // 開発環境では固定のユーザーID（"1"）を使用
        const activeOnly = c.req.query("activeOnly") === "true";
        const result = await subscriptionService.getSubscriptions("1", activeOnly);

        logger.info("開発用サブスクリプション一覧を取得しました", {
          userId: "1",
          activeOnly,
          total: result.total,
          activeCount: result.activeCount,
        });

        return c.json(result);
      } catch (error) {
        return handleError(c, error instanceof Error ? error : new Error(String(error)), {
          context: "開発用サブスクリプション一覧取得",
        });
      }
    });
  }

  // サブスクリプション作成
  app.post("/api/v1/subscriptions", authMiddleware, async (c) => {
    try {
      const user = c.get("user");
      const body = await c.req.json();

      const subscription = await subscriptionService.createSubscription(user.id, body);

      logger.info("サブスクリプションを作成しました", {
        userId: user.id,
        subscriptionId: subscription.id,
        name: subscription.name,
      });

      return c.json(subscription, 201);
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプション作成",
      });
    }
  });

  // サブスクリプション更新
  app.put("/api/v1/subscriptions/:id", authMiddleware, async (c) => {
    try {
      const user = c.get("user");
      const subscriptionId = parseInt(c.req.param("id"), 10);
      const body = await c.req.json();

      if (isNaN(subscriptionId)) {
        throw createValidationError(
          "有効なサブスクリプションIDが指定されていません",
          "id",
          subscriptionId,
          "valid number required",
        );
      }

      const subscription = await subscriptionService.updateSubscription(
        user.id,
        subscriptionId,
        body,
      );

      logger.info("サブスクリプションを更新しました", {
        userId: user.id,
        subscriptionId,
      });

      return c.json(subscription);
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプション更新",
      });
    }
  });

  // サブスクリプションステータス切り替え
  app.patch("/api/v1/subscriptions/:id/toggle", authMiddleware, async (c) => {
    try {
      const user = c.get("user");
      const subscriptionId = parseInt(c.req.param("id"), 10);

      if (isNaN(subscriptionId)) {
        throw createValidationError(
          "有効なサブスクリプションIDが指定されていません",
          "id",
          subscriptionId,
          "valid number required",
        );
      }

      const subscription = await subscriptionService.toggleSubscriptionStatus(
        user.id,
        subscriptionId,
      );

      logger.info("サブスクリプションのステータスを切り替えました", {
        userId: user.id,
        subscriptionId,
        newStatus: subscription.is_active,
      });

      return c.json(subscription);
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプションステータス切り替え",
      });
    }
  });

  // サブスクリプション削除
  app.delete("/api/v1/subscriptions/:id", authMiddleware, async (c) => {
    try {
      const user = c.get("user");
      const subscriptionId = parseInt(c.req.param("id"), 10);

      if (isNaN(subscriptionId)) {
        throw createValidationError(
          "有効なサブスクリプションIDが指定されていません",
          "id",
          subscriptionId,
          "valid number required",
        );
      }

      await subscriptionService.deleteSubscription(user.id, subscriptionId);

      logger.info("サブスクリプションを削除しました", {
        userId: user.id,
        subscriptionId,
      });

      return c.json({
        success: true,
        message: "サブスクリプションが正常に削除されました",
        subscriptionId,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプション削除",
      });
    }
  });

  // 月額サブスクリプション合計取得
  app.get("/api/v1/subscriptions/monthly-total", authMiddleware, async (c) => {
    try {
      const user = c.get("user");

      const result = await subscriptionService.getMonthlyTotal(user.id);

      logger.info("月額サブスクリプション合計を取得しました", {
        userId: user.id,
        monthlyTotal: result.monthlyTotal,
        activeSubscriptions: result.activeSubscriptions,
      });

      return c.json(result);
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "月額サブスクリプション合計取得",
      });
    }
  });

  // サブスクリプション領収書アップロード
  app.post("/api/v1/subscriptions/receipt/upload", authMiddleware, async (c) => {
    try {
      const user = c.get("user");
      const body = await c.req.json();

      const { subscriptionId, fileName, fileData } = body;

      if (!subscriptionId || !fileName || !fileData) {
        throw createValidationError(
          "必要なパラメータが不足しています",
          "body",
          { subscriptionId, fileName, fileData: !!fileData },
          "subscriptionId, fileName, fileData are required",
        );
      }

      // Base64データをデコード
      const buffer = Buffer.from(fileData, "base64");

      // ファイルキーを生成（users/subscriptionsフォルダを使用）
      const timestamp = Date.now();
      const fileKey = `users/${user.id}/subscriptions/${subscriptionId}/${timestamp}_${fileName}`;

      // R2にアップロード
      await r2Client.uploadFile(fileKey, buffer);

      // HTTPS URLを生成（R2クライアントのputObjectメソッドが返すURLを使用）
      const httpsUrl = await r2Client.putObject(fileKey, buffer, "application/octet-stream");

      logger.info("サブスクリプション領収書をアップロードしました", {
        userId: user.id,
        subscriptionId,
        fileKey,
        fileSize: buffer.length,
        httpsUrl,
      });

      return c.json({
        success: true,
        url: httpsUrl,
        fileKey,
        fileSize: buffer.length,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプション領収書アップロード",
      });
    }
  });

  // サブスクリプション領収書削除
  app.delete("/api/v1/subscriptions/:id/receipt", authMiddleware, async (c) => {
    try {
      const user = c.get("user");
      const subscriptionId = parseInt(c.req.param("id"), 10);

      if (isNaN(subscriptionId)) {
        throw createValidationError(
          "有効なサブスクリプションIDが指定されていません",
          "id",
          subscriptionId,
          "valid number required",
        );
      }

      // サブスクリプションフォルダ内のファイルを削除
      const folderPrefix = `users/${user.id}/subscriptions/${subscriptionId}/`;

      try {
        // フォルダ内のファイル一覧を取得して削除
        const files = await r2Client.listFiles(folderPrefix);
        for (const file of files) {
          await r2Client.deleteFile(file.key);
        }
      } catch (error) {
        // ファイルが存在しない場合は正常として扱う
        logger.debug("サブスクリプション領収書フォルダが見つかりませんでした", {
          userId: user.id,
          subscriptionId,
          folderPrefix,
          error: error instanceof Error ? error.message : String(error),
        });
      }

      logger.info("サブスクリプション領収書を削除しました", {
        userId: user.id,
        subscriptionId,
        folderPrefix,
      });

      return c.json({
        success: true,
        message: "サブスクリプション領収書が正常に削除されました",
        subscriptionId,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプション領収書削除",
      });
    }
  });

  // ファイルアップロード関連エンドポイント

  // 単一ファイルアップロード
  app.post("/api/v1/receipts/upload", authMiddleware, fileUploadPermissionMiddleware, async (c) => {
    try {
      const user = c.get("user");
      const body = await c.req.parseBody();

      // ファイルとメタデータを取得
      const file = body.file as File;
      const expenseId = parseInt(body.expenseId as string, 10);
      const description = body.description as string;
      const category = body.category as string;

      if (!file) {
        throw createValidationError(
          "アップロードするファイルが指定されていません",
          "file",
          undefined,
          "required",
        );
      }

      if (!expenseId || isNaN(expenseId)) {
        throw createValidationError(
          "有効な経費IDが指定されていません",
          "expenseId",
          expenseId,
          "valid number required",
        );
      }

      // メタデータを構築
      const metadata = {
        expenseId,
        userId: user.id,
        description,
        category,
      };

      // ファイルアップロード実行
      const result = await fileUploadService.uploadFile(file, metadata);

      if (result.success) {
        logger.info("ファイルアップロードが成功しました", {
          userId: user.id,
          expenseId,
          fileKey: result.fileKey,
          fileSize: result.fileSize,
        });

        return c.json(result);
      } else {
        logger.warn("ファイルアップロードが失敗しました", {
          userId: user.id,
          expenseId,
          error: result.error,
        });

        throw new AppError(
          ErrorCode.UPLOAD_FAILED,
          result.error || "ファイルアップロードに失敗しました",
        );
      }
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "単一ファイルアップロード",
      });
    }
  });

  // 複数ファイル並列アップロード
  app.post(
    "/api/v1/receipts/upload/multiple",
    authMiddleware,
    fileUploadPermissionMiddleware,
    async (c) => {
      try {
        const user = c.get("user");
        const body = await c.req.parseBody();

        // ファイル配列を取得
        const files: File[] = [];
        const metadata: any[] = [];

        // multipart/form-dataから複数ファイルを抽出
        for (const [key, value] of Object.entries(body)) {
          if (key.startsWith("file_") && value instanceof File) {
            const index = parseInt(key.replace("file_", ""), 10);
            files[index] = value;
          } else if (key.startsWith("expenseId_")) {
            const index = parseInt(key.replace("expenseId_", ""), 10);
            if (!metadata[index]) metadata[index] = {};
            metadata[index].expenseId = parseInt(value as string, 10);
          } else if (key.startsWith("description_")) {
            const index = parseInt(key.replace("description_", ""), 10);
            if (!metadata[index]) metadata[index] = {};
            metadata[index].description = value as string;
          } else if (key.startsWith("category_")) {
            const index = parseInt(key.replace("category_", ""), 10);
            if (!metadata[index]) metadata[index] = {};
            metadata[index].category = value as string;
          }
        }

        if (files.length === 0) {
          throw createValidationError(
            "アップロードするファイルが指定されていません",
            "files",
            files.length,
            "at least one file required",
          );
        }

        // メタデータにユーザーIDを追加
        const uploadMetadata = metadata.map((meta) => ({
          ...meta,
          userId: user.id,
        }));

        // 複数ファイルアップロード実行
        const result = await fileUploadService.uploadMultipleFiles(files, uploadMetadata);

        logger.info("複数ファイルアップロードが完了しました", {
          userId: user.id,
          totalFiles: result.totalFiles,
          successfulUploads: result.successfulUploads,
          failedUploads: result.failedUploads,
          totalDurationMs: result.totalDurationMs,
        });

        return c.json(result);
      } catch (error) {
        return handleError(c, error instanceof Error ? error : new Error(String(error)), {
          context: "複数ファイルアップロード",
        });
      }
    },
  );

  // ファイル削除
  app.delete(
    "/api/v1/receipts/:fileKey",
    authMiddleware,
    fileUploadPermissionMiddleware,
    async (c) => {
      try {
        const user = c.get("user");
        const fileKey = c.req.param("fileKey");

        if (!fileKey) {
          throw createValidationError(
            "削除するファイルキーが指定されていません",
            "fileKey",
            fileKey,
            "required",
          );
        }

        // デコードされたファイルキー（URLエンコードされている可能性があるため）
        const decodedFileKey = decodeURIComponent(fileKey);

        // ファイルキーがユーザーのものかチェック（セキュリティ）
        if (!decodedFileKey.startsWith(`receipts/${user.id}/`)) {
          logSecurityEvent(c, "UNAUTHORIZED_FILE_DELETE", {
            userId: user.id,
            fileKey: decodedFileKey,
          });

          throw createAuthorizationError("このファイルを削除する権限がありません");
        }

        // ファイル削除実行
        await fileUploadService.deleteFile(decodedFileKey);

        logger.info("ファイル削除が完了しました", {
          userId: user.id,
          fileKey: decodedFileKey,
        });

        return c.json({
          success: true,
          message: "ファイルが正常に削除されました",
          fileKey: decodedFileKey,
          timestamp: new Date().toISOString(),
        });
      } catch (error) {
        return handleError(c, error instanceof Error ? error : new Error(String(error)), {
          context: "ファイル削除",
        });
      }
    },
  );

  // ファイル取得（プリサインドURL）
  app.get("/api/v1/receipts/:fileKey/url", authMiddleware, async (c) => {
    try {
      const user = c.get("user");
      const fileKey = c.req.param("fileKey");
      const expiresIn = parseInt(c.req.query("expiresIn") || "3600", 10);

      if (!fileKey) {
        throw createValidationError(
          "ファイルキーが指定されていません",
          "fileKey",
          fileKey,
          "required",
        );
      }

      // デコードされたファイルキー
      const decodedFileKey = decodeURIComponent(fileKey);

      // ファイルキーがユーザーのものかチェック（セキュリティ）
      if (!decodedFileKey.startsWith(`receipts/${user.id}/`)) {
        logSecurityEvent(c, "UNAUTHORIZED_FILE_ACCESS", {
          userId: user.id,
          fileKey: decodedFileKey,
        });

        throw createAuthorizationError("このファイルにアクセスする権限がありません");
      }

      // プリサインドURL生成
      const presignedUrl = await r2Client.generatePresignedUrl(decodedFileKey, expiresIn);

      logger.debug("プリサインドURLを生成しました", {
        userId: user.id,
        fileKey: decodedFileKey,
        expiresIn,
      });

      return c.json({
        success: true,
        fileKey: decodedFileKey,
        presignedUrl,
        expiresIn,
        expiresAt: new Date(Date.now() + expiresIn * 1000).toISOString(),
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "プリサインドURL生成",
      });
    }
  });

  // ファイルデータ取得（Base64エンコード） - 複数のルートパターンで対応
  app.get(
    "/api/v1/receipts/users/:userId/receipts/:receiptId/:filename/data",
    authMiddleware,
    async (c) => {
      try {
        const user = c.get("user");
        const userId = c.req.param("userId");
        const receiptId = c.req.param("receiptId");
        const filename = c.req.param("filename");

        // ファイルキーを構築
        const fileKey = `users/${userId}/receipts/${receiptId}/${filename}`;

        logger.debug("ファイルデータ取得リクエスト", {
          userId: user.id,
          requestedUserId: userId,
          receiptId,
          filename,
          fileKey,
        });

        // ユーザーIDの一致をチェック（セキュリティ）
        if (userId !== user.id) {
          logSecurityEvent(c, "UNAUTHORIZED_FILE_ACCESS", {
            userId: user.id,
            requestedUserId: userId,
            fileKey,
          });

          throw createAuthorizationError("このファイルにアクセスする権限がありません");
        }

        // R2からファイルを取得
        const fileData = await r2Client.getFile(fileKey);

        if (!fileData) {
          throw createNotFoundError("ファイルが見つかりません");
        }

        // ファイルデータをBase64エンコード
        const base64Data = Buffer.from(fileData).toString("base64");

        // Content-Typeを推定
        const contentType = getContentTypeFromFileKey(fileKey);

        logger.debug("ファイルデータを取得しました", {
          userId: user.id,
          fileKey,
          fileSize: fileData.length,
          contentType,
        });

        return c.json({
          success: true,
          data: base64Data,
          content_type: contentType,
          file_size: fileData.length,
          timestamp: new Date().toISOString(),
        });
      } catch (error) {
        return handleError(c, error instanceof Error ? error : new Error(String(error)), {
          context: "ファイルデータ取得",
        });
      }
    },
  );

  // より汎用的なファイルデータ取得（フォールバック）
  app.get("/api/v1/receipts/users/:userId/*", authMiddleware, async (c) => {
    try {
      const user = c.get("user");
      const userId = c.req.param("userId");
      const fullPath = c.req.path;

      logger.debug("汎用ファイルデータ取得ルートにマッチしました", {
        userId: user.id,
        requestedUserId: userId,
        fullPath,
      });

      // /data で終わるかチェック
      if (!fullPath.endsWith("/data")) {
        logger.debug("パスが/dataで終わっていません", { fullPath });
        throw createNotFoundError("エンドポイントが見つかりません");
      }

      // パスからファイルキーを抽出
      const pathMatch = fullPath.match(/\/api\/v1\/receipts\/(users\/[^/]+\/.*?)\/data$/);
      if (!pathMatch) {
        logger.debug("ファイルキーの抽出に失敗しました", { fullPath });
        throw createValidationError(
          "ファイルキーの抽出に失敗しました",
          "path",
          fullPath,
          "invalid format",
        );
      }

      const fileKey = decodeURIComponent(pathMatch[1]);

      logger.debug("汎用ファイルデータ取得リクエスト", {
        userId: user.id,
        requestedUserId: userId,
        fullPath,
        fileKey,
      });

      // ユーザーIDの一致をチェック（セキュリティ）
      if (userId !== user.id) {
        logSecurityEvent(c, "UNAUTHORIZED_FILE_ACCESS", {
          userId: user.id,
          requestedUserId: userId,
          fileKey,
        });

        throw createAuthorizationError("このファイルにアクセスする権限がありません");
      }

      // R2からファイルを取得
      const fileData = await r2Client.getFile(fileKey);

      if (!fileData) {
        logger.debug("R2からファイルが見つかりませんでした", { fileKey });
        throw createNotFoundError("ファイルが見つかりません");
      }

      // ファイルデータをBase64エンコード
      const base64Data = Buffer.from(fileData).toString("base64");

      // Content-Typeを推定
      const contentType = getContentTypeFromFileKey(fileKey);

      logger.debug("ファイルデータを取得しました", {
        userId: user.id,
        fileKey,
        fileSize: fileData.length,
        contentType,
      });

      return c.json({
        success: true,
        data: base64Data,
        content_type: contentType,
        file_size: fileData.length,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      logger.error("汎用ファイルデータ取得でエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined,
      });
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "汎用ファイルデータ取得",
      });
    }
  });

  // 404ハンドラー
  app.notFound((c) => {
    logSecurityEvent(c, "NOT_FOUND_ACCESS", {
      path: c.req.path,
      method: c.req.method,
      userAgent: c.req.header("user-agent"),
    });

    const error = new AppError(ErrorCode.FILE_NOT_FOUND, "エンドポイントが見つかりません");

    return handleError(c, error);
  });

  // エラーハンドラー
  app.onError((error, c) => {
    return handleError(c, error, { context: "グローバルエラーハンドラー" });
  });

  return app;
}

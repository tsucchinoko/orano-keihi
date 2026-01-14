/**
 * 認証ミドルウェア
 * APIリクエストの認証トークンを検証する
 */

import type { Context, Next } from "hono";
import type { AuthService } from "../services/auth-service.js";
import { enhancedLogger } from "../utils/logger.js";
import { logSecurityEvent } from "./logging-middleware.js";

/**
 * 認証されたユーザー情報をコンテキストに追加する型拡張
 */
declare module "hono" {
  interface ContextVariableMap {
    user: {
      id: string; // nanoIdに変更
      googleId: string;
      email: string;
      name: string;
      pictureUrl?: string;
      createdAt: string;
      updatedAt: string;
    };
    requestId: string;
  }
}

/**
 * 認証ミドルウェアを作成する
 * @param authService 認証サービス
 * @returns 認証ミドルウェア関数
 */
export function createAuthMiddleware(authService: AuthService) {
  return async (c: Context, next: Next) => {
    try {
      // Authorizationヘッダーからトークンを取得
      const authHeader = c.req.header("Authorization");

      if (!authHeader) {
        logSecurityEvent(c, "MISSING_AUTH_HEADER", {
          severity: "medium",
          path: c.req.path,
          method: c.req.method,
        });

        return c.json(
          {
            error: {
              code: "MISSING_AUTH_HEADER",
              message: "認証ヘッダーが必要です",
              timestamp: new Date().toISOString(),
              requestId: c.get("requestId") || crypto.randomUUID(),
            },
          },
          401,
        );
      }

      // Bearer トークンの形式をチェック
      const tokenMatch = authHeader.match(/^Bearer\s+(.+)$/);
      if (!tokenMatch) {
        logSecurityEvent(c, "INVALID_AUTH_HEADER", {
          severity: "medium",
          authHeaderFormat: "invalid",
          path: c.req.path,
          method: c.req.method,
        });

        return c.json(
          {
            error: {
              code: "INVALID_AUTH_HEADER",
              message: "認証ヘッダーの形式が正しくありません（Bearer <token>）",
              timestamp: new Date().toISOString(),
              requestId: c.get("requestId") || crypto.randomUUID(),
            },
          },
          401,
        );
      }

      const token = tokenMatch[1];

      // トークンを検証
      const validationResult = await authService.validateToken(token);

      if (!validationResult.isValid || !validationResult.user) {
        logSecurityEvent(c, "INVALID_TOKEN", {
          severity: "high",
          error: validationResult.error,
          path: c.req.path,
          method: c.req.method,
        });

        return c.json(
          {
            error: {
              code: "INVALID_TOKEN",
              message: validationResult.error || "無効なトークンです",
              timestamp: new Date().toISOString(),
              requestId: c.get("requestId") || crypto.randomUUID(),
            },
          },
          401,
        );
      }

      // 認証されたユーザー情報をコンテキストに設定
      c.set("user", validationResult.user);

      enhancedLogger.debug("認証が成功しました", {
        userId: validationResult.user.id,
        email: validationResult.user.email,
        path: c.req.path,
        method: c.req.method,
      });

      await next();
    } catch (error) {
      enhancedLogger.systemFailure("認証ミドルウェアでエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
        path: c.req.path,
        method: c.req.method,
      });

      return c.json(
        {
          error: {
            code: "AUTH_MIDDLEWARE_ERROR",
            message: "認証処理でエラーが発生しました",
            timestamp: new Date().toISOString(),
            requestId: c.get("requestId") || crypto.randomUUID(),
          },
        },
        500,
      );
    }
  };
}

/**
 * 権限チェックミドルウェアを作成する
 * @param authService 認証サービス
 * @param resource 必要なリソース権限
 * @returns 権限チェックミドルウェア関数
 */
export function createPermissionMiddleware(authService: AuthService, resource: string) {
  return async (c: Context, next: Next) => {
    try {
      const user = c.get("user");

      if (!user) {
        enhancedLogger.systemFailure("権限チェック時にユーザー情報が見つかりません", {
          path: c.req.path,
          method: c.req.method,
          resource,
        });

        return c.json(
          {
            error: {
              code: "USER_NOT_FOUND",
              message: "ユーザー情報が見つかりません",
              timestamp: new Date().toISOString(),
              requestId: c.get("requestId") || crypto.randomUUID(),
            },
          },
          401,
        );
      }

      // 権限をチェック
      const hasPermission = await authService.checkPermission(user.id, resource);

      if (!hasPermission) {
        logSecurityEvent(c, "INSUFFICIENT_PERMISSIONS", {
          severity: "high",
          userId: user.id,
          resource,
          path: c.req.path,
          method: c.req.method,
        });

        return c.json(
          {
            error: {
              code: "INSUFFICIENT_PERMISSIONS",
              message: `リソース '${resource}' へのアクセス権限がありません`,
              timestamp: new Date().toISOString(),
              requestId: c.get("requestId") || crypto.randomUUID(),
            },
          },
          403,
        );
      }

      enhancedLogger.debug("権限チェックが成功しました", {
        userId: user.id,
        resource,
        path: c.req.path,
        method: c.req.method,
      });

      await next();
    } catch (error) {
      enhancedLogger.systemFailure("権限チェックミドルウェアでエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
        resource,
        path: c.req.path,
        method: c.req.method,
      });

      return c.json(
        {
          error: {
            code: "PERMISSION_MIDDLEWARE_ERROR",
            message: "権限チェック処理でエラーが発生しました",
            timestamp: new Date().toISOString(),
            requestId: c.get("requestId") || crypto.randomUUID(),
          },
        },
        500,
      );
    }
  };
}

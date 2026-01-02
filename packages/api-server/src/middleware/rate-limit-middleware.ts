/**
 * レート制限ミドルウェア
 * IPアドレスベースでリクエスト数を制限する
 */

import type { Context, Next } from "hono";
import type { RateLimitConfig } from "../types/config.js";
import { logger } from "../utils/logger.js";

interface RateLimitEntry {
  count: number;
  resetTime: number;
}

/**
 * レート制限ストレージ（メモリベース）
 * 本番環境ではRedisなどの外部ストレージを使用することを推奨
 */
class RateLimitStore {
  private store = new Map<string, RateLimitEntry>();

  /**
   * リクエスト数を取得・更新
   * @param key クライアント識別キー（通常はIPアドレス）
   * @param windowMs 制限時間窓（ミリ秒）
   * @returns 現在のリクエスト数と制限時間
   */
  increment(key: string, windowMs: number): { count: number; resetTime: number } {
    const now = Date.now();
    const entry = this.store.get(key);

    if (!entry || now > entry.resetTime) {
      // 新しいエントリまたは期限切れの場合
      const newEntry: RateLimitEntry = {
        count: 1,
        resetTime: now + windowMs,
      };
      this.store.set(key, newEntry);
      return newEntry;
    }

    // 既存エントリのカウントを増加
    entry.count++;
    this.store.set(key, entry);
    return entry;
  }

  /**
   * 期限切れエントリをクリーンアップ
   */
  cleanup(): void {
    const now = Date.now();
    for (const [key, entry] of this.store.entries()) {
      if (now > entry.resetTime) {
        this.store.delete(key);
      }
    }
  }

  /**
   * 特定のキーをリセット（テスト用）
   * @param key クライアント識別キー
   */
  reset(key: string): void {
    this.store.delete(key);
  }

  /**
   * 全エントリをクリア（テスト用）
   */
  clear(): void {
    this.store.clear();
  }
}

// グローバルストレージインスタンス
const rateLimitStore = new RateLimitStore();

// 定期的なクリーンアップ（5分間隔）
setInterval(
  () => {
    rateLimitStore.cleanup();
  },
  5 * 60 * 1000,
);

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
 * レート制限ミドルウェアを作成
 * @param config レート制限設定
 * @returns ミドルウェア関数
 */
export function createRateLimitMiddleware(config: RateLimitConfig) {
  return async (c: Context, next: Next) => {
    const clientIP = getClientIP(c);
    const key = `rate_limit:${clientIP}`;

    try {
      const { count, resetTime } = rateLimitStore.increment(key, config.windowMs);

      // レスポンスヘッダーに制限情報を追加
      c.header("X-RateLimit-Limit", config.maxRequests.toString());
      c.header("X-RateLimit-Remaining", Math.max(0, config.maxRequests - count).toString());
      c.header("X-RateLimit-Reset", Math.ceil(resetTime / 1000).toString());

      if (count > config.maxRequests) {
        // レート制限に達した場合
        const retryAfter = Math.ceil((resetTime - Date.now()) / 1000);
        c.header("Retry-After", retryAfter.toString());

        logger.warn("レート制限に達しました", {
          clientIP,
          count,
          limit: config.maxRequests,
          resetTime: new Date(resetTime).toISOString(),
          path: c.req.path,
          method: c.req.method,
          userAgent: c.req.header("user-agent"),
        });

        return c.json(
          {
            error: {
              code: "RATE_LIMIT_EXCEEDED",
              message: "リクエスト数が制限を超えました。しばらく待ってから再試行してください。",
              details: {
                limit: config.maxRequests,
                windowMs: config.windowMs,
                retryAfter,
              },
              timestamp: new Date().toISOString(),
              requestId: crypto.randomUUID(),
            },
          },
          429,
        );
      }

      // リクエスト情報をログに記録
      logger.debug("レート制限チェック通過", {
        clientIP,
        count,
        limit: config.maxRequests,
        remaining: config.maxRequests - count,
        path: c.req.path,
        method: c.req.method,
      });

      await next();
    } catch (error) {
      logger.error("レート制限ミドルウェアでエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
        clientIP,
        path: c.req.path,
        method: c.req.method,
      });

      // エラーが発生してもリクエストは通す（フェイルオープン）
      await next();
    }
  };
}

/**
 * テスト用のユーティリティ関数
 */
export const rateLimitUtils = {
  /**
   * 特定のIPのレート制限をリセット
   * @param ip IPアドレス
   */
  resetIP: (ip: string) => {
    rateLimitStore.reset(`rate_limit:${ip}`);
  },

  /**
   * 全てのレート制限をクリア
   */
  clearAll: () => {
    rateLimitStore.clear();
  },

  /**
   * クリーンアップを手動実行
   */
  cleanup: () => {
    rateLimitStore.cleanup();
  },
};

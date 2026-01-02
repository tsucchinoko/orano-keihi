/**
 * リトライ機能の実装
 * 指数バックオフとジッターを使用したリトライメカニズム
 */

import { logger } from "./logger.js";

/**
 * リトライ設定
 */
export interface RetryConfig {
  maxAttempts: number;
  baseDelayMs: number;
  maxDelayMs: number;
  backoffMultiplier: number;
  jitterMs: number;
  retryableErrors?: (error: Error) => boolean;
}

/**
 * デフォルトリトライ設定
 */
export const DEFAULT_RETRY_CONFIG: RetryConfig = {
  maxAttempts: 3,
  baseDelayMs: 1000,
  maxDelayMs: 10000,
  backoffMultiplier: 2,
  jitterMs: 100,
};

/**
 * R2操作用リトライ設定
 */
export const R2_RETRY_CONFIG: RetryConfig = {
  maxAttempts: 3,
  baseDelayMs: 500,
  maxDelayMs: 5000,
  backoffMultiplier: 2,
  jitterMs: 200,
  retryableErrors: (error: Error) => {
    // ネットワークエラー、タイムアウト、5xx系エラーはリトライ対象
    const message = error.message.toLowerCase();
    return (
      message.includes("network") ||
      message.includes("timeout") ||
      message.includes("connection") ||
      message.includes("503") ||
      message.includes("502") ||
      message.includes("504")
    );
  },
};

/**
 * 認証用リトライ設定
 */
export const AUTH_RETRY_CONFIG: RetryConfig = {
  maxAttempts: 2,
  baseDelayMs: 200,
  maxDelayMs: 1000,
  backoffMultiplier: 2,
  jitterMs: 50,
  retryableErrors: (error: Error) => {
    // 一時的な認証サービス障害のみリトライ
    const message = error.message.toLowerCase();
    return message.includes("service unavailable") || message.includes("timeout");
  },
};

/**
 * データベース操作用リトライ設定
 */
export const DATABASE_RETRY_CONFIG: RetryConfig = {
  maxAttempts: 2,
  baseDelayMs: 100,
  maxDelayMs: 2000,
  backoffMultiplier: 2,
  jitterMs: 50,
  retryableErrors: (error: Error) => {
    // ロック競合、一時的な接続エラーのみリトライ
    const message = error.message.toLowerCase();
    return message.includes("locked") || message.includes("busy") || message.includes("connection");
  },
};

/**
 * 遅延計算（指数バックオフ + ジッター）
 */
function calculateDelay(attempt: number, config: RetryConfig): number {
  const exponentialDelay = config.baseDelayMs * Math.pow(config.backoffMultiplier, attempt - 1);
  const jitter = Math.random() * config.jitterMs;
  const totalDelay = Math.min(exponentialDelay + jitter, config.maxDelayMs);

  return Math.floor(totalDelay);
}

/**
 * 遅延実行
 */
function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * リトライ実行
 */
export async function withRetry<T>(
  operation: () => Promise<T>,
  config: RetryConfig = DEFAULT_RETRY_CONFIG,
  context?: string,
): Promise<T> {
  let lastError: Error;

  for (let attempt = 1; attempt <= config.maxAttempts; attempt++) {
    try {
      const result = await operation();

      if (attempt > 1) {
        logger.info("リトライが成功しました", {
          context,
          attempt,
          totalAttempts: config.maxAttempts,
        });
      }

      return result;
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));

      // 最後の試行の場合はエラーを投げる
      if (attempt === config.maxAttempts) {
        logger.error("リトライが最大回数に達しました", {
          context,
          attempt,
          totalAttempts: config.maxAttempts,
          error: lastError.message,
        });
        throw lastError;
      }

      // リトライ可能かチェック
      if (config.retryableErrors && !config.retryableErrors(lastError)) {
        logger.warn("リトライ不可能なエラーです", {
          context,
          attempt,
          error: lastError.message,
        });
        throw lastError;
      }

      const delayMs = calculateDelay(attempt, config);

      logger.warn("操作が失敗しました。リトライします", {
        context,
        attempt,
        totalAttempts: config.maxAttempts,
        error: lastError.message,
        nextRetryInMs: delayMs,
      });

      await delay(delayMs);
    }
  }

  throw lastError!;
}
/**
 * R2操作用リトライラッパー
 */
export async function withR2Retry<T>(operation: () => Promise<T>, context?: string): Promise<T> {
  return withRetry(operation, R2_RETRY_CONFIG, context);
}

/**
 * 認証操作用リトライラッパー
 */
export async function withAuthRetry<T>(operation: () => Promise<T>, context?: string): Promise<T> {
  return withRetry(operation, AUTH_RETRY_CONFIG, context);
}

/**
 * データベース操作用リトライラッパー
 */
export async function withDatabaseRetry<T>(
  operation: () => Promise<T>,
  context?: string,
): Promise<T> {
  return withRetry(operation, DATABASE_RETRY_CONFIG, context);
}

/**
 * カスタムリトライ設定でのリトライラッパー
 */
export async function withCustomRetry<T>(
  operation: () => Promise<T>,
  maxAttempts: number,
  baseDelayMs: number,
  context?: string,
): Promise<T> {
  const config: RetryConfig = {
    ...DEFAULT_RETRY_CONFIG,
    maxAttempts,
    baseDelayMs,
  };

  return withRetry(operation, config, context);
}

/**
 * リトライ統計情報
 */
export interface RetryStats {
  totalOperations: number;
  successfulOperations: number;
  failedOperations: number;
  totalRetries: number;
  averageRetries: number;
}

/**
 * リトライ統計トラッカー
 */
class RetryStatsTracker {
  private stats: RetryStats = {
    totalOperations: 0,
    successfulOperations: 0,
    failedOperations: 0,
    totalRetries: 0,
    averageRetries: 0,
  };

  recordOperation(success: boolean, retryCount: number) {
    this.stats.totalOperations++;
    if (success) {
      this.stats.successfulOperations++;
    } else {
      this.stats.failedOperations++;
    }

    if (retryCount > 0) {
      this.stats.totalRetries += retryCount;
    }

    this.stats.averageRetries = this.stats.totalRetries / this.stats.totalOperations;
  }

  getStats(): RetryStats {
    return { ...this.stats };
  }

  reset() {
    this.stats = {
      totalOperations: 0,
      successfulOperations: 0,
      failedOperations: 0,
      totalRetries: 0,
      averageRetries: 0,
    };
  }
}

export const retryStatsTracker = new RetryStatsTracker();

/**
 * 統計付きリトライ実行
 */
export async function withRetryAndStats<T>(
  operation: () => Promise<T>,
  config: RetryConfig = DEFAULT_RETRY_CONFIG,
  context?: string,
): Promise<T> {
  let retryCount = 0;
  let success = false;

  try {
    const result = await withRetry(
      async () => {
        try {
          return await operation();
        } catch (error) {
          retryCount++;
          throw error;
        }
      },
      config,
      context,
    );

    success = true;
    return result;
  } catch (error) {
    success = false;
    throw error;
  } finally {
    retryStatsTracker.recordOperation(success, retryCount);
  }
}

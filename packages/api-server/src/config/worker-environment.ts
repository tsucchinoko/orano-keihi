/**
 * Cloudflare Workers環境用の設定
 */

import type { ApiServerConfig } from "../types/config.js";
import type { Env } from "../worker.js";

/**
 * Workers環境用の設定を読み込み
 */
export function loadWorkerConfig(env: Env): ApiServerConfig {
  return {
    // サーバー設定（Workersでは使用しない）
    host: "0.0.0.0",
    port: 8080,
    nodeEnv: (env.NODE_ENV as "development" | "production" | "test") || "production",

    // R2設定（Workersではバインディングを使用するため、ダミー値でも可）
    r2: {
      endpoint: env.R2_ENDPOINT || "auto", // Workersではバインディングを使用
      accessKeyId: env.R2_ACCESS_KEY_ID || "binding", // Workersではバインディングを使用
      secretAccessKey: env.R2_SECRET_ACCESS_KEY || "binding", // Workersではバインディングを使用
      bucketName: env.R2_BUCKET_NAME || "orano-keihi-dev", // バケット名は必要
      region: env.R2_REGION || "auto",
    },

    // 認証設定
    auth: {
      jwtSecret: env.JWT_SECRET || "development-secret-key-for-testing-only",
      sessionEncryptionKey:
        env.SESSION_ENCRYPTION_KEY || env.JWT_SECRET || "development-encryption-key-32-bytes",
      sessionExpirationDays: Number(env.SESSION_EXPIRATION_DAYS) || 30,
    },

    // CORS設定
    cors: {
      origin: env.CORS_ORIGIN?.split(",") || ["http://localhost:1420", "tauri://localhost"],
      methods: ["GET", "POST", "PUT", "DELETE", "OPTIONS"],
      headers: ["Content-Type", "Authorization", "X-Requested-With", "Accept", "Origin"],
    },

    // ファイルアップロード設定
    fileUpload: {
      maxFileSize: parseInt(env.MAX_FILE_SIZE || "10485760"), // 10MB
      allowedTypes: env.ALLOWED_FILE_TYPES?.split(",") || [
        "image/jpeg",
        "image/png",
        "image/gif",
        "image/webp",
        "application/pdf",
        "text/plain",
      ],
      maxFiles: parseInt(env.MAX_FILES_PER_REQUEST || "10"),
    },

    // レート制限設定
    rateLimit: {
      windowMs: parseInt(env.RATE_LIMIT_WINDOW_MS || "900000"), // 15分
      maxRequests: parseInt(env.RATE_LIMIT_MAX_REQUESTS || "100"),
    },

    // ログ設定
    logging: {
      level: (env.LOG_LEVEL as "error" | "warn" | "info" | "debug") || "info",
      file: env.LOG_FILE || "logs/app.log",
    },
  };
}

/**
 * 設定情報を表示用にマスク（機密情報を隠す）
 */
export function getWorkerConfigForDisplay(config: ApiServerConfig) {
  return {
    nodeEnv: config.nodeEnv,
    r2: {
      endpoint: config.r2.endpoint ? "***設定済み***" : "未設定",
      bucketName: config.r2.bucketName || "未設定",
      region: config.r2.region,
    },
    auth: {
      jwtSecret: config.auth.jwtSecret ? "***設定済み***" : "未設定",
    },
    cors: config.cors,
    fileUpload: config.fileUpload,
    rateLimit: config.rateLimit,
    logging: config.logging,
  };
}

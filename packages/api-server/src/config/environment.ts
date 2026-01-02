/**
 * 環境変数による設定管理
 * アプリケーションの設定を環境変数から読み込む
 */

import { config } from "dotenv";
import { z } from "zod";
import type { ApiServerConfig, R2Config } from "../types/config.js";

// 環境変数を読み込み
config();

// 環境変数のスキーマ定義
const envSchema = z.object({
  // サーバー設定
  PORT: z.string().default("3000").transform(Number),
  HOST: z.string().default("localhost"),
  NODE_ENV: z.enum(["development", "production", "test"]).default("development"),

  // CORS設定
  CORS_ORIGIN: z.string().default("http://localhost:1420,tauri://localhost"),

  // R2設定
  R2_ENDPOINT: z.string(),
  R2_ACCESS_KEY_ID: z.string(),
  R2_SECRET_ACCESS_KEY: z.string(),
  R2_BUCKET_NAME: z.string(),
  R2_REGION: z.string().default("auto"),

  // 認証設定
  JWT_SECRET: z.string(),

  // ファイルアップロード設定
  MAX_FILE_SIZE: z.string().default("10485760").transform(Number), // 10MB
  ALLOWED_FILE_TYPES: z.string().default("image/jpeg,image/png,image/gif,application/pdf"),
  MAX_FILES_PER_REQUEST: z.string().default("10").transform(Number),

  // レート制限設定
  RATE_LIMIT_WINDOW_MS: z.string().default("900000").transform(Number), // 15分
  RATE_LIMIT_MAX_REQUESTS: z.string().default("100").transform(Number),

  // ログ設定
  LOG_LEVEL: z.enum(["error", "warn", "info", "debug"]).default("info"),
  LOG_FILE: z.string().default("logs/app.log"),
});

/**
 * 環境変数を検証して設定オブジェクトを作成
 */
export function loadConfig(): ApiServerConfig {
  try {
    const env = envSchema.parse(process.env);

    const r2Config: R2Config = {
      endpoint: env.R2_ENDPOINT,
      accessKeyId: env.R2_ACCESS_KEY_ID,
      secretAccessKey: env.R2_SECRET_ACCESS_KEY,
      bucketName: env.R2_BUCKET_NAME,
      region: env.R2_REGION,
    };

    const config: ApiServerConfig = {
      port: env.PORT,
      host: env.HOST,
      nodeEnv: env.NODE_ENV,
      cors: {
        origin: env.CORS_ORIGIN.split(",").map((origin) => origin.trim()),
        methods: ["GET", "POST", "PUT", "DELETE", "OPTIONS"],
        headers: ["Content-Type", "Authorization"],
      },
      r2: r2Config,
      auth: {
        jwtSecret: env.JWT_SECRET,
      },
      fileUpload: {
        maxFileSize: env.MAX_FILE_SIZE,
        allowedTypes: env.ALLOWED_FILE_TYPES.split(",").map((type) => type.trim()),
        maxFiles: env.MAX_FILES_PER_REQUEST,
      },
      rateLimit: {
        windowMs: env.RATE_LIMIT_WINDOW_MS,
        maxRequests: env.RATE_LIMIT_MAX_REQUESTS,
      },
      logging: {
        level: env.LOG_LEVEL,
        file: env.LOG_FILE,
      },
    };

    return config;
  } catch (error) {
    if (error instanceof z.ZodError) {
      console.error("環境変数の設定に問題があります:");
      error.errors.forEach((err) => {
        console.error(`- ${err.path.join(".")}: ${err.message}`);
      });
    } else {
      console.error("設定の読み込みに失敗しました:", error);
    }
    process.exit(1);
  }
}

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
  SESSION_ENCRYPTION_KEY: z.string().optional(),
  SESSION_EXPIRATION_DAYS: z.string().default("30").transform(Number),

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
 * R2設定の詳細バリデーション
 * @param r2Config R2設定オブジェクト
 * @returns バリデーション結果
 */
export function validateR2Config(r2Config: R2Config): { isValid: boolean; errors: string[] } {
  const errors: string[] = [];

  // エンドポイントの形式チェック
  if (!r2Config.endpoint) {
    errors.push("R2_ENDPOINTが設定されていません");
  } else if (!r2Config.endpoint.includes(".") && r2Config.endpoint.length < 32) {
    // アカウントIDの形式チェック（32文字のハッシュまたはドメイン形式）
    errors.push(
      "R2_ENDPOINTの形式が正しくありません（アカウントIDまたは完全なエンドポイントURLを指定してください）",
    );
  }

  // アクセスキーの形式チェック
  if (!r2Config.accessKeyId) {
    errors.push("R2_ACCESS_KEY_IDが設定されていません");
  } else if (r2Config.accessKeyId.length < 20) {
    errors.push("R2_ACCESS_KEY_IDの形式が正しくありません");
  }

  // シークレットキーの形式チェック
  if (!r2Config.secretAccessKey) {
    errors.push("R2_SECRET_ACCESS_KEYが設定されていません");
  } else if (r2Config.secretAccessKey.length < 40) {
    errors.push("R2_SECRET_ACCESS_KEYの形式が正しくありません");
  }

  // バケット名の形式チェック
  if (!r2Config.bucketName) {
    errors.push("R2_BUCKET_NAMEが設定されていません");
  } else if (
    !/^[a-z0-9][a-z0-9-]*[a-z0-9]$/.test(r2Config.bucketName) ||
    r2Config.bucketName.length < 3 ||
    r2Config.bucketName.length > 63
  ) {
    errors.push(
      "R2_BUCKET_NAMEの形式が正しくありません（3-63文字、小文字・数字・ハイフンのみ、先頭末尾は英数字）",
    );
  }

  return {
    isValid: errors.length === 0,
    errors,
  };
}

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

    // R2設定の詳細バリデーション
    const r2Validation = validateR2Config(r2Config);
    if (!r2Validation.isValid) {
      console.error("R2設定に問題があります:");
      r2Validation.errors.forEach((error) => {
        console.error(`- ${error}`);
      });
      process.exit(1);
    }

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
        sessionEncryptionKey: env.SESSION_ENCRYPTION_KEY || env.JWT_SECRET,
        sessionExpirationDays: env.SESSION_EXPIRATION_DAYS || 30,
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

/**
 * 設定情報を安全に表示（機密情報をマスク）
 * @param config APIサーバー設定
 * @returns 表示用設定オブジェクト
 */
export function getConfigForDisplay(config: ApiServerConfig) {
  return {
    ...config,
    r2: {
      ...config.r2,
      secretAccessKey: "[HIDDEN]",
    },
    auth: {
      jwtSecret: "[HIDDEN]",
    },
  };
}

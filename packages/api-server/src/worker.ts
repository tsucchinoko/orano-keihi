/**
 * Cloudflare Workers用のエントリーポイント
 * Honoアプリケーションをワーカー環境で実行
 */

import { createApp } from "./app.js";
import { loadWorkerConfig } from "./config/worker-environment.js";
import { logger } from "./utils/logger.js";

// Cloudflare Workers環境の型定義
export interface Env {
  // 基本環境変数
  NODE_ENV: string;

  // D1データベース（バインディング）
  DB: D1Database;

  // R2バケット（バインディング）
  R2_BUCKET: R2Bucket;

  // R2設定（環境変数）
  R2_ACCOUNT_ID?: string;
  R2_ENDPOINT?: string;
  R2_ACCESS_KEY_ID?: string;
  R2_SECRET_ACCESS_KEY?: string;
  R2_BUCKET_NAME?: string;
  R2_REGION?: string;

  // Google OAuth設定（機密情報）
  GOOGLE_CLIENT_ID?: string;
  GOOGLE_CLIENT_SECRET?: string;

  // 認証設定（機密情報）
  JWT_SECRET?: string;
  SESSION_ENCRYPTION_KEY?: string;
  SESSION_EXPIRATION_DAYS?: string;

  // CORS設定
  CORS_ORIGIN?: string;

  // ファイルアップロード設定
  MAX_FILE_SIZE?: string;
  ALLOWED_FILE_TYPES?: string;
  MAX_FILES_PER_REQUEST?: string;

  // レート制限設定
  RATE_LIMIT_WINDOW_MS?: string;
  RATE_LIMIT_MAX_REQUESTS?: string;

  // ログ設定
  LOG_LEVEL?: string;
  LOG_FILE?: string;
}

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    try {
      // Workers環境用の設定を読み込み
      const config = loadWorkerConfig(env);

      // Honoアプリケーションを作成（D1データベース、R2バケットバインディング、アカウントIDを渡す）
      const app = createApp(config, env.DB, env.R2_BUCKET, env.R2_ACCOUNT_ID);

      // リクエストを処理
      return await app.fetch(request, env, ctx);
    } catch (error) {
      // より詳細なエラー情報をログに記録
      const errorMessage = error instanceof Error ? error.message : String(error);
      const errorStack = error instanceof Error ? error.stack : undefined;

      console.error("Workerでエラーが発生しました:", {
        message: errorMessage,
        stack: errorStack,
        error: error,
      });

      logger.error("Workerでエラーが発生しました", {
        message: errorMessage,
        stack: errorStack,
      });

      return new Response(
        JSON.stringify({
          error: {
            code: "WORKER_ERROR",
            message: "内部サーバーエラーが発生しました",
            details: errorMessage, // 開発環境では詳細なエラーメッセージを含める
            timestamp: new Date().toISOString(),
          },
        }),
        {
          status: 500,
          headers: {
            "Content-Type": "application/json",
          },
        },
      );
    }
  },
};

/**
 * APIサーバーのエントリーポイント
 * TypeScriptとHonoを使用したファイルアップロードAPIサーバー
 */

import { serve } from "@hono/node-server";
import { createApp } from "./app.js";
import { loadConfig } from "./config/environment.js";
import { logger } from "./utils/logger.js";

async function startServer() {
  try {
    // 設定を読み込み
    const config = loadConfig();

    // Honoアプリケーションを作成
    const app = createApp(config);

    // サーバーを起動
    serve({
      fetch: app.fetch,
      port: config.port,
      hostname: config.host,
    });

    logger.info(`APIサーバーが起動しました`, {
      host: config.host,
      port: config.port,
      environment: config.nodeEnv,
    });

    // グレースフルシャットダウンの設定
    process.on("SIGTERM", () => {
      logger.info("SIGTERMを受信しました。サーバーを停止します...");
      process.exit(0);
    });

    process.on("SIGINT", () => {
      logger.info("SIGINTを受信しました。サーバーを停止します...");
      process.exit(0);
    });
  } catch (error) {
    logger.error("サーバーの起動に失敗しました", { error });
    process.exit(1);
  }
}

// サーバーを起動
startServer();

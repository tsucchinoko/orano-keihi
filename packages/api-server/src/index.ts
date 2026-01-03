/**
 * APIサーバーのエントリーポイント
 * TypeScriptとHonoを使用したファイルアップロードAPIサーバー
 */

import { serve } from "@hono/node-server";
import { createApp } from "./app.js";
import { loadConfig, getConfigForDisplay } from "./config/environment.js";
import { logger } from "./utils/logger.js";
import { createR2Client, createR2TestService } from "./services/index.js";

async function startServer() {
  try {
    // 設定を読み込み
    const config = loadConfig();

    // 設定情報を表示（機密情報はマスク）
    logger.info("APIサーバーの設定を読み込みました", getConfigForDisplay(config));

    // R2接続の初期確認
    logger.info("R2接続を確認しています...");
    const r2Client = createR2Client(config.r2);
    const r2TestService = createR2TestService(r2Client);

    const connectionTest = await r2TestService.quickConnectionTest();
    if (connectionTest) {
      logger.info("R2接続が確認できました");
    } else {
      logger.warn(
        "R2接続に問題があります。詳細は /api/v1/system/r2/test エンドポイントで確認してください",
      );
    }

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
      endpoints: {
        health: `http://${config.host}:${config.port}/api/v1/health`,
        r2Test: `http://${config.host}:${config.port}/api/v1/system/r2/test`,
        r2Ping: `http://${config.host}:${config.port}/api/v1/system/r2/ping`,
      },
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

// サーバーを起動（エラーを無視）
void startServer();

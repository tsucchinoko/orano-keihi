/**
 * APIサーバーのエントリーポイント
 * TypeScriptとHonoを使用したファイルアップロードAPIサーバー
 */

import { loadConfig, getConfigForDisplay } from "./config/environment.js";
import { logger } from "./utils/logger.js";

async function startServer() {
  try {
    // 設定を読み込み
    const config = loadConfig();

    // 設定情報を表示（機密情報はマスク）
    logger.info("APIサーバーの設定を読み込みました", getConfigForDisplay(config));

    // ローカル開発環境ではD1を使用できないため、エラーを表示
    logger.error("ローカル開発環境ではD1データベースが利用できません。");
    logger.error("Cloudflare Workers環境（wrangler dev）でAPIサーバーを起動してください。");
    logger.error("コマンド: npm run dev");
    process.exit(1);
  } catch (error) {
    logger.error("サーバーの起動に失敗しました", { error });
    process.exit(1);
  }
}

// サーバーを起動（エラーを無視）
void startServer();

/**
 * 構造化ログシステム
 * Winstonを使用したログ出力機能
 */

import winston from "winston";
import { existsSync, mkdirSync } from "fs";

// ログディレクトリを作成
const logDir = "logs";
if (!existsSync(logDir)) {
  mkdirSync(logDir, { recursive: true });
}

// ログフォーマット
const logFormat = winston.format.combine(
  winston.format.timestamp({
    format: "YYYY-MM-DD HH:mm:ss",
  }),
  winston.format.errors({ stack: true }),
  winston.format.json(),
);

// コンソール用フォーマット
const consoleFormat = winston.format.combine(
  winston.format.colorize(),
  winston.format.timestamp({
    format: "YYYY-MM-DD HH:mm:ss",
  }),
  winston.format.printf(({ timestamp, level, message, ...meta }) => {
    let log = `${timestamp} [${level}]: ${message}`;
    if (Object.keys(meta).length > 0) {
      log += ` ${JSON.stringify(meta)}`;
    }
    return log;
  }),
);

// ロガーの作成
export const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || "info",
  format: logFormat,
  defaultMeta: { service: "api-server" },
  transports: [
    // ファイル出力
    new winston.transports.File({
      filename: "logs/error.log",
      level: "error",
      maxsize: 5242880, // 5MB
      maxFiles: 5,
    }),
    new winston.transports.File({
      filename: "logs/app.log",
      maxsize: 5242880, // 5MB
      maxFiles: 5,
    }),
  ],
});

// 開発環境ではコンソールにも出力
if (process.env.NODE_ENV !== "production") {
  logger.add(
    new winston.transports.Console({
      format: consoleFormat,
    }),
  );
}

// プロセス終了時のログ
process.on("exit", () => {
  logger.info("APIサーバーが停止しました");
});

process.on("uncaughtException", (error) => {
  logger.error("キャッチされていない例外が発生しました", { error });
  process.exit(1);
});

process.on("unhandledRejection", (reason, promise) => {
  logger.error("処理されていないPromise拒否が発生しました", {
    reason,
    promise: promise.toString(),
  });
});

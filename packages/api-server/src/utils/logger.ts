/**
 * 構造化ログシステム
 * Winstonを使用したログ出力機能とアラート生成
 */

import winston from "winston";

// Cloudflare Workers環境かどうかを判定
const isCloudflareWorkers = typeof globalThis !== "undefined" && "WorkerGlobalScope" in globalThis;

// Node.js環境でのみファイルシステムを使用
let canUseFileSystem = false;
if (!isCloudflareWorkers) {
  try {
    // 動的インポートを使用してファイルシステムの利用可能性をチェック
    const fs = require("fs");
    const logDir = "logs";
    if (!fs.existsSync(logDir)) {
      fs.mkdirSync(logDir, { recursive: true });
    }
    canUseFileSystem = true;
  } catch (error) {
    // ファイルシステムが利用できない環境では何もしない
    console.warn("ファイルシステムが利用できません。ログはコンソールのみに出力されます。");
    canUseFileSystem = false;
  }
}

/**
 * アラートレベルの定義
 */
export enum AlertLevel {
  LOW = "low",
  MEDIUM = "medium",
  HIGH = "high",
  CRITICAL = "critical",
}

/**
 * アラート情報の型定義
 */
export interface AlertInfo {
  level: AlertLevel;
  title: string;
  message: string;
  details?: Record<string, any>;
  timestamp: string;
  source: string;
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
  winston.format.printf((info) => {
    const { timestamp, level, message, ...meta } = info as any;
    const messageStr = typeof message === "string" ? message : JSON.stringify(message);
    let log = `${String(timestamp)} [${String(level)}]: ${messageStr}`;
    if (Object.keys(meta).length > 0) {
      log += ` ${JSON.stringify(meta)}`;
    }
    return log;
  }),
);

// ロガーの作成
const transports: winston.transport[] = [];

// Node.js環境でのみファイルトランスポートを追加
if (canUseFileSystem) {
  transports.push(
    // エラーログファイル
    new winston.transports.File({
      filename: "logs/error.log",
      level: "error",
      maxsize: 5242880, // 5MB
      maxFiles: 5,
    }),
    // 警告ログファイル
    new winston.transports.File({
      filename: "logs/warn.log",
      level: "warn",
      maxsize: 5242880, // 5MB
      maxFiles: 5,
    }),
    // 一般ログファイル
    new winston.transports.File({
      filename: "logs/app.log",
      maxsize: 5242880, // 5MB
      maxFiles: 5,
    }),
    // セキュリティログファイル
    new winston.transports.File({
      filename: "logs/security.log",
      level: "warn",
      maxsize: 5242880, // 5MB
      maxFiles: 10,
      format: winston.format.combine(
        winston.format.timestamp(),
        winston.format.json(),
        winston.format.printf((info) => {
          // セキュリティ関連のログのみを記録
          if (info.type === "security_event" || info.event || info.level === "error") {
            return JSON.stringify(info);
          }
          return "";
        }),
      ),
    }),
  );
}

// コンソールトランスポートは常に追加
transports.push(
  new winston.transports.Console({
    format: consoleFormat,
  }),
);

export const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || "info",
  format: logFormat,
  defaultMeta: { service: "api-server" },
  transports,
});

/**
 * アラート生成システム
 */
class AlertSystem {
  private alerts: AlertInfo[] = [];
  private readonly maxAlerts = 1000; // メモリ内に保持する最大アラート数

  /**
   * アラートを生成
   * @param alert アラート情報
   */
  generateAlert(alert: AlertInfo): void {
    // アラートをメモリに保存
    this.alerts.unshift(alert);
    if (this.alerts.length > this.maxAlerts) {
      this.alerts = this.alerts.slice(0, this.maxAlerts);
    }

    // アラートログファイルに記録
    logger.error("アラートが生成されました", {
      type: "alert",
      alert,
    });

    // コンソールにも出力（重要なアラートの場合）
    if (alert.level === AlertLevel.HIGH || alert.level === AlertLevel.CRITICAL) {
      console.error(`🚨 ${alert.level.toUpperCase()} ALERT: ${alert.title}`);
      console.error(`   Message: ${alert.message}`);
      console.error(`   Source: ${alert.source}`);
      console.error(`   Time: ${alert.timestamp}`);
      if (alert.details) {
        console.error(`   Details:`, alert.details);
      }
    }

    // 本番環境では外部アラートシステム（Slack、メール等）に送信
    if (process.env.NODE_ENV === "production") {
      void this.sendExternalAlert(alert);
    }
  }

  /**
   * 外部アラートシステムに送信（実装例）
   * @param alert アラート情報
   */
  private async sendExternalAlert(alert: AlertInfo): Promise<void> {
    try {
      // 実際の実装では、Slack Webhook、メール送信、PagerDuty等を使用
      // ここでは例として、重要なアラートのみ処理
      if (alert.level === AlertLevel.CRITICAL) {
        // 例: Slack Webhook URL（環境変数から取得）
        const webhookUrl = process.env.SLACK_WEBHOOK_URL;
        if (webhookUrl) {
          // Slack通知の実装（実際のHTTPリクエストは省略）
          logger.info("外部アラートシステムに通知を送信しました", {
            type: "external_alert",
            alert: {
              level: alert.level,
              title: alert.title,
              source: alert.source,
            },
          });
        }
      }
    } catch (error) {
      logger.error("外部アラートシステムへの送信に失敗しました", {
        error: error instanceof Error ? error.message : String(error),
        alert: alert.title,
      });
    }
  }

  /**
   * 最近のアラートを取得
   * @param limit 取得件数
   * @returns アラート配列
   */
  getRecentAlerts(limit = 50): AlertInfo[] {
    return this.alerts.slice(0, limit);
  }

  /**
   * 特定レベル以上のアラートを取得
   * @param minLevel 最小アラートレベル
   * @param limit 取得件数
   * @returns アラート配列
   */
  getAlertsByLevel(minLevel: AlertLevel, limit = 50): AlertInfo[] {
    const levelOrder = {
      [AlertLevel.LOW]: 1,
      [AlertLevel.MEDIUM]: 2,
      [AlertLevel.HIGH]: 3,
      [AlertLevel.CRITICAL]: 4,
    };

    return this.alerts
      .filter((alert) => levelOrder[alert.level] >= levelOrder[minLevel])
      .slice(0, limit);
  }

  /**
   * アラート統計を取得
   * @returns アラート統計情報
   */
  getAlertStats(): Record<AlertLevel, number> {
    const stats = {
      [AlertLevel.LOW]: 0,
      [AlertLevel.MEDIUM]: 0,
      [AlertLevel.HIGH]: 0,
      [AlertLevel.CRITICAL]: 0,
    };

    for (const alert of this.alerts) {
      stats[alert.level]++;
    }

    return stats;
  }
}

// グローバルアラートシステムインスタンス
export const alertSystem = new AlertSystem();

/**
 * 拡張ロガー関数
 */
export const enhancedLogger = {
  // 基本的なログ関数
  error: logger.error.bind(logger),
  warn: logger.warn.bind(logger),
  info: logger.info.bind(logger),
  debug: logger.debug.bind(logger),

  /**
   * セキュリティ関連のエラーログ
   * @param message ログメッセージ
   * @param meta メタデータ
   */
  security: (message: string, meta?: Record<string, any>) => {
    logger.warn(message, { type: "security_event", ...meta });

    // 重要なセキュリティイベントの場合はアラートを生成
    if (meta?.severity === "high" || meta?.severity === "critical") {
      alertSystem.generateAlert({
        level: meta.severity === "critical" ? AlertLevel.CRITICAL : AlertLevel.HIGH,
        title: "セキュリティイベント",
        message,
        details: meta,
        timestamp: new Date().toISOString(),
        source: "security",
      });
    }
  },

  /**
   * システム障害ログ
   * @param message ログメッセージ
   * @param meta メタデータ
   */
  systemFailure: (message: string, meta?: Record<string, any>) => {
    logger.error(message, { type: "system_failure", ...meta });

    alertSystem.generateAlert({
      level: AlertLevel.CRITICAL,
      title: "システム障害",
      message,
      details: meta,
      timestamp: new Date().toISOString(),
      source: "system",
    });
  },

  /**
   * パフォーマンス警告ログ
   * @param message ログメッセージ
   * @param meta メタデータ
   */
  performance: (message: string, meta?: Record<string, any>) => {
    logger.warn(message, { type: "performance_warning", ...meta });

    // 重大なパフォーマンス問題の場合はアラートを生成
    if (meta?.duration && meta.duration > 10000) {
      alertSystem.generateAlert({
        level: AlertLevel.MEDIUM,
        title: "パフォーマンス警告",
        message,
        details: meta,
        timestamp: new Date().toISOString(),
        source: "performance",
      });
    }
  },

  /**
   * ビジネスロジックエラーログ
   * @param message ログメッセージ
   * @param meta メタデータ
   */
  business: (message: string, meta?: Record<string, any>) => {
    const level = meta?.severity === "error" ? "error" : "warn";
    logger[level](message, { type: "business_event", ...meta });

    // 重要なビジネスエラーの場合はアラートを生成
    if (meta?.severity === "error" && meta?.critical) {
      alertSystem.generateAlert({
        level: AlertLevel.HIGH,
        title: "ビジネスロジックエラー",
        message,
        details: meta,
        timestamp: new Date().toISOString(),
        source: "business",
      });
    }
  },
};

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
    promise: promise ? JSON.stringify(promise, null, 2) : "unknown promise",
  });
});

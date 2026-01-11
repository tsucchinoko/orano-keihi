/**
 * 領収書関連のAPIエンドポイント
 */

import { Hono } from "hono";
import type { Context } from "hono";
import { logger } from "../utils/logger.js";
import {
  handleError,
  createValidationError,
  createAuthorizationError,
} from "../utils/error-handler.js";
import { logSecurityEvent } from "../middleware/index.js";
import type { R2ClientInterface } from "../services/r2-client.js";

/**
 * 領収書ルーターを作成
 * @param r2Client R2クライアント
 * @returns 領収書ルーター
 */
export function createReceiptsRouter(r2Client: R2ClientInterface): Hono {
  const receiptsApp = new Hono();

  /**
   * 領収書URLからファイルキーを抽出する
   * @param receiptUrl 領収書URL
   * @returns ファイルキー
   */
  function extractFileKeyFromUrl(receiptUrl: string): string {
    try {
      const url = new URL(receiptUrl);

      // R2 URLの形式: https://{account_id}.r2.cloudflarestorage.com/{bucket_name}/{file_key}
      // パスから先頭の'/'とバケット名を除去してファイルキーとして使用
      const pathParts = url.pathname.substring(1).split("/");

      // 最初の部分はバケット名なので除去
      if (pathParts.length > 1) {
        const encodedFileKey = pathParts.slice(1).join("/");

        // URLデコードを実行
        const fileKey = decodeURIComponent(encodedFileKey);

        logger.debug("URLからファイルキーを抽出", {
          receiptUrl,
          pathname: url.pathname,
          pathParts,
          encodedFileKey,
          decodedFileKey: fileKey,
        });

        return fileKey;
      } else {
        throw new Error("ファイルキーの抽出に失敗しました");
      }
    } catch (error) {
      logger.error("URLからのファイルキー抽出エラー", {
        receiptUrl,
        error: error instanceof Error ? error.message : String(error),
      });

      throw createValidationError(
        "無効な領収書URLです",
        "receiptUrl",
        receiptUrl,
        "valid URL required",
      );
    }
  }

  /**
   * ユーザーがファイルにアクセス権限を持っているかチェック
   * @param userId ユーザーID
   * @param fileKey ファイルキー
   * @returns アクセス権限があるかどうか
   */
  function hasFileAccess(userId: number, fileKey: string): boolean {
    // ファイルキーがユーザーのディレクトリ配下にあるかチェック
    return fileKey.startsWith(`users/${userId}/`) || fileKey.startsWith(`receipts/${userId}/`);
  }

  // 領収書URL経由でのファイル削除
  receiptsApp.delete("/delete-by-url", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createAuthorizationError("認証が必要です");
      }

      const body = await c.req.json();
      const { receiptUrl } = body;

      if (!receiptUrl) {
        throw createValidationError(
          "領収書URLが指定されていません",
          "receiptUrl",
          receiptUrl,
          "required",
        );
      }

      logger.info("領収書URL経由でのファイル削除開始", {
        userId: user.id,
        receiptUrl,
      });

      // URLからファイルキーを抽出
      let fileKey: string;
      try {
        fileKey = extractFileKeyFromUrl(receiptUrl);
      } catch (error) {
        logger.error("ファイルキー抽出エラー", {
          receiptUrl,
          error: error instanceof Error ? error.message : String(error),
        });
        throw error;
      }

      logger.info("ファイルキーを抽出しました", {
        userId: user.id,
        fileKey,
        receiptUrl,
      });

      // ユーザーのアクセス権限をチェック
      const hasAccess = hasFileAccess(user.id, fileKey);

      logger.info("アクセス権限チェック結果", {
        userId: user.id,
        fileKey,
        hasAccess,
        expectedPatterns: [`users/${user.id}/`, `receipts/${user.id}/`],
      });

      if (!hasAccess) {
        logSecurityEvent(c, "UNAUTHORIZED_FILE_DELETE", {
          userId: user.id,
          fileKey,
          receiptUrl,
        });

        throw createAuthorizationError("このファイルを削除する権限がありません");
      }

      // R2からファイルを削除
      try {
        logger.info("R2からのファイル削除を開始します", {
          userId: user.id,
          fileKey,
        });

        // 削除前にファイルの存在確認
        const fileExists = await r2Client.fileExists(fileKey);
        logger.info("削除前のファイル存在確認", {
          userId: user.id,
          fileKey,
          exists: fileExists,
        });

        if (!fileExists) {
          logger.warn("削除対象のファイルが存在しません", {
            userId: user.id,
            fileKey,
          });

          // デバッグ用：ユーザーのディレクトリ内のファイル一覧を取得
          try {
            const userFiles = await r2Client.listFiles(`users/${user.id}/receipts/`);
            logger.info("ユーザーのファイル一覧（デバッグ用）", {
              userId: user.id,
              prefix: `users/${user.id}/receipts/`,
              files: userFiles.map((f) => ({
                key: f.key,
                size: f.size,
                lastModified: f.lastModified,
              })),
            });
          } catch (listError) {
            logger.error("ファイル一覧取得エラー", {
              userId: user.id,
              error: listError instanceof Error ? listError.message : String(listError),
            });
          }

          // ファイルが存在しない場合でも成功として扱う
        } else {
          await r2Client.deleteFile(fileKey);

          // 削除後の存在確認
          const stillExists = await r2Client.fileExists(fileKey);
          logger.info("削除後のファイル存在確認", {
            userId: user.id,
            fileKey,
            stillExists: stillExists,
          });

          if (stillExists) {
            logger.error("ファイル削除後もファイルが存在しています", {
              userId: user.id,
              fileKey,
            });
            throw new Error("ファイルの削除に失敗しました（削除後も存在）");
          }
        }

        logger.info("R2からのファイル削除が完了しました", {
          userId: user.id,
          fileKey,
        });
      } catch (error) {
        logger.error("R2ファイル削除エラー", {
          userId: user.id,
          fileKey,
          error: error instanceof Error ? error.message : String(error),
        });
        throw error;
      }

      logger.info("領収書URL経由でのファイル削除が完了しました", {
        userId: user.id,
        fileKey,
        receiptUrl,
      });

      return c.json({
        success: true,
        message: "ファイルが正常に削除されました",
        receiptUrl,
        fileKey,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      logger.error("領収書削除処理でエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined,
      });

      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "領収書URL経由ファイル削除",
      });
    }
  });

  // 領収書の存在確認
  receiptsApp.post("/check-exists", async (c: Context) => {
    try {
      const user = c.get("user");
      const body = await c.req.json();
      const { receiptUrl } = body;

      if (!receiptUrl) {
        throw createValidationError(
          "領収書URLが指定されていません",
          "receiptUrl",
          receiptUrl,
          "required",
        );
      }

      // URLからファイルキーを抽出
      const fileKey = extractFileKeyFromUrl(receiptUrl);

      // ユーザーのアクセス権限をチェック
      if (!hasFileAccess(user.id, fileKey)) {
        logSecurityEvent(c, "UNAUTHORIZED_FILE_ACCESS", {
          userId: user.id,
          fileKey,
          receiptUrl,
        });

        throw createAuthorizationError("このファイルにアクセスする権限がありません");
      }

      // ファイルの存在確認
      const exists = await r2Client.fileExists(fileKey);

      logger.debug("領収書の存在確認", {
        userId: user.id,
        fileKey,
        receiptUrl,
        exists,
      });

      return c.json({
        success: true,
        exists,
        receiptUrl,
        fileKey,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "領収書存在確認",
      });
    }
  });

  return receiptsApp;
}

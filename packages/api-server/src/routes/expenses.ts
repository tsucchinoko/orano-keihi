/**
 * 経費関連のAPIエンドポイント
 */

import { Hono } from "hono";
import type { Context } from "hono";
import { logger } from "../utils/logger.js";
import { handleError, createValidationError, createNotFoundError } from "../utils/error-handler.js";
import type { ExpenseRepository } from "../repositories/expense-repository.js";
import type { CreateExpenseDto, UpdateExpenseDto } from "../types/d1-dtos.js";
import type { R2ClientInterface } from "../services/r2-client.js";

/**
 * 経費ルーターを作成
 * @param expenseRepository 経費リポジトリ
 * @param r2Client R2クライアント（オプション）
 * @returns 経費ルーター
 */
export function createExpensesRouter(
  expenseRepository: ExpenseRepository,
  r2Client?: R2ClientInterface,
): Hono {
  const expensesApp = new Hono();

  // POST /api/v1/expenses - 経費を作成
  expensesApp.post("/", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      // リクエストボディを取得
      const body = await c.req.json<CreateExpenseDto>();

      logger.debug("経費作成リクエスト", {
        userId: user.id,
        createData: body,
      });

      // バリデーション
      if (!body.date || typeof body.date !== "string") {
        throw createValidationError(
          "日付は必須で文字列である必要があります",
          "date",
          body.date,
          "string required (YYYY-MM-DD format)",
        );
      }

      if (!body.amount || typeof body.amount !== "number") {
        throw createValidationError(
          "金額は必須で数値である必要があります",
          "amount",
          body.amount,
          "number required",
        );
      }

      if (!body.category || typeof body.category !== "string") {
        throw createValidationError(
          "カテゴリは必須で文字列である必要があります",
          "category",
          body.category,
          "string required",
        );
      }

      if (
        body.description !== undefined &&
        body.description !== null &&
        typeof body.description !== "string"
      ) {
        throw createValidationError(
          "説明は文字列である必要があります",
          "description",
          body.description,
          "string required",
        );
      }

      // 日付形式のバリデーション（YYYY-MM-DD）
      const datePattern = /^\d{4}-\d{2}-\d{2}$/;
      if (!datePattern.test(body.date)) {
        throw createValidationError(
          "日付はYYYY-MM-DD形式である必要があります",
          "date",
          body.date,
          "YYYY-MM-DD format required",
        );
      }

      // 経費を作成
      const expense = await expenseRepository.create(body, user.id);

      logger.info("経費を作成しました", {
        userId: user.id,
        expenseId: expense.id,
        amount: expense.amount,
      });

      return c.json(
        {
          success: true,
          expense,
          timestamp: new Date().toISOString(),
        },
        201,
      );
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "経費作成",
      });
    }
  });

  // GET /api/v1/expenses - 経費一覧を取得
  expensesApp.get("/", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      // クエリパラメータを取得
      const month = c.req.query("month"); // YYYY-MM形式
      const category = c.req.query("category");

      logger.debug("経費一覧取得リクエスト", {
        userId: user.id,
        month,
        category,
      });

      // 月フィルターのバリデーション（指定されている場合）
      if (month) {
        const monthPattern = /^\d{4}-\d{2}$/;
        if (!monthPattern.test(month)) {
          throw createValidationError(
            "月はYYYY-MM形式である必要があります",
            "month",
            month,
            "YYYY-MM format required",
          );
        }
      }

      // 経費一覧を取得（フィルタリング）
      const expenses = await expenseRepository.findAll(user.id, month, category);

      logger.info("経費一覧を取得しました", {
        userId: user.id,
        count: expenses.length,
        month,
        category,
      });

      return c.json({
        success: true,
        expenses,
        count: expenses.length,
        filters: {
          month: month || null,
          category: category || null,
        },
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "経費一覧取得",
      });
    }
  });

  // GET /api/v1/expenses/:id - 経費を取得
  expensesApp.get("/:id", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      const expenseId = parseInt(c.req.param("id"), 10);

      if (isNaN(expenseId)) {
        throw createValidationError(
          "有効な経費IDが指定されていません",
          "id",
          c.req.param("id"),
          "valid number required",
        );
      }

      logger.debug("経費取得リクエスト", {
        userId: user.id,
        expenseId,
      });

      // 経費を取得（アクセス制御：自分の経費のみ）
      const expense = await expenseRepository.findById(expenseId, user.id);

      if (!expense) {
        logger.warn("経費が見つかりませんでした", {
          userId: user.id,
          expenseId,
        });
        throw createNotFoundError("経費が見つかりません");
      }

      logger.info("経費を取得しました", {
        userId: user.id,
        expenseId: expense.id,
      });

      return c.json({
        success: true,
        expense,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "経費取得",
      });
    }
  });

  // PUT /api/v1/expenses/:id - 経費を更新
  expensesApp.put("/:id", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      const expenseId = parseInt(c.req.param("id"), 10);

      if (isNaN(expenseId)) {
        throw createValidationError(
          "有効な経費IDが指定されていません",
          "id",
          c.req.param("id"),
          "valid number required",
        );
      }

      // リクエストボディを取得
      const body = await c.req.json<UpdateExpenseDto>();

      logger.debug("経費更新リクエスト", {
        userId: user.id,
        expenseId,
        updateData: body,
      });

      // バリデーション
      if (body.date !== undefined && typeof body.date !== "string") {
        throw createValidationError(
          "日付は文字列である必要があります",
          "date",
          body.date,
          "string required (YYYY-MM-DD format)",
        );
      }

      if (body.amount !== undefined && typeof body.amount !== "number") {
        throw createValidationError(
          "金額は数値である必要があります",
          "amount",
          body.amount,
          "number required",
        );
      }

      if (body.category !== undefined && typeof body.category !== "string") {
        throw createValidationError(
          "カテゴリは文字列である必要があります",
          "category",
          body.category,
          "string required",
        );
      }

      if (
        body.description !== undefined &&
        body.description !== null &&
        typeof body.description !== "string"
      ) {
        throw createValidationError(
          "説明は文字列である必要があります",
          "description",
          body.description,
          "string required",
        );
      }

      if (
        body.receipt_url !== undefined &&
        body.receipt_url !== null &&
        typeof body.receipt_url !== "string"
      ) {
        throw createValidationError(
          "領収書URLは文字列である必要があります",
          "receipt_url",
          body.receipt_url,
          "string required (HTTPS URL)",
        );
      }

      // 日付形式のバリデーション（指定されている場合）
      if (body.date) {
        const datePattern = /^\d{4}-\d{2}-\d{2}$/;
        if (!datePattern.test(body.date)) {
          throw createValidationError(
            "日付はYYYY-MM-DD形式である必要があります",
            "date",
            body.date,
            "YYYY-MM-DD format required",
          );
        }
      }

      // 領収書URLのバリデーション（指定されている場合）
      // 空文字列またはnullの場合は削除を意味するのでスキップ
      if (body.receipt_url && body.receipt_url !== "" && !body.receipt_url.startsWith("https://")) {
        throw createValidationError(
          "領収書URLはHTTPSで始まる必要があります",
          "receipt_url",
          body.receipt_url,
          "HTTPS URL required",
        );
      }

      // 経費を更新（アクセス制御：自分の経費のみ）
      const expense = await expenseRepository.update(expenseId, body, user.id);

      logger.info("経費を更新しました", {
        userId: user.id,
        expenseId: expense.id,
      });

      return c.json({
        success: true,
        expense,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "経費更新",
      });
    }
  });

  // DELETE /api/v1/expenses/:id - 経費を削除
  expensesApp.delete("/:id", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      const expenseId = parseInt(c.req.param("id"), 10);

      if (isNaN(expenseId)) {
        throw createValidationError(
          "有効な経費IDが指定されていません",
          "id",
          c.req.param("id"),
          "valid number required",
        );
      }

      logger.debug("経費削除リクエスト", {
        userId: user.id,
        expenseId,
      });

      // 経費を削除する前に領収書URLを取得
      const receiptUrl = await expenseRepository.getReceiptUrl(expenseId, user.id);

      // 経費を削除（アクセス制御：自分の経費のみ）
      await expenseRepository.delete(expenseId, user.id);

      // 領収書がある場合はR2から削除
      if (receiptUrl && r2Client) {
        try {
          logger.debug("領収書URL解析開始", {
            userId: user.id,
            expenseId,
            receiptUrl,
          });

          // URLからR2キーを抽出
          // 想定される形式:
          // 1. https://orano-keihi-dev.account.r2.cloudflarestorage.com/users/xxx/receipts/yyy/file.jpg
          // 2. https://account.r2.cloudflarestorage.com/bucket/users/xxx/receipts/yyy/file.jpg
          let key: string | null = null;

          try {
            const url = new URL(receiptUrl);
            // パス部分を取得（先頭の/を除く）
            const pathname = url.pathname.startsWith("/") ? url.pathname.slice(1) : url.pathname;

            // バケット名が含まれている場合は除外
            // 例: bucket/users/xxx/... -> users/xxx/...
            const pathParts = pathname.split("/");

            // "users/"で始まるパスを探す
            const usersIndex = pathParts.findIndex((part) => part === "users");
            if (usersIndex !== -1) {
              key = pathParts.slice(usersIndex).join("/");
            } else {
              // バケット名の次からがキーの可能性
              key = pathname;
            }
          } catch (urlError) {
            logger.warn("URL解析に失敗しました。文字列分割で試行します", {
              userId: user.id,
              expenseId,
              receiptUrl,
              error: urlError instanceof Error ? urlError.message : String(urlError),
            });

            // フォールバック: 文字列分割
            const urlParts = receiptUrl.split("/");
            const usersIndex = urlParts.findIndex((part) => part === "users");
            if (usersIndex !== -1) {
              key = urlParts.slice(usersIndex).join("/");
            }
          }

          if (key) {
            logger.debug("R2から領収書を削除します", {
              userId: user.id,
              expenseId,
              receiptUrl,
              key,
            });

            await r2Client.deleteObject(key);

            logger.info("R2から領収書を削除しました", {
              userId: user.id,
              expenseId,
              key,
            });
          } else {
            logger.warn("領収書URLからR2キーを抽出できませんでした", {
              userId: user.id,
              expenseId,
              receiptUrl,
            });
          }
        } catch (r2Error) {
          // R2削除エラーはログに記録するが、経費削除は成功とする
          logger.error("R2から領収書の削除に失敗しましたが、経費は削除されました", {
            userId: user.id,
            expenseId,
            receiptUrl,
            error: r2Error instanceof Error ? r2Error.message : String(r2Error),
          });
        }
      } else if (receiptUrl && !r2Client) {
        logger.warn("R2クライアントが利用できないため、領収書を削除できませんでした", {
          userId: user.id,
          expenseId,
          receiptUrl,
        });
      }

      logger.info("経費を削除しました", {
        userId: user.id,
        expenseId,
        hadReceipt: !!receiptUrl,
      });

      return c.json({
        success: true,
        message: "経費が正常に削除されました",
        expenseId,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "経費削除",
      });
    }
  });

  return expensesApp;
}

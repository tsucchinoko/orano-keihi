/**
 * サブスクリプション関連のAPIエンドポイント
 */

import { Hono } from "hono";
import type { Context } from "hono";
import { logger } from "../utils/logger.js";
import { handleError, createValidationError, createNotFoundError } from "../utils/error-handler.js";
import type { SubscriptionRepository } from "../repositories/subscription-repository.js";
import type { CreateSubscriptionDto, UpdateSubscriptionDto } from "../types/d1-dtos.js";

/**
 * サブスクリプションルーターを作成
 * @param subscriptionRepository サブスクリプションリポジトリ
 * @returns サブスクリプションルーター
 */
export function createSubscriptionsRouter(subscriptionRepository: SubscriptionRepository): Hono {
  const subscriptionsApp = new Hono();

  // GET /api/v1/subscriptions/monthly-total - 月額合計を取得
  // 注意: このエンドポイントは /api/v1/subscriptions/:id より前に定義する必要がある
  subscriptionsApp.get("/monthly-total", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      logger.debug("月額合計取得リクエスト", {
        userId: user.id,
      });

      // 月額合計を計算
      const monthlyTotal = await subscriptionRepository.calculateMonthlyTotal(user.id);

      // アクティブなサブスクリプション数も取得
      const activeSubscriptions = await subscriptionRepository.findAll(user.id, true);

      logger.info("月額合計を取得しました", {
        userId: user.id,
        monthlyTotal,
        activeSubscriptions: activeSubscriptions.length,
      });

      return c.json({
        success: true,
        monthlyTotal,
        activeSubscriptions: activeSubscriptions.length,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "月額合計取得",
      });
    }
  });

  // GET /api/v1/subscriptions - サブスクリプション一覧を取得
  subscriptionsApp.get("/", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      // クエリパラメータを取得
      const activeOnly = c.req.query("activeOnly") === "true";

      logger.debug("サブスクリプション一覧取得リクエスト", {
        userId: user.id,
        activeOnly,
      });

      // サブスクリプション一覧を取得（フィルタリング）
      const subscriptions = await subscriptionRepository.findAll(user.id, activeOnly);

      logger.info("サブスクリプション一覧を取得しました", {
        userId: user.id,
        count: subscriptions.length,
        activeOnly,
      });

      return c.json({
        success: true,
        subscriptions,
        count: subscriptions.length,
        filters: {
          activeOnly,
        },
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプション一覧取得",
      });
    }
  });

  // POST /api/v1/subscriptions - サブスクリプションを作成
  subscriptionsApp.post("/", async (c: Context) => {
    try {
      const user = c.get("user");

      logger.debug("サブスクリプション作成エンドポイントに到達しました", {
        hasUser: !!user,
        user: user ? { id: user.id, email: user.email } : null,
      });

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      // リクエストボディを取得
      const body = await c.req.json<CreateSubscriptionDto>();

      logger.debug("サブスクリプション作成リクエスト", {
        userId: user.id,
        createData: body,
      });

      // バリデーション
      if (!body.name || typeof body.name !== "string") {
        throw createValidationError(
          "サービス名は必須で文字列である必要があります",
          "name",
          body.name,
          "string required",
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

      if (!body.billing_cycle || typeof body.billing_cycle !== "string") {
        throw createValidationError(
          "請求サイクルは必須で文字列である必要があります",
          "billing_cycle",
          body.billing_cycle,
          "string required",
        );
      }

      // billing_cycleの値チェック
      if (body.billing_cycle !== "monthly" && body.billing_cycle !== "annual") {
        throw createValidationError(
          "請求サイクルは'monthly'または'annual'である必要があります",
          "billing_cycle",
          body.billing_cycle,
          "'monthly' or 'annual' required",
        );
      }

      if (!body.start_date || typeof body.start_date !== "string") {
        throw createValidationError(
          "開始日は必須で文字列である必要があります",
          "start_date",
          body.start_date,
          "string required (YYYY-MM-DD format)",
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

      // 日付形式のバリデーション（YYYY-MM-DD）
      const datePattern = /^\d{4}-\d{2}-\d{2}$/;
      if (!datePattern.test(body.start_date)) {
        throw createValidationError(
          "開始日はYYYY-MM-DD形式である必要があります",
          "start_date",
          body.start_date,
          "YYYY-MM-DD format required",
        );
      }

      // サブスクリプションを作成
      const subscription = await subscriptionRepository.create(body, user.id);

      logger.info("サブスクリプションを作成しました", {
        userId: user.id,
        subscriptionId: subscription.id,
        name: subscription.name,
      });

      return c.json(
        {
          success: true,
          subscription,
          timestamp: new Date().toISOString(),
        },
        201,
      );
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプション作成",
      });
    }
  });

  // GET /api/v1/subscriptions/:id - サブスクリプションを取得
  subscriptionsApp.get("/:id", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      const subscriptionId = parseInt(c.req.param("id"), 10);

      if (isNaN(subscriptionId)) {
        throw createValidationError(
          "有効なサブスクリプションIDが指定されていません",
          "id",
          c.req.param("id"),
          "valid number required",
        );
      }

      logger.debug("サブスクリプション取得リクエスト", {
        userId: user.id,
        subscriptionId,
      });

      // サブスクリプションを取得（アクセス制御：自分のサブスクリプションのみ）
      const subscription = await subscriptionRepository.findById(subscriptionId, user.id);

      if (!subscription) {
        logger.warn("サブスクリプションが見つかりませんでした", {
          userId: user.id,
          subscriptionId,
        });
        throw createNotFoundError("サブスクリプションが見つかりません");
      }

      logger.info("サブスクリプションを取得しました", {
        userId: user.id,
        subscriptionId: subscription.id,
      });

      return c.json({
        success: true,
        subscription,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプション取得",
      });
    }
  });

  // PUT /api/v1/subscriptions/:id - サブスクリプションを更新
  subscriptionsApp.put("/:id", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      const subscriptionId = parseInt(c.req.param("id"), 10);

      if (isNaN(subscriptionId)) {
        throw createValidationError(
          "有効なサブスクリプションIDが指定されていません",
          "id",
          c.req.param("id"),
          "valid number required",
        );
      }

      // リクエストボディを取得
      const body = await c.req.json<UpdateSubscriptionDto>();

      logger.debug("サブスクリプション更新リクエスト", {
        userId: user.id,
        subscriptionId,
        updateData: body,
      });

      // バリデーション
      if (body.name !== undefined && body.name !== null && typeof body.name !== "string") {
        throw createValidationError(
          "サービス名は文字列である必要があります",
          "name",
          body.name,
          "string required",
        );
      }

      if (body.amount !== undefined && body.amount !== null && typeof body.amount !== "number") {
        throw createValidationError(
          "金額は数値である必要があります",
          "amount",
          body.amount,
          "number required",
        );
      }

      if (
        body.billing_cycle !== undefined &&
        body.billing_cycle !== null &&
        typeof body.billing_cycle !== "string"
      ) {
        throw createValidationError(
          "請求サイクルは文字列である必要があります",
          "billing_cycle",
          body.billing_cycle,
          "string required",
        );
      }

      // billing_cycleの値チェック（指定されている場合）
      if (
        body.billing_cycle !== undefined &&
        body.billing_cycle !== null &&
        body.billing_cycle !== "monthly" &&
        body.billing_cycle !== "annual"
      ) {
        throw createValidationError(
          "請求サイクルは'monthly'または'annual'である必要があります",
          "billing_cycle",
          body.billing_cycle,
          "'monthly' or 'annual' required",
        );
      }

      if (
        body.start_date !== undefined &&
        body.start_date !== null &&
        typeof body.start_date !== "string"
      ) {
        throw createValidationError(
          "開始日は文字列である必要があります",
          "start_date",
          body.start_date,
          "string required (YYYY-MM-DD format)",
        );
      }

      if (
        body.category !== undefined &&
        body.category !== null &&
        typeof body.category !== "string"
      ) {
        throw createValidationError(
          "カテゴリは文字列である必要があります",
          "category",
          body.category,
          "string required",
        );
      }

      if (
        body.receipt_path !== undefined &&
        body.receipt_path !== null &&
        typeof body.receipt_path !== "string"
      ) {
        throw createValidationError(
          "領収書パスは文字列である必要があります",
          "receipt_path",
          body.receipt_path,
          "string required",
        );
      }

      // 日付形式のバリデーション（指定されている場合）
      if (body.start_date) {
        const datePattern = /^\d{4}-\d{2}-\d{2}$/;
        if (!datePattern.test(body.start_date)) {
          throw createValidationError(
            "開始日はYYYY-MM-DD形式である必要があります",
            "start_date",
            body.start_date,
            "YYYY-MM-DD format required",
          );
        }
      }

      // サブスクリプションを更新（アクセス制御：自分のサブスクリプションのみ）
      const subscription = await subscriptionRepository.update(subscriptionId, body, user.id);

      logger.info("サブスクリプションを更新しました", {
        userId: user.id,
        subscriptionId: subscription.id,
      });

      return c.json({
        success: true,
        subscription,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプション更新",
      });
    }
  });

  // PATCH /api/v1/subscriptions/:id/toggle - サブスクリプションのステータスを切り替え
  subscriptionsApp.patch("/:id/toggle", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      const subscriptionId = parseInt(c.req.param("id"), 10);

      if (isNaN(subscriptionId)) {
        throw createValidationError(
          "有効なサブスクリプションIDが指定されていません",
          "id",
          c.req.param("id"),
          "valid number required",
        );
      }

      logger.debug("サブスクリプションステータス切り替えリクエスト", {
        userId: user.id,
        subscriptionId,
      });

      // サブスクリプションのステータスを切り替え（アクセス制御：自分のサブスクリプションのみ）
      const subscription = await subscriptionRepository.toggleStatus(subscriptionId, user.id);

      logger.info("サブスクリプションのステータスを切り替えました", {
        userId: user.id,
        subscriptionId: subscription.id,
        newStatus: subscription.is_active,
      });

      return c.json({
        success: true,
        subscription,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプションステータス切り替え",
      });
    }
  });

  // DELETE /api/v1/subscriptions/:id - サブスクリプションを削除
  subscriptionsApp.delete("/:id", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      const subscriptionId = parseInt(c.req.param("id"), 10);

      if (isNaN(subscriptionId)) {
        throw createValidationError(
          "有効なサブスクリプションIDが指定されていません",
          "id",
          c.req.param("id"),
          "valid number required",
        );
      }

      logger.debug("サブスクリプション削除リクエスト", {
        userId: user.id,
        subscriptionId,
      });

      // サブスクリプションを削除（アクセス制御：自分のサブスクリプションのみ）
      await subscriptionRepository.delete(subscriptionId, user.id);

      logger.info("サブスクリプションを削除しました", {
        userId: user.id,
        subscriptionId,
      });

      return c.json({
        success: true,
        message: "サブスクリプションが正常に削除されました",
        subscriptionId,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプション削除",
      });
    }
  });

  return subscriptionsApp;
}

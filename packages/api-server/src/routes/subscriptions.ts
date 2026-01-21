/**
 * サブスクリプション関連のAPIエンドポイント
 */

import { Hono } from "hono";
import type { Context } from "hono";
import { logger } from "../utils/logger.js";
import { handleError, createValidationError, createNotFoundError } from "../utils/error-handler.js";
import type { SubscriptionRepository } from "../repositories/subscription-repository.js";
import type { CreateSubscriptionDto, UpdateSubscriptionDto } from "../types/d1-dtos.js";
import type { R2ClientInterface } from "../services/r2-client.js";

/**
 * サブスクリプションルーターを作成
 * @param subscriptionRepository サブスクリプションリポジトリ
 * @param r2Client R2クライアント（オプション）
 * @returns サブスクリプションルーター
 */
export function createSubscriptionsRouter(
  subscriptionRepository: SubscriptionRepository,
  r2Client?: R2ClientInterface,
): Hono {
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

  // DELETE /api/v1/subscriptions/:id/receipt - サブスクリプションの領収書を削除
  // 注意: このエンドポイントは /:id より前に定義する必要がある
  subscriptionsApp.delete("/:id/receipt", async (c: Context) => {
    logger.info("DELETE /:id/receipt エンドポイントが呼ばれました", {
      path: c.req.path,
      params: c.req.param(),
    });

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

      logger.debug("サブスクリプション領収書削除リクエスト", {
        userId: user.id,
        subscriptionId,
      });

      // サブスクリプションを取得して領収書パスを確認
      const subscription = await subscriptionRepository.findById(subscriptionId, user.id);

      if (!subscription) {
        logger.warn("サブスクリプションが見つかりませんでした", {
          userId: user.id,
          subscriptionId,
        });
        throw createNotFoundError("サブスクリプションが見つかりません");
      }

      logger.debug("サブスクリプション情報を取得しました", {
        subscriptionId,
        hasReceiptPath: !!subscription.receipt_path,
        receiptPath: subscription.receipt_path,
        hasR2Client: !!r2Client,
      });

      // 領収書パスが存在する場合、R2から削除
      if (subscription.receipt_path && r2Client) {
        try {
          // receipt_pathからファイルキーを抽出
          // receipt_pathは "https://pub-xxx.r2.cloudflarestorage.com/bucket-name/users/xxx/subscriptions/xxx/xxx.jpg" の形式
          const url = new URL(subscription.receipt_path);
          const pathname = url.pathname.substring(1); // 先頭の "/" を除去

          // パス名からバケット名を除去
          // pathname = "bucket-name/users/xxx/subscriptions/xxx/xxx.jpg"
          // fileKey = "users/xxx/subscriptions/xxx/xxx.jpg"
          const pathParts = pathname.split("/");
          const fileKey = pathParts.slice(1).join("/"); // 最初の部分（バケット名）を除去

          logger.debug("R2からファイルを削除します", {
            fileKey,
            receiptPath: subscription.receipt_path,
            pathname,
          });

          await r2Client.deleteObject(fileKey);

          logger.info("R2からファイルを削除しました", {
            fileKey,
            subscriptionId,
          });
        } catch (error) {
          // R2削除に失敗してもデータベースは更新する
          logger.error("R2からのファイル削除に失敗しました", {
            error: error instanceof Error ? error.message : String(error),
            receiptPath: subscription.receipt_path,
            subscriptionId,
          });
        }
      } else {
        logger.warn("R2削除をスキップしました", {
          hasReceiptPath: !!subscription.receipt_path,
          hasR2Client: !!r2Client,
          receiptPath: subscription.receipt_path,
        });
      }

      // データベースの領収書パスを空にする
      const updatedSubscription = await subscriptionRepository.update(
        subscriptionId,
        { receipt_path: "" },
        user.id,
      );

      logger.info("サブスクリプションの領収書を削除しました", {
        userId: user.id,
        subscriptionId,
      });

      return c.json({
        success: true,
        message: "領収書が正常に削除されました",
        subscription: updatedSubscription,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "サブスクリプション領収書削除",
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
    logger.info("PUT /:id エンドポイントが呼ばれました", {
      path: c.req.path,
      params: c.req.param(),
    });

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

      logger.info("サブスクリプション更新リクエスト", {
        userId: user.id,
        subscriptionId,
        updateData: body,
        hasReceiptPath: body.receipt_path !== undefined,
        receiptPathValue: body.receipt_path,
        receiptPathIsEmptyString: body.receipt_path === "",
        receiptPathType: typeof body.receipt_path,
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

      // 領収書削除処理：receipt_pathが空文字列の場合、R2から削除
      if (body.receipt_path === "" && r2Client) {
        logger.info("領収書削除処理を開始します", {
          subscriptionId,
          userId: user.id,
        });

        // 現在のサブスクリプション情報を取得
        const currentSubscription = await subscriptionRepository.findById(subscriptionId, user.id);

        if (currentSubscription && currentSubscription.receipt_path) {
          try {
            // receipt_pathからファイルキーを抽出
            const url = new URL(currentSubscription.receipt_path);
            const pathname = url.pathname.substring(1); // 先頭の "/" を除去
            const pathParts = pathname.split("/");
            const fileKey = pathParts.slice(1).join("/"); // バケット名を除去

            logger.info("R2からファイルを削除します", {
              fileKey,
              receiptPath: currentSubscription.receipt_path,
            });

            await r2Client.deleteObject(fileKey);

            logger.info("R2からファイルを削除しました", {
              fileKey,
              subscriptionId,
            });
          } catch (error) {
            // R2削除に失敗してもデータベースは更新する
            logger.error("R2からのファイル削除に失敗しました", {
              error: error instanceof Error ? error.message : String(error),
              receiptPath: currentSubscription.receipt_path,
              subscriptionId,
            });
          }
        } else {
          logger.info("削除する領収書がありません", {
            subscriptionId,
            hasCurrentSubscription: !!currentSubscription,
            hasReceiptPath: currentSubscription?.receipt_path ? true : false,
          });
        }
      } else {
        logger.info("領収書削除処理をスキップしました", {
          subscriptionId,
          receiptPathValue: body.receipt_path,
          hasR2Client: !!r2Client,
        });
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
    console.log("🗑️ DELETE /:id エンドポイントが呼ばれました", {
      path: c.req.path,
      params: c.req.param(),
      method: c.req.method,
    });

    logger.info("🗑️ DELETE /:id エンドポイントが呼ばれました", {
      path: c.req.path,
      params: c.req.param(),
      method: c.req.method,
    });

    try {
      const user = c.get("user");

      if (!user) {
        console.error("❌ ユーザー情報が見つかりません");
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      const subscriptionId = parseInt(c.req.param("id"), 10);

      if (isNaN(subscriptionId)) {
        console.error("❌ 無効なサブスクリプションID", c.req.param("id"));
        throw createValidationError(
          "有効なサブスクリプションIDが指定されていません",
          "id",
          c.req.param("id"),
          "valid number required",
        );
      }

      console.log("📋 サブスクリプション削除リクエスト", {
        userId: user.id,
        subscriptionId,
      });

      logger.debug("サブスクリプション削除リクエスト", {
        userId: user.id,
        subscriptionId,
      });

      // サブスクリプションを削除する前に領収書パスを取得
      console.log("📂 領収書パス取得開始", { subscriptionId, userId: user.id });
      const receiptPath = await subscriptionRepository.getReceiptPath(subscriptionId, user.id);
      console.log("📂 領収書パス取得結果", {
        subscriptionId,
        receiptPath,
        hasReceiptPath: !!receiptPath,
      });

      // サブスクリプションを削除（アクセス制御：自分のサブスクリプションのみ）
      console.log("🗄️ データベースから削除開始", { subscriptionId, userId: user.id });
      await subscriptionRepository.delete(subscriptionId, user.id);
      console.log("✅ データベースから削除完了", { subscriptionId });

      // 領収書がある場合はR2から削除
      if (receiptPath && r2Client) {
        console.log("🔍 R2削除処理開始", { receiptPath, hasR2Client: !!r2Client });
        try {
          logger.debug("領収書パス解析開始", {
            userId: user.id,
            subscriptionId,
            receiptPath,
          });

          // URLからR2キーを抽出
          // 想定される形式:
          // 1. https://orano-keihi-dev.account.r2.cloudflarestorage.com/users/xxx/subscriptions/yyy/file.jpg
          // 2. https://account.r2.cloudflarestorage.com/bucket/users/xxx/subscriptions/yyy/file.jpg
          let key: string | null = null;

          try {
            const url = new URL(receiptPath);
            // パス部分を取得（先頭の/を除く）
            let pathname = url.pathname.startsWith("/") ? url.pathname.slice(1) : url.pathname;

            // URLデコードを実行（日本語ファイル名対応）
            pathname = decodeURIComponent(pathname);

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
              subscriptionId,
              receiptPath,
              error: urlError instanceof Error ? urlError.message : String(urlError),
            });

            // フォールバック: 文字列分割
            const urlParts = receiptPath.split("/");
            const usersIndex = urlParts.findIndex((part) => part === "users");
            if (usersIndex !== -1) {
              // URLデコードを実行
              key = urlParts
                .slice(usersIndex)
                .map((part) => decodeURIComponent(part))
                .join("/");
            }
          }

          if (key) {
            console.log("🗑️ R2から削除実行", { key, subscriptionId });
            logger.debug("R2から領収書を削除します", {
              userId: user.id,
              subscriptionId,
              receiptPath,
              key,
            });

            await r2Client.deleteObject(key);

            console.log("✅ R2から削除成功", { key, subscriptionId });
            logger.info("R2から領収書を削除しました", {
              userId: user.id,
              subscriptionId,
              key,
            });
          } else {
            console.warn("⚠️ R2キー抽出失敗", { receiptPath, subscriptionId });
            logger.warn("領収書パスからR2キーを抽出できませんでした", {
              userId: user.id,
              subscriptionId,
              receiptPath,
            });
          }
        } catch (r2Error) {
          // R2削除エラーはログに記録するが、サブスクリプション削除は成功とする
          console.error("❌ R2削除エラー（サブスクリプションは削除済み）", {
            subscriptionId,
            receiptPath,
            error: r2Error instanceof Error ? r2Error.message : String(r2Error),
          });
          logger.error("R2から領収書の削除に失敗しましたが、サブスクリプションは削除されました", {
            userId: user.id,
            subscriptionId,
            receiptPath,
            error: r2Error instanceof Error ? r2Error.message : String(r2Error),
          });
        }
      } else if (receiptPath && !r2Client) {
        console.warn("⚠️ R2クライアント未設定", { receiptPath, subscriptionId });
        logger.warn("R2クライアントが利用できないため、領収書を削除できませんでした", {
          userId: user.id,
          subscriptionId,
          receiptPath,
        });
      } else {
        console.log("ℹ️ 領収書なし、R2削除スキップ", { subscriptionId });
      }

      console.log("✅ サブスクリプション削除完了", {
        userId: user.id,
        subscriptionId,
        hadReceipt: !!receiptPath,
      });

      logger.info("サブスクリプションを削除しました", {
        userId: user.id,
        subscriptionId,
        hadReceipt: !!receiptPath,
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

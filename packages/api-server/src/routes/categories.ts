/**
 * カテゴリー関連のAPIエンドポイント
 */

import { Hono } from "hono";
import type { Context } from "hono";
import { logger } from "../utils/logger.js";
import { handleError, createValidationError, createNotFoundError } from "../utils/error-handler.js";
import type { CategoryRepository } from "../repositories/category-repository.js";
import type { CreateCategoryDto, UpdateCategoryDto } from "../types/d1-dtos.js";

/**
 * カテゴリールーターを作成
 * @param categoryRepository カテゴリーリポジトリ
 * @returns カテゴリールーター
 */
export function createCategoriesRouter(categoryRepository: CategoryRepository): Hono {
  const categoriesApp = new Hono();

  // GET /api/v1/categories - カテゴリー一覧を取得
  categoriesApp.get("/", async (c: Context) => {
    try {
      // クエリパラメータを取得
      const includeInactive = c.req.query("include_inactive") === "true";

      logger.debug("カテゴリー一覧取得リクエスト", {
        includeInactive,
      });

      // カテゴリー一覧を取得
      const categories = await categoryRepository.findAll(includeInactive);

      logger.info("カテゴリー一覧を取得しました", {
        count: categories.length,
        includeInactive,
      });

      return c.json({
        success: true,
        categories,
        count: categories.length,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "カテゴリー一覧取得",
      });
    }
  });

  // GET /api/v1/categories/:id - カテゴリーを取得
  categoriesApp.get("/:id", async (c: Context) => {
    try {
      const categoryId = parseInt(c.req.param("id"), 10);

      if (isNaN(categoryId)) {
        throw createValidationError(
          "有効なカテゴリーIDが指定されていません",
          "id",
          c.req.param("id"),
          "valid number required",
        );
      }

      logger.debug("カテゴリー取得リクエスト", {
        categoryId,
      });

      // カテゴリーを取得
      const category = await categoryRepository.findById(categoryId);

      if (!category) {
        logger.warn("カテゴリーが見つかりませんでした", {
          categoryId,
        });
        throw createNotFoundError("カテゴリーが見つかりません");
      }

      logger.info("カテゴリーを取得しました", {
        categoryId: category.id,
        name: category.name,
      });

      return c.json({
        success: true,
        category,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "カテゴリー取得",
      });
    }
  });

  // POST /api/v1/categories - カテゴリーを作成
  categoriesApp.post("/", async (c: Context) => {
    try {
      // リクエストボディを取得
      const body = await c.req.json<CreateCategoryDto>();

      logger.debug("カテゴリー作成リクエスト", {
        createData: body,
      });

      // バリデーション
      if (!body.name || typeof body.name !== "string") {
        throw createValidationError(
          "カテゴリー名は必須で文字列である必要があります",
          "name",
          body.name,
          "string required",
        );
      }

      if (body.name.length > 50) {
        throw createValidationError(
          "カテゴリー名は50文字以内である必要があります",
          "name",
          body.name,
          "max 50 characters",
        );
      }

      if (!body.icon || typeof body.icon !== "string") {
        throw createValidationError(
          "アイコンは必須で文字列である必要があります",
          "icon",
          body.icon,
          "string required",
        );
      }

      if (body.display_order !== undefined && typeof body.display_order !== "number") {
        throw createValidationError(
          "表示順序は数値である必要があります",
          "display_order",
          body.display_order,
          "number required",
        );
      }

      // カテゴリー名の重複チェック
      const existingCategory = await categoryRepository.findByName(body.name);
      if (existingCategory) {
        throw createValidationError(
          "同じ名前のカテゴリーが既に存在します",
          "name",
          body.name,
          "unique name required",
        );
      }

      // カテゴリーを作成
      const category = await categoryRepository.create(body);

      logger.info("カテゴリーを作成しました", {
        categoryId: category.id,
        name: category.name,
      });

      return c.json(
        {
          success: true,
          category,
          timestamp: new Date().toISOString(),
        },
        201,
      );
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "カテゴリー作成",
      });
    }
  });

  // PUT /api/v1/categories/:id - カテゴリーを更新
  categoriesApp.put("/:id", async (c: Context) => {
    try {
      const categoryId = parseInt(c.req.param("id"), 10);

      if (isNaN(categoryId)) {
        throw createValidationError(
          "有効なカテゴリーIDが指定されていません",
          "id",
          c.req.param("id"),
          "valid number required",
        );
      }

      // リクエストボディを取得
      const body = await c.req.json<UpdateCategoryDto>();

      logger.debug("カテゴリー更新リクエスト", {
        categoryId,
        updateData: body,
      });

      // バリデーション
      if (body.name !== undefined) {
        if (typeof body.name !== "string") {
          throw createValidationError(
            "カテゴリー名は文字列である必要があります",
            "name",
            body.name,
            "string required",
          );
        }
        if (body.name.length > 50) {
          throw createValidationError(
            "カテゴリー名は50文字以内である必要があります",
            "name",
            body.name,
            "max 50 characters",
          );
        }

        // カテゴリー名の重複チェック（自分自身は除く）
        const existingCategory = await categoryRepository.findByName(body.name);
        if (existingCategory && existingCategory.id !== categoryId) {
          throw createValidationError(
            "同じ名前のカテゴリーが既に存在します",
            "name",
            body.name,
            "unique name required",
          );
        }
      }

      if (body.icon !== undefined && typeof body.icon !== "string") {
        throw createValidationError(
          "アイコンは文字列である必要があります",
          "icon",
          body.icon,
          "string required",
        );
      }

      if (body.display_order !== undefined && typeof body.display_order !== "number") {
        throw createValidationError(
          "表示順序は数値である必要があります",
          "display_order",
          body.display_order,
          "number required",
        );
      }

      if (body.is_active !== undefined && typeof body.is_active !== "boolean") {
        throw createValidationError(
          "有効フラグはブール値である必要があります",
          "is_active",
          body.is_active,
          "boolean required",
        );
      }

      // カテゴリーを更新
      const category = await categoryRepository.update(categoryId, body);

      logger.info("カテゴリーを更新しました", {
        categoryId: category.id,
        name: category.name,
      });

      return c.json({
        success: true,
        category,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "カテゴリー更新",
      });
    }
  });

  // DELETE /api/v1/categories/:id - カテゴリーを削除（論理削除）
  categoriesApp.delete("/:id", async (c: Context) => {
    try {
      const categoryId = parseInt(c.req.param("id"), 10);

      if (isNaN(categoryId)) {
        throw createValidationError(
          "有効なカテゴリーIDが指定されていません",
          "id",
          c.req.param("id"),
          "valid number required",
        );
      }

      logger.debug("カテゴリー削除リクエスト", {
        categoryId,
      });

      // カテゴリーを削除（論理削除）
      await categoryRepository.delete(categoryId);

      logger.info("カテゴリーを削除しました", {
        categoryId,
      });

      return c.json({
        success: true,
        message: "カテゴリーが正常に削除されました",
        categoryId,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "カテゴリー削除",
      });
    }
  });

  return categoriesApp;
}

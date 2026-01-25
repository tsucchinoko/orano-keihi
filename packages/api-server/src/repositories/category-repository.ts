/**
 * カテゴリーリポジトリ
 * D1データベースのcategoriesテーブルへのアクセスを提供
 */

import type { D1Database } from "@cloudflare/workers-types";
import type { Category } from "../types/d1-models.js";
import type { CreateCategoryDto, UpdateCategoryDto } from "../types/d1-dtos.js";
import { logger } from "../utils/logger.js";

/**
 * カテゴリーリポジトリクラス
 */
export class CategoryRepository {
  constructor(private db: D1Database) {}

  /**
   * カテゴリーを作成する
   * @param dto カテゴリー作成DTO
   * @returns 作成されたカテゴリー情報
   */
  async create(dto: CreateCategoryDto): Promise<Category> {
    try {
      const now = new Date().toISOString();

      // display_orderが指定されていない場合は、既存の最大値+1を使用
      let displayOrder = dto.display_order;
      if (displayOrder === undefined) {
        const maxOrderResult = await this.db
          .prepare("SELECT MAX(display_order) as max_order FROM categories")
          .first<{ max_order: number | null }>();
        displayOrder = (maxOrderResult?.max_order ?? 0) + 1;
      }

      const result = await this.db
        .prepare(
          `INSERT INTO categories (name, icon, display_order, is_active, created_at, updated_at)
           VALUES (?, ?, ?, ?, 1, ?, ?)`,
        )
        .bind(dto.name, dto.icon, displayOrder, now, now)
        .run();

      if (!result.success) {
        logger.error("カテゴリー作成に失敗しました", {
          error: result.error,
        });
        throw new Error(`カテゴリー作成に失敗しました: ${result.error}`);
      }

      const categoryId = result.meta.last_row_id;
      if (!categoryId) {
        throw new Error("作成されたカテゴリーのIDを取得できませんでした");
      }

      logger.info("カテゴリーを作成しました", {
        categoryId,
        name: dto.name,
      });

      const newCategory = await this.findById(categoryId);
      if (!newCategory) {
        throw new Error("作成したカテゴリーの取得に失敗しました");
      }

      return newCategory;
    } catch (error) {
      logger.error("createでエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * カテゴリーIDでカテゴリーを取得する
   * @param id カテゴリーID
   * @returns カテゴリー情報、または見つからない場合はnull
   */
  async findById(id: number): Promise<Category | null> {
    try {
      // データベースから取得する生の型（is_activeはnumber）
      type CategoryRow = Omit<Category, "is_active"> & { is_active: number };

      const result = await this.db
        .prepare("SELECT * FROM categories WHERE id = ?")
        .bind(id)
        .first<CategoryRow>();

      if (!result) {
        logger.debug("カテゴリーが見つかりませんでした", { id });
        return null;
      }

      // is_activeをbooleanに変換
      const category: Category = {
        id: result.id,
        name: result.name,
        icon: result.icon,
        display_order: result.display_order,
        is_active: result.is_active === 1,
        created_at: result.created_at,
        updated_at: result.updated_at,
      };

      logger.debug("カテゴリーを取得しました", {
        id: category.id,
        name: category.name,
      });

      return category;
    } catch (error) {
      logger.error("findByIdでエラーが発生しました", {
        id,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * カテゴリー名でカテゴリーを取得する
   * @param name カテゴリー名
   * @returns カテゴリー情報、または見つからない場合はnull
   */
  async findByName(name: string): Promise<Category | null> {
    try {
      // データベースから取得する生の型（is_activeはnumber）
      type CategoryRow = Omit<Category, "is_active"> & { is_active: number };

      const result = await this.db
        .prepare("SELECT * FROM categories WHERE name = ?")
        .bind(name)
        .first<CategoryRow>();

      if (!result) {
        logger.debug("カテゴリーが見つかりませんでした", { name });
        return null;
      }

      // is_activeをbooleanに変換
      const category: Category = {
        id: result.id,
        name: result.name,
        icon: result.icon,
        display_order: result.display_order,
        is_active: result.is_active === 1,
        created_at: result.created_at,
        updated_at: result.updated_at,
      };

      logger.debug("カテゴリーを取得しました", {
        id: category.id,
        name: category.name,
      });

      return category;
    } catch (error) {
      logger.error("findByNameでエラーが発生しました", {
        name,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * 全カテゴリー一覧を取得する（有効なカテゴリーのみ、表示順でソート）
   * @param includeInactive 無効なカテゴリーも含めるか（デフォルト: false）
   * @returns カテゴリー一覧
   */
  async findAll(includeInactive = false): Promise<Category[]> {
    try {
      let query = "SELECT * FROM categories";
      if (!includeInactive) {
        query += " WHERE is_active = 1";
      }
      query += " ORDER BY display_order ASC, id ASC";

      // データベースから取得する生の型（is_activeはnumber）
      type CategoryRow = Omit<Category, "is_active"> & { is_active: number };

      const result = await this.db.prepare(query).all<CategoryRow>();

      if (!result.success) {
        logger.error("カテゴリー一覧取得に失敗しました", {
          error: result.error,
        });
        throw new Error(`カテゴリー一覧取得に失敗しました: ${result.error}`);
      }

      // is_activeをbooleanに変換
      const categories: Category[] = result.results.map((row) => ({
        id: row.id,
        name: row.name,
        icon: row.icon,
        display_order: row.display_order,
        is_active: row.is_active === 1,
        created_at: row.created_at,
        updated_at: row.updated_at,
      }));

      logger.debug("カテゴリー一覧を取得しました", {
        count: categories.length,
        includeInactive,
      });

      return categories;
    } catch (error) {
      logger.error("findAllでエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * カテゴリー情報を更新する
   * @param id カテゴリーID
   * @param dto カテゴリー更新DTO
   * @returns 更新後のカテゴリー情報
   */
  async update(id: number, dto: UpdateCategoryDto): Promise<Category> {
    try {
      const now = new Date().toISOString();

      // 更新するフィールドを動的に構築
      const updates: string[] = [];
      const params: (string | number | null)[] = [];

      if (dto.name !== undefined) {
        updates.push("name = ?");
        params.push(dto.name);
      }
      if (dto.icon !== undefined) {
        updates.push("icon = ?");
        params.push(dto.icon);
      }
      if (dto.display_order !== undefined) {
        updates.push("display_order = ?");
        params.push(dto.display_order);
      }
      if (dto.is_active !== undefined) {
        updates.push("is_active = ?");
        params.push(dto.is_active ? 1 : 0);
      }

      if (updates.length === 0) {
        throw new Error("更新するフィールドが指定されていません");
      }

      // updated_atは常に更新
      updates.push("updated_at = ?");
      params.push(now);

      // WHERE句のパラメータを追加
      params.push(id);

      const query = `UPDATE categories SET ${updates.join(", ")} WHERE id = ?`;

      const result = await this.db
        .prepare(query)
        .bind(...params)
        .run();

      if (!result.success) {
        logger.error("カテゴリー更新に失敗しました", {
          id,
          error: result.error,
        });
        throw new Error(`カテゴリー更新に失敗しました: ${result.error}`);
      }

      if (result.meta.changes === 0) {
        logger.warn("更新対象のカテゴリーが見つかりませんでした", { id });
        throw new Error(`カテゴリーが見つかりません: ${id}`);
      }

      logger.info("カテゴリー情報を更新しました", { id });

      const updatedCategory = await this.findById(id);
      if (!updatedCategory) {
        throw new Error("更新したカテゴリーの取得に失敗しました");
      }

      return updatedCategory;
    } catch (error) {
      logger.error("updateでエラーが発生しました", {
        id,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * カテゴリーを削除（論理削除: is_active=0）する
   * @param id カテゴリーID
   */
  async delete(id: number): Promise<void> {
    try {
      const now = new Date().toISOString();

      const result = await this.db
        .prepare("UPDATE categories SET is_active = 0, updated_at = ? WHERE id = ?")
        .bind(now, id)
        .run();

      if (!result.success) {
        logger.error("カテゴリー削除に失敗しました", {
          id,
          error: result.error,
        });
        throw new Error(`カテゴリー削除に失敗しました: ${result.error}`);
      }

      if (result.meta.changes === 0) {
        logger.warn("削除対象のカテゴリーが見つかりませんでした", { id });
        throw new Error(`カテゴリーが見つかりません: ${id}`);
      }

      logger.info("カテゴリーを削除（論理削除）しました", { id });
    } catch (error) {
      logger.error("deleteでエラーが発生しました", {
        id,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }
}

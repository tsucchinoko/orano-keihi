/**
 * 経費リポジトリ
 * D1データベースのexpensesテーブルへのアクセスを提供
 */

import type { D1Database } from "@cloudflare/workers-types";
import type { Expense } from "../types/d1-models.js";
import type { CreateExpenseDto, UpdateExpenseDto } from "../types/d1-dtos.js";
import { logger } from "../utils/logger.js";

/**
 * 経費リポジトリクラス
 */
export class ExpenseRepository {
  constructor(private db: D1Database) {}

  /**
   * 経費を作成する
   * @param dto 経費作成DTO
   * @param userId ユーザーID
   * @returns 作成された経費情報
   */
  async create(dto: CreateExpenseDto, userId: string): Promise<Expense> {
    try {
      const now = new Date().toISOString(); // RFC3339形式（JST）

      const result = await this.db
        .prepare(
          `INSERT INTO expenses (user_id, date, amount, category, description, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?)`,
        )
        .bind(userId, dto.date, dto.amount, dto.category, dto.description || null, now, now)
        .run();

      if (!result.success) {
        logger.error("経費作成に失敗しました", {
          userId,
          error: result.error,
        });
        throw new Error(`経費作成に失敗しました: ${result.error}`);
      }

      // 作成された経費のIDを取得
      const expenseId = result.meta.last_row_id;
      if (!expenseId) {
        throw new Error("作成された経費のIDを取得できませんでした");
      }

      logger.info("経費を作成しました", {
        expenseId,
        userId,
        amount: dto.amount,
      });

      // 作成した経費を取得して返す
      const newExpense = await this.findById(expenseId, userId);
      if (!newExpense) {
        throw new Error("作成した経費の取得に失敗しました");
      }

      return newExpense;
    } catch (error) {
      logger.error("createでエラーが発生しました", {
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * 経費IDで経費を取得する
   * @param id 経費ID
   * @param userId ユーザーID（アクセス制御用）
   * @returns 経費情報、または見つからない場合はnull
   */
  async findById(id: number, userId: string): Promise<Expense | null> {
    try {
      const result = await this.db
        .prepare("SELECT * FROM expenses WHERE id = ? AND user_id = ?")
        .bind(id, userId)
        .first<Expense>();

      if (!result) {
        logger.debug("経費が見つかりませんでした", { id, userId });
        return null;
      }

      logger.debug("経費を取得しました", {
        id: result.id,
        userId: result.user_id,
        amount: result.amount,
      });

      return result;
    } catch (error) {
      logger.error("findByIdでエラーが発生しました", {
        id,
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * 経費一覧を取得する（月とカテゴリでフィルタリング可能）
   * @param userId ユーザーID
   * @param month 月フィルター（YYYY-MM形式、オプション）
   * @param category カテゴリフィルター（オプション）
   * @returns 経費一覧
   */
  async findAll(userId: string, month?: string, category?: string): Promise<Expense[]> {
    try {
      let query = "SELECT * FROM expenses WHERE user_id = ?";
      const params: (string | number)[] = [userId];

      // 月フィルター
      if (month) {
        query += " AND date LIKE ?";
        params.push(`${month}%`); // YYYY-MM-DD形式の日付に対してYYYY-MM%でマッチ
      }

      // カテゴリフィルター
      if (category) {
        query += " AND category = ?";
        params.push(category);
      }

      query += " ORDER BY date DESC, created_at DESC";

      const result = await this.db
        .prepare(query)
        .bind(...params)
        .all<Expense>();

      if (!result.success) {
        logger.error("経費一覧取得に失敗しました", {
          userId,
          error: result.error,
        });
        throw new Error(`経費一覧取得に失敗しました: ${result.error}`);
      }

      logger.debug("経費一覧を取得しました", {
        userId,
        count: result.results.length,
        month,
        category,
      });

      return result.results;
    } catch (error) {
      logger.error("findAllでエラーが発生しました", {
        userId,
        month,
        category,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * 経費情報を更新する
   * @param id 経費ID
   * @param dto 経費更新DTO
   * @param userId ユーザーID（アクセス制御用）
   * @returns 更新後の経費情報
   */
  async update(id: number, dto: UpdateExpenseDto, userId: string): Promise<Expense> {
    try {
      const now = new Date().toISOString(); // RFC3339形式（JST）

      // 更新するフィールドを動的に構築
      const updates: string[] = [];
      const params: (string | number | null)[] = [];

      if (dto.date !== undefined) {
        updates.push("date = ?");
        params.push(dto.date);
      }
      if (dto.amount !== undefined) {
        updates.push("amount = ?");
        params.push(dto.amount);
      }
      if (dto.category !== undefined) {
        updates.push("category = ?");
        params.push(dto.category);
      }
      if (dto.description !== undefined) {
        updates.push("description = ?");
        params.push(dto.description);
      }
      if (dto.receipt_url !== undefined) {
        updates.push("receipt_url = ?");
        // 空文字列の場合はNULLに変換（CHECK制約対応）
        params.push(dto.receipt_url === "" ? null : dto.receipt_url);
      }

      // 更新するフィールドがない場合はエラー
      if (updates.length === 0) {
        throw new Error("更新するフィールドが指定されていません");
      }

      // updated_atは常に更新
      updates.push("updated_at = ?");
      params.push(now);

      // WHERE句のパラメータを追加
      params.push(id, userId);

      const query = `UPDATE expenses SET ${updates.join(", ")} WHERE id = ? AND user_id = ?`;

      const result = await this.db
        .prepare(query)
        .bind(...params)
        .run();

      if (!result.success) {
        logger.error("経費更新に失敗しました", {
          id,
          userId,
          error: result.error,
        });
        throw new Error(`経費更新に失敗しました: ${result.error}`);
      }

      // 更新されたレコードが存在するか確認
      if (result.meta.changes === 0) {
        logger.warn("更新対象の経費が見つかりませんでした", {
          id,
          userId,
        });
        throw new Error(`経費が見つかりません: ${id}`);
      }

      logger.info("経費情報を更新しました", {
        id,
        userId,
      });

      // 更新後の経費情報を取得して返す
      const updatedExpense = await this.findById(id, userId);
      if (!updatedExpense) {
        throw new Error("更新した経費の取得に失敗しました");
      }

      return updatedExpense;
    } catch (error) {
      logger.error("updateでエラーが発生しました", {
        id,
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * 経費を削除する
   * @param id 経費ID
   * @param userId ユーザーID（アクセス制御用）
   */
  async delete(id: number, userId: string): Promise<void> {
    try {
      const result = await this.db
        .prepare("DELETE FROM expenses WHERE id = ? AND user_id = ?")
        .bind(id, userId)
        .run();

      if (!result.success) {
        logger.error("経費削除に失敗しました", {
          id,
          userId,
          error: result.error,
        });
        throw new Error(`経費削除に失敗しました: ${result.error}`);
      }

      // 削除されたレコードが存在するか確認
      if (result.meta.changes === 0) {
        logger.warn("削除対象の経費が見つかりませんでした", {
          id,
          userId,
        });
        throw new Error(`経費が見つかりません: ${id}`);
      }

      logger.info("経費を削除しました", { id, userId });
    } catch (error) {
      logger.error("deleteでエラーが発生しました", {
        id,
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * 領収書URLを設定する
   * @param id 経費ID
   * @param receiptUrl 領収書URL
   * @param userId ユーザーID（アクセス制御用）
   * @returns 更新後の経費情報
   */
  async setReceiptUrl(id: number, receiptUrl: string, userId: string): Promise<Expense> {
    try {
      const now = new Date().toISOString(); // RFC3339形式（JST）

      const result = await this.db
        .prepare(
          `UPDATE expenses 
           SET receipt_url = ?, updated_at = ?
           WHERE id = ? AND user_id = ?`,
        )
        .bind(receiptUrl, now, id, userId)
        .run();

      if (!result.success) {
        logger.error("領収書URL設定に失敗しました", {
          id,
          userId,
          error: result.error,
        });
        throw new Error(`領収書URL設定に失敗しました: ${result.error}`);
      }

      // 更新されたレコードが存在するか確認
      if (result.meta.changes === 0) {
        logger.warn("領収書URL設定対象の経費が見つかりませんでした", {
          id,
          userId,
        });
        throw new Error(`経費が見つかりません: ${id}`);
      }

      logger.info("領収書URLを設定しました", {
        id,
        userId,
        receiptUrl,
      });

      // 更新後の経費情報を取得して返す
      const updatedExpense = await this.findById(id, userId);
      if (!updatedExpense) {
        throw new Error("更新した経費の取得に失敗しました");
      }

      return updatedExpense;
    } catch (error) {
      logger.error("setReceiptUrlでエラーが発生しました", {
        id,
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * 領収書URLを取得する
   * @param id 経費ID
   * @param userId ユーザーID（アクセス制御用）
   * @returns 領収書URL、または見つからない場合はnull
   */
  async getReceiptUrl(id: number, userId: string): Promise<string | null> {
    try {
      const result = await this.db
        .prepare("SELECT receipt_url FROM expenses WHERE id = ? AND user_id = ?")
        .bind(id, userId)
        .first<{ receipt_url: string | null }>();

      if (!result) {
        logger.debug("経費が見つかりませんでした", { id, userId });
        return null;
      }

      logger.debug("領収書URLを取得しました", {
        id,
        userId,
        hasReceiptUrl: result.receipt_url !== null,
      });

      return result.receipt_url;
    } catch (error) {
      logger.error("getReceiptUrlでエラーが発生しました", {
        id,
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }
}

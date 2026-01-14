/**
 * サブスクリプションリポジトリ
 * D1データベースのsubscriptionsテーブルへのアクセスを提供
 */

import type { D1Database } from "@cloudflare/workers-types";
import type { Subscription } from "../types/d1-models.js";
import type { CreateSubscriptionDto, UpdateSubscriptionDto } from "../types/d1-dtos.js";
import { logger } from "../utils/logger.js";

/**
 * サブスクリプションリポジトリクラス
 */
export class SubscriptionRepository {
  constructor(private db: D1Database) {}

  /**
   * サブスクリプションを作成する
   * @param dto サブスクリプション作成DTO
   * @param userId ユーザーID
   * @returns 作成されたサブスクリプション情報
   */
  async create(dto: CreateSubscriptionDto, userId: string): Promise<Subscription> {
    try {
      const now = new Date().toISOString(); // RFC3339形式（JST）

      const result = await this.db
        .prepare(
          `INSERT INTO subscriptions (user_id, name, amount, billing_cycle, start_date, category, is_active, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?)`,
        )
        .bind(
          userId,
          dto.name,
          dto.amount,
          dto.billing_cycle,
          dto.start_date,
          dto.category,
          now,
          now,
        )
        .run();

      if (!result.success) {
        logger.error("サブスクリプション作成に失敗しました", {
          userId,
          error: result.error,
        });
        throw new Error(`サブスクリプション作成に失敗しました: ${result.error}`);
      }

      // 作成されたサブスクリプションのIDを取得
      const subscriptionId = result.meta.last_row_id;
      if (!subscriptionId) {
        throw new Error("作成されたサブスクリプションのIDを取得できませんでした");
      }

      logger.info("サブスクリプションを作成しました", {
        subscriptionId,
        userId,
        name: dto.name,
        amount: dto.amount,
      });

      // 作成したサブスクリプションを取得して返す
      const newSubscription = await this.findById(subscriptionId, userId);
      if (!newSubscription) {
        throw new Error("作成したサブスクリプションの取得に失敗しました");
      }

      return newSubscription;
    } catch (error) {
      logger.error("createでエラーが発生しました", {
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * サブスクリプションIDでサブスクリプションを取得する
   * @param id サブスクリプションID
   * @param userId ユーザーID（アクセス制御用）
   * @returns サブスクリプション情報、または見つからない場合はnull
   */
  async findById(id: number, userId: string): Promise<Subscription | null> {
    try {
      const result = await this.db
        .prepare("SELECT * FROM subscriptions WHERE id = ? AND user_id = ?")
        .bind(id, userId)
        .first<Subscription>();

      if (!result) {
        logger.debug("サブスクリプションが見つかりませんでした", { id, userId });
        return null;
      }

      // is_activeをbooleanに変換（SQLiteは0/1で保存）
      const subscription: Subscription = {
        ...result,
        is_active: Boolean(result.is_active),
      };

      logger.debug("サブスクリプションを取得しました", {
        id: subscription.id,
        userId: subscription.user_id,
        name: subscription.name,
      });

      return subscription;
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
   * サブスクリプション一覧を取得する（アクティブフィルター可能）
   * @param userId ユーザーID
   * @param activeOnly アクティブなサブスクリプションのみを取得するか
   * @returns サブスクリプション一覧
   */
  async findAll(userId: string, activeOnly: boolean = false): Promise<Subscription[]> {
    try {
      let query = "SELECT * FROM subscriptions WHERE user_id = ?";
      const params: (string | number)[] = [userId];

      // アクティブフィルター
      if (activeOnly) {
        query += " AND is_active = 1";
      }

      query += " ORDER BY created_at DESC";

      const result = await this.db
        .prepare(query)
        .bind(...params)
        .all<Subscription>();

      if (!result.success) {
        logger.error("サブスクリプション一覧取得に失敗しました", {
          userId,
          error: result.error,
        });
        throw new Error(`サブスクリプション一覧取得に失敗しました: ${result.error}`);
      }

      // is_activeをbooleanに変換
      const subscriptions = result.results.map((sub) => ({
        ...sub,
        is_active: Boolean(sub.is_active),
      }));

      logger.debug("サブスクリプション一覧を取得しました", {
        userId,
        count: subscriptions.length,
        activeOnly,
      });

      return subscriptions;
    } catch (error) {
      logger.error("findAllでエラーが発生しました", {
        userId,
        activeOnly,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * サブスクリプション情報を更新する
   * @param id サブスクリプションID
   * @param dto サブスクリプション更新DTO
   * @param userId ユーザーID（アクセス制御用）
   * @returns 更新後のサブスクリプション情報
   */
  async update(id: number, dto: UpdateSubscriptionDto, userId: string): Promise<Subscription> {
    try {
      const now = new Date().toISOString(); // RFC3339形式（JST）

      // 更新するフィールドを動的に構築
      const updates: string[] = [];
      const params: (string | number)[] = [];

      if (dto.name !== undefined) {
        updates.push("name = ?");
        params.push(dto.name);
      }
      if (dto.amount !== undefined) {
        updates.push("amount = ?");
        params.push(dto.amount);
      }
      if (dto.billing_cycle !== undefined) {
        updates.push("billing_cycle = ?");
        params.push(dto.billing_cycle);
      }
      if (dto.start_date !== undefined) {
        updates.push("start_date = ?");
        params.push(dto.start_date);
      }
      if (dto.category !== undefined) {
        updates.push("category = ?");
        params.push(dto.category);
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

      const query = `UPDATE subscriptions SET ${updates.join(", ")} WHERE id = ? AND user_id = ?`;

      const result = await this.db
        .prepare(query)
        .bind(...params)
        .run();

      if (!result.success) {
        logger.error("サブスクリプション更新に失敗しました", {
          id,
          userId,
          error: result.error,
        });
        throw new Error(`サブスクリプション更新に失敗しました: ${result.error}`);
      }

      // 更新されたレコードが存在するか確認
      if (result.meta.changes === 0) {
        logger.warn("更新対象のサブスクリプションが見つかりませんでした", {
          id,
          userId,
        });
        throw new Error(`サブスクリプションが見つかりません: ${id}`);
      }

      logger.info("サブスクリプション情報を更新しました", {
        id,
        userId,
      });

      // 更新後のサブスクリプション情報を取得して返す
      const updatedSubscription = await this.findById(id, userId);
      if (!updatedSubscription) {
        throw new Error("更新したサブスクリプションの取得に失敗しました");
      }

      return updatedSubscription;
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
   * サブスクリプションのステータスを切り替える
   * @param id サブスクリプションID
   * @param userId ユーザーID（アクセス制御用）
   * @returns 更新後のサブスクリプション情報
   */
  async toggleStatus(id: number, userId: string): Promise<Subscription> {
    try {
      const now = new Date().toISOString(); // RFC3339形式（JST）

      // 現在のステータスを取得
      const currentSubscription = await this.findById(id, userId);
      if (!currentSubscription) {
        throw new Error(`サブスクリプションが見つかりません: ${id}`);
      }

      // ステータスを反転
      const newStatus = currentSubscription.is_active ? 0 : 1;

      const result = await this.db
        .prepare(
          `UPDATE subscriptions 
           SET is_active = ?, updated_at = ?
           WHERE id = ? AND user_id = ?`,
        )
        .bind(newStatus, now, id, userId)
        .run();

      if (!result.success) {
        logger.error("サブスクリプションステータス切り替えに失敗しました", {
          id,
          userId,
          error: result.error,
        });
        throw new Error(`サブスクリプションステータス切り替えに失敗しました: ${result.error}`);
      }

      logger.info("サブスクリプションステータスを切り替えました", {
        id,
        userId,
        newStatus: newStatus === 1,
      });

      // 更新後のサブスクリプション情報を取得して返す
      const updatedSubscription = await this.findById(id, userId);
      if (!updatedSubscription) {
        throw new Error("更新したサブスクリプションの取得に失敗しました");
      }

      return updatedSubscription;
    } catch (error) {
      logger.error("toggleStatusでエラーが発生しました", {
        id,
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * サブスクリプションを削除する
   * @param id サブスクリプションID
   * @param userId ユーザーID（アクセス制御用）
   */
  async delete(id: number, userId: string): Promise<void> {
    try {
      const result = await this.db
        .prepare("DELETE FROM subscriptions WHERE id = ? AND user_id = ?")
        .bind(id, userId)
        .run();

      if (!result.success) {
        logger.error("サブスクリプション削除に失敗しました", {
          id,
          userId,
          error: result.error,
        });
        throw new Error(`サブスクリプション削除に失敗しました: ${result.error}`);
      }

      // 削除されたレコードが存在するか確認
      if (result.meta.changes === 0) {
        logger.warn("削除対象のサブスクリプションが見つかりませんでした", {
          id,
          userId,
        });
        throw new Error(`サブスクリプションが見つかりません: ${id}`);
      }

      logger.info("サブスクリプションを削除しました", { id, userId });
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
   * 月額合計を計算する
   * @param userId ユーザーID
   * @returns 月額合計金額
   */
  async calculateMonthlyTotal(userId: string): Promise<number> {
    try {
      // アクティブなサブスクリプションのみを取得
      const activeSubscriptions = await this.findAll(userId, true);

      // 月額換算して合計を計算
      const total = activeSubscriptions.reduce((sum, subscription) => {
        let monthlyAmount = subscription.amount;

        // 年額の場合は12で割って月額に換算
        if (subscription.billing_cycle === "annual") {
          monthlyAmount = subscription.amount / 12;
        }

        return sum + monthlyAmount;
      }, 0);

      logger.debug("月額合計を計算しました", {
        userId,
        total,
        subscriptionCount: activeSubscriptions.length,
      });

      return total;
    } catch (error) {
      logger.error("calculateMonthlyTotalでエラーが発生しました", {
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * 領収書パスを設定する
   * @param id サブスクリプションID
   * @param receiptPath 領収書パス
   * @param userId ユーザーID（アクセス制御用）
   */
  async setReceiptPath(id: number, receiptPath: string, userId: string): Promise<void> {
    try {
      const now = new Date().toISOString(); // RFC3339形式（JST）

      const result = await this.db
        .prepare(
          `UPDATE subscriptions 
           SET receipt_path = ?, updated_at = ?
           WHERE id = ? AND user_id = ?`,
        )
        .bind(receiptPath, now, id, userId)
        .run();

      if (!result.success) {
        logger.error("領収書パス設定に失敗しました", {
          id,
          userId,
          error: result.error,
        });
        throw new Error(`領収書パス設定に失敗しました: ${result.error}`);
      }

      // 更新されたレコードが存在するか確認
      if (result.meta.changes === 0) {
        logger.warn("領収書パス設定対象のサブスクリプションが見つかりませんでした", {
          id,
          userId,
        });
        throw new Error(`サブスクリプションが見つかりません: ${id}`);
      }

      logger.info("領収書パスを設定しました", {
        id,
        userId,
        receiptPath,
      });
    } catch (error) {
      logger.error("setReceiptPathでエラーが発生しました", {
        id,
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * 領収書パスを取得する
   * @param id サブスクリプションID
   * @param userId ユーザーID（アクセス制御用）
   * @returns 領収書パス、または見つからない場合はnull
   */
  async getReceiptPath(id: number, userId: string): Promise<string | null> {
    try {
      const result = await this.db
        .prepare("SELECT receipt_path FROM subscriptions WHERE id = ? AND user_id = ?")
        .bind(id, userId)
        .first<{ receipt_path: string | null }>();

      if (!result) {
        logger.debug("サブスクリプションが見つかりませんでした", { id, userId });
        return null;
      }

      logger.debug("領収書パスを取得しました", {
        id,
        userId,
        hasReceiptPath: result.receipt_path !== null,
      });

      return result.receipt_path;
    } catch (error) {
      logger.error("getReceiptPathでエラーが発生しました", {
        id,
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }
}

/**
 * サブスクリプションサービス
 * サブスクリプションのCRUD操作とビジネスロジックを提供
 */

import type {
  Subscription,
  CreateSubscriptionDto,
  UpdateSubscriptionDto,
  SubscriptionListResponse,
  MonthlyTotalResponse,
} from "../types/config.js";
import { logger } from "../utils/logger.js";
import { AppError, ErrorCode } from "../utils/error-handler.js";
import { withDatabaseRetry } from "../utils/retry.js";

/**
 * サブスクリプションサービスクラス
 */
export class SubscriptionService {
  // TODO: 実際の実装では、データベース接続を注入する
  // 現在はモックデータを使用
  private subscriptions: Map<number, Subscription> = new Map();
  private nextId = 1;

  constructor() {
    // モックデータの初期化
    this.initializeMockData();
    logger.info("サブスクリプションサービスを初期化しました");
  }

  /**
   * モックデータの初期化
   */
  private initializeMockData(): void {
    const mockSubscriptions: Subscription[] = [
      {
        id: 1,
        userId: 1,
        name: "Netflix",
        amount: 1490,
        billing_cycle: "monthly",
        start_date: "2024-01-01",
        category: "エンターテイメント",
        is_active: true,
        created_at: "2024-01-01T00:00:00Z",
        updated_at: "2024-01-01T00:00:00Z",
      },
      {
        id: 2,
        userId: 1,
        name: "Adobe Creative Cloud",
        amount: 65760,
        billing_cycle: "annual",
        start_date: "2024-02-01",
        category: "ソフトウェア",
        is_active: true,
        created_at: "2024-02-01T00:00:00Z",
        updated_at: "2024-02-01T00:00:00Z",
      },
      {
        id: 3,
        userId: 1,
        name: "Spotify",
        amount: 980,
        billing_cycle: "monthly",
        start_date: "2024-01-15",
        category: "エンターテイメント",
        is_active: false,
        created_at: "2024-01-15T00:00:00Z",
        updated_at: "2024-11-01T00:00:00Z",
      },
    ];

    for (const subscription of mockSubscriptions) {
      this.subscriptions.set(subscription.id, subscription);
      this.nextId = Math.max(this.nextId, subscription.id + 1);
    }
  }

  /**
   * サブスクリプション一覧を取得する
   * @param userId ユーザーID
   * @param activeOnly アクティブなサブスクリプションのみ取得するか
   * @returns サブスクリプション一覧
   */
  async getSubscriptions(
    userId: number,
    activeOnly: boolean = false,
  ): Promise<SubscriptionListResponse> {
    return withDatabaseRetry(async () => {
      try {
        // ユーザーのサブスクリプションを取得
        const userSubscriptions = Array.from(this.subscriptions.values())
          .filter((sub) => sub.userId === userId)
          .filter((sub) => !activeOnly || sub.is_active)
          .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());

        // 統計情報を計算
        const activeSubscriptions = userSubscriptions.filter((sub) => sub.is_active);
        const monthlyTotal = this.calculateMonthlyTotal(activeSubscriptions);

        logger.info("サブスクリプション一覧を取得しました", {
          userId,
          total: userSubscriptions.length,
          activeCount: activeSubscriptions.length,
          monthlyTotal,
        });

        return {
          subscriptions: userSubscriptions,
          total: userSubscriptions.length,
          activeCount: activeSubscriptions.length,
          monthlyTotal,
        };
      } catch (error) {
        logger.error("サブスクリプション一覧の取得に失敗しました", { userId, error });
        throw new AppError(ErrorCode.DATABASE_ERROR, "サブスクリプション一覧の取得に失敗しました");
      }
    });
  }

  /**
   * サブスクリプションを作成する
   * @param userId ユーザーID
   * @param dto 作成データ
   * @returns 作成されたサブスクリプション
   */
  async createSubscription(userId: number, dto: CreateSubscriptionDto): Promise<Subscription> {
    return withDatabaseRetry(async () => {
      try {
        // バリデーション
        this.validateSubscriptionDto(dto);

        const now = new Date().toISOString();
        const subscription: Subscription = {
          id: this.nextId++,
          userId,
          name: dto.name,
          amount: dto.amount,
          billing_cycle: dto.billing_cycle,
          start_date: dto.start_date,
          category: dto.category,
          is_active: true,
          receipt_path: dto.receipt_path,
          created_at: now,
          updated_at: now,
        };

        this.subscriptions.set(subscription.id, subscription);

        logger.info("サブスクリプションを作成しました", {
          userId,
          subscriptionId: subscription.id,
          name: subscription.name,
          amount: subscription.amount,
        });

        return subscription;
      } catch (error) {
        if (error instanceof AppError) {
          throw error;
        }
        logger.error("サブスクリプションの作成に失敗しました", { userId, dto, error });
        throw new AppError(ErrorCode.DATABASE_ERROR, "サブスクリプションの作成に失敗しました");
      }
    });
  }

  /**
   * サブスクリプションを更新する
   * @param userId ユーザーID
   * @param subscriptionId サブスクリプションID
   * @param dto 更新データ
   * @returns 更新されたサブスクリプション
   */
  async updateSubscription(
    userId: number,
    subscriptionId: number,
    dto: UpdateSubscriptionDto,
  ): Promise<Subscription> {
    return withDatabaseRetry(async () => {
      try {
        const subscription = this.subscriptions.get(subscriptionId);
        if (!subscription) {
          throw new AppError(ErrorCode.NOT_FOUND, "サブスクリプションが見つかりません");
        }

        // 所有者チェック
        if (subscription.userId !== userId) {
          throw new AppError(
            ErrorCode.FORBIDDEN,
            "このサブスクリプションにアクセスする権限がありません",
          );
        }

        // 部分バリデーション
        if (
          dto.name !== undefined ||
          dto.amount !== undefined ||
          dto.billing_cycle !== undefined ||
          dto.start_date !== undefined
        ) {
          this.validateSubscriptionDto({
            name: dto.name ?? subscription.name,
            amount: dto.amount ?? subscription.amount,
            billing_cycle: dto.billing_cycle ?? subscription.billing_cycle,
            start_date: dto.start_date ?? subscription.start_date,
            category: dto.category ?? subscription.category,
          });
        }

        // 更新
        const updatedSubscription: Subscription = {
          ...subscription,
          name: dto.name ?? subscription.name,
          amount: dto.amount ?? subscription.amount,
          billing_cycle: dto.billing_cycle ?? subscription.billing_cycle,
          start_date: dto.start_date ?? subscription.start_date,
          category: dto.category ?? subscription.category,
          receipt_path: dto.receipt_path ?? subscription.receipt_path,
          updated_at: new Date().toISOString(),
        };

        this.subscriptions.set(subscriptionId, updatedSubscription);

        logger.info("サブスクリプションを更新しました", {
          userId,
          subscriptionId,
          changes: dto,
        });

        return updatedSubscription;
      } catch (error) {
        if (error instanceof AppError) {
          throw error;
        }
        logger.error("サブスクリプションの更新に失敗しました", {
          userId,
          subscriptionId,
          dto,
          error,
        });
        throw new AppError(ErrorCode.DATABASE_ERROR, "サブスクリプションの更新に失敗しました");
      }
    });
  }

  /**
   * サブスクリプションのアクティブ状態を切り替える
   * @param userId ユーザーID
   * @param subscriptionId サブスクリプションID
   * @returns 更新されたサブスクリプション
   */
  async toggleSubscriptionStatus(userId: number, subscriptionId: number): Promise<Subscription> {
    return withDatabaseRetry(async () => {
      try {
        const subscription = this.subscriptions.get(subscriptionId);
        if (!subscription) {
          throw new AppError(ErrorCode.NOT_FOUND, "サブスクリプションが見つかりません");
        }

        // 所有者チェック
        if (subscription.userId !== userId) {
          throw new AppError(
            ErrorCode.FORBIDDEN,
            "このサブスクリプションにアクセスする権限がありません",
          );
        }

        // ステータス切り替え
        const updatedSubscription: Subscription = {
          ...subscription,
          is_active: !subscription.is_active,
          updated_at: new Date().toISOString(),
        };

        this.subscriptions.set(subscriptionId, updatedSubscription);

        logger.info("サブスクリプションのステータスを切り替えました", {
          userId,
          subscriptionId,
          newStatus: updatedSubscription.is_active,
        });

        return updatedSubscription;
      } catch (error) {
        if (error instanceof AppError) {
          throw error;
        }
        logger.error("サブスクリプションのステータス切り替えに失敗しました", {
          userId,
          subscriptionId,
          error,
        });
        throw new AppError(
          ErrorCode.DATABASE_ERROR,
          "サブスクリプションのステータス切り替えに失敗しました",
        );
      }
    });
  }

  /**
   * サブスクリプションを削除する
   * @param userId ユーザーID
   * @param subscriptionId サブスクリプションID
   */
  async deleteSubscription(userId: number, subscriptionId: number): Promise<void> {
    return withDatabaseRetry(async () => {
      try {
        const subscription = this.subscriptions.get(subscriptionId);
        if (!subscription) {
          throw new AppError(ErrorCode.NOT_FOUND, "サブスクリプションが見つかりません");
        }

        // 所有者チェック
        if (subscription.userId !== userId) {
          throw new AppError(
            ErrorCode.FORBIDDEN,
            "このサブスクリプションにアクセスする権限がありません",
          );
        }

        this.subscriptions.delete(subscriptionId);

        logger.info("サブスクリプションを削除しました", {
          userId,
          subscriptionId,
          name: subscription.name,
        });
      } catch (error) {
        if (error instanceof AppError) {
          throw error;
        }
        logger.error("サブスクリプションの削除に失敗しました", {
          userId,
          subscriptionId,
          error,
        });
        throw new AppError(ErrorCode.DATABASE_ERROR, "サブスクリプションの削除に失敗しました");
      }
    });
  }

  /**
   * 月額サブスクリプション合計を取得する
   * @param userId ユーザーID
   * @returns 月額合計情報
   */
  async getMonthlyTotal(userId: number): Promise<MonthlyTotalResponse> {
    return withDatabaseRetry(async () => {
      try {
        const activeSubscriptions = Array.from(this.subscriptions.values()).filter(
          (sub) => sub.userId === userId && sub.is_active,
        );

        const monthlyTotal = this.calculateMonthlyTotal(activeSubscriptions);

        logger.info("月額サブスクリプション合計を取得しました", {
          userId,
          monthlyTotal,
          activeSubscriptions: activeSubscriptions.length,
        });

        return {
          monthlyTotal,
          activeSubscriptions: activeSubscriptions.length,
        };
      } catch (error) {
        logger.error("月額サブスクリプション合計の取得に失敗しました", { userId, error });
        throw new AppError(
          ErrorCode.DATABASE_ERROR,
          "月額サブスクリプション合計の取得に失敗しました",
        );
      }
    });
  }

  /**
   * 月額合計を計算する
   * @param subscriptions サブスクリプション一覧
   * @returns 月額合計
   */
  private calculateMonthlyTotal(subscriptions: Subscription[]): number {
    return subscriptions.reduce((total, sub) => {
      if (sub.billing_cycle === "monthly") {
        return total + sub.amount;
      } else if (sub.billing_cycle === "annual") {
        // 年額を月額に換算
        return total + Math.round(sub.amount / 12);
      }
      return total;
    }, 0);
  }

  /**
   * サブスクリプションDTOのバリデーション
   * @param dto バリデーション対象のDTO
   */
  private validateSubscriptionDto(dto: CreateSubscriptionDto): void {
    // 名前のバリデーション
    if (!dto.name || dto.name.trim().length === 0) {
      throw new AppError(ErrorCode.VALIDATION_ERROR, "サブスクリプション名は必須です");
    }
    if (dto.name.length > 100) {
      throw new AppError(
        ErrorCode.VALIDATION_ERROR,
        "サブスクリプション名は100文字以内で入力してください",
      );
    }

    // 金額のバリデーション
    if (dto.amount === undefined || dto.amount === null) {
      throw new AppError(ErrorCode.VALIDATION_ERROR, "金額は必須です");
    }
    if (dto.amount <= 0) {
      throw new AppError(ErrorCode.VALIDATION_ERROR, "金額は0より大きい値を入力してください");
    }
    if (dto.amount > 10000000) {
      throw new AppError(ErrorCode.VALIDATION_ERROR, "金額は1000万円以下で入力してください");
    }

    // 支払いサイクルのバリデーション
    if (!["monthly", "annual"].includes(dto.billing_cycle)) {
      throw new AppError(
        ErrorCode.VALIDATION_ERROR,
        "支払いサイクルは 'monthly' または 'annual' を指定してください",
      );
    }

    // 開始日のバリデーション
    if (!dto.start_date) {
      throw new AppError(ErrorCode.VALIDATION_ERROR, "開始日は必須です");
    }
    const startDate = new Date(dto.start_date);
    if (isNaN(startDate.getTime())) {
      throw new AppError(ErrorCode.VALIDATION_ERROR, "開始日の形式が正しくありません");
    }

    // カテゴリのバリデーション
    if (!dto.category || dto.category.trim().length === 0) {
      throw new AppError(ErrorCode.VALIDATION_ERROR, "カテゴリは必須です");
    }
    if (dto.category.length > 50) {
      throw new AppError(ErrorCode.VALIDATION_ERROR, "カテゴリは50文字以内で入力してください");
    }
  }
}

/**
 * サブスクリプションサービスのインスタンスを作成
 */
export function createSubscriptionService(): SubscriptionService {
  return new SubscriptionService();
}

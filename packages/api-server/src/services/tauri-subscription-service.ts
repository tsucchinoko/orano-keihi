/**
 * Tauriサブスクリプションサービス
 * TauriのSQLiteデータベースからサブスクリプションデータを取得
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
 * Tauriコマンドを実行するためのインターフェース
 * 実際の実装では、Tauriアプリケーションとの通信を行う
 */
interface TauriCommandExecutor {
  invoke<T>(command: string, args?: any): Promise<T>;
}

/**
 * Tauriサブスクリプションサービスクラス
 */
export class TauriSubscriptionService {
  private tauriExecutor: TauriCommandExecutor;

  constructor(tauriExecutor: TauriCommandExecutor) {
    this.tauriExecutor = tauriExecutor;
    logger.info("Tauriサブスクリプションサービスを初期化しました");
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
        // Tauriコマンドを実行してサブスクリプション一覧を取得
        const subscriptions = await this.tauriExecutor.invoke<Subscription[]>("get_subscriptions", {
          activeOnly,
          session_token: this.getSessionTokenForUser(userId),
        });

        // 統計情報を計算
        const activeSubscriptions = subscriptions.filter((sub) => sub.is_active);
        const monthlyTotal = this.calculateMonthlyTotal(activeSubscriptions);

        logger.info("サブスクリプション一覧を取得しました", {
          userId,
          total: subscriptions.length,
          activeCount: activeSubscriptions.length,
          monthlyTotal,
        });

        return {
          subscriptions,
          total: subscriptions.length,
          activeCount: activeSubscriptions.length,
          monthlyTotal,
        };
      } catch (error) {
        logger.error("サブスクリプション一覧の取得に失敗しました", {
          userId,
          error,
        });
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
        // Tauriコマンドを実行してサブスクリプションを作成
        const subscription = await this.tauriExecutor.invoke<Subscription>("create_subscription", {
          dto,
          sessionToken: this.getSessionTokenForUser(userId),
        });

        logger.info("サブスクリプションを作成しました", {
          userId,
          subscriptionId: subscription.id,
          name: subscription.name,
          amount: subscription.amount,
        });

        return subscription;
      } catch (error) {
        logger.error("サブスクリプションの作成に失敗しました", {
          userId,
          dto,
          error,
        });
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
        // Tauriコマンドを実行してサブスクリプションを更新
        const subscription = await this.tauriExecutor.invoke<Subscription>("update_subscription", {
          id: subscriptionId,
          dto,
          sessionToken: this.getSessionTokenForUser(userId),
        });

        logger.info("サブスクリプションを更新しました", {
          userId,
          subscriptionId,
          changes: dto,
        });

        return subscription;
      } catch (error) {
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
        // Tauriコマンドを実行してステータスを切り替え
        const subscription = await this.tauriExecutor.invoke<Subscription>(
          "toggle_subscription_status",
          {
            id: subscriptionId,
            sessionToken: this.getSessionTokenForUser(userId),
          },
        );

        logger.info("サブスクリプションのステータスを切り替えました", {
          userId,
          subscriptionId,
          newStatus: subscription.is_active,
        });

        return subscription;
      } catch (error) {
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
        // Tauriコマンドを実行してサブスクリプションを削除
        await this.tauriExecutor.invoke<void>("delete_subscription", {
          id: subscriptionId,
          sessionToken: this.getSessionTokenForUser(userId),
        });

        logger.info("サブスクリプションを削除しました", {
          userId,
          subscriptionId,
        });
      } catch (error) {
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
        // Tauriコマンドを実行して月額合計を取得
        const monthlyTotal = await this.tauriExecutor.invoke<number>(
          "get_monthly_subscription_total",
          {
            sessionToken: this.getSessionTokenForUser(userId),
          },
        );

        // アクティブなサブスクリプション数を取得
        const subscriptions = await this.tauriExecutor.invoke<Subscription[]>("get_subscriptions", {
          activeOnly: true,
          sessionToken: this.getSessionTokenForUser(userId),
        });

        logger.info("月額サブスクリプション合計を取得しました", {
          userId,
          monthlyTotal,
          activeSubscriptions: subscriptions.length,
        });

        return {
          monthlyTotal,
          activeSubscriptions: subscriptions.length,
        };
      } catch (error) {
        logger.error("月額サブスクリプション合計の取得に失敗しました", {
          userId,
          error,
        });
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
   * ユーザーIDからセッショントークンを取得する
   * 実際の実装では、適切な認証メカニズムを使用する
   * @param userId ユーザーID
   * @returns セッショントークン
   */
  private getSessionTokenForUser(userId: number): string {
    // TODO: 実際の実装では、適切な認証トークンを生成または取得する
    // 現在は開発用の固定トークンを返す
    return `dev-token-user-${userId}`;
  }
}

/**
 * モックTauriコマンド実行器（開発・テスト用）
 */
export class MockTauriCommandExecutor implements TauriCommandExecutor {
  private mockData: Map<number, Subscription[]> = new Map();
  private nextId = 1;

  constructor() {
    // モックデータの初期化
    this.initializeMockData();
  }

  async invoke<T>(command: string, args?: any): Promise<T> {
    logger.debug("モックTauriコマンドを実行", { command, args });

    switch (command) {
      case "get_subscriptions":
        return this.handleGetSubscriptions(args) as T;
      case "create_subscription":
        return this.handleCreateSubscription(args) as T;
      case "update_subscription":
        return this.handleUpdateSubscription(args) as T;
      case "toggle_subscription_status":
        return this.handleToggleSubscriptionStatus(args) as T;
      case "delete_subscription":
        return this.handleDeleteSubscription(args) as T;
      case "get_monthly_subscription_total":
        return this.handleGetMonthlyTotal(args) as T;
      default:
        throw new Error(`未知のTauriコマンド: ${command}`);
    }
  }

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
        receipt_path: undefined,
        created_at: "2024-01-01T00:00:00+09:00",
        updated_at: "2024-01-01T00:00:00+09:00",
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
        receipt_path: undefined,
        created_at: "2024-02-01T00:00:00+09:00",
        updated_at: "2024-02-01T00:00:00+09:00",
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
        receipt_path: undefined,
        created_at: "2024-01-15T00:00:00+09:00",
        updated_at: "2024-11-01T00:00:00+09:00",
      },
    ];

    this.mockData.set(1, mockSubscriptions);
    this.nextId = 4;
  }

  private handleGetSubscriptions(args: any): Subscription[] {
    const userId = this.extractUserIdFromToken(args.sessionToken);
    const subscriptions = this.mockData.get(userId) || [];

    if (args.activeOnly) {
      return subscriptions.filter((sub) => sub.is_active);
    }

    return subscriptions;
  }

  private handleCreateSubscription(args: any): Subscription {
    const userId = this.extractUserIdFromToken(args.sessionToken);
    const dto = args.dto as CreateSubscriptionDto;

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
      receipt_path: undefined,
      created_at: now,
      updated_at: now,
    };

    const userSubscriptions = this.mockData.get(userId) || [];
    userSubscriptions.push(subscription);
    this.mockData.set(userId, userSubscriptions);

    return subscription;
  }

  private handleUpdateSubscription(args: any): Subscription {
    const userId = this.extractUserIdFromToken(args.sessionToken);
    const subscriptionId = args.id;
    const dto = args.dto as UpdateSubscriptionDto;

    const userSubscriptions = this.mockData.get(userId) || [];
    const subscriptionIndex = userSubscriptions.findIndex((sub) => sub.id === subscriptionId);

    if (subscriptionIndex === -1) {
      throw new Error("サブスクリプションが見つかりません");
    }

    const subscription = userSubscriptions[subscriptionIndex];
    const updatedSubscription: Subscription = {
      ...subscription,
      name: dto.name ?? subscription.name,
      amount: dto.amount ?? subscription.amount,
      billing_cycle: dto.billing_cycle ?? subscription.billing_cycle,
      start_date: dto.start_date ?? subscription.start_date,
      category: dto.category ?? subscription.category,
      updated_at: new Date().toISOString(),
    };

    userSubscriptions[subscriptionIndex] = updatedSubscription;
    this.mockData.set(userId, userSubscriptions);

    return updatedSubscription;
  }

  private handleToggleSubscriptionStatus(args: any): Subscription {
    const userId = this.extractUserIdFromToken(args.sessionToken);
    const subscriptionId = args.id;

    const userSubscriptions = this.mockData.get(userId) || [];
    const subscriptionIndex = userSubscriptions.findIndex((sub) => sub.id === subscriptionId);

    if (subscriptionIndex === -1) {
      throw new Error("サブスクリプションが見つかりません");
    }

    const subscription = userSubscriptions[subscriptionIndex];
    const updatedSubscription: Subscription = {
      ...subscription,
      is_active: !subscription.is_active,
      updated_at: new Date().toISOString(),
    };

    userSubscriptions[subscriptionIndex] = updatedSubscription;
    this.mockData.set(userId, userSubscriptions);

    return updatedSubscription;
  }

  private handleDeleteSubscription(args: any): void {
    const userId = this.extractUserIdFromToken(args.sessionToken);
    const subscriptionId = args.id;

    const userSubscriptions = this.mockData.get(userId) || [];
    const subscriptionIndex = userSubscriptions.findIndex((sub) => sub.id === subscriptionId);

    if (subscriptionIndex === -1) {
      throw new Error("サブスクリプションが見つかりません");
    }

    // サブスクリプションを削除
    userSubscriptions.splice(subscriptionIndex, 1);
    this.mockData.set(userId, userSubscriptions);
  }

  private handleGetMonthlyTotal(args: any): number {
    const userId = this.extractUserIdFromToken(args.sessionToken);
    const subscriptions = this.mockData.get(userId) || [];

    return subscriptions
      .filter((sub) => sub.is_active)
      .reduce((total, sub) => {
        if (sub.billing_cycle === "monthly") {
          return total + sub.amount;
        } else if (sub.billing_cycle === "annual") {
          return total + Math.round(sub.amount / 12);
        }
        return total;
      }, 0);
  }

  private extractUserIdFromToken(token: string): number {
    // dev-token-user-{userId} の形式からユーザーIDを抽出
    const match = token.match(/dev-token-user-(\d+)/);
    return match ? parseInt(match[1], 10) : 1;
  }
}

/**
 * 実際のTauriコマンド実行器（Node.js環境用）
 * 注意: この実装は概念的なものです。実際の環境では、
 * TauriアプリケーションとAPIサーバー間の通信メカニズムが必要です。
 */
export class NodeTauriCommandExecutor implements TauriCommandExecutor {
  async invoke<T>(command: string, args?: any): Promise<T> {
    // TODO: 実際の実装では、以下のいずれかの方法でTauriアプリケーションと通信する：
    // 1. IPC（Inter-Process Communication）
    // 2. HTTP API経由でTauriアプリケーションと通信
    // 3. 共有データベースファイル経由
    // 4. WebSocket接続

    logger.warn("NodeTauriCommandExecutor: 実際のTauriコマンド実行は未実装です", {
      command,
      args,
    });

    // 現在はモック実行器にフォールバック
    const mockExecutor = new MockTauriCommandExecutor();
    return mockExecutor.invoke<T>(command, args);
  }
}

/**
 * Tauriサブスクリプションサービスのインスタンスを作成
 */
export function createTauriSubscriptionService(
  tauriExecutor?: TauriCommandExecutor,
): TauriSubscriptionService {
  // 実際のTauriコマンド実行器が提供されない場合は、モック実行器を使用
  const executor = tauriExecutor || new MockTauriCommandExecutor();
  return new TauriSubscriptionService(executor);
}

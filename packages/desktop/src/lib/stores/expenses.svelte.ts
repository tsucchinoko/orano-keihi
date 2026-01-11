import type { Expense, Subscription } from '../types';
import {
  getExpenses,
  createExpense,
  updateExpense,
  deleteExpense,
  getSubscriptions,
  createSubscription,
  updateSubscription,
  toggleSubscriptionStatus,
  deleteSubscription,
  getMonthlySubscriptionTotal,
} from '../utils/tauri';

/**
 * 経費とサブスクリプションの状態管理ストア
 * Svelte 5のrunesを使用したリアクティブな状態管理
 */
class ExpenseStore {
  // 経費データ
  expenses = $state<Expense[]>([]);

  // サブスクリプションデータ
  subscriptions = $state<Subscription[]>([]);

  // 選択された月（YYYY-MM形式）
  selectedMonth = $state<string>(this.getCurrentMonth());

  // 選択されたカテゴリフィルター
  selectedCategories = $state<string[]>([]);

  // ローディング状態
  isLoading = $state<boolean>(false);

  // エラーメッセージ
  error = $state<string | null>(null);

  // 月額サブスクリプション合計
  monthlySubscriptionTotal = $state<number>(0);

  /**
   * フィルタリングされた経費リスト（派生状態）
   */
  filteredExpenses = $derived.by(() => {
    return this.expenses.filter((expense) => {
      // 月でフィルタリング
      const expenseMonth = expense.date.substring(0, 7); // YYYY-MM
      if (expenseMonth !== this.selectedMonth) {
        return false;
      }

      // カテゴリでフィルタリング
      if (
        this.selectedCategories.length > 0 &&
        !this.selectedCategories.includes(expense.category)
      ) {
        return false;
      }

      return true;
    });
  });

  /**
   * カテゴリ別の合計金額（派生状態）
   */
  categoryTotals = $derived.by(() => {
    const totals: Record<string, number> = {};

    for (const expense of this.filteredExpenses) {
      if (!totals[expense.category]) {
        totals[expense.category] = 0;
      }
      totals[expense.category] += expense.amount;
    }

    return totals;
  });

  /**
   * 選択された月の合計金額（派生状態）
   */
  monthlyTotal = $derived.by(() => {
    return this.filteredExpenses.reduce(
      (sum, expense) => sum + expense.amount,
      0
    );
  });

  /**
   * アクティブなサブスクリプションのみ（派生状態）
   */
  activeSubscriptions = $derived.by(() => {
    return this.subscriptions.filter((sub) => sub.is_active);
  });

  /**
   * 現在の月を取得（YYYY-MM形式）
   */
  private getCurrentMonth(): string {
    const now = new Date();
    const year = now.getFullYear();
    const month = String(now.getMonth() + 1).padStart(2, '0');
    return `${year}-${month}`;
  }

  /**
   * 経費一覧を読み込む
   */
  async loadExpenses(): Promise<void> {
    this.isLoading = true;
    this.error = null;

    try {
      const result = await getExpenses(
        this.selectedMonth,
        this.selectedCategories.length > 0
          ? this.selectedCategories.join(',')
          : undefined
      );

      if (result.error) {
        this.error = result.error;
      } else if (result.data) {
        this.expenses = result.data;
      }
    } catch (err) {
      this.error = `経費の読み込みに失敗しました: ${String(err)}`;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * 新しい経費を作成する
   */
  async addExpense(
    expense: Omit<Expense, 'id' | 'created_at' | 'updated_at'>
  ): Promise<boolean> {
    this.isLoading = true;
    this.error = null;

    try {
      const result = await createExpense({
        date: expense.date,
        amount: expense.amount,
        category: expense.category,
        description: expense.description,
      });

      if (result.error) {
        this.error = result.error;
        return false;
      }

      if (result.data) {
        // 新しい経費をリストに追加
        this.expenses = [...this.expenses, result.data];
        return true;
      }

      return false;
    } catch (err) {
      this.error = `経費の作成に失敗しました: ${String(err)}`;
      return false;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * 経費を更新する
   */
  async modifyExpense(
    id: number,
    updates: Partial<Omit<Expense, 'id' | 'created_at' | 'updated_at'>>
  ): Promise<boolean> {
    this.isLoading = true;
    this.error = null;

    try {
      const result = await updateExpense(id, updates);

      if (result.error) {
        console.error('updateExpenseエラー:', result.error);
        this.error = result.error;
        return false;
      }

      if (result.data) {
        // 経費リストを更新
        const updatedExpense = result.data;
        this.expenses = this.expenses.map((exp) =>
          exp.id === id ? updatedExpense : exp
        );
        return true;
      }

      console.warn('updateExpenseで有効なデータが返されませんでした');
      return false;
    } catch (err) {
      console.error('modifyExpenseでエラー:', err);
      this.error = `経費の更新に失敗しました: ${String(err)}`;
      return false;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * 経費を削除する
   */
  async removeExpense(id: number): Promise<boolean> {
    this.isLoading = true;
    this.error = null;

    try {
      const result = await deleteExpense(id);

      if (result.error) {
        console.error(`📋 ストア: 削除エラー:`, result.error);
        this.error = result.error;
        return false;
      }

      // 経費リストから削除
      this.expenses = this.expenses.filter((exp) => exp.id !== id);
      return true;
    } catch (err) {
      console.error(`📋 ストア: 削除例外:`, err);
      this.error = `経費の削除に失敗しました: ${String(err)}`;
      return false;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * サブスクリプション一覧を読み込む
   */
  async loadSubscriptions(activeOnly: boolean = false): Promise<void> {
    this.isLoading = true;
    this.error = null;

    try {
      const result = await getSubscriptions(activeOnly);
      this.subscriptions = result.data ?? [];
      // 月額合計は別途取得する必要がある
      await this.loadMonthlySubscriptionTotal();
    } catch (err) {
      this.error = `サブスクリプションの読み込みに失敗しました: ${String(err)}`;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * 新しいサブスクリプションを作成する
   */
  async addSubscription(
    subscription: Omit<
      Subscription,
      'id' | 'is_active' | 'created_at' | 'updated_at'
    >
  ): Promise<boolean> {
    this.isLoading = true;
    this.error = null;

    try {
      const result = await createSubscription(subscription);

      if (result.error) {
        this.error = result.error;
        return false;
      }

      if (result.data) {
        // 新しいサブスクリプションをリストに追加
        this.subscriptions = [...this.subscriptions, result.data];
        // 月額合計を再計算
        await this.loadMonthlySubscriptionTotal();
        return true;
      }

      return false;
    } catch (err) {
      this.error = `サブスクリプションの作成に失敗しました: ${String(err)}`;
      return false;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * サブスクリプションを更新する
   */
  async modifySubscription(
    id: number,
    updates: Partial<
      Omit<Subscription, 'id' | 'is_active' | 'created_at' | 'updated_at'>
    >
  ): Promise<boolean> {
    this.isLoading = true;
    this.error = null;

    try {
      const result = await updateSubscription(id, updates);

      if (result.error) {
        this.error = result.error;
        return false;
      }

      if (result.data) {
        // サブスクリプションリストを更新
        this.subscriptions = this.subscriptions.map((sub) =>
          sub.id === id ? result.data! : sub
        );
        // 月額合計を再計算
        await this.loadMonthlySubscriptionTotal();
        return true;
      }

      return false;
    } catch (err) {
      this.error = `サブスクリプションの更新に失敗しました: ${String(err)}`;
      return false;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * サブスクリプションのアクティブ状態を切り替える
   */
  async toggleSubscription(id: number): Promise<boolean> {
    this.isLoading = true;
    this.error = null;

    try {
      const result = await toggleSubscriptionStatus(id);

      if (result.error) {
        this.error = result.error;
        return false;
      }

      if (result.data) {
        // サブスクリプションリストを更新
        this.subscriptions = this.subscriptions.map((sub) =>
          sub.id === id ? result.data! : sub
        );
        // 月額合計を再計算
        await this.loadMonthlySubscriptionTotal();
        return true;
      }

      return false;
    } catch (err) {
      this.error = `サブスクリプションの状態切り替えに失敗しました: ${String(err)}`;
      return false;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * サブスクリプションを削除する
   */
  async removeSubscription(id: number): Promise<boolean> {
    this.isLoading = true;
    this.error = null;

    try {
      const result = await deleteSubscription(id);

      if (result.error) {
        this.error = result.error;
        return false;
      }

      // サブスクリプションリストから削除
      this.subscriptions = this.subscriptions.filter((sub) => sub.id !== id);
      // 月額合計を再計算
      await this.loadMonthlySubscriptionTotal();
      return true;
    } catch (err) {
      this.error = `サブスクリプションの削除に失敗しました: ${String(err)}`;
      return false;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * 月額サブスクリプション合計を読み込む
   */
  async loadMonthlySubscriptionTotal(): Promise<void> {
    try {
      const result = await getMonthlySubscriptionTotal();
      this.monthlySubscriptionTotal = result.data ?? 0;
    } catch (err) {
      this.error = `月額合計の読み込みに失敗しました: ${String(err)}`;
    }
  }

  /**
   * 選択された月を変更する
   */
  setSelectedMonth(month: string): void {
    this.selectedMonth = month;
    // 月が変更されたら経費を再読み込み
    void this.loadExpenses();
  }

  /**
   * カテゴリフィルターを設定する
   */
  setSelectedCategories(categories: string[]): void {
    this.selectedCategories = categories;
    // カテゴリが変更されたら経費を再読み込み
    void this.loadExpenses();
  }

  /**
   * エラーをクリアする
   */
  clearError(): void {
    this.error = null;
  }
}

// シングルトンインスタンスをエクスポート
export const expenseStore = new ExpenseStore();

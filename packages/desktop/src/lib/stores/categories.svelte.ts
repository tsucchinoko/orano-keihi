/**
 * カテゴリーストア
 *
 * DBから取得したカテゴリーデータを管理するストア。
 * UIコンポーネントはこのストアからカテゴリー情報を取得します。
 */

import type { Category } from '$lib/types';
import { getCategories } from '$lib/utils/tauri';

/**
 * カテゴリーカラー配列（フロントエンド固定値）
 * カテゴリーIDをこの配列の長さで割った余りをインデックスとして使用
 */
const CATEGORY_COLORS: string[] = [
  'bg-category-transport', // 交通費
  'bg-category-meals', // 飲食費
  'bg-category-communication', // 通信費
  'bg-category-supplies', // 消耗品費
  'bg-category-entertainment', // 接待交際費
  'bg-category-other', // その他
];

/**
 * カテゴリーストアクラス
 */
class CategoryStore {
  /** カテゴリー一覧 */
  categories = $state<Category[]>([]);

  /** 読み込み中フラグ */
  isLoading = $state(false);

  /** エラーメッセージ */
  error = $state<string | null>(null);

  /** 初期化済みフラグ */
  private initialized = false;

  /**
   * カテゴリー一覧を読み込む
   */
  async loadCategories(): Promise<void> {
    // 既にロード中の場合はスキップ
    if (this.isLoading) {
      return;
    }

    this.isLoading = true;
    this.error = null;

    try {
      const result = await getCategories();

      if (result.error) {
        console.error('カテゴリー取得エラー:', result.error);
        this.error = result.error;
      } else if (result.data) {
        this.categories = result.data;
        console.info('カテゴリー一覧を取得しました:', this.categories.length);
      }

      this.initialized = true;
    } catch (err) {
      console.error('カテゴリー取得中に予期せぬエラー:', err);
      this.error =
        err instanceof Error ? err.message : '不明なエラーが発生しました';
      this.initialized = true;
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * 初期化（未初期化の場合のみロード）
   */
  async initialize(): Promise<void> {
    if (!this.initialized) {
      await this.loadCategories();
    }
  }

  /**
   * カテゴリーIDからカテゴリーを取得
   *
   * @param id カテゴリーID
   * @returns カテゴリー情報、見つからない場合はundefined
   */
  getCategoryById(id: number): Category | undefined {
    return this.categories.find((c) => c.id === id);
  }

  /**
   * カテゴリー名からカテゴリーを取得
   *
   * @param name カテゴリー名
   * @returns カテゴリー情報、見つからない場合はundefined
   */
  getCategoryByName(name: string): Category | undefined {
    return this.categories.find((c) => c.name === name);
  }

  /**
   * カテゴリーIDからアイコンを取得
   *
   * @param id カテゴリーID
   * @returns アイコン文字列、見つからない場合はデフォルトアイコン
   */
  getIconById(id: number): string {
    const category = this.getCategoryById(id);
    return category?.icon ?? '📋';
  }

  /**
   * カテゴリー名からアイコンを取得
   *
   * @param name カテゴリー名
   * @returns アイコン文字列、見つからない場合はデフォルトアイコン
   */
  getIconByName(name: string): string {
    const category = this.getCategoryByName(name);
    return category?.icon ?? '📋';
  }

  /**
   * カテゴリーIDからカラークラスを取得
   * カテゴリーIDを配列の長さで割った余りをインデックスとして使用（循環アクセス）
   *
   * @param id カテゴリーID
   * @returns CSSカラークラス
   */
  getColorById(id: number): string {
    const colorIndex = id % CATEGORY_COLORS.length;
    return CATEGORY_COLORS[colorIndex];
  }

  /**
   * カテゴリー名からカラークラスを取得
   *
   * @param name カテゴリー名
   * @returns CSSカラークラス、見つからない場合はデフォルト
   */
  getColorByName(name: string): string {
    const category = this.getCategoryByName(name);
    if (category) {
      return this.getColorById(category.id);
    }
    return CATEGORY_COLORS[0]; // デフォルトは最初の色
  }

  /**
   * ストアをリセット
   */
  reset(): void {
    this.categories = [];
    this.isLoading = false;
    this.error = null;
    this.initialized = false;
  }
}

/** カテゴリーストアのシングルトンインスタンス */
export const categoryStore = new CategoryStore();

/**
 * D1データベースモデルの型定義
 */

/**
 * ユーザー型
 */
export interface User {
  id: string; // nanoId形式（21文字）
  google_id: string; // Google OAuth ID
  email: string; // メールアドレス
  name: string; // ユーザー名
  picture_url: string | null; // プロフィール画像URL
  created_at: string; // RFC3339形式（JST）
  updated_at: string; // RFC3339形式（JST）
}

/**
 * カテゴリ型
 */
export interface Category {
  id: number; // 自動採番ID
  name: string; // カテゴリ名（例：交通費）
  icon: string; // 絵文字アイコン（例：🚗）
  display_order: number; // 表示順序
  is_active: boolean; // 有効/無効フラグ
  created_at: string; // RFC3339形式（JST）
  updated_at: string; // RFC3339形式（JST）
}

/**
 * 経費型
 */
export interface Expense {
  id: number; // 自動採番ID
  user_id: string; // ユーザーID（nanoId形式）
  date: string; // YYYY-MM-DD形式
  amount: number; // 金額
  category: string; // カテゴリ（後方互換性のため残す）
  category_id: number | null; // カテゴリID（categoriesテーブルへの外部キー）
  description: string | null; // 説明（オプション）
  receipt_url: string | null; // 領収書URL（HTTPS）
  created_at: string; // RFC3339形式（JST）
  updated_at: string; // RFC3339形式（JST）
}

/**
 * サブスクリプション型
 */
export interface Subscription {
  id: number; // 自動採番ID
  user_id: string; // ユーザーID（nanoId形式）
  name: string; // サービス名
  amount: number; // 金額
  billing_cycle: "monthly" | "annual"; // 請求サイクル
  start_date: string; // YYYY-MM-DD形式
  category: string; // カテゴリ（後方互換性のため残す）
  category_id: number | null; // カテゴリID（categoriesテーブルへの外部キー）
  is_active: boolean; // 有効/無効（0=無効, 1=有効）
  receipt_path: string | null; // 領収書パス
  created_at: string; // RFC3339形式（JST）
  updated_at: string; // RFC3339形式（JST）
}

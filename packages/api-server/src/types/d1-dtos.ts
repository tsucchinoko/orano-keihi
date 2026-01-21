/**
 * D1データベース用のDTO（Data Transfer Object）型定義
 */

/**
 * Google OAuth ユーザー情報
 */
export interface GoogleUser {
  google_id: string; // Google OAuth ID
  email: string; // メールアドレス
  name: string; // ユーザー名
  picture_url?: string; // プロフィール画像URL（オプション）
}

/**
 * ユーザー更新DTO
 */
export interface UpdateUserDto {
  email?: string; // メールアドレス
  name?: string; // ユーザー名
  picture_url?: string; // プロフィール画像URL
}

/**
 * 経費作成DTO
 */
export interface CreateExpenseDto {
  date: string; // YYYY-MM-DD形式
  amount: number; // 金額
  category: string; // カテゴリ
  description?: string; // 説明（オプション）
}

/**
 * 経費更新DTO
 */
export interface UpdateExpenseDto {
  date?: string; // YYYY-MM-DD形式
  amount?: number; // 金額
  category?: string; // カテゴリ
  description?: string; // 説明
  receipt_url?: string; // 領収書URL（HTTPS）
}

/**
 * サブスクリプション作成DTO
 */
export interface CreateSubscriptionDto {
  name: string; // サービス名
  amount: number; // 金額
  billing_cycle: "monthly" | "annual"; // 請求サイクル
  start_date: string; // YYYY-MM-DD形式
  category: string; // カテゴリ
}

/**
 * サブスクリプション更新DTO
 */
export interface UpdateSubscriptionDto {
  name?: string; // サービス名
  amount?: number; // 金額
  billing_cycle?: "monthly" | "annual"; // 請求サイクル
  start_date?: string; // YYYY-MM-DD形式
  category?: string; // カテゴリ
  receipt_path?: string; // 領収書パス
}

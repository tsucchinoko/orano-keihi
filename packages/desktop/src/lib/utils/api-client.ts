/**
 * APIサーバークライアント
 * サブスクリプション関連のAPI呼び出しを提供
 */

import type {
  Subscription,
  CreateSubscriptionDto,
  UpdateSubscriptionDto,
  SubscriptionListResponse,
  MonthlyTotalResponse,
} from '../types';
import { getAuthToken } from './tauri';

// APIサーバーのベースURL（環境変数から取得）
const API_BASE_URL =
  import.meta.env.VITE_API_SERVER_URL || 'http://localhost:3000';

/**
 * APIリクエストのベース設定
 */
interface ApiRequestOptions {
  method?: string;
  headers?: Record<string, string>;
  body?: any;
}

/**
 * APIリクエストを実行する共通関数
 */
async function apiRequest<T>(
  endpoint: string,
  options: ApiRequestOptions = {}
): Promise<T> {
  const { method = 'GET', headers = {}, body } = options;

  // 認証トークンを取得
  const authToken = getAuthToken();

  const requestHeaders: Record<string, string> = {
    'Content-Type': 'application/json',
    ...headers,
  };

  // 認証トークンがある場合は追加
  if (authToken) {
    requestHeaders['Authorization'] = `Bearer ${authToken}`;
  }

  const requestOptions: RequestInit = {
    method,
    headers: requestHeaders,
  };

  // ボディがある場合は追加
  if (body && method !== 'GET') {
    requestOptions.body = JSON.stringify(body);
  }

  try {
    const response = await fetch(`${API_BASE_URL}${endpoint}`, requestOptions);

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        errorData.error?.message ||
          errorData.message ||
          `HTTP ${response.status}: ${response.statusText}`
      );
    }

    return await response.json();
  } catch (error) {
    console.error('API request failed:', { endpoint, method, error });
    throw error;
  }
}

/**
 * サブスクリプション一覧を取得する
 */
export async function fetchSubscriptions(
  activeOnly: boolean = false
): Promise<SubscriptionListResponse> {
  const queryParam = activeOnly ? '?activeOnly=true' : '';
  return apiRequest<SubscriptionListResponse>(
    `/api/v1/subscriptions${queryParam}`
  );
}

/**
 * サブスクリプションを作成する
 */
export async function createSubscriptionApi(
  dto: CreateSubscriptionDto
): Promise<Subscription> {
  return apiRequest<Subscription>('/api/v1/subscriptions', {
    method: 'POST',
    body: dto,
  });
}

/**
 * サブスクリプションを更新する
 */
export async function updateSubscriptionApi(
  id: number,
  dto: UpdateSubscriptionDto
): Promise<Subscription> {
  return apiRequest<Subscription>(`/api/v1/subscriptions/${id}`, {
    method: 'PUT',
    body: dto,
  });
}

/**
 * サブスクリプションのアクティブ状態を切り替える
 */
export async function toggleSubscriptionStatusApi(
  id: number
): Promise<Subscription> {
  return apiRequest<Subscription>(`/api/v1/subscriptions/${id}/toggle`, {
    method: 'PATCH',
  });
}

/**
 * サブスクリプションを削除する
 */
export async function deleteSubscriptionApi(id: number): Promise<void> {
  await apiRequest<{ success: boolean }>(`/api/v1/subscriptions/${id}`, {
    method: 'DELETE',
  });
}

/**
 * 月額サブスクリプション合計を取得する
 */
export async function fetchMonthlySubscriptionTotal(): Promise<MonthlyTotalResponse> {
  return apiRequest<MonthlyTotalResponse>(
    '/api/v1/subscriptions/monthly-total'
  );
}

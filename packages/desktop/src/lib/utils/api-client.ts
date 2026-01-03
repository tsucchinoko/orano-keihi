/**
 * APIサーバークライアント
 * Tauriバックエンド経由でサブスクリプション関連のAPI呼び出しを提供
 */

import { invoke } from '@tauri-apps/api/core';
import type {
  Subscription,
  CreateSubscriptionDto,
  UpdateSubscriptionDto,
  SubscriptionListResponse,
  MonthlyTotalResponse,
} from '../types';
import { getAuthToken } from './tauri';

/**
 * サブスクリプション一覧を取得する（Tauriバックエンド経由）
 */
export async function fetchSubscriptions(
  activeOnly: boolean = false
): Promise<SubscriptionListResponse> {
  const authToken = getAuthToken();

  return await invoke('fetch_subscriptions_via_api', {
    activeOnly,
    sessionToken: authToken,
  });
}

/**
 * サブスクリプションを作成する（Tauriバックエンド経由）
 */
export async function createSubscriptionApi(
  dto: CreateSubscriptionDto
): Promise<Subscription> {
  const authToken = getAuthToken();

  return await invoke('create_subscription_via_api', {
    dto,
    sessionToken: authToken,
  });
}

/**
 * サブスクリプションを更新する（Tauriバックエンド経由）
 */
export async function updateSubscriptionApi(
  id: number,
  dto: UpdateSubscriptionDto
): Promise<Subscription> {
  const authToken = getAuthToken();

  return await invoke('update_subscription_via_api', {
    id,
    dto,
    sessionToken: authToken,
  });
}

/**
 * サブスクリプションのアクティブ状態を切り替える（Tauriバックエンド経由）
 */
export async function toggleSubscriptionStatusApi(
  id: number
): Promise<Subscription> {
  const authToken = getAuthToken();

  return await invoke('toggle_subscription_status_via_api', {
    id,
    sessionToken: authToken,
  });
}

/**
 * サブスクリプションを削除する（Tauriバックエンド経由）
 */
export async function deleteSubscriptionApi(id: number): Promise<void> {
  const authToken = getAuthToken();

  await invoke('delete_subscription_via_api', {
    id,
    sessionToken: authToken,
  });
}

/**
 * 月額サブスクリプション合計を取得する（Tauriバックエンド経由）
 */
export async function fetchMonthlySubscriptionTotal(): Promise<MonthlyTotalResponse> {
  const authToken = getAuthToken();

  return await invoke('fetch_monthly_subscription_total_via_api', {
    sessionToken: authToken,
  });
}

import { requireGuest } from '$lib/utils/auth-guard';

// ログインページは未認証ユーザーのみアクセス可能
export const load = requireGuest();

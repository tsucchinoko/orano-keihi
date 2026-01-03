import { requireAuth } from '$lib/utils/auth-guard';

// 認証が必要なページとして設定
export const load = requireAuth();

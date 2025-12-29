import { requireAuth } from "$lib/utils/auth-guard";

// サブスクリプションページは認証が必要
export const load = requireAuth();

import { requireAuth } from "$lib/utils/auth-guard";

// メインページは認証が必要
export const load = requireAuth();

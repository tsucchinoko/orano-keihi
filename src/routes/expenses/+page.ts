import { requireAuth } from "$lib/utils/auth-guard";

// 経費ページは認証が必要
export const load = requireAuth();

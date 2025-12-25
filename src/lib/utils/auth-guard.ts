import { redirect } from "@sveltejs/kit";
import type { Load } from "@sveltejs/kit";
import { authStore } from "$lib/stores";

/**
 * 認証が必要なページを保護するガード関数
 *
 * @param load - 元のload関数
 * @returns 認証チェック付きのload関数
 */
export function requireAuth(load?: Load): Load {
	return async (event) => {
		// 認証状態を確認
		await authStore.checkSession();

		// 未認証の場合はログインページにリダイレクト
		if (authStore.requiresAuth()) {
			throw redirect(302, "/login");
		}

		// 認証済みの場合は元のload関数を実行
		if (load) {
			return await load(event);
		}

		// load関数が指定されていない場合は空のオブジェクトを返す
		return {};
	};
}

/**
 * 認証済みユーザーのみアクセス可能なページ用のガード
 * ログイン済みの場合はメインページにリダイレクト
 *
 * @param load - 元のload関数
 * @returns 認証チェック付きのload関数
 */
export function requireGuest(load?: Load): Load {
	return async (event) => {
		// 認証状態を確認
		await authStore.checkSession();

		// 認証済みの場合はメインページにリダイレクト
		if (!authStore.requiresAuth()) {
			throw redirect(302, "/");
		}

		// 未認証の場合は元のload関数を実行
		if (load) {
			return await load(event);
		}

		// load関数が指定されていない場合は空のオブジェクトを返す
		return {};
	};
}

/**
 * 認証状態に関係なくアクセス可能だが、認証状態を確認するガード
 *
 * @param load - 元のload関数
 * @returns 認証チェック付きのload関数
 */
export function checkAuth(load?: Load): Load {
	return async (event) => {
		// 認証状態を確認（リダイレクトはしない）
		await authStore.checkSession();

		// 元のload関数を実行
		if (load) {
			return await load(event);
		}

		// load関数が指定されていない場合は空のオブジェクトを返す
		return {};
	};
}

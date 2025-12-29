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
		try {
			console.log("認証ガード: 認証チェックを開始します");

			// 認証状態を確認（初期化も含む）
			await authStore.initialize();
			await authStore.checkSession();

			// 未認証の場合はログインページにリダイレクト
			if (authStore.requiresAuth()) {
				console.log(
					"認証ガード: 未認証のため、ログインページにリダイレクトします",
				);
				throw redirect(302, "/login");
			}

			console.log("認証ガード: 認証済みユーザーのアクセスを許可します");

			// 認証済みの場合は元のload関数を実行
			if (load) {
				return await load(event);
			}

			// load関数が指定されていない場合は空のオブジェクトを返す
			return {};
		} catch (error) {
			// redirectエラーは再スローする
			if (
				error &&
				typeof error === "object" &&
				"status" in error &&
				error.status === 302
			) {
				throw error;
			}

			// その他のエラーはログに記録してログインページにリダイレクト
			console.error("認証ガードでエラーが発生しました:", error);
			throw redirect(302, "/login");
		}
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
		try {
			console.log("ゲストガード: 認証チェックを開始します");

			// 認証状態を確認（初期化も含む）
			await authStore.initialize();
			await authStore.checkSession();

			// 認証済みの場合はメインページにリダイレクト
			if (!authStore.requiresAuth()) {
				console.log(
					"ゲストガード: 認証済みユーザーのため、メインページにリダイレクトします",
				);
				throw redirect(302, "/");
			}

			console.log("ゲストガード: 未認証ユーザーのアクセスを許可します");

			// 未認証の場合は元のload関数を実行
			if (load) {
				return await load(event);
			}

			// load関数が指定されていない場合は空のオブジェクトを返す
			return {};
		} catch (error) {
			// redirectエラーは再スローする
			if (
				error &&
				typeof error === "object" &&
				"status" in error &&
				error.status === 302
			) {
				throw error;
			}

			// その他のエラーはログに記録して続行
			console.error("ゲストガードでエラーが発生しました:", error);

			// エラーが発生した場合は元のload関数を実行
			if (load) {
				return await load(event);
			}
			return {};
		}
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

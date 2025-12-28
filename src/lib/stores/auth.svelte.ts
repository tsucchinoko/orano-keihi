import type { User, AuthState } from "../types";
import {
	startOAuthFlow,
	handleAuthCallback,
	validateSession,
	logout as logoutCommand,
} from "../utils/tauri";
import { toastStore } from "./toast.svelte";

/**
 * 認証状態管理ストア
 * Svelte 5のrunesを使用したリアクティブな認証状態管理
 */
class AuthStore {
	// ユーザー情報
	user = $state<User | null>(null);

	// 認証状態
	isAuthenticated = $state<boolean>(false);

	// ローディング状態
	isLoading = $state<boolean>(false);

	// エラーメッセージ
	error = $state<string | null>(null);

	// セッショントークン（ローカルストレージに保存）
	private sessionToken = $state<string | null>(null);

	// 初期化フラグ
	private initialized = $state<boolean>(false);

	// セッショントークンのローカルストレージキー
	private readonly SESSION_TOKEN_KEY = "auth_session_token";

	/**
	 * 認証状態の初期化
	 * アプリケーション起動時に呼び出される
	 */
	async initialize(): Promise<void> {
		// 既に初期化済みの場合はスキップ
		if (this.initialized) {
			console.log("認証ストアは既に初期化済みです");
			return;
		}

		console.log("認証ストアの初期化を開始します");
		this.isLoading = true;
		this.error = null;

		try {
			// ローカルストレージからセッショントークンを取得
			const storedToken = localStorage.getItem(this.SESSION_TOKEN_KEY);
			console.log(
				"保存されたセッショントークン:",
				storedToken ? "存在" : "なし",
			);

			if (storedToken) {
				this.sessionToken = storedToken;
				// セッションを検証
				await this.checkSession();
			} else {
				// セッショントークンがない場合は未認証状態
				console.log("セッショントークンがないため、未認証状態に設定します");
				this.setUnauthenticatedState();
			}

			this.initialized = true;
			console.log("認証ストアの初期化が完了しました");
		} catch (err) {
			console.error("認証状態の初期化エラー:", err);
			this.error = `認証状態の初期化に失敗しました: ${err}`;
			this.setUnauthenticatedState();
			this.initialized = true; // エラーでも初期化完了とする
		} finally {
			this.isLoading = false;
		}
	}

	/**
	 * Googleログインを開始する
	 */
	async login(): Promise<void> {
		this.isLoading = true;
		this.error = null;

		try {
			// OAuth認証フローを開始
			const result = await startOAuthFlow();

			if (result.error) {
				this.error = result.error;
				toastStore.error(`ログインに失敗しました: ${result.error}`);
				return;
			}

			if (result.data) {
				// 認証URLをブラウザで開く
				const { auth_url, code_verifier, state } = result.data;

				// PKCE検証子と状態パラメータを一時保存
				sessionStorage.setItem("oauth_code_verifier", code_verifier);
				sessionStorage.setItem("oauth_state", state);

				// 外部ブラウザで認証URLを開く
				window.open(auth_url, "_blank");

				toastStore.info("ブラウザでGoogleログインを完了してください");
			}
		} catch (err) {
			console.error("ログイン開始エラー:", err);
			this.error = `ログインの開始に失敗しました: ${err}`;
			toastStore.error(this.error);
		} finally {
			this.isLoading = false;
		}
	}

	/**
	 * 認証コールバックを処理する
	 * 認証完了後にブラウザから呼び出される
	 */
	async handleCallback(code: string, state: string): Promise<boolean> {
		this.isLoading = true;
		this.error = null;

		try {
			// セッションストレージから保存された値を取得
			const storedCodeVerifier = sessionStorage.getItem("oauth_code_verifier");
			const storedState = sessionStorage.getItem("oauth_state");

			if (!storedCodeVerifier || !storedState) {
				this.error = "認証情報が見つかりません。再度ログインしてください。";
				toastStore.error(this.error);
				return false;
			}

			// 状態パラメータを検証
			if (state !== storedState) {
				this.error =
					"認証状態が一致しません。セキュリティ上の理由でログインを中止しました。";
				toastStore.error(this.error);
				return false;
			}

			// 認証コールバックを処理
			const result = await handleAuthCallback({
				code,
				state,
				code_verifier: storedCodeVerifier,
			});

			if (result.error) {
				this.error = result.error;
				toastStore.error(`認証に失敗しました: ${result.error}`);
				return false;
			}

			if (result.data) {
				const { user, session_token } = result.data;

				// 認証状態を更新
				this.user = user;
				this.isAuthenticated = true;
				this.sessionToken = session_token;

				// セッショントークンをローカルストレージに保存
				localStorage.setItem(this.SESSION_TOKEN_KEY, session_token);

				// 一時保存された認証情報をクリア
				sessionStorage.removeItem("oauth_code_verifier");
				sessionStorage.removeItem("oauth_state");

				toastStore.success(`${user.name}さん、ログインしました`);
				return true;
			}

			return false;
		} catch (err) {
			console.error("認証コールバック処理エラー:", err);
			this.error = `認証処理に失敗しました: ${err}`;
			toastStore.error(this.error);
			return false;
		} finally {
			this.isLoading = false;
		}
	}

	/**
	 * ログアウト処理
	 */
	async logout(): Promise<void> {
		this.isLoading = true;
		this.error = null;

		try {
			if (this.sessionToken) {
				// バックエンドでセッションを無効化
				const result = await logoutCommand(this.sessionToken);

				if (result.error) {
					console.warn("サーバー側ログアウトエラー:", result.error);
					// サーバー側のエラーでもクライアント側のログアウトは続行
				}
			}

			// クライアント側の認証状態をクリア
			this.setUnauthenticatedState();

			// ローカルストレージからセッショントークンを削除
			localStorage.removeItem(this.SESSION_TOKEN_KEY);

			// セッションストレージもクリア
			sessionStorage.removeItem("oauth_code_verifier");
			sessionStorage.removeItem("oauth_state");

			toastStore.success("ログアウトしました");
		} catch (err) {
			console.error("ログアウトエラー:", err);
			// エラーが発生してもクライアント側の状態はクリア
			this.setUnauthenticatedState();
			localStorage.removeItem(this.SESSION_TOKEN_KEY);

			this.error = `ログアウト処理でエラーが発生しましたが、ローカルの認証状態はクリアされました: ${err}`;
			toastStore.warning("ログアウトしました（一部エラーが発生）");
		} finally {
			this.isLoading = false;
		}
	}

	/**
	 * セッション状態を確認する
	 * アプリケーション起動時や定期的な確認で使用
	 */
	async checkSession(): Promise<void> {
		console.log("セッション状態を確認します");

		if (!this.sessionToken) {
			console.log("セッショントークンがないため、未認証状態に設定します");
			this.setUnauthenticatedState();
			return;
		}

		try {
			console.log("セッション検証を実行します");
			const result = await validateSession(this.sessionToken);

			if (result.error) {
				console.warn("セッション検証エラー:", result.error);
				this.setUnauthenticatedState();
				localStorage.removeItem(this.SESSION_TOKEN_KEY);
				return;
			}

			if (result.data?.is_authenticated) {
				// セッションが有効な場合
				console.log("セッションが有効です。認証済み状態に設定します");
				this.user = result.data.user;
				this.isAuthenticated = true;
			} else {
				// セッションが無効な場合
				console.log("セッションが無効です。未認証状態に設定します");
				this.setUnauthenticatedState();
				localStorage.removeItem(this.SESSION_TOKEN_KEY);
			}
		} catch (err) {
			console.error("セッション確認エラー:", err);
			this.setUnauthenticatedState();
			localStorage.removeItem(this.SESSION_TOKEN_KEY);
		}
	}

	/**
	 * 現在のセッショントークンを取得する
	 * APIリクエスト時に使用
	 */
	getSessionToken(): string | null {
		return this.sessionToken;
	}

	/**
	 * 認証が必要かどうかを確認する
	 */
	requiresAuth(): boolean {
		const result = !this.isAuthenticated;
		console.log(
			`認証が必要かどうか: ${result} (isAuthenticated: ${this.isAuthenticated})`,
		);
		return result;
	}

	/**
	 * エラーをクリアする
	 */
	clearError(): void {
		this.error = null;
	}

	/**
	 * 未認証状態に設定する（プライベートメソッド）
	 */
	private setUnauthenticatedState(): void {
		console.log("未認証状態に設定します");
		this.user = null;
		this.isAuthenticated = false;
		this.sessionToken = null;
	}

	/**
	 * 認証状態を監視するためのリアクティブな値
	 */
	get authState(): AuthState {
		return {
			user: this.user,
			is_authenticated: this.isAuthenticated,
			is_loading: this.isLoading,
		};
	}
}

// シングルトンインスタンスをエクスポート
export const authStore = new AuthStore();

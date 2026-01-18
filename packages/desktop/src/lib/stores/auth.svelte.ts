import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import type { User, AuthState } from '../types';
import {
  startOAuthFlow,
  waitForAuthCompletion,
  validateSession,
  logout as logoutCommand,
} from '../utils/tauri';
import { toastStore } from './toast.svelte';

/**
 * セキュアストレージに保存される認証情報の型
 */
interface StoredAuthInfo {
  session_token: string;
  user_id: string;
  last_login: string;
}

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

  // セッショントークン（セキュアストレージに保存）
  private sessionToken = $state<string | null>(null);

  // 初期化フラグ
  private initialized = $state<boolean>(false);

  /**
   * 認証状態の初期化
   * アプリケーション起動時に呼び出される
   */
  async initialize(): Promise<void> {
    // 既に初期化済みの場合はスキップ
    if (this.initialized) {
      console.info('認証ストアは既に初期化済みです');
      return;
    }

    console.info('認証ストアの初期化を開始します');
    this.isLoading = true;
    this.error = null;

    try {
      // セキュアストレージから認証情報を取得
      const storedAuthInfo = await invoke<StoredAuthInfo | null>(
        'get_stored_auth_info'
      );
      console.info('保存された認証情報:', storedAuthInfo ? '存在' : 'なし');

      if (storedAuthInfo) {
        this.sessionToken = storedAuthInfo.session_token;
        // セッションを検証
        await this.checkSession();
      } else {
        // 認証情報がない場合は未認証状態
        console.info('認証情報がないため、未認証状態に設定します');
        this.setUnauthenticatedState();
      }

      this.initialized = true;
      console.info('認証ストアの初期化が完了しました');
    } catch (err) {
      console.error('認証状態の初期化エラー:', err);
      this.error = `認証状態の初期化に失敗しました: ${String(err)}`;
      this.setUnauthenticatedState();
      this.initialized = true; // エラーでも初期化完了とする
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Googleログインを開始する（ループバック方式）
   */
  async login(): Promise<void> {
    console.info('🔐 ログイン処理を開始します');
    this.isLoading = true;
    this.error = null;

    try {
      console.info('🔐 OAuth認証フロー開始します');
      // OAuth認証フローを開始
      const startResult = await startOAuthFlow();
      console.info('🔐 OAuth認証フロー開始結果:', startResult);

      if (startResult.error) {
        console.error('🔐 OAuth認証フロー開始エラー:', startResult.error);
        this.error = startResult.error;
        toastStore.error(`ログインに失敗しました: ${startResult.error}`);
        return;
      }

      if (startResult.data) {
        const { auth_url, loopback_port } = startResult.data;
        console.info('🔐 認証URL:', auth_url);
        console.info('🔐 ループバックポート:', loopback_port);

        // 外部ブラウザで認証URLを開く
        console.info('🔐 外部ブラウザで認証URLを開きます');
        try {
          // Tauri shell pluginを使用
          await open(auth_url);
          console.info('🔐 Tauri shell pluginで認証URLを開きました');

          // 認証完了を待機
          console.info('🔐 認証完了を待機します');
          toastStore.info(
            'ブラウザでGoogleログインを完了してください。認証完了まで待機中...'
          );

          console.info('🔐 waitForAuthCompletion()を呼び出す直前');
          const authResult = await waitForAuthCompletion();
          console.info('🔐 waitForAuthCompletion()が完了しました');
          console.info('🔐 認証完了結果:', authResult);

          if (authResult.error) {
            console.error('🔐 認証完了エラー:', authResult.error);
            this.error = authResult.error;
            toastStore.error(`認証に失敗しました: ${authResult.error}`);
            return;
          }

          if (authResult.data) {
            const { user, session_token } = authResult.data;
            console.info('🔐 認証データを受け取りました:', {
              user,
              session_token,
            });

            // 認証状態を更新
            console.info('🔐 認証状態を更新します...');
            this.user = user;
            this.sessionToken = session_token;

            // セッショントークンはバックエンドのセキュアストレージに既に保存済み
            console.info(
              '🔐 セキュアストレージにセッショントークンが保存されました'
            );

            // 最後に認証状態をtrueに設定（リアクティブな更新をトリガー）
            this.isAuthenticated = true;
            console.info('🔐 isAuthenticated =', this.isAuthenticated);

            toastStore.success(`${user.name}さん、ログインしました`);
            console.info('🔐 ログイン処理が正常に完了しました');
          } else {
            console.warn('🔐 authResult.dataが存在しません');
          }
        } catch (openError) {
          console.warn('🔐 外部ブラウザでの認証URLオープンに失敗:', openError);
          // URLをクリップボードにコピーして、ユーザーに手動で開いてもらう
          try {
            await navigator.clipboard.writeText(auth_url);
            const userConfirmed = confirm(
              `外部ブラウザを自動で開けませんでした。\n\n以下のURLを手動でブラウザにコピーして開いてください：\n\n${auth_url}\n\nOKを押すとURLがクリップボードにコピーされます。`
            );
            if (userConfirmed) {
              toastStore.info(
                '認証URLをクリップボードにコピーしました。ブラウザに貼り付けて開いてください。'
              );

              // 手動でブラウザを開いた場合も認証完了を待機
              console.info('🔐 手動ブラウザオープン後、認証完了を待機します');
              const authResult = await waitForAuthCompletion();

              if (authResult.error) {
                console.error('🔐 認証完了エラー:', authResult.error);
                this.error = authResult.error;
                toastStore.error(`認証に失敗しました: ${authResult.error}`);
                return;
              }

              if (authResult.data) {
                const { user, session_token } = authResult.data;
                this.user = user;
                this.sessionToken = session_token;

                // セッショントークンはバックエンドのセキュアストレージに既に保存済み

                // 最後に認証状態をtrueに設定
                this.isAuthenticated = true;
                toastStore.success(`${user.name}さん、ログインしました`);
              }
            }
          } catch (clipboardError) {
            console.error('🔐 クリップボードへのコピーに失敗:', clipboardError);
            this.error =
              '外部ブラウザを開けませんでした。手動でブラウザを開いてください。';
            toastStore.error(this.error);
          }
        }
      }
    } catch (err) {
      console.error('🔐 ログイン開始エラー:', err);
      this.error = `ログインの開始に失敗しました: ${String(err)}`;
      toastStore.error(this.error);
    } finally {
      this.isLoading = false;
      console.info('🔐 ログイン処理が完了しました');
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
        // バックエンドでセッションを無効化（セキュアストレージからも削除される）
        const result = await logoutCommand(this.sessionToken);

        if (result.error) {
          console.warn('サーバー側ログアウトエラー:', result.error);
          // サーバー側のエラーでもクライアント側のログアウトは続行
        }
      }

      // クライアント側の認証状態をクリア
      this.setUnauthenticatedState();

      toastStore.success('ログアウトしました');
    } catch (err) {
      console.error('ログアウトエラー:', err);
      // エラーが発生してもクライアント側の状態はクリア
      this.setUnauthenticatedState();

      this.error = `ログアウト処理でエラーが発生しましたが、ローカルの認証状態はクリアされました: ${String(err)}`;
      toastStore.warning('ログアウトしました（一部エラーが発生）');
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * セッション状態を確認する
   * アプリケーション起動時や定期的な確認で使用
   */
  async checkSession(): Promise<void> {
    console.info('セッション状態を確認します');

    // セキュアストレージから最新の認証情報を取得
    try {
      const storedAuthInfo = await invoke<StoredAuthInfo | null>(
        'get_stored_auth_info'
      );
      if (storedAuthInfo) {
        this.sessionToken = storedAuthInfo.session_token;
        console.info('セキュアストレージからセッショントークンを復元しました');
      }
    } catch (err) {
      console.warn('セキュアストレージからの認証情報取得エラー:', err);
    }

    if (!this.sessionToken) {
      console.info('セッショントークンがないため、未認証状態に設定します');
      this.setUnauthenticatedState();
      return;
    }

    try {
      console.info('セッション検証を実行します');
      const result = await validateSession(this.sessionToken);

      if (result.error) {
        console.warn('セッション検証エラー:', result.error);
        this.setUnauthenticatedState();
        return;
      }

      if (result.data?.is_authenticated) {
        // セッションが有効な場合
        console.info('セッションが有効です。認証済み状態に設定します');
        this.user = result.data.user;
        this.isAuthenticated = true;
      } else {
        // セッションが無効な場合
        console.info('セッションが無効です。未認証状態に設定します');
        this.setUnauthenticatedState();
      }
    } catch (err) {
      console.error('セッション確認エラー:', err);
      this.setUnauthenticatedState();
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
    console.info(
      `認証が必要かどうか: ${result} (isAuthenticated: ${this.isAuthenticated})`
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
    console.info('未認証状態に設定します');
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

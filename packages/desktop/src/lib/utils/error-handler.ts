// フロントエンド用統一エラーハンドリング

import type {
  UserFriendlyError,
  OperationResult,
  RetryConfig,
} from '$lib/types';

/**
 * エラーハンドリングユーティリティクラス
 */
export class ErrorHandler {
  /**
   * Tauriコマンドエラーをユーザーフレンドリーなエラーに変換
   */
  static handleTauriError(error: string): UserFriendlyError {
    // APIサーバー関連のエラー
    if (error.includes('APIサーバーが一時的に利用できません')) {
      return {
        title: 'APIサーバー接続エラー',
        message: error,
        canRetry: true,
        severity: 'warning',
        actions: [
          {
            label: '同期を試行',
            action: async () => {
              try {
                const { syncFallbackFiles } =
                  await import('$lib/types/api-client');
                const result = await syncFallbackFiles();
                if (result.successful_syncs > 0) {
                  alert(`${result.successful_syncs}個のファイルを同期しました`);
                } else {
                  alert('同期が必要なファイルはありません');
                }
              } catch (e) {
                alert(`同期に失敗しました: ${e}`);
              }
            },
            primary: true,
          },
          {
            label: '後で再試行',
            action: () => {
              console.info('後で再試行してください');
            },
          },
        ],
      };
    }

    if (error.includes('APIサーバーへの接続に失敗')) {
      return {
        title: 'APIサーバー接続失敗',
        message:
          'APIサーバーに接続できません。サーバーが起動していることを確認してください。',
        canRetry: true,
        severity: 'error',
        actions: [
          {
            label: 'ヘルスチェック',
            action: async () => {
              try {
                const { checkApiServerHealthDetailed } =
                  await import('$lib/types/api-client');
                const result = await checkApiServerHealthDetailed();
                if (result.is_healthy) {
                  alert('APIサーバーは正常に動作しています');
                } else {
                  alert(
                    `APIサーバーエラー: ${result.error_message || '不明なエラー'}`
                  );
                }
              } catch (e) {
                alert(`ヘルスチェックに失敗しました: ${e}`);
              }
            },
            primary: true,
          },
          {
            label: '再試行',
            action: () => window.location.reload(),
          },
        ],
      };
    }

    // 認証関連のエラー
    if (error.includes('認証に失敗') || error.includes('401')) {
      return {
        title: '認証エラー',
        message: '認証に失敗しました。再度ログインしてください。',
        canRetry: false,
        severity: 'critical',
        actions: [
          {
            label: '再ログイン',
            action: () => {
              // 認証画面にリダイレクト
              window.location.href = '/auth/login';
            },
            primary: true,
          },
        ],
      };
    }

    // ファイル関連のエラー
    if (error.includes('ネットワーク') || error.includes('接続')) {
      return {
        title: 'ネットワークエラー',
        message: error,
        canRetry: true,
        severity: 'warning',
        actions: [
          {
            label: '再試行',
            action: () => window.location.reload(),
            primary: true,
          },
          {
            label: 'ネットワーク設定を確認',
            action: () => {
              console.info('ネットワーク設定の確認が必要です');
            },
          },
        ],
      };
    }

    if (
      error.includes('ファイル形式') ||
      error.includes('サポートされていない')
    ) {
      return {
        title: 'ファイル形式エラー',
        message: error,
        canRetry: false,
        severity: 'error',
        actions: [
          {
            label: '対応形式を確認',
            action: () => {
              alert('対応形式: PNG, JPG, JPEG, PDF（最大10MB）');
            },
            primary: true,
          },
        ],
      };
    }

    if (error.includes('ファイルサイズ') || error.includes('10MB')) {
      return {
        title: 'ファイルサイズエラー',
        message: error,
        canRetry: false,
        severity: 'error',
        actions: [
          {
            label: 'ファイルを圧縮',
            action: () => {
              alert('ファイルサイズを10MB以下に圧縮してください');
            },
            primary: true,
          },
        ],
      };
    }

    if (error.includes('権限') || error.includes('403')) {
      return {
        title: '権限エラー',
        message: 'この操作を実行する権限がありません。',
        canRetry: false,
        severity: 'critical',
        actions: [
          {
            label: '管理者に連絡',
            action: () => {
              console.info('管理者への連絡が必要です');
            },
            primary: true,
          },
        ],
      };
    }

    if (error.includes('データベース')) {
      return {
        title: 'データベースエラー',
        message: error,
        canRetry: true,
        severity: 'error',
        actions: [
          {
            label: '再試行',
            action: () => window.location.reload(),
            primary: true,
          },
        ],
      };
    }

    // デフォルトのエラー処理
    return {
      title: 'エラーが発生しました',
      message: error,
      canRetry: true,
      severity: 'error',
      actions: [
        {
          label: '再試行',
          action: () => window.location.reload(),
          primary: true,
        },
      ],
    };
  }

  /**
   * 操作を実行し、エラーハンドリングを適用
   */
  static async executeWithErrorHandling<T>(
    operation: () => Promise<T>,
    operationName: string = '操作'
  ): Promise<OperationResult<T>> {
    try {
      const data = await operation();
      return {
        success: true,
        data,
      };
    } catch (error) {
      console.error(`${operationName}中にエラーが発生しました:`, error);

      const errorMessage =
        error instanceof Error ? error.message : String(error);
      const userFriendlyError = ErrorHandler.handleTauriError(errorMessage);

      return {
        success: false,
        error: userFriendlyError,
      };
    }
  }

  /**
   * リトライ機能付きで操作を実行
   */
  static async executeWithRetry<T>(
    operation: () => Promise<T>,
    config: RetryConfig = {
      maxRetries: 3,
      baseDelay: 1000,
      maxDelay: 10000,
      exponentialBackoff: true,
    },
    operationName: string = '操作'
  ): Promise<OperationResult<T>> {
    let lastError: Error | null = null;

    for (let attempt = 0; attempt <= config.maxRetries; attempt++) {
      try {
        const data = await operation();

        if (attempt > 0) {
          console.info(
            `${operationName}が${attempt + 1}回目の試行で成功しました`
          );
        }

        return {
          success: true,
          data,
        };
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));

        if (attempt < config.maxRetries) {
          const delay = config.exponentialBackoff
            ? Math.min(config.baseDelay * 2 ** attempt, config.maxDelay)
            : config.baseDelay;

          console.warn(
            `${operationName}が失敗しました（${attempt + 1}/${config.maxRetries + 1}）。${delay}ms後に再試行します:`,
            error
          );

          await new Promise((resolve) => setTimeout(resolve, delay));
        }
      }
    }

    console.error(
      `${operationName}が最大試行回数（${config.maxRetries + 1}回）で失敗しました:`,
      lastError
    );

    const errorMessage = lastError?.message || '不明なエラー';
    const userFriendlyError = ErrorHandler.handleTauriError(errorMessage);

    return {
      success: false,
      error: userFriendlyError,
    };
  }

  /**
   * ファイルアップロード専用のエラーハンドリング
   */
  static async handleFileUpload(
    uploadFunction: () => Promise<string>,
    fileName: string
  ): Promise<OperationResult<string>> {
    return ErrorHandler.executeWithRetry(
      uploadFunction,
      {
        maxRetries: 2, // ファイルアップロードは2回まで
        baseDelay: 2000,
        maxDelay: 8000,
        exponentialBackoff: true,
      },
      `ファイル「${fileName}」のアップロード`
    );
  }

  /**
   * ファイルダウンロード専用のエラーハンドリング
   */
  static async handleFileDownload(
    downloadFunction: () => Promise<string>,
    fileName: string = '領収書'
  ): Promise<OperationResult<string>> {
    return ErrorHandler.executeWithRetry(
      downloadFunction,
      {
        maxRetries: 3,
        baseDelay: 1000,
        maxDelay: 5000,
        exponentialBackoff: true,
      },
      `${fileName}のダウンロード`
    );
  }

  /**
   * ファイル削除専用のエラーハンドリング
   */
  static async handleFileDelete(
    deleteFunction: () => Promise<boolean>,
    fileName: string = '領収書'
  ): Promise<OperationResult<boolean>> {
    return ErrorHandler.executeWithErrorHandling(
      deleteFunction,
      `${fileName}の削除`
    );
  }

  /**
   * エラーメッセージを表示用に整形
   */
  static formatErrorForDisplay(error: UserFriendlyError): string {
    return `${error.title}: ${error.message}`;
  }

  /**
   * エラーの重要度に基づいてCSSクラスを取得
   */
  static getErrorCssClass(severity: UserFriendlyError['severity']): string {
    switch (severity) {
      case 'info':
        return 'alert-info';
      case 'warning':
        return 'alert-warning';
      case 'error':
        return 'alert-error';
      case 'critical':
        return 'alert-critical';
      default:
        return 'alert-error';
    }
  }

  /**
   * ファイル形式を検証
   */
  static validateFileFormat(file: File): OperationResult<void> {
    const allowedTypes = [
      'image/png',
      'image/jpeg',
      'image/jpg',
      'application/pdf',
    ];
    const maxSize = 10 * 1024 * 1024; // 10MB

    if (!allowedTypes.includes(file.type)) {
      return {
        success: false,
        error: {
          title: 'ファイル形式エラー',
          message:
            'サポートされていないファイル形式です。PNG、JPG、PDFファイルのみアップロード可能です。',
          canRetry: false,
          severity: 'error',
        },
      };
    }

    if (file.size > maxSize) {
      return {
        success: false,
        error: {
          title: 'ファイルサイズエラー',
          message: `ファイルサイズが制限を超えています。最大サイズ: 10MB（現在: ${(file.size / (1024 * 1024)).toFixed(1)}MB）`,
          canRetry: false,
          severity: 'error',
        },
      };
    }

    return { success: true };
  }
}

/**
 * エラー状態管理用のReactiveストア
 */
export function createErrorStore() {
  let errorState = $state({
    hasError: false,
    error: null as UserFriendlyError | null,
    isRetrying: false,
    retryCount: 0,
    maxRetries: 3,
  });

  return {
    get state() {
      return errorState;
    },

    setError(error: UserFriendlyError) {
      errorState.hasError = true;
      errorState.error = error;
      errorState.isRetrying = false;
    },

    clearError() {
      errorState.hasError = false;
      errorState.error = null;
      errorState.isRetrying = false;
      errorState.retryCount = 0;
    },

    startRetry() {
      errorState.isRetrying = true;
      errorState.retryCount++;
    },

    canRetry(): boolean {
      return (
        errorState.error?.canRetry === true &&
        errorState.retryCount < errorState.maxRetries
      );
    },
  };
}

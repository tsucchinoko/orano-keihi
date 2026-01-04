/**
 * サービス層のエクスポート
 */

export { R2Client, createR2Client } from "./r2-client.js";
export { R2WorkerClient, createR2WorkerClient } from "./r2-worker-client.js";
export { R2TestService, createR2TestService } from "./r2-test-service.js";
export { AuthService, createAuthService, AuthError } from "./auth-service.js";
export { FileUploadService, createFileUploadService } from "./file-upload-service.js";
export { SubscriptionService, createSubscriptionService } from "./subscription-service.js";
export {
  TauriSubscriptionService,
  createTauriSubscriptionService,
} from "./tauri-subscription-service.js";
export type { UploadResult, R2ClientInterface } from "./r2-client.js";
export type { R2TestResult } from "./r2-test-service.js";
export type { FileUploadServiceInterface } from "./file-upload-service.js";

// R2Clientクラスをインポート
import { R2Client } from "./r2-client.js";
import { createR2WorkerClient } from "./r2-worker-client.js";

/**
 * 環境に応じたR2クライアントを作成
 * @param config R2設定
 * @param r2Bucket Workers環境でのR2バケットバインディング（オプション）
 * @param accountId CloudflareアカウントID（Workers環境で必要）
 * @returns 適切なR2クライアントインスタンス
 */
export function createEnvironmentAwareR2Client(
  config: import("../types/config.js").R2Config,
  r2Bucket?: R2Bucket,
  accountId?: string,
): import("./r2-client.js").R2ClientInterface {
  // Workers環境の場合（R2バケットバインディングが利用可能）
  if (r2Bucket && typeof r2Bucket.put === "function") {
    // アカウントIDが必要
    if (!accountId) {
      throw new Error("Workers環境ではアカウントIDが必要です");
    }
    // R2WorkerClientを使用
    return createR2WorkerClient(r2Bucket, config.bucketName, accountId);
  }

  // Node.js環境の場合（AWS SDK使用）
  return new R2Client(config);
}

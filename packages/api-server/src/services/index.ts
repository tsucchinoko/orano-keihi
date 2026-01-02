/**
 * サービス層のエクスポート
 */

export { R2Client, createR2Client } from "./r2-client.js";
export { R2WorkerClient, createR2WorkerClient } from "./r2-worker-client.js";
export { R2TestService, createR2TestService } from "./r2-test-service.js";
export { AuthService, createAuthService, AuthError } from "./auth-service.js";
export { FileUploadService, createFileUploadService } from "./file-upload-service.js";
export type { UploadResult, R2ClientInterface } from "./r2-client.js";
export type { R2TestResult } from "./r2-test-service.js";
export type { FileUploadServiceInterface } from "./file-upload-service.js";

/**
 * 環境に応じたR2クライアントを作成
 * @param config R2設定
 * @param r2Bucket Workers環境でのR2バケットバインディング（オプション）
 * @returns 適切なR2クライアントインスタンス
 */
export function createEnvironmentAwareR2Client(
  config: import("../types/config.js").R2Config,
  r2Bucket?: R2Bucket,
): R2ClientInterface {
  // Workers環境の場合（R2バケットバインディングが利用可能）
  if (r2Bucket && typeof r2Bucket.put === "function") {
    return createR2WorkerClient(r2Bucket, config.bucketName, config.accountId);
  }

  // Node.js環境の場合（AWS SDK使用）
  return createR2Client(config);
}

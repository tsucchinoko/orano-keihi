/**
 * Cloudflare R2クライアント
 * AWS S3互換APIを使用してR2ストレージにアクセス
 */

import {
  S3Client,
  PutObjectCommand,
  DeleteObjectCommand,
  GetObjectCommand,
} from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import type { R2Config } from "../types/config.js";
import { logger } from "../utils/logger.js";
import { withR2Retry } from "../utils/retry.js";
import { ErrorCode, createR2Error } from "../utils/error-handler.js";

export interface UploadResult {
  success: boolean;
  fileUrl?: string;
  fileKey: string;
  error?: string;
}

export interface R2ClientInterface {
  putObject(key: string, data: Buffer, contentType: string): Promise<string>;
  deleteObject(key: string): Promise<void>;
  getFile(key: string): Promise<Buffer | null>;
  generatePresignedUrl(key: string, expiresIn: number): Promise<string>;
  testConnection(): Promise<boolean>;
  getConfig(): R2Config;
}

/**
 * R2クライアントクラス
 * AWS SDK for JavaScriptを使用してCloudflare R2にアクセス
 */
export class R2Client implements R2ClientInterface {
  private s3Client: S3Client;
  private bucketName: string;

  constructor(private config: R2Config) {
    // R2エンドポイントの形式を確認・修正
    let endpoint = config.endpoint;
    if (!endpoint.startsWith("https://")) {
      // アカウントIDのみが提供された場合、完全なエンドポイントURLを構築
      endpoint = `https://${config.endpoint}.r2.cloudflarestorage.com`;
    }

    this.s3Client = new S3Client({
      region: config.region,
      endpoint: endpoint,
      credentials: {
        accessKeyId: config.accessKeyId,
        secretAccessKey: config.secretAccessKey,
      },
      // R2固有の設定
      forcePathStyle: false, // R2はvirtual-hosted-style URLsを使用
    });

    this.bucketName = config.bucketName;

    logger.info("R2クライアントを初期化しました", {
      endpoint: endpoint,
      bucketName: this.bucketName,
      region: config.region,
    });
  }

  /**
   * ファイルをR2にアップロード
   * @param key ファイルキー（パス）
   * @param data ファイルデータ
   * @param contentType MIMEタイプ
   * @returns アップロードされたファイルのURL
   */
  async putObject(key: string, data: Buffer, contentType: string): Promise<string> {
    return withR2Retry(async () => {
      try {
        const command = new PutObjectCommand({
          Bucket: this.bucketName,
          Key: key,
          Body: data,
          ContentType: contentType,
          // R2でのパブリックアクセス設定
          ACL: "public-read",
        });

        await this.s3Client.send(command);

        // パブリックURLを生成
        const fileUrl = this.generatePublicUrl(key);

        logger.info("ファイルのアップロードが完了しました", {
          fileKey: key,
          fileUrl: fileUrl,
          contentType: contentType,
          fileSize: data.length,
        });

        return fileUrl;
      } catch (error) {
        logger.error("ファイルのアップロードに失敗しました", {
          fileKey: key,
          error: error instanceof Error ? error.message : String(error),
        });

        throw createR2Error(
          ErrorCode.R2_UPLOAD_ERROR,
          `R2アップロードエラー: ${error instanceof Error ? error.message : String(error)}`,
          true,
        );
      }
    }, `R2アップロード: ${key}`);
  }

  /**
   * ファイルをR2から削除
   * @param key ファイルキー（パス）
   */
  async deleteObject(key: string): Promise<void> {
    return withR2Retry(async () => {
      try {
        const command = new DeleteObjectCommand({
          Bucket: this.bucketName,
          Key: key,
        });

        await this.s3Client.send(command);

        logger.info("ファイルの削除が完了しました", {
          fileKey: key,
        });
      } catch (error) {
        logger.error("ファイルの削除に失敗しました", {
          fileKey: key,
          error: error instanceof Error ? error.message : String(error),
        });

        throw createR2Error(
          ErrorCode.R2_CONNECTION_ERROR,
          `R2削除エラー: ${error instanceof Error ? error.message : String(error)}`,
          true,
        );
      }
    }, `R2削除: ${key}`);
  }

  /**
   * ファイルをR2から取得
   * @param key ファイルキー（パス）
   * @returns ファイルデータ、または見つからない場合はnull
   */
  async getFile(key: string): Promise<Buffer | null> {
    return withR2Retry(async () => {
      try {
        const command = new GetObjectCommand({
          Bucket: this.bucketName,
          Key: key,
        });

        const response = await this.s3Client.send(command);

        if (!response.Body) {
          logger.warn("ファイルが見つかりません", {
            fileKey: key,
          });
          return null;
        }

        // ストリームをBufferに変換
        const chunks: Uint8Array[] = [];
        const reader = response.Body.transformToWebStream().getReader();

        try {
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            chunks.push(value);
          }
        } finally {
          reader.releaseLock();
        }

        // 全チャンクを結合してBufferを作成
        const totalLength = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
        const buffer = new Uint8Array(totalLength);
        let offset = 0;
        for (const chunk of chunks) {
          buffer.set(chunk, offset);
          offset += chunk.length;
        }

        const fileBuffer = Buffer.from(buffer);

        logger.debug("ファイルの取得が完了しました", {
          fileKey: key,
          fileSize: fileBuffer.length,
        });

        return fileBuffer;
      } catch (error: any) {
        // ファイルが存在しない場合
        if (error.name === "NoSuchKey" || error.$metadata?.httpStatusCode === 404) {
          logger.warn("ファイルが見つかりません", {
            fileKey: key,
          });
          return null;
        }

        logger.error("ファイルの取得に失敗しました", {
          fileKey: key,
          error: error instanceof Error ? error.message : String(error),
        });

        throw createR2Error(
          ErrorCode.R2_CONNECTION_ERROR,
          `R2取得エラー: ${error instanceof Error ? error.message : String(error)}`,
          true,
        );
      }
    }, `R2取得: ${key}`);
  }

  /**
   * プリサインドURLを生成
   * @param key ファイルキー（パス）
   * @param expiresIn 有効期限（秒）
   * @returns プリサインドURL
   */
  async generatePresignedUrl(key: string, expiresIn: number = 3600): Promise<string> {
    return withR2Retry(async () => {
      try {
        const command = new GetObjectCommand({
          Bucket: this.bucketName,
          Key: key,
        });

        const signedUrl = await getSignedUrl(this.s3Client, command, {
          expiresIn: expiresIn,
        });

        logger.debug("プリサインドURLを生成しました", {
          fileKey: key,
          expiresIn: expiresIn,
        });

        return signedUrl;
      } catch (error) {
        logger.error("プリサインドURLの生成に失敗しました", {
          fileKey: key,
          error: error instanceof Error ? error.message : String(error),
        });

        throw createR2Error(
          ErrorCode.R2_CONNECTION_ERROR,
          `プリサインドURL生成エラー: ${error instanceof Error ? error.message : String(error)}`,
          true,
        );
      }
    }, `プリサインドURL生成: ${key}`);
  }

  /**
   * R2接続テスト
   * @returns 接続成功の場合true
   */
  async testConnection(): Promise<boolean> {
    try {
      // テスト用の小さなファイルをアップロードして削除
      const testKey = `test-connection-${Date.now()}.txt`;
      const testData = Buffer.from("R2接続テスト", "utf-8");

      // アップロードテスト
      await this.putObject(testKey, testData, "text/plain");

      // 削除テスト
      await this.deleteObject(testKey);

      logger.info("R2接続テストが成功しました");
      return true;
    } catch (error) {
      logger.error("R2接続テストに失敗しました", {
        error: error instanceof Error ? error.message : String(error),
      });
      return false;
    }
  }

  /**
   * パブリックURLを生成
   * @param key ファイルキー
   * @returns パブリックURL
   */
  private generatePublicUrl(key: string): string {
    // R2のパブリックURLの形式
    // https://<bucket-name>.<account-id>.r2.cloudflarestorage.com/<key>
    const baseUrl = this.config.endpoint.startsWith("https://")
      ? this.config.endpoint.replace("https://", `https://${this.bucketName}.`)
      : `https://${this.bucketName}.${this.config.endpoint}.r2.cloudflarestorage.com`;

    return `${baseUrl}/${key}`;
  }

  /**
   * 設定情報を取得（デバッグ用）
   */
  getConfig(): R2Config {
    return {
      endpoint: this.config.endpoint,
      accessKeyId: this.config.accessKeyId,
      secretAccessKey: "[HIDDEN]",
      bucketName: this.config.bucketName,
      region: this.config.region,
    };
  }
}

/**
 * R2クライアントのファクトリー関数
 * @param config R2設定
 * @returns R2クライアントインスタンス
 */
export function createR2Client(config: R2Config): R2Client {
  return new R2Client(config);
}

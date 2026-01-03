/**
 * Cloudflare Workers環境用のR2クライアント
 * 直接R2バインディングを使用してアクセス
 */

import type { R2ClientInterface } from "./r2-client.js";
import { logger } from "../utils/logger.js";
import { withR2Retry } from "../utils/retry.js";
import { ErrorCode, createR2Error } from "../utils/error-handler.js";

/**
 * Workers環境用R2クライアントクラス
 * R2バインディングを直接使用
 */
export class R2WorkerClient implements R2ClientInterface {
  constructor(
    private r2Bucket: R2Bucket,
    private bucketName: string,
  ) {
    logger.info("Workers環境用R2クライアントを初期化しました", {
      bucketName: this.bucketName,
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
        // BufferをArrayBufferに変換
        const arrayBuffer = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength);

        await this.r2Bucket.put(key, arrayBuffer, {
          httpMetadata: {
            contentType: contentType,
          },
        });

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
        await this.r2Bucket.delete(key);

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
        const object = await this.r2Bucket.get(key);

        if (!object) {
          logger.warn("ファイルが見つかりません", {
            fileKey: key,
          });
          return null;
        }

        // ArrayBufferをBufferに変換
        const arrayBuffer = await object.arrayBuffer();
        const buffer = Buffer.from(arrayBuffer);

        logger.debug("ファイルの取得が完了しました", {
          fileKey: key,
          fileSize: buffer.length,
        });

        return buffer;
      } catch (error) {
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
   * Workers環境では制限があるため、パブリックURLまたは一時的なアクセス方法を使用
   * @param key ファイルキー（パス）
   * @param expiresIn 有効期限（秒）- Workers環境では制限あり
   * @returns アクセス可能なURL
   */
  async generatePresignedUrl(key: string, expiresIn: number = 3600): Promise<string> {
    return withR2Retry(async () => {
      try {
        // Workers環境では直接的なプリサインドURLの生成に制限があるため、
        // パブリックURLを返すか、カスタムエンドポイント経由でアクセスする

        // オプション1: パブリックURLを返す（バケットがパブリックの場合）
        const publicUrl = this.generatePublicUrl(key);

        // オプション2: カスタムドメイン経由でのアクセス（推奨）
        // const customUrl = `https://your-custom-domain.com/files/${key}`;

        logger.debug("ファイルアクセスURLを生成しました", {
          fileKey: key,
          expiresIn: expiresIn,
        });

        return publicUrl;
      } catch (error) {
        logger.error("ファイルアクセスURLの生成に失敗しました", {
          fileKey: key,
          error: error instanceof Error ? error.message : String(error),
        });

        throw createR2Error(
          ErrorCode.R2_CONNECTION_ERROR,
          `ファイルアクセスURL生成エラー: ${error instanceof Error ? error.message : String(error)}`,
          true,
        );
      }
    }, `ファイルアクセスURL生成: ${key}`);
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
   * @returns 実際のR2パブリックURL
   */
  private generatePublicUrl(key: string): string {
    // 環境変数からR2の設定を取得
    const accountId = process.env.R2_ACCOUNT_ID || "";

    // 実際のR2パブリックURLを生成
    return `https://${accountId}.r2.cloudflarestorage.com/${this.bucketName}/${key}`;
  }

  /**
   * ファイルの存在確認
   * @param key ファイルキー
   * @returns ファイルが存在する場合true
   */
  async fileExists(key: string): Promise<boolean> {
    try {
      const object = await this.r2Bucket.head(key);
      return object !== null;
    } catch (error) {
      logger.debug("ファイル存在確認でエラー", {
        fileKey: key,
        error: error instanceof Error ? error.message : String(error),
      });
      return false;
    }
  }

  /**
   * ファイルのメタデータを取得
   * @param key ファイルキー
   * @returns ファイルのメタデータ
   */
  async getObjectMetadata(key: string): Promise<R2Object | null> {
    try {
      return await this.r2Bucket.head(key);
    } catch (error) {
      logger.error("ファイルメタデータの取得に失敗しました", {
        fileKey: key,
        error: error instanceof Error ? error.message : String(error),
      });
      return null;
    }
  }

  /**
   * 設定情報を取得（デバッグ用）
   */
  getConfig(): import("../types/config.js").R2Config {
    return {
      endpoint: "[WORKERS_BINDING]",
      accessKeyId: "[WORKERS_BINDING]",
      secretAccessKey: "[WORKERS_BINDING]",
      bucketName: this.bucketName,
      region: "auto",
    };
  }
}

/**
 * Workers環境用R2クライアントのファクトリー関数
 * @param r2Bucket R2バケットバインディング
 * @param bucketName バケット名
 * @returns Workers環境用R2クライアントインスタンス
 */
export function createR2WorkerClient(r2Bucket: R2Bucket, bucketName: string): R2WorkerClient {
  return new R2WorkerClient(r2Bucket, bucketName);
}

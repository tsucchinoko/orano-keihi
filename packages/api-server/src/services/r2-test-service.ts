/**
 * R2接続テストサービス
 * R2ストレージの接続状態を確認する機能を提供
 */

import type { R2Client } from "./r2-client.js";
import { logger } from "../utils/logger.js";

export interface R2TestResult {
  success: boolean;
  message: string;
  details?: {
    endpoint: string;
    bucketName: string;
    region: string;
    testDurationMs: number;
  };
  error?: string;
}

/**
 * R2接続テストサービス
 */
export class R2TestService {
  constructor(private r2Client: R2Client) {}

  /**
   * 包括的なR2接続テストを実行
   * @returns テスト結果
   */
  async runComprehensiveTest(): Promise<R2TestResult> {
    const startTime = Date.now();

    try {
      logger.info("R2接続テストを開始します");

      // 基本的な接続テスト
      const connectionSuccess = await this.r2Client.testConnection();

      if (!connectionSuccess) {
        return {
          success: false,
          message: "R2への基本接続に失敗しました",
          error: "接続テストが失敗しました",
        };
      }

      // 詳細テスト: 複数の操作を順次実行
      await this.runDetailedTests();

      const endTime = Date.now();
      const duration = endTime - startTime;

      const config = this.r2Client.getConfig();

      logger.info("R2接続テストが完了しました", {
        duration: duration,
        endpoint: config.endpoint,
        bucketName: config.bucketName,
      });

      return {
        success: true,
        message: "R2接続テストが成功しました",
        details: {
          endpoint: config.endpoint,
          bucketName: config.bucketName,
          region: config.region,
          testDurationMs: duration,
        },
      };
    } catch (error) {
      const endTime = Date.now();
      const duration = endTime - startTime;

      logger.error("R2接続テストでエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
        duration: duration,
      });

      return {
        success: false,
        message: "R2接続テストでエラーが発生しました",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * 詳細なテストを実行
   * 複数のファイル操作をテストして接続の安定性を確認
   */
  private async runDetailedTests(): Promise<void> {
    const testFiles = [
      {
        key: `test-detailed-${Date.now()}-1.txt`,
        content: "詳細テスト用ファイル1",
        contentType: "text/plain",
      },
      {
        key: `test-detailed-${Date.now()}-2.json`,
        content: JSON.stringify({ test: true, timestamp: new Date().toISOString() }),
        contentType: "application/json",
      },
    ];

    try {
      // 複数ファイルのアップロードテスト
      const uploadPromises = testFiles.map(async (file) => {
        const buffer = Buffer.from(file.content, "utf-8");
        return await this.r2Client.putObject(file.key, buffer, file.contentType);
      });

      const uploadResults = await Promise.all(uploadPromises);
      logger.debug("詳細テスト: アップロード完了", {
        uploadedFiles: uploadResults.length,
      });

      // プリサインドURL生成テスト
      const presignedUrls = await Promise.all(
        testFiles.map((file) => this.r2Client.generatePresignedUrl(file.key, 300)),
      );
      logger.debug("詳細テスト: プリサインドURL生成完了", {
        generatedUrls: presignedUrls.length,
      });

      // ファイル削除テスト
      const deletePromises = testFiles.map((file) => this.r2Client.deleteObject(file.key));
      await Promise.all(deletePromises);
      logger.debug("詳細テスト: ファイル削除完了", {
        deletedFiles: testFiles.length,
      });
    } catch (error) {
      // クリーンアップ: エラーが発生した場合でもテストファイルを削除
      await this.cleanupTestFiles(testFiles.map((f) => f.key));
      throw error;
    }
  }

  /**
   * テストファイルのクリーンアップ
   * @param fileKeys 削除するファイルキーの配列
   */
  private async cleanupTestFiles(fileKeys: string[]): Promise<void> {
    try {
      const deletePromises = fileKeys.map(async (key) => {
        try {
          await this.r2Client.deleteObject(key);
        } catch (error) {
          // 個別の削除エラーはログに記録するが、全体の処理は継続
          logger.warn("テストファイルの削除に失敗しました", {
            fileKey: key,
            error: error instanceof Error ? error.message : String(error),
          });
        }
      });

      await Promise.all(deletePromises);
      logger.debug("テストファイルのクリーンアップが完了しました");
    } catch (error) {
      logger.warn("テストファイルのクリーンアップでエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }

  /**
   * 簡単な接続確認テスト
   * @returns 接続成功の場合true
   */
  async quickConnectionTest(): Promise<boolean> {
    try {
      return await this.r2Client.testConnection();
    } catch (error) {
      logger.error("簡単な接続テストに失敗しました", {
        error: error instanceof Error ? error.message : String(error),
      });
      return false;
    }
  }
}

/**
 * R2テストサービスのファクトリー関数
 * @param r2Client R2クライアントインスタンス
 * @returns R2テストサービスインスタンス
 */
export function createR2TestService(r2Client: R2Client): R2TestService {
  return new R2TestService(r2Client);
}

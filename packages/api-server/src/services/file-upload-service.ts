/**
 * ファイルアップロードサービス
 * ファイルの検証、アップロード、削除を管理
 */

import type { R2ClientInterface } from "./r2-client.js";
import type {
  UploadMetadata,
  UploadResponse,
  MultipleUploadResponse,
  FileValidationResult,
  FileUploadConfig,
} from "../types/config.js";
import { logger, enhancedLogger } from "../utils/logger.js";
import { ErrorCode, createFileError, createValidationError } from "../utils/error-handler.js";

export interface FileUploadServiceInterface {
  // 単一ファイルアップロード
  uploadFile(file: File, metadata: UploadMetadata): Promise<UploadResponse>;

  // 複数ファイル並列アップロード
  uploadMultipleFiles(files: File[], metadata: UploadMetadata[]): Promise<MultipleUploadResponse>;

  // ファイル検証
  validateFile(file: File): Promise<FileValidationResult>;

  // R2アップロード
  uploadToR2(fileKey: string, fileData: Buffer, contentType: string): Promise<string>;

  // ファイル削除
  deleteFile(fileKey: string): Promise<void>;
}

/**
 * ファイルアップロードサービスクラス
 */
export class FileUploadService implements FileUploadServiceInterface {
  constructor(
    private r2Client: R2ClientInterface,
    private config: FileUploadConfig,
  ) {
    logger.info("FileUploadServiceを初期化しました", {
      maxFileSize: this.config.maxFileSize,
      allowedTypes: this.config.allowedTypes,
      maxFiles: this.config.maxFiles,
    });
  }

  /**
   * 単一ファイルアップロード
   * @param file アップロードするファイル
   * @param metadata ファイルメタデータ
   * @returns アップロード結果
   */
  async uploadFile(file: File, metadata: UploadMetadata): Promise<UploadResponse> {
    const startTime = Date.now();

    try {
      logger.info("ファイルアップロードを開始します", {
        fileName: file.name,
        fileSize: file.size,
        contentType: file.type,
        userId: metadata.userId,
        expenseId: metadata.expenseId,
      });

      // ファイル検証
      const validationResult = await this.validateFile(file);
      if (!validationResult.isValid) {
        const errorResponse: UploadResponse = {
          success: false,
          fileKey: "",
          fileSize: file.size,
          contentType: file.type,
          uploadedAt: new Date().toISOString(),
          error: validationResult.error,
        };

        logger.warn("ファイル検証に失敗しました", {
          fileName: file.name,
          error: validationResult.error,
          details: validationResult.details,
        });

        return errorResponse;
      }

      // ファイルキーを生成（ユーザーID/経費ID/タイムスタンプ_ファイル名）
      const timestamp = Date.now();
      const sanitizedFileName = this.sanitizeFileName(file.name);
      const fileKey = `receipts/${metadata.userId}/${metadata.expenseId}/${timestamp}_${sanitizedFileName}`;

      // ファイルデータを読み込み
      const fileData = Buffer.from(await file.arrayBuffer());

      // R2にアップロード
      const fileUrl = await this.uploadToR2(fileKey, fileData, file.type);

      const uploadResponse: UploadResponse = {
        success: true,
        fileUrl: fileUrl,
        fileKey: fileKey,
        fileSize: file.size,
        contentType: file.type,
        uploadedAt: new Date().toISOString(),
      };

      const duration = Date.now() - startTime;
      logger.info("ファイルアップロードが完了しました", {
        fileName: file.name,
        fileKey: fileKey,
        fileUrl: fileUrl,
        fileSize: file.size,
        durationMs: duration,
      });

      return uploadResponse;
    } catch (error) {
      const duration = Date.now() - startTime;
      const errorMessage = error instanceof Error ? error.message : String(error);

      enhancedLogger.systemFailure("ファイルアップロードに失敗しました", {
        fileName: file.name,
        error: errorMessage,
        userId: metadata.userId,
        expenseId: metadata.expenseId,
        durationMs: duration,
      });

      return {
        success: false,
        fileKey: "",
        fileSize: file.size,
        contentType: file.type,
        uploadedAt: new Date().toISOString(),
        error: `アップロードエラー: ${errorMessage}`,
      };
    }
  }

  /**
   * 複数ファイル並列アップロード
   * @param files アップロードするファイル配列
   * @param metadata ファイルメタデータ配列
   * @returns 複数ファイルアップロード結果
   */
  async uploadMultipleFiles(
    files: File[],
    metadata: UploadMetadata[],
  ): Promise<MultipleUploadResponse> {
    const startTime = Date.now();

    try {
      logger.info("複数ファイルアップロードを開始します", {
        fileCount: files.length,
        totalSize: files.reduce((sum, file) => sum + file.size, 0),
      });

      // ファイル数の制限チェック
      if (files.length > this.config.maxFiles) {
        throw new Error(`ファイル数が制限を超えています（最大: ${this.config.maxFiles}）`);
      }

      // メタデータの数がファイル数と一致するかチェック
      if (files.length !== metadata.length) {
        throw new Error("ファイル数とメタデータ数が一致しません");
      }

      // 並列アップロード実行
      const uploadPromises = files.map((file, index) => this.uploadFile(file, metadata[index]));

      const results = await Promise.all(uploadPromises);

      // 結果を集計
      const successfulUploads = results.filter((result) => result.success).length;
      const failedUploads = results.length - successfulUploads;
      const totalDurationMs = Date.now() - startTime;

      const response: MultipleUploadResponse = {
        totalFiles: files.length,
        successfulUploads,
        failedUploads,
        results,
        totalDurationMs,
      };

      logger.info("複数ファイルアップロードが完了しました", {
        totalFiles: files.length,
        successfulUploads,
        failedUploads,
        totalDurationMs,
      });

      return response;
    } catch (error) {
      const totalDurationMs = Date.now() - startTime;
      const errorMessage = error instanceof Error ? error.message : String(error);

      enhancedLogger.systemFailure("複数ファイルアップロードに失敗しました", {
        fileCount: files.length,
        error: errorMessage,
        totalDurationMs,
      });

      // エラー時は全て失敗として扱う
      const errorResults: UploadResponse[] = files.map((file) => ({
        success: false,
        fileKey: "",
        fileSize: file.size,
        contentType: file.type,
        uploadedAt: new Date().toISOString(),
        error: errorMessage,
      }));

      return {
        totalFiles: files.length,
        successfulUploads: 0,
        failedUploads: files.length,
        results: errorResults,
        totalDurationMs,
      };
    }
  }

  /**
   * ファイル検証
   * @param file 検証するファイル
   * @returns 検証結果
   */
  async validateFile(file: File): Promise<FileValidationResult> {
    try {
      // ファイルサイズチェック
      if (file.size > this.config.maxFileSize) {
        throw createFileError(
          ErrorCode.FILE_TOO_LARGE,
          `ファイルサイズが制限を超えています（最大: ${this.config.maxFileSize} bytes）`,
          {
            field: "fileSize",
            value: file.size,
            constraint: `maxSize: ${this.config.maxFileSize}`,
          },
        );
      }

      // 空ファイルチェック
      if (file.size === 0) {
        throw createValidationError(
          "空のファイルはアップロードできません",
          "fileSize",
          file.size,
          "minSize: 1",
        );
      }

      // ファイルタイプチェック
      if (!this.config.allowedTypes.includes(file.type)) {
        throw createFileError(
          ErrorCode.INVALID_FILE_TYPE,
          `許可されていないファイル形式です（許可形式: ${this.config.allowedTypes.join(", ")}）`,
          {
            field: "contentType",
            value: file.type,
            constraint: `allowedTypes: ${this.config.allowedTypes.join(", ")}`,
          },
        );
      }

      // ファイル名チェック
      if (!file.name || file.name.trim() === "") {
        return {
          isValid: false,
          error: "ファイル名が指定されていません",
          details: {
            field: "fileName",
            value: file.name,
            constraint: "required",
          },
        };
      }

      // ファイル名の長さチェック
      if (file.name.length > 255) {
        return {
          isValid: false,
          error: "ファイル名が長すぎます（最大: 255文字）",
          details: {
            field: "fileName",
            value: file.name.length,
            constraint: "maxLength: 255",
          },
        };
      }

      // 危険な文字のチェック
      const dangerousChars = /[<>:"/\\|?*\x00-\x1f]/;
      if (dangerousChars.test(file.name)) {
        return {
          isValid: false,
          error: "ファイル名に使用できない文字が含まれています",
          details: {
            field: "fileName",
            value: file.name,
            constraint: "no dangerous characters",
          },
        };
      }

      logger.debug("ファイル検証が成功しました", {
        fileName: file.name,
        fileSize: file.size,
        contentType: file.type,
      });

      return {
        isValid: true,
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      logger.error("ファイル検証中にエラーが発生しました", {
        fileName: file.name,
        error: errorMessage,
      });

      return {
        isValid: false,
        error: `検証エラー: ${errorMessage}`,
      };
    }
  }

  /**
   * R2アップロード
   * @param fileKey ファイルキー
   * @param fileData ファイルデータ
   * @param contentType コンテンツタイプ
   * @returns アップロードされたファイルのURL
   */
  async uploadToR2(fileKey: string, fileData: Buffer, contentType: string): Promise<string> {
    try {
      const fileUrl = await this.r2Client.putObject(fileKey, fileData, contentType);

      logger.debug("R2アップロードが成功しました", {
        fileKey,
        fileUrl,
        contentType,
        fileSize: fileData.length,
      });

      return fileUrl;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      logger.error("R2アップロードに失敗しました", {
        fileKey,
        error: errorMessage,
      });
      throw error;
    }
  }

  /**
   * ファイル削除
   * @param fileKey 削除するファイルのキー
   */
  async deleteFile(fileKey: string): Promise<void> {
    try {
      await this.r2Client.deleteObject(fileKey);

      logger.info("ファイル削除が完了しました", {
        fileKey,
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      logger.error("ファイル削除に失敗しました", {
        fileKey,
        error: errorMessage,
      });
      throw error;
    }
  }

  /**
   * ファイル名をサニタイズ
   * @param fileName 元のファイル名
   * @returns サニタイズされたファイル名
   */
  private sanitizeFileName(fileName: string): string {
    // 危険な文字を除去し、安全な文字に置換
    return fileName
      .replace(/[<>:"/\\|?*\x00-\x1f]/g, "_") // 危険な文字をアンダースコアに置換
      .replace(/\s+/g, "_") // 空白をアンダースコアに置換
      .replace(/_{2,}/g, "_") // 連続するアンダースコアを1つに
      .replace(/^_+|_+$/g, "") // 先頭末尾のアンダースコアを除去
      .substring(0, 200); // 長さを制限
  }
}

/**
 * FileUploadServiceのファクトリー関数
 * @param r2Client R2クライアント
 * @param config ファイルアップロード設定
 * @returns FileUploadServiceインスタンス
 */
export function createFileUploadService(
  r2Client: R2ClientInterface,
  config: FileUploadConfig,
): FileUploadService {
  return new FileUploadService(r2Client, config);
}

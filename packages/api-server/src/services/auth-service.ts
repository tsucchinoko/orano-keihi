/**
 * 認証サービス
 * トークン検証、ユーザー認証、権限チェック機能を提供
 */

import { createCipheriv, createDecipheriv, randomBytes } from "node:crypto";
import type { AuthConfig, User, Session, AuthResult, ValidationResult } from "../types/config.js";
import { logger } from "../utils/logger.js";

/**
 * 認証エラーの種類
 */
export class AuthError extends Error {
  constructor(
    message: string,
    public code: string,
    public statusCode: number = 401,
  ) {
    super(message);
    this.name = "AuthError";
  }
}

/**
 * 認証サービスクラス
 */
export class AuthService {
  private readonly encryptionKey: Buffer;
  private readonly algorithm = "aes-256-gcm";
  private readonly sessionExpirationMs: number;

  constructor(config: AuthConfig) {
    // 暗号化キーを32バイトに調整
    const keyBytes = Buffer.from(config.sessionEncryptionKey, "utf8");
    this.encryptionKey = Buffer.alloc(32);
    keyBytes.copy(this.encryptionKey, 0, 0, Math.min(keyBytes.length, 32));

    // セッション有効期限をミリ秒に変換
    this.sessionExpirationMs = config.sessionExpirationDays * 24 * 60 * 60 * 1000;

    logger.info("認証サービスを初期化しました", {
      sessionExpirationDays: config.sessionExpirationDays,
    });
  }

  /**
   * セッショントークンを検証する
   * @param token 暗号化されたセッショントークン
   * @returns 検証結果
   */
  async validateToken(token: string): Promise<ValidationResult> {
    try {
      // トークンを復号化してセッションIDを取得
      const sessionId = this.decryptToken(token);

      // TODO: 実際の実装では、データベースからセッション情報を取得する
      // 現在はモックデータを使用
      const session = await this.getSessionFromDatabase(sessionId);

      if (!session) {
        logger.warn("セッションが見つかりません", { sessionId });
        return {
          isValid: false,
          error: "セッションが見つかりません",
        };
      }

      // セッションの有効期限をチェック
      const expiresAt = new Date(session.expiresAt);
      if (expiresAt < new Date()) {
        logger.warn("セッションが期限切れです", { sessionId, expiresAt });
        return {
          isValid: false,
          error: "セッションが期限切れです",
        };
      }

      // ユーザー情報を取得
      const user = await this.getUserById(session.userId);
      if (!user) {
        logger.error("ユーザーが見つかりません", { userId: session.userId });
        return {
          isValid: false,
          error: "ユーザーが見つかりません",
        };
      }

      logger.debug("トークン検証が成功しました", {
        userId: user.id,
        sessionId,
      });

      return {
        isValid: true,
        user,
      };
    } catch (error) {
      logger.error("トークン検証でエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
      });

      return {
        isValid: false,
        error: "トークン検証に失敗しました",
      };
    }
  }

  /**
   * ユーザーを認証する
   * @param userId ユーザーID
   * @returns 認証結果
   */
  async authenticateUser(userId: number): Promise<AuthResult> {
    try {
      const user = await this.getUserById(userId);

      if (!user) {
        logger.warn("認証対象のユーザーが見つかりません", { userId });
        return {
          success: false,
          error: "ユーザーが見つかりません",
        };
      }

      logger.info("ユーザー認証が成功しました", { userId });

      return {
        success: true,
        user,
      };
    } catch (error) {
      logger.error("ユーザー認証でエラーが発生しました", {
        userId,
        error: error instanceof Error ? error.message : String(error),
      });

      return {
        success: false,
        error: "認証に失敗しました",
      };
    }
  }

  /**
   * ユーザーの権限をチェックする
   * @param userId ユーザーID
   * @param resource リソース名
   * @returns 権限があるかどうか
   */
  async checkPermission(userId: number, resource: string): Promise<boolean> {
    try {
      // TODO: 実際の実装では、データベースから権限情報を取得する
      // 現在は基本的な権限チェックのみ実装

      const user = await this.getUserById(userId);
      if (!user) {
        logger.warn("権限チェック対象のユーザーが見つかりません", { userId });
        return false;
      }

      // 基本的な権限チェック（すべてのユーザーに基本権限を付与）
      const basicResources = ["file_upload", "file_download", "file_delete"];
      const hasPermission = basicResources.includes(resource);

      logger.debug("権限チェックを実行しました", {
        userId,
        resource,
        hasPermission,
      });

      return hasPermission;
    } catch (error) {
      logger.error("権限チェックでエラーが発生しました", {
        userId,
        resource,
        error: error instanceof Error ? error.message : String(error),
      });

      return false;
    }
  }

  /**
   * セッションIDを暗号化してトークンを生成する
   * @param sessionId セッションID
   * @returns 暗号化されたトークン
   */
  encryptSessionId(sessionId: string): string {
    try {
      const iv = randomBytes(12); // GCMモードでは12バイトのIVを使用
      const cipher = createCipheriv(this.algorithm, this.encryptionKey, iv);

      let encrypted = cipher.update(sessionId, "utf8", "base64");
      encrypted += cipher.final("base64");

      const authTag = cipher.getAuthTag();

      // IV、認証タグ、暗号文を結合してBase64エンコード
      const combined = Buffer.concat([iv, authTag, Buffer.from(encrypted, "base64")]);
      return combined.toString("base64");
    } catch (error) {
      logger.error("セッションID暗号化でエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
      });
      throw new AuthError("セッションID暗号化に失敗しました", "ENCRYPTION_ERROR", 500);
    }
  }

  /**
   * トークンを復号化してセッションIDを取得する
   * @param token 暗号化されたトークン
   * @returns セッションID
   */
  private decryptToken(token: string): string {
    try {
      const combined = Buffer.from(token, "base64");

      if (combined.length < 28) {
        // IV(12) + AuthTag(16) = 28バイト最小
        throw new AuthError("トークンが短すぎます", "INVALID_TOKEN");
      }

      // IV、認証タグ、暗号文を分離
      const iv = combined.subarray(0, 12);
      const authTag = combined.subarray(12, 28);
      const encrypted = combined.subarray(28);

      const decipher = createDecipheriv(this.algorithm, this.encryptionKey, iv);
      decipher.setAuthTag(authTag);

      let decrypted = decipher.update(encrypted, undefined, "utf8");
      decrypted += decipher.final("utf8");

      return decrypted;
    } catch (error) {
      logger.warn("トークン復号化に失敗しました", {
        error: error instanceof Error ? error.message : String(error),
      });
      throw new AuthError("無効なトークンです", "INVALID_TOKEN");
    }
  }

  /**
   * データベースからセッション情報を取得する（モック実装）
   * @param sessionId セッションID
   * @returns セッション情報
   */
  private async getSessionFromDatabase(sessionId: string): Promise<Session | null> {
    // TODO: 実際の実装では、データベースからセッション情報を取得する
    // 現在はモックデータを返す

    // テスト用のモックセッション
    if (sessionId === "test-session-id") {
      return {
        id: sessionId,
        userId: 1,
        expiresAt: new Date(Date.now() + this.sessionExpirationMs).toISOString(),
        createdAt: new Date().toISOString(),
      };
    }

    return null;
  }

  /**
   * ユーザーIDからユーザー情報を取得する（モック実装）
   * @param userId ユーザーID
   * @returns ユーザー情報
   */
  private async getUserById(userId: number): Promise<User | null> {
    // TODO: 実際の実装では、データベースからユーザー情報を取得する
    // 現在はモックデータを返す

    // テスト用のモックユーザー
    if (userId === 1) {
      return {
        id: userId,
        googleId: "test-google-id",
        email: "test@example.com",
        name: "テストユーザー",
        pictureUrl: "https://example.com/avatar.jpg",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
    }

    return null;
  }
}

/**
 * AuthServiceインスタンスを作成する
 * @param config 認証設定
 * @returns AuthServiceインスタンス
 */
export function createAuthService(config: AuthConfig): AuthService {
  return new AuthService(config);
}

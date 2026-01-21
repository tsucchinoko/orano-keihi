/**
 * 認証サービス
 * トークン検証、ユーザー認証、権限チェック機能を提供
 */

import { jwtVerify } from "jose";
import type { AuthConfig, User, AuthResult, ValidationResult } from "../types/config.js";
import { logger } from "../utils/logger.js";
import { withAuthRetry, withDatabaseRetry } from "../utils/retry.js";
import { AppError } from "../utils/error-handler.js";
import type { UserRepository } from "../repositories/user-repository.js";

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
  private readonly userRepository: UserRepository;
  private readonly jwtSecret: Uint8Array;

  constructor(config: AuthConfig, userRepository: UserRepository) {
    this.userRepository = userRepository;
    // JWT検証用のシークレットキーを設定
    this.jwtSecret = new TextEncoder().encode(config.jwtSecret);
    logger.info("認証サービスを初期化しました");
  }

  /**
   * セッショントークンを検証する
   * JWTトークンをデコードしてユーザーIDを取得し、ユーザー情報を返す
   * @param token JWTトークン
   * @returns 検証結果
   */
  async validateToken(token: string): Promise<ValidationResult> {
    return withAuthRetry(async () => {
      try {
        logger.debug("JWTトークン検証を実行", {
          tokenLength: token.length,
        });

        // JWTトークンを検証してデコード
        const { payload } = await jwtVerify(token, this.jwtSecret);

        logger.debug("JWTトークンのデコードに成功", {
          sub: payload.sub,
          email: payload.email,
        });

        // subフィールドからユーザーIDを取得（Google IDの場合）
        const googleId = payload.sub;
        if (!googleId || typeof googleId !== "string") {
          logger.warn("JWTトークンにsubフィールドがありません");
          return {
            isValid: false,
            error: "トークンが無効です（subフィールドなし）",
          };
        }

        // Google IDからユーザーを検索
        const user = await this.getUserByGoogleId(googleId);
        if (user) {
          logger.debug("トークン検証が成功しました", {
            userId: user.id,
            googleId: user.googleId,
            email: user.email,
          });
          return {
            isValid: true,
            user,
          };
        }

        logger.warn("トークン検証が失敗しました（ユーザーが見つかりません）", {
          googleId,
        });
        return {
          isValid: false,
          error: "ユーザーが見つかりません",
        };
      } catch (error) {
        logger.error("トークン検証でエラーが発生しました", {
          error: error instanceof Error ? error.message : String(error),
        });

        // JWT検証エラーの場合
        if (error instanceof Error && error.name === "JWTExpired") {
          return {
            isValid: false,
            error: "トークンの有効期限が切れています",
          };
        }

        if (error instanceof AppError) {
          throw error;
        }

        return {
          isValid: false,
          error: "トークン検証に失敗しました",
        };
      }
    }, "トークン検証");
  }

  /**
   * ユーザーを認証する
   * @param userId ユーザーID
   * @returns 認証結果
   */
  async authenticateUser(userId: string): Promise<AuthResult> {
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
  async checkPermission(userId: string, resource: string): Promise<boolean> {
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
   * ユーザーIDからユーザー情報を取得する
   * @param userId ユーザーID
   * @returns ユーザー情報
   */
  private async getUserById(userId: string): Promise<User | null> {
    return withDatabaseRetry(async () => {
      try {
        const dbUser = await this.userRepository.getUserById(userId);
        if (dbUser) {
          // D1のUser型をconfig.jsのUser型に変換
          return {
            id: dbUser.id,
            googleId: dbUser.google_id,
            email: dbUser.email,
            name: dbUser.name,
            pictureUrl: dbUser.picture_url || undefined,
            createdAt: dbUser.created_at,
            updatedAt: dbUser.updated_at,
          };
        }
        return null;
      } catch (error) {
        logger.error("データベースからのユーザー取得に失敗しました", {
          userId,
          error: error instanceof Error ? error.message : String(error),
        });
        throw error;
      }
    }, `ユーザー取得: ${userId}`);
  }

  /**
   * Google IDからユーザー情報を取得する
   * @param googleId Google ID
   * @returns ユーザー情報
   */
  private async getUserByGoogleId(googleId: string): Promise<User | null> {
    return withDatabaseRetry(async () => {
      try {
        const dbUser = await this.userRepository.getUserByGoogleId(googleId);
        if (dbUser) {
          // D1のUser型をconfig.jsのUser型に変換
          return {
            id: dbUser.id,
            googleId: dbUser.google_id,
            email: dbUser.email,
            name: dbUser.name,
            pictureUrl: dbUser.picture_url || undefined,
            createdAt: dbUser.created_at,
            updatedAt: dbUser.updated_at,
          };
        }
        return null;
      } catch (error) {
        logger.error("データベースからのユーザー取得に失敗しました（Google ID）", {
          googleId,
          error: error instanceof Error ? error.message : String(error),
        });
        throw error;
      }
    }, `ユーザー取得（Google ID）: ${googleId}`);
  }
}

/**
 * AuthServiceインスタンスを作成する
 * @param config 認証設定
 * @param userRepository ユーザーリポジトリ
 * @returns AuthServiceインスタンス
 */
export function createAuthService(config: AuthConfig, userRepository: UserRepository): AuthService {
  return new AuthService(config, userRepository);
}

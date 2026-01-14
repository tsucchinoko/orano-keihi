/**
 * ユーザーリポジトリ
 * D1データベースのusersテーブルへのアクセスを提供
 */

import type { D1Database } from "@cloudflare/workers-types";
import type { User } from "../types/d1-models.js";
import type { GoogleUser } from "../types/d1-dtos.js";
import { logger } from "../utils/logger.js";
import { nanoid } from "nanoid";

/**
 * ユーザーリポジトリクラス
 */
export class UserRepository {
  constructor(private db: D1Database) {}

  /**
   * Google OAuth情報からユーザーを検索または作成する
   * @param googleUser Google OAuth ユーザー情報
   * @returns ユーザー情報
   */
  async findOrCreateUser(googleUser: GoogleUser): Promise<User> {
    try {
      // まず既存のユーザーを検索
      const existingUser = await this.getUserByGoogleId(googleUser.google_id);

      if (existingUser) {
        logger.debug("既存のユーザーが見つかりました", {
          userId: existingUser.id,
          email: existingUser.email,
        });
        return existingUser;
      }

      // 新規ユーザーを作成
      const userId = nanoid(); // 21文字のnanoIdを生成
      const now = new Date().toISOString(); // RFC3339形式（JST）

      const result = await this.db
        .prepare(
          `INSERT INTO users (id, google_id, email, name, picture_url, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?)`,
        )
        .bind(
          userId,
          googleUser.google_id,
          googleUser.email,
          googleUser.name,
          googleUser.picture_url || null,
          now,
          now,
        )
        .run();

      if (!result.success) {
        logger.error("ユーザー作成に失敗しました", {
          googleId: googleUser.google_id,
          error: result.error,
        });
        throw new Error(`ユーザー作成に失敗しました: ${result.error}`);
      }

      logger.info("新規ユーザーを作成しました", {
        userId,
        email: googleUser.email,
      });

      // 作成したユーザーを取得して返す
      const newUser = await this.getUserById(userId);
      if (!newUser) {
        throw new Error("作成したユーザーの取得に失敗しました");
      }

      return newUser;
    } catch (error) {
      logger.error("findOrCreateUserでエラーが発生しました", {
        googleId: googleUser.google_id,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * ユーザーIDでユーザーを取得する
   * @param userId ユーザーID
   * @returns ユーザー情報、または見つからない場合はnull
   */
  async getUserById(userId: string): Promise<User | null> {
    try {
      const result = await this.db
        .prepare("SELECT * FROM users WHERE id = ?")
        .bind(userId)
        .first<User>();

      if (!result) {
        logger.debug("ユーザーが見つかりませんでした", { userId });
        return null;
      }

      logger.debug("ユーザーを取得しました", {
        userId: result.id,
        email: result.email,
      });

      return result;
    } catch (error) {
      logger.error("getUserByIdでエラーが発生しました", {
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * Google IDでユーザーを取得する
   * @param googleId Google OAuth ID
   * @returns ユーザー情報、または見つからない場合はnull
   */
  async getUserByGoogleId(googleId: string): Promise<User | null> {
    try {
      const result = await this.db
        .prepare("SELECT * FROM users WHERE google_id = ?")
        .bind(googleId)
        .first<User>();

      if (!result) {
        logger.debug("Google IDでユーザーが見つかりませんでした", { googleId });
        return null;
      }

      logger.debug("Google IDでユーザーを取得しました", {
        userId: result.id,
        email: result.email,
      });

      return result;
    } catch (error) {
      logger.error("getUserByGoogleIdでエラーが発生しました", {
        googleId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * ユーザー情報を更新する
   * @param user 更新するユーザー情報
   * @returns 更新後のユーザー情報
   */
  async updateUser(user: User): Promise<User> {
    try {
      const now = new Date().toISOString(); // RFC3339形式（JST）

      const result = await this.db
        .prepare(
          `UPDATE users 
           SET email = ?, name = ?, picture_url = ?, updated_at = ?
           WHERE id = ?`,
        )
        .bind(user.email, user.name, user.picture_url, now, user.id)
        .run();

      if (!result.success) {
        logger.error("ユーザー更新に失敗しました", {
          userId: user.id,
          error: result.error,
        });
        throw new Error(`ユーザー更新に失敗しました: ${result.error}`);
      }

      // 更新されたレコードが存在するか確認
      if (result.meta.changes === 0) {
        logger.warn("更新対象のユーザーが見つかりませんでした", {
          userId: user.id,
        });
        throw new Error(`ユーザーが見つかりません: ${user.id}`);
      }

      logger.info("ユーザー情報を更新しました", {
        userId: user.id,
        email: user.email,
      });

      // 更新後のユーザー情報を取得して返す
      const updatedUser = await this.getUserById(user.id);
      if (!updatedUser) {
        throw new Error("更新したユーザーの取得に失敗しました");
      }

      return updatedUser;
    } catch (error) {
      logger.error("updateUserでエラーが発生しました", {
        userId: user.id,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * ユーザーを削除する
   * @param userId ユーザーID
   */
  async deleteUser(userId: string): Promise<void> {
    try {
      const result = await this.db.prepare("DELETE FROM users WHERE id = ?").bind(userId).run();

      if (!result.success) {
        logger.error("ユーザー削除に失敗しました", {
          userId,
          error: result.error,
        });
        throw new Error(`ユーザー削除に失敗しました: ${result.error}`);
      }

      // 削除されたレコードが存在するか確認
      if (result.meta.changes === 0) {
        logger.warn("削除対象のユーザーが見つかりませんでした", {
          userId,
        });
        throw new Error(`ユーザーが見つかりません: ${userId}`);
      }

      logger.info("ユーザーを削除しました", { userId });
    } catch (error) {
      logger.error("deleteUserでエラーが発生しました", {
        userId,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * すべてのユーザーを取得する
   * @returns ユーザー一覧
   */
  async getAllUsers(): Promise<User[]> {
    try {
      const result = await this.db
        .prepare("SELECT * FROM users ORDER BY created_at DESC")
        .all<User>();

      if (!result.success) {
        logger.error("ユーザー一覧取得に失敗しました", {
          error: result.error,
        });
        throw new Error(`ユーザー一覧取得に失敗しました: ${result.error}`);
      }

      logger.debug("ユーザー一覧を取得しました", {
        count: result.results.length,
      });

      return result.results;
    } catch (error) {
      logger.error("getAllUsersでエラーが発生しました", {
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }
}

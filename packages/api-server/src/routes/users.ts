/**
 * ユーザー関連のAPIエンドポイント
 */

import { Hono } from "hono";
import type { Context } from "hono";
import { logger } from "../utils/logger.js";
import { handleError, createValidationError, createNotFoundError } from "../utils/error-handler.js";
import type { UserRepository } from "../repositories/user-repository.js";
import type { User } from "../types/d1-models.js";
import type { UpdateUserDto } from "../types/d1-dtos.js";

/**
 * ユーザールーターを作成
 * @param userRepository ユーザーリポジトリ
 * @returns ユーザールーター
 */
export function createUsersRouter(userRepository: UserRepository): Hono {
  const usersApp = new Hono();

  // GET /api/v1/users/me - 現在のユーザー情報を取得
  usersApp.get("/me", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      logger.debug("現在のユーザー情報を取得", {
        userId: user.id,
        email: user.email,
      });

      // D1からユーザー情報を取得
      const dbUser = await userRepository.getUserById(user.id);

      if (!dbUser) {
        logger.error("データベースにユーザー情報が見つかりません", {
          userId: user.id,
        });
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      logger.info("現在のユーザー情報を取得しました", {
        userId: dbUser.id,
        email: dbUser.email,
      });

      return c.json({
        success: true,
        user: dbUser,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "現在のユーザー情報取得",
      });
    }
  });

  // PUT /api/v1/users/me - 現在のユーザー情報を更新
  usersApp.put("/me", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      // リクエストボディを取得
      const body = await c.req.json<UpdateUserDto>();

      logger.debug("ユーザー情報更新リクエスト", {
        userId: user.id,
        updateData: body,
      });

      // バリデーション
      if (body.email !== undefined && typeof body.email !== "string") {
        throw createValidationError(
          "メールアドレスは文字列である必要があります",
          "email",
          body.email,
          "string required",
        );
      }

      if (body.name !== undefined && typeof body.name !== "string") {
        throw createValidationError(
          "ユーザー名は文字列である必要があります",
          "name",
          body.name,
          "string required",
        );
      }

      if (body.picture_url !== undefined && typeof body.picture_url !== "string") {
        throw createValidationError(
          "プロフィール画像URLは文字列である必要があります",
          "picture_url",
          body.picture_url,
          "string required",
        );
      }

      // 現在のユーザー情報を取得
      const currentUser = await userRepository.getUserById(user.id);

      if (!currentUser) {
        logger.error("データベースにユーザー情報が見つかりません", {
          userId: user.id,
        });
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      // 更新データをマージ
      const updatedUser: User = {
        ...currentUser,
        email: body.email ?? currentUser.email,
        name: body.name ?? currentUser.name,
        picture_url: body.picture_url ?? currentUser.picture_url,
        updated_at: new Date().toISOString(),
      };

      // ユーザー情報を更新
      const result = await userRepository.updateUser(updatedUser);

      logger.info("ユーザー情報を更新しました", {
        userId: result.id,
        email: result.email,
      });

      return c.json({
        success: true,
        user: result,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "ユーザー情報更新",
      });
    }
  });

  // DELETE /api/v1/users/me - 現在のユーザーを削除
  usersApp.delete("/me", async (c: Context) => {
    try {
      const user = c.get("user");

      if (!user) {
        logger.error("ユーザー情報が見つかりません");
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      logger.debug("ユーザー削除リクエスト", {
        userId: user.id,
        email: user.email,
      });

      // ユーザーが存在するか確認
      const existingUser = await userRepository.getUserById(user.id);

      if (!existingUser) {
        logger.error("データベースにユーザー情報が見つかりません", {
          userId: user.id,
        });
        throw createNotFoundError("ユーザー情報が見つかりません");
      }

      // ユーザーを削除（CASCADE制約により関連データも削除される）
      await userRepository.deleteUser(user.id);

      logger.info("ユーザーを削除しました", {
        userId: user.id,
        email: existingUser.email,
      });

      return c.json({
        success: true,
        message: "ユーザーが正常に削除されました",
        userId: user.id,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return handleError(c, error instanceof Error ? error : new Error(String(error)), {
        context: "ユーザー削除",
      });
    }
  });

  return usersApp;
}

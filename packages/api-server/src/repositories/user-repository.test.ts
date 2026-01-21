/**
 * UserRepositoryのテスト
 */

import { describe, it, expect, beforeEach } from "vitest";
import { UserRepository } from "./user-repository.js";
import type { GoogleUser } from "../types/d1-dtos.js";

// テスト用のD1データベースモック
// 実際のテストでは、Miniflareを使用してローカルD1データベースに接続します
describe("UserRepository", () => {
  let repository: UserRepository;

  beforeEach(async () => {
    // テスト用のD1データベースを取得
    // この部分は実際のMiniflare環境で実行する必要があります
    // 現在はスキップします
  });

  it.skip("findOrCreateUser - 新規ユーザーを作成する", async () => {
    const googleUser: GoogleUser = {
      google_id: "test-google-id-123",
      email: "test@example.com",
      name: "テストユーザー",
      picture_url: "https://example.com/avatar.jpg",
    };

    const user = await repository.findOrCreateUser(googleUser);

    expect(user).toBeDefined();
    expect(user.google_id).toBe(googleUser.google_id);
    expect(user.email).toBe(googleUser.email);
    expect(user.name).toBe(googleUser.name);
    expect(user.picture_url).toBe(googleUser.picture_url);
    expect(user.id).toBeDefined();
    expect(user.id.length).toBe(21); // nanoIdは21文字
  });

  it.skip("findOrCreateUser - 既存ユーザーを返す", async () => {
    const googleUser: GoogleUser = {
      google_id: "test-google-id-456",
      email: "existing@example.com",
      name: "既存ユーザー",
    };

    // 1回目: 新規作成
    const user1 = await repository.findOrCreateUser(googleUser);

    // 2回目: 既存ユーザーを返す
    const user2 = await repository.findOrCreateUser(googleUser);

    expect(user1.id).toBe(user2.id);
    expect(user1.google_id).toBe(user2.google_id);
  });

  it.skip("getUserById - ユーザーを取得する", async () => {
    const googleUser: GoogleUser = {
      google_id: "test-google-id-789",
      email: "getbyid@example.com",
      name: "GetByIdテスト",
    };

    const createdUser = await repository.findOrCreateUser(googleUser);
    const fetchedUser = await repository.getUserById(createdUser.id);

    expect(fetchedUser).toBeDefined();
    expect(fetchedUser?.id).toBe(createdUser.id);
    expect(fetchedUser?.email).toBe(createdUser.email);
  });

  it.skip("getUserById - 存在しないユーザーはnullを返す", async () => {
    const user = await repository.getUserById("non-existent-id");
    expect(user).toBeNull();
  });

  it.skip("getUserByGoogleId - ユーザーを取得する", async () => {
    const googleUser: GoogleUser = {
      google_id: "test-google-id-abc",
      email: "getbygoogleid@example.com",
      name: "GetByGoogleIdテスト",
    };

    const createdUser = await repository.findOrCreateUser(googleUser);
    const fetchedUser = await repository.getUserByGoogleId(googleUser.google_id);

    expect(fetchedUser).toBeDefined();
    expect(fetchedUser?.google_id).toBe(createdUser.google_id);
    expect(fetchedUser?.email).toBe(createdUser.email);
  });

  it.skip("updateUser - ユーザー情報を更新する", async () => {
    const googleUser: GoogleUser = {
      google_id: "test-google-id-update",
      email: "update@example.com",
      name: "更新前",
    };

    const createdUser = await repository.findOrCreateUser(googleUser);

    // ユーザー情報を更新
    const updatedData = {
      ...createdUser,
      name: "更新後",
      email: "updated@example.com",
    };

    const updatedUser = await repository.updateUser(updatedData);

    expect(updatedUser.name).toBe("更新後");
    expect(updatedUser.email).toBe("updated@example.com");
    expect(updatedUser.id).toBe(createdUser.id);
  });

  it.skip("deleteUser - ユーザーを削除する", async () => {
    const googleUser: GoogleUser = {
      google_id: "test-google-id-delete",
      email: "delete@example.com",
      name: "削除テスト",
    };

    const createdUser = await repository.findOrCreateUser(googleUser);

    // ユーザーを削除
    await repository.deleteUser(createdUser.id);

    // 削除後は取得できない
    const deletedUser = await repository.getUserById(createdUser.id);
    expect(deletedUser).toBeNull();
  });

  it.skip("getAllUsers - すべてのユーザーを取得する", async () => {
    // テストデータを作成
    const googleUsers: GoogleUser[] = [
      {
        google_id: "test-google-id-all-1",
        email: "all1@example.com",
        name: "全取得テスト1",
      },
      {
        google_id: "test-google-id-all-2",
        email: "all2@example.com",
        name: "全取得テスト2",
      },
    ];

    for (const googleUser of googleUsers) {
      await repository.findOrCreateUser(googleUser);
    }

    const allUsers = await repository.getAllUsers();

    expect(allUsers.length).toBeGreaterThanOrEqual(2);
    expect(allUsers.some((u) => u.email === "all1@example.com")).toBe(true);
    expect(allUsers.some((u) => u.email === "all2@example.com")).toBe(true);
  });
});

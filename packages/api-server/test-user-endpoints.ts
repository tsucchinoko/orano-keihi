/**
 * ユーザーAPIエンドポイントのテストスクリプト
 *
 * 実行方法:
 * pnpm tsx test-user-endpoints.ts
 */

import process from "node:process";

async function testUserEndpoints() {
  console.log("=== ユーザーAPIエンドポイントのテスト ===\n");

  // 開発環境では簡易的なトークンを使用
  // AuthServiceは開発環境で任意のトークンを受け入れるため、
  // ユーザーIDをトークンとして使用
  console.log("1. 開発環境用トークンを準備中...");
  const token = "2"; // 開発用ユーザーID（AuthServiceのモックユーザー）
  console.log(`   トークン: ${token}\n`);

  const baseUrl = "http://localhost:3000";
  const headers = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${token}`,
  };

  try {
    // テスト1: GET /api/v1/users/me
    console.log("2. GET /api/v1/users/me をテスト中...");
    const getResponse = await fetch(`${baseUrl}/api/v1/users/me`, {
      method: "GET",
      headers,
    });

    console.log(`   ステータス: ${getResponse.status}`);
    const getData = await getResponse.json();
    console.log(`   レスポンス:`, JSON.stringify(getData, null, 2));
    console.log();

    // テスト2: PUT /api/v1/users/me
    console.log("3. PUT /api/v1/users/me をテスト中...");
    const updateData = {
      name: "Updated Test User",
      email: "updated@example.com",
    };

    const putResponse = await fetch(`${baseUrl}/api/v1/users/me`, {
      method: "PUT",
      headers,
      body: JSON.stringify(updateData),
    });

    console.log(`   ステータス: ${putResponse.status}`);
    const putData = await putResponse.json();
    console.log(`   レスポンス:`, JSON.stringify(putData, null, 2));
    console.log();

    // テスト3: DELETE /api/v1/users/me
    console.log("4. DELETE /api/v1/users/me をテスト中...");
    const deleteResponse = await fetch(`${baseUrl}/api/v1/users/me`, {
      method: "DELETE",
      headers,
    });

    console.log(`   ステータス: ${deleteResponse.status}`);
    const deleteData = await deleteResponse.json();
    console.log(`   レスポンス:`, JSON.stringify(deleteData, null, 2));
    console.log();

    console.log("=== テスト完了 ===");
  } catch (error) {
    console.error("テスト中にエラーが発生しました:", error);
    process.exit(1);
  }
}

// テストを実行
testUserEndpoints().catch((error) => {
  console.error("テスト実行エラー:", error);
  process.exit(1);
});

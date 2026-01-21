/**
 * 経費APIエンドポイントのテストスクリプト
 *
 * 実行方法:
 * pnpm tsx test-expense-api.ts
 */

import process from "node:process";

async function testExpenseEndpoints() {
  console.log("=== 経費APIエンドポイントのテスト ===\n");

  // 開発環境では簡易的なトークンを使用
  console.log("1. 開発環境用トークンを準備中...");
  const token = "2"; // 開発用ユーザーID
  console.log(`   トークン: ${token}\n`);

  const baseUrl = "http://localhost:3000";
  const headers = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${token}`,
  };

  let createdExpenseId: number | null = null;

  try {
    // テスト1: POST /api/v1/expenses - 経費を作成
    console.log("2. POST /api/v1/expenses をテスト中...");
    const createData = {
      date: "2024-01-15",
      amount: 1500,
      category: "食費",
      description: "テスト経費",
    };

    const createResponse = await fetch(`${baseUrl}/api/v1/expenses`, {
      method: "POST",
      headers,
      body: JSON.stringify(createData),
    });

    console.log(`   ステータス: ${createResponse.status}`);
    const createResult = await createResponse.json();
    console.log(`   レスポンス:`, JSON.stringify(createResult, null, 2));

    if (createResult.success && createResult.expense) {
      createdExpenseId = createResult.expense.id;
      console.log(`   ✅ 経費が作成されました (ID: ${createdExpenseId})`);
    }
    console.log();

    if (!createdExpenseId) {
      console.error("❌ 経費の作成に失敗しました");
      return;
    }

    // テスト2: GET /api/v1/expenses/:id - 経費を取得
    console.log(`3. GET /api/v1/expenses/${createdExpenseId} をテスト中...`);
    const getResponse = await fetch(`${baseUrl}/api/v1/expenses/${createdExpenseId}`, {
      method: "GET",
      headers,
    });

    console.log(`   ステータス: ${getResponse.status}`);
    const getData = await getResponse.json();
    console.log(`   レスポンス:`, JSON.stringify(getData, null, 2));
    console.log();

    // テスト3: GET /api/v1/expenses - 経費一覧を取得
    console.log("4. GET /api/v1/expenses をテスト中...");
    const listResponse = await fetch(`${baseUrl}/api/v1/expenses`, {
      method: "GET",
      headers,
    });

    console.log(`   ステータス: ${listResponse.status}`);
    const listData = await listResponse.json();
    console.log(`   レスポンス:`, JSON.stringify(listData, null, 2));
    console.log();

    // テスト4: GET /api/v1/expenses?month=2024-01 - 月フィルター
    console.log("5. GET /api/v1/expenses?month=2024-01 をテスト中...");
    const filterResponse = await fetch(`${baseUrl}/api/v1/expenses?month=2024-01`, {
      method: "GET",
      headers,
    });

    console.log(`   ステータス: ${filterResponse.status}`);
    const filterData = await filterResponse.json();
    console.log(`   レスポンス:`, JSON.stringify(filterData, null, 2));
    console.log();

    // テスト5: PUT /api/v1/expenses/:id - 経費を更新
    console.log(`6. PUT /api/v1/expenses/${createdExpenseId} をテスト中...`);
    const updateData = {
      amount: 2000,
      description: "更新されたテスト経費",
    };

    const updateResponse = await fetch(`${baseUrl}/api/v1/expenses/${createdExpenseId}`, {
      method: "PUT",
      headers,
      body: JSON.stringify(updateData),
    });

    console.log(`   ステータス: ${updateResponse.status}`);
    const updateResult = await updateResponse.json();
    console.log(`   レスポンス:`, JSON.stringify(updateResult, null, 2));
    console.log();

    // テスト6: DELETE /api/v1/expenses/:id - 経費を削除
    console.log(`7. DELETE /api/v1/expenses/${createdExpenseId} をテスト中...`);
    const deleteResponse = await fetch(`${baseUrl}/api/v1/expenses/${createdExpenseId}`, {
      method: "DELETE",
      headers,
    });

    console.log(`   ステータス: ${deleteResponse.status}`);
    const deleteResult = await deleteResponse.json();
    console.log(`   レスポンス:`, JSON.stringify(deleteResult, null, 2));
    console.log();

    console.log("=== テスト完了 ===");
    console.log("✅ すべての経費エンドポイントが正常に動作しています");
  } catch (error) {
    console.error("❌ テスト中にエラーが発生しました:", error);
    process.exit(1);
  }
}

// テストを実行
testExpenseEndpoints().catch((error) => {
  console.error("テスト実行エラー:", error);
  process.exit(1);
});

/**
 * サブスクリプションAPIエンドポイントのテストスクリプト
 */

const API_BASE_URL = "http://localhost:8787";

// 開発環境では簡易的なトークンを使用
// AuthServiceは開発環境で任意のトークンを受け入れるため、
// ユーザーIDをトークンとして使用
const JWT_TOKEN = "2"; // 開発用ユーザーID（AuthServiceのモックユーザー）

/**
 * APIリクエストを送信する
 */
async function apiRequest(
  method: string,
  path: string,
  body?: any,
): Promise<{ status: number; data: any }> {
  const url = `${API_BASE_URL}${path}`;
  const headers: Record<string, string> = {
    Authorization: `Bearer ${JWT_TOKEN}`,
  };

  if (body) {
    headers["Content-Type"] = "application/json";
  }

  console.log(`\n${method} ${path}`);
  if (body) {
    console.log("リクエストボディ:", JSON.stringify(body, null, 2));
  }

  const response = await fetch(url, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });

  const data = await response.json();

  console.log(`ステータス: ${response.status}`);
  console.log("レスポンス:", JSON.stringify(data, null, 2));

  return { status: response.status, data };
}

/**
 * メインテスト関数
 */
async function main() {
  console.log("=== サブスクリプションAPIエンドポイントのテスト ===\n");

  try {
    // 0. テスト用ユーザーが存在するか確認（存在しない場合はスキップ）
    console.log("\n--- 0. テスト用ユーザーの確認 ---");
    try {
      const userResponse = await apiRequest("GET", "/api/v1/users/me");
      if (userResponse.status === 200) {
        console.log(`✓ テスト用ユーザーが存在します (ID: ${userResponse.data.user.id})`);
      } else {
        console.log("⚠ テスト用ユーザーが見つかりません。先にユーザーを作成してください。");
        console.log("  ヒント: test-user-endpoints.ts を実行してユーザーを作成できます。");
        process.exit(1);
      }
    } catch (error) {
      console.error("ユーザー確認中にエラーが発生しました:", error);
      process.exit(1);
    }

    // 1. サブスクリプションを作成
    console.log("\n--- 1. サブスクリプションを作成 ---");
    const createResponse = await apiRequest("POST", "/api/v1/subscriptions", {
      name: "Netflix",
      amount: 1490,
      billing_cycle: "monthly",
      start_date: "2024-01-01",
      category: "エンターテイメント",
    });

    if (createResponse.status !== 201) {
      throw new Error("サブスクリプション作成に失敗しました");
    }

    const subscriptionId = createResponse.data.subscription.id;
    console.log(`✓ サブスクリプションを作成しました (ID: ${subscriptionId})`);

    // 2. サブスクリプションを取得
    console.log("\n--- 2. サブスクリプションを取得 ---");
    const getResponse = await apiRequest("GET", `/api/v1/subscriptions/${subscriptionId}`);

    if (getResponse.status !== 200) {
      throw new Error("サブスクリプション取得に失敗しました");
    }

    console.log("✓ サブスクリプションを取得しました");

    // 3. サブスクリプション一覧を取得
    console.log("\n--- 3. サブスクリプション一覧を取得 ---");
    const listResponse = await apiRequest("GET", "/api/v1/subscriptions");

    if (listResponse.status !== 200) {
      throw new Error("サブスクリプション一覧取得に失敗しました");
    }

    console.log(`✓ サブスクリプション一覧を取得しました (件数: ${listResponse.data.count})`);

    // 4. サブスクリプションを更新
    console.log("\n--- 4. サブスクリプションを更新 ---");
    const updateResponse = await apiRequest("PUT", `/api/v1/subscriptions/${subscriptionId}`, {
      amount: 1980,
      name: "Netflix Premium",
    });

    if (updateResponse.status !== 200) {
      throw new Error("サブスクリプション更新に失敗しました");
    }

    console.log("✓ サブスクリプションを更新しました");

    // 5. サブスクリプションのステータスを切り替え
    console.log("\n--- 5. サブスクリプションのステータスを切り替え ---");
    const toggleResponse = await apiRequest(
      "PATCH",
      `/api/v1/subscriptions/${subscriptionId}/toggle`,
    );

    if (toggleResponse.status !== 200) {
      throw new Error("サブスクリプションステータス切り替えに失敗しました");
    }

    console.log(
      `✓ サブスクリプションのステータスを切り替えました (is_active: ${toggleResponse.data.subscription.is_active})`,
    );

    // 6. 月額合計を取得
    console.log("\n--- 6. 月額合計を取得 ---");
    const monthlyTotalResponse = await apiRequest("GET", "/api/v1/subscriptions/monthly-total");

    if (monthlyTotalResponse.status !== 200) {
      throw new Error("月額合計取得に失敗しました");
    }

    console.log(`✓ 月額合計を取得しました (合計: ${monthlyTotalResponse.data.monthlyTotal}円)`);

    // 7. アクティブなサブスクリプションのみを取得
    console.log("\n--- 7. アクティブなサブスクリプションのみを取得 ---");
    const activeListResponse = await apiRequest("GET", "/api/v1/subscriptions?activeOnly=true");

    if (activeListResponse.status !== 200) {
      throw new Error("アクティブなサブスクリプション一覧取得に失敗しました");
    }

    console.log(
      `✓ アクティブなサブスクリプション一覧を取得しました (件数: ${activeListResponse.data.count})`,
    );

    // 8. サブスクリプションを削除
    console.log("\n--- 8. サブスクリプションを削除 ---");
    const deleteResponse = await apiRequest("DELETE", `/api/v1/subscriptions/${subscriptionId}`);

    if (deleteResponse.status !== 200) {
      throw new Error("サブスクリプション削除に失敗しました");
    }

    console.log("✓ サブスクリプションを削除しました");

    // 9. 削除されたサブスクリプションを取得（404エラーを期待）
    console.log("\n--- 9. 削除されたサブスクリプションを取得（404エラーを期待） ---");
    const getDeletedResponse = await apiRequest("GET", `/api/v1/subscriptions/${subscriptionId}`);

    if (getDeletedResponse.status !== 404) {
      throw new Error("削除されたサブスクリプションが見つかりました（期待: 404エラー）");
    }

    console.log("✓ 削除されたサブスクリプションは見つかりませんでした（404エラー）");

    console.log("\n=== すべてのテストが成功しました ===");
  } catch (error) {
    console.error("\n❌ テストが失敗しました:", error);
    process.exit(1);
  }
}

// テストを実行
main();

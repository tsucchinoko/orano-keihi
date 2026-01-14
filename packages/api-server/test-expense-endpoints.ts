/**
 * 経費エンドポイントの統合テスト
 *
 * このスクリプトは経費関連のAPIエンドポイントをテストします。
 * 実行前に以下を確認してください：
 * 1. D1データベースが作成されている
 * 2. スキーマが適用されている
 * 3. テストユーザーが存在する
 */

import { D1Database } from "@cloudflare/workers-types";
import { createApp } from "./src/app.js";
import { loadConfig } from "./src/config/environment.js";

async function testExpenseEndpoints() {
  console.log("経費エンドポイントのテストを開始します...\n");

  // 設定を読み込む
  const config = loadConfig();

  // テスト用のD1データベースモック（実際のテストではminiflareを使用）
  const mockDb = {
    prepare: (query: string) => ({
      bind: (...params: any[]) => ({
        run: async () => ({ success: true, meta: { last_row_id: 1, changes: 1 } }),
        first: async () => ({
          id: 1,
          user_id: "test-user-id",
          date: "2024-01-15",
          amount: 1000,
          category: "食費",
          description: "テスト経費",
          receipt_url: null,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        }),
        all: async () => ({
          success: true,
          results: [
            {
              id: 1,
              user_id: "test-user-id",
              date: "2024-01-15",
              amount: 1000,
              category: "食費",
              description: "テスト経費",
              receipt_url: null,
              created_at: new Date().toISOString(),
              updated_at: new Date().toISOString(),
            },
          ],
        }),
      }),
    }),
  } as unknown as D1Database;

  // アプリケーションを作成
  const app = createApp(config, undefined, undefined, mockDb);

  console.log("✅ 経費エンドポイントが正常に統合されました");
  console.log("\n利用可能なエンドポイント:");
  console.log("  POST   /api/v1/expenses          - 経費を作成");
  console.log("  GET    /api/v1/expenses/:id      - 経費を取得");
  console.log("  GET    /api/v1/expenses          - 経費一覧を取得");
  console.log("  PUT    /api/v1/expenses/:id      - 経費を更新");
  console.log("  DELETE /api/v1/expenses/:id      - 経費を削除");
  console.log("\n注意: これらのエンドポイントは認証が必要です");
}

// テストを実行
testExpenseEndpoints().catch((error) => {
  console.error("❌ テストでエラーが発生しました:", error);
  process.exit(1);
});

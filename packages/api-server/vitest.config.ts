/**
 * Vitestの設定ファイル
 * テスト環境の設定とプロパティベーステストの設定
 */

import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // テスト環境の設定
    environment: "node",

    // グローバル設定
    globals: true,

    // テストファイルのパターン
    include: ["src/**/*.test.ts", "tests/**/*.test.ts"],

    // カバレッジ設定
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "html"],
      exclude: ["node_modules/", "dist/", "**/*.d.ts", "**/*.config.*", "tests/fixtures/**"],
    },

    // タイムアウト設定
    testTimeout: 10000,

    // プロパティベーステストの設定
    // fast-checkのテストは最小100回実行
    setupFiles: ["./tests/setup.ts"],
  },
});

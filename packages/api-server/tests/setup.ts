/**
 * テストセットアップファイル
 * プロパティベーステストの設定とテスト環境の初期化
 */

import { beforeAll, afterAll } from "vitest";
import { config } from "dotenv";

// テスト用環境変数を読み込み
beforeAll(() => {
  config({ path: ".env.test" });

  // テスト用の環境変数を設定
  process.env.NODE_ENV = "test";
  process.env.LOG_LEVEL = "error"; // テスト中はエラーログのみ
});

afterAll(() => {
  // テスト後のクリーンアップ
});

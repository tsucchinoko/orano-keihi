/**
 * UserRepositoryの動作確認スクリプト
 *
 * 実行方法:
 * pnpm wrangler dev --local --test-scheduled
 *
 * または、直接D1データベースに接続してテスト
 */

import { UserRepository } from "./src/repositories/user-repository.js";
import type { GoogleUser } from "./src/types/d1-dtos.js";

// このスクリプトは、wrangler dev環境で実行する必要があります
// 実際のテストは、Miniflareを使用したテスト環境で実行します

console.log("UserRepositoryのテストスクリプト");
console.log("このスクリプトは、wrangler dev環境で実行する必要があります");
console.log("実際のテストは、vitest + Miniflareを使用して実行してください");

// テスト用のGoogleユーザー情報
const testGoogleUser: GoogleUser = {
  google_id: "test-google-id-12345",
  email: "test@example.com",
  name: "テストユーザー",
  picture_url: "https://example.com/avatar.jpg",
};

console.log("テスト用Googleユーザー情報:", testGoogleUser);

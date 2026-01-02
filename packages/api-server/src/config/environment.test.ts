/**
 * 環境設定のテスト
 * Feature: api-server-integration, Property 1: CORS設定の適切性
 */

import { describe, it, expect, beforeEach } from "vitest";
import { loadConfig } from "./environment.js";

describe("環境設定", () => {
  beforeEach(() => {
    // テスト用環境変数をリセット
    delete process.env.PORT;
    delete process.env.HOST;
    delete process.env.NODE_ENV;
  });

  it("デフォルト値が正しく設定される", () => {
    // 必須の環境変数を設定
    process.env.R2_ENDPOINT = "https://test.r2.cloudflarestorage.com";
    process.env.R2_ACCESS_KEY_ID = "test-key";
    process.env.R2_SECRET_ACCESS_KEY = "test-secret";
    process.env.R2_BUCKET_NAME = "test-bucket";
    process.env.JWT_SECRET = "test-jwt-secret";

    const config = loadConfig();

    expect(config.port).toBe(3000);
    expect(config.host).toBe("localhost");
    expect(config.nodeEnv).toBe("development");
  });

  it("環境変数が正しく読み込まれる", () => {
    // 環境変数を設定
    process.env.PORT = "8080";
    process.env.HOST = "0.0.0.0";
    process.env.NODE_ENV = "production";
    process.env.R2_ENDPOINT = "https://prod.r2.cloudflarestorage.com";
    process.env.R2_ACCESS_KEY_ID = "prod-key";
    process.env.R2_SECRET_ACCESS_KEY = "prod-secret";
    process.env.R2_BUCKET_NAME = "prod-bucket";
    process.env.JWT_SECRET = "prod-jwt-secret";

    const config = loadConfig();

    expect(config.port).toBe(8080);
    expect(config.host).toBe("0.0.0.0");
    expect(config.nodeEnv).toBe("production");
    expect(config.r2.endpoint).toBe("https://prod.r2.cloudflarestorage.com");
  });

  it("CORS設定が正しく解析される", () => {
    // 必須の環境変数を設定
    process.env.R2_ENDPOINT = "https://test.r2.cloudflarestorage.com";
    process.env.R2_ACCESS_KEY_ID = "test-key";
    process.env.R2_SECRET_ACCESS_KEY = "test-secret";
    process.env.R2_BUCKET_NAME = "test-bucket";
    process.env.JWT_SECRET = "test-jwt-secret";
    process.env.CORS_ORIGIN = "http://localhost:3000,https://example.com";

    const config = loadConfig();

    expect(config.cors.origin).toEqual(["http://localhost:3000", "https://example.com"]);
    expect(config.cors.methods).toContain("GET");
    expect(config.cors.methods).toContain("POST");
    expect(config.cors.headers).toContain("Content-Type");
    expect(config.cors.headers).toContain("Authorization");
  });
});

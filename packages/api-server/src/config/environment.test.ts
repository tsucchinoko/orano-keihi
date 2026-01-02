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
    // 必須の環境変数を設定（有効な形式）
    process.env.R2_ENDPOINT = "https://d6392b1230a419b37b30f45fc13de9cf.r2.cloudflarestorage.com";
    process.env.R2_ACCESS_KEY_ID = "fae3b529bd5d1a59e862e9a7f645e343"; // 32文字の有効なキー
    process.env.R2_SECRET_ACCESS_KEY =
      "39d074627acee325e3b1a023b80f64194a8cad19bcec574de7232c615564e59d"; // 64文字の有効なシークレット
    process.env.R2_BUCKET_NAME = "test-bucket";
    process.env.JWT_SECRET = "test-jwt-secret-key-for-testing-purposes";

    const config = loadConfig();

    expect(config.port).toBe(3000);
    expect(config.host).toBe("localhost");
    expect(config.nodeEnv).toBe("development");
  });

  it("環境変数が正しく読み込まれる", () => {
    // 環境変数を設定（有効な形式）
    process.env.PORT = "8080";
    process.env.HOST = "0.0.0.0";
    process.env.NODE_ENV = "production";
    process.env.R2_ENDPOINT = "https://d6392b1230a419b37b30f45fc13de9cf.r2.cloudflarestorage.com";
    process.env.R2_ACCESS_KEY_ID = "fae3b529bd5d1a59e862e9a7f645e343"; // 32文字の有効なキー
    process.env.R2_SECRET_ACCESS_KEY =
      "39d074627acee325e3b1a023b80f64194a8cad19bcec574de7232c615564e59d"; // 64文字の有効なシークレット
    process.env.R2_BUCKET_NAME = "prod-bucket";
    process.env.JWT_SECRET = "prod-jwt-secret-key-for-testing-purposes";

    const config = loadConfig();

    expect(config.port).toBe(8080);
    expect(config.host).toBe("0.0.0.0");
    expect(config.nodeEnv).toBe("production");
    expect(config.r2.endpoint).toBe(
      "https://d6392b1230a419b37b30f45fc13de9cf.r2.cloudflarestorage.com",
    );
  });

  it("CORS設定が正しく解析される", () => {
    // 必須の環境変数を設定（有効な形式）
    process.env.R2_ENDPOINT = "https://d6392b1230a419b37b30f45fc13de9cf.r2.cloudflarestorage.com";
    process.env.R2_ACCESS_KEY_ID = "fae3b529bd5d1a59e862e9a7f645e343"; // 32文字の有効なキー
    process.env.R2_SECRET_ACCESS_KEY =
      "39d074627acee325e3b1a023b80f64194a8cad19bcec574de7232c615564e59d"; // 64文字の有効なシークレット
    process.env.R2_BUCKET_NAME = "test-bucket";
    process.env.JWT_SECRET = "test-jwt-secret-key-for-testing-purposes";
    process.env.CORS_ORIGIN = "http://localhost:3000,https://example.com";

    const config = loadConfig();

    expect(config.cors.origin).toEqual(["http://localhost:3000", "https://example.com"]);
    expect(config.cors.methods).toContain("GET");
    expect(config.cors.methods).toContain("POST");
    expect(config.cors.headers).toContain("Content-Type");
    expect(config.cors.headers).toContain("Authorization");
  });
});

import { Hono } from "hono";
import { z } from "zod";
import { zValidator } from "@hono/zod-validator";
import { SignJWT } from "jose";
import type { Env } from "../worker.js";

// リクエストスキーマ
const AuthStartRequestSchema = z.object({
  redirect_uri: z.string().url(),
});

const AuthCallbackRequestSchema = z.object({
  code: z.string(),
  state: z.string(),
  code_verifier: z.string(),
  redirect_uri: z.string().url(),
});

// レスポンス型
interface AuthStartResponse {
  auth_url: string;
  state: string;
  code_verifier: string;
}

interface AuthCallbackResponse {
  access_token: string;
  token_type: string;
  expires_in: number;
  user: {
    id: string;
    email: string;
    name: string;
    picture?: string;
  };
}

// Google OAuth トークンレスポンス型
interface GoogleTokenResponse {
  access_token: string;
  expires_in: number;
  refresh_token?: string;
  scope: string;
  token_type: string;
  id_token?: string;
}

// Google ユーザー情報レスポンス型
interface GoogleUserInfo {
  id: string;
  email: string;
  verified_email: boolean;
  name: string;
  given_name?: string;
  family_name?: string;
  picture?: string;
  locale?: string;
}

// ランダム文字列生成（PKCE用）
function generateRandomString(length: number): string {
  const charset = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
  const randomValues = new Uint8Array(length);
  crypto.getRandomValues(randomValues);
  return Array.from(randomValues)
    .map((x) => charset[x % charset.length])
    .join("");
}

// SHA256ハッシュ生成
async function sha256(plain: string): Promise<ArrayBuffer> {
  const encoder = new TextEncoder();
  const data = encoder.encode(plain);
  return await crypto.subtle.digest("SHA-256", data);
}

// Base64 URL エンコード
function base64UrlEncode(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
}

// PKCE code_challenge生成
async function generateCodeChallenge(codeVerifier: string): Promise<string> {
  const hashed = await sha256(codeVerifier);
  return base64UrlEncode(hashed);
}

const app = new Hono<{ Bindings: Env }>();

// 認証フロー開始エンドポイント
app.post("/google/start", zValidator("json", AuthStartRequestSchema), async (c) => {
  try {
    const { redirect_uri } = c.req.valid("json");

    // 環境変数からGoogle OAuth設定を取得
    const clientId = c.env.GOOGLE_CLIENT_ID;
    const clientSecret = c.env.GOOGLE_CLIENT_SECRET;

    if (!clientId || !clientSecret) {
      return c.json({ error: "Google OAuth設定が不完全です" }, 500);
    }

    // PKCE パラメータ生成
    const codeVerifier = generateRandomString(128);
    const codeChallenge = await generateCodeChallenge(codeVerifier);
    const state = generateRandomString(32);

    // Google OAuth認証URLを構築
    const authUrl = new URL("https://accounts.google.com/o/oauth2/v2/auth");
    authUrl.searchParams.set("client_id", clientId);
    authUrl.searchParams.set("redirect_uri", redirect_uri);
    authUrl.searchParams.set("response_type", "code");
    authUrl.searchParams.set("scope", "openid email profile");
    authUrl.searchParams.set("state", state);
    authUrl.searchParams.set("code_challenge", codeChallenge);
    authUrl.searchParams.set("code_challenge_method", "S256");
    authUrl.searchParams.set("access_type", "offline");
    authUrl.searchParams.set("prompt", "consent");

    const response: AuthStartResponse = {
      auth_url: authUrl.toString(),
      state,
      code_verifier: codeVerifier,
    };

    return c.json(response);
  } catch (error) {
    console.error("認証開始エラー:", error);
    return c.json({ error: "認証開始に失敗しました" }, 500);
  }
});

// 認証コールバック処理エンドポイント
app.post("/google/callback", zValidator("json", AuthCallbackRequestSchema), async (c) => {
  try {
    const { code, state, code_verifier, redirect_uri } = c.req.valid("json");

    // 環境変数からGoogle OAuth設定を取得
    const clientId = c.env.GOOGLE_CLIENT_ID;
    const clientSecret = c.env.GOOGLE_CLIENT_SECRET;
    const jwtSecret = c.env.JWT_SECRET;

    if (!clientId || !clientSecret || !jwtSecret) {
      return c.json({ error: "サーバー設定が不完全です" }, 500);
    }

    // Googleトークンエンドポイントにリクエスト
    const tokenResponse = await fetch("https://oauth2.googleapis.com/token", {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: new URLSearchParams({
        code,
        client_id: clientId,
        client_secret: clientSecret,
        redirect_uri,
        grant_type: "authorization_code",
        code_verifier,
      }),
    });

    if (!tokenResponse.ok) {
      const errorText = await tokenResponse.text();
      console.error("トークン交換エラー:", errorText);
      return c.json({ error: "トークン交換に失敗しました" }, 400);
    }

    const tokenData = (await tokenResponse.json()) as GoogleTokenResponse;
    const accessToken = tokenData.access_token;

    // Googleユーザー情報を取得
    const userInfoResponse = await fetch("https://www.googleapis.com/oauth2/v2/userinfo", {
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });

    if (!userInfoResponse.ok) {
      return c.json({ error: "ユーザー情報の取得に失敗しました" }, 400);
    }

    const userInfo = (await userInfoResponse.json()) as GoogleUserInfo;

    // メール認証済みかチェック
    if (!userInfo.verified_email) {
      return c.json({ error: "メールアドレスが認証されていません" }, 400);
    }

    // JWT トークンを生成（1時間有効）
    const jwtSecretKey = new TextEncoder().encode(jwtSecret);

    const jwt = await new SignJWT({
      sub: userInfo.id,
      email: userInfo.email,
      name: userInfo.name,
      picture: userInfo.picture,
    })
      .setProtectedHeader({ alg: "HS256", typ: "JWT" })
      .setIssuedAt()
      .setExpirationTime("1h") // 1時間後に期限切れ
      .sign(jwtSecretKey);

    const response: AuthCallbackResponse = {
      access_token: jwt,
      token_type: "Bearer",
      expires_in: 3600,
      user: {
        id: userInfo.id,
        email: userInfo.email,
        name: userInfo.name,
        picture: userInfo.picture,
      },
    };

    return c.json(response);
  } catch (error) {
    console.error("認証コールバックエラー:", error);
    return c.json({ error: "認証処理に失敗しました" }, 500);
  }
});

export default app;

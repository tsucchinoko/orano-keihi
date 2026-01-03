import { Hono } from "hono";
import { cors } from "hono/cors";

// アップデート情報の型定義
interface UpdateManifest {
  version: string;
  notes: string;
  pub_date: string;
  platforms: {
    [key: string]: {
      signature: string;
      url: string;
      with_elevated_task?: boolean;
    };
  };
}

// 最新バージョン情報（実際の運用では外部ストレージやデータベースから取得）
const LATEST_VERSION = "0.2.0";
const RELEASE_NOTES = `
## バージョン 0.2.0 の新機能

### 新機能
- 自動アップデート機能を追加
- アップデート通知UI を実装
- バックグラウンドでの定期アップデートチェック

### 改善
- パフォーマンスの向上
- UI/UXの改善
- セキュリティの強化

### バグ修正
- 軽微なバグの修正
- 安定性の向上
`;

// プラットフォーム別のダウンロードURL（実際の運用では適切なURLに変更）
const DOWNLOAD_URLS = {
  "darwin-x86_64":
    "https://github.com/your-org/your-app/releases/download/v0.2.0/your-app_0.2.0_x64.dmg",
  "darwin-aarch64":
    "https://github.com/your-org/your-app/releases/download/v0.2.0/your-app_0.2.0_aarch64.dmg",
  "windows-x86_64":
    "https://github.com/your-org/your-app/releases/download/v0.2.0/your-app_0.2.0_x64-setup.exe",
  "linux-x86_64":
    "https://github.com/your-org/your-app/releases/download/v0.2.0/your-app_0.2.0_amd64.AppImage",
};

// プラットフォーム別の署名（実際の運用では適切な署名を設定）
const SIGNATURES = {
  "darwin-x86_64":
    "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUldSVE1qVXhNVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeA==",
  "darwin-aarch64":
    "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUldSVE1qVXhNVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeA==",
  "windows-x86_64":
    "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUldSVE1qVXhNVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeA==",
  "linux-x86_64":
    "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUldSVE1qVXhNVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeE5qVXhOVEV4TlRJeA==",
};

const updaterApp = new Hono();

// CORS設定
updaterApp.use(
  "*",
  cors({
    origin: ["http://localhost:1420", "tauri://localhost"],
    allowMethods: ["GET", "POST", "PUT", "DELETE", "OPTIONS"],
    allowHeaders: ["Content-Type", "Authorization"],
  }),
);

/**
 * バージョン比較関数
 * @param version1 バージョン1
 * @param version2 バージョン2
 * @returns version1 > version2 なら 1、version1 < version2 なら -1、同じなら 0
 */
function compareVersions(version1: string, version2: string): number {
  const v1Parts = version1.split(".").map(Number);
  const v2Parts = version2.split(".").map(Number);

  const maxLength = Math.max(v1Parts.length, v2Parts.length);

  for (let i = 0; i < maxLength; i++) {
    const v1Part = v1Parts[i] || 0;
    const v2Part = v2Parts[i] || 0;

    if (v1Part > v2Part) return 1;
    if (v1Part < v2Part) return -1;
  }

  return 0;
}

/**
 * アップデートチェックエンドポイント
 * パス: /api/updater/{target}/{arch}/{current_version}
 */
updaterApp.get("/:target/:arch/:current_version", async (c) => {
  try {
    const target = c.req.param("target");
    const arch = c.req.param("arch");
    const currentVersion = c.req.param("current_version");

    console.log(`アップデートチェック: target=${target}, arch=${arch}, current=${currentVersion}`);

    // プラットフォーム識別子を構築
    const platformKey = `${target}-${arch}`;

    // サポートされているプラットフォームかチェック
    if (!DOWNLOAD_URLS[platformKey as keyof typeof DOWNLOAD_URLS]) {
      console.log(`サポートされていないプラットフォーム: ${platformKey}`);
      return c.json({ error: "Unsupported platform" }, 400);
    }

    // バージョン比較
    const versionComparison = compareVersions(LATEST_VERSION, currentVersion);

    if (versionComparison <= 0) {
      // 最新バージョンまたは現在のバージョンの方が新しい場合
      console.log(`アップデート不要: latest=${LATEST_VERSION}, current=${currentVersion}`);
      return c.body(null, 204); // No Content
    }

    // アップデートが利用可能な場合
    const updateManifest: UpdateManifest = {
      version: LATEST_VERSION,
      notes: RELEASE_NOTES.trim(),
      pub_date: new Date().toISOString(),
      platforms: {
        [platformKey]: {
          signature: SIGNATURES[platformKey as keyof typeof SIGNATURES],
          url: DOWNLOAD_URLS[platformKey as keyof typeof DOWNLOAD_URLS],
          with_elevated_task: target === "windows", // Windowsの場合は管理者権限が必要
        },
      },
    };

    console.log(`アップデート利用可能: ${currentVersion} -> ${LATEST_VERSION}`);
    return c.json(updateManifest);
  } catch (error) {
    console.error("アップデートチェックエラー:", error);
    return c.json({ error: "Internal server error" }, 500);
  }
});

/**
 * 最新バージョン情報取得エンドポイント
 */
updaterApp.get("/latest", async (c) => {
  try {
    return c.json({
      version: LATEST_VERSION,
      notes: RELEASE_NOTES.trim(),
      pub_date: new Date().toISOString(),
      platforms: Object.keys(DOWNLOAD_URLS),
    });
  } catch (error) {
    console.error("最新バージョン情報取得エラー:", error);
    return c.json({ error: "Internal server error" }, 500);
  }
});

/**
 * ヘルスチェックエンドポイント
 */
updaterApp.get("/health", async (c) => {
  return c.json({
    status: "ok",
    service: "updater",
    timestamp: new Date().toISOString(),
    latest_version: LATEST_VERSION,
  });
});

export { updaterApp };

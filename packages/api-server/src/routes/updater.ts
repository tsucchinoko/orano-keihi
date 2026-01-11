import { Hono } from "hono";
import { cors } from "hono/cors";

// Cloudflare Workers環境変数の型定義
type Bindings = {
  GITHUB_TOKEN: string;
};

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

const updaterApp = new Hono<{ Bindings: Bindings }>();

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
 * GitHubプライベートリポジトリからマニフェストファイルを取得するプロキシエンドポイント
 * パス: /api/updater/manifest/{target}/{arch}
 */
updaterApp.get("/manifest/:target/:arch", async (c) => {
  try {
    const target = c.req.param("target");
    const arch = c.req.param("arch");

    console.log(`マニフェスト取得: target=${target}, arch=${arch}`);

    // GitHub Personal Access Token（環境変数から取得）
    const githubToken = c.env?.GITHUB_TOKEN;

    if (!githubToken) {
      console.error("GITHUB_TOKENが設定されていません");
      return c.json({ error: "Server configuration error" }, 500);
    }

    // GitHubリリースアセットのURL（GitHub API経由）
    const manifestFileName = `${target}-${arch}.json`;
    const owner = "tsucchinoko";
    const repo = "orano-keihi";

    // まず全リリースを取得
    const releasesUrl = `https://api.github.com/repos/${owner}/${repo}/releases`;
    console.log(`リリース一覧を取得: ${releasesUrl}`);

    const releasesResponse = await fetch(releasesUrl, {
      headers: {
        Authorization: `Bearer ${githubToken}`,
        Accept: "application/vnd.github+json",
        "User-Agent": "Orano-Keihi-Updater",
        "X-GitHub-Api-Version": "2022-11-28",
      },
    });

    if (!releasesResponse.ok) {
      console.error(
        `リリース一覧取得エラー: ${releasesResponse.status} ${releasesResponse.statusText}`,
      );
      const errorText = await releasesResponse.text();
      console.error(`エラー詳細: ${errorText}`);
      return c.json(
        { error: "Failed to fetch releases from GitHub" },
        releasesResponse.status as 404 | 500,
      );
    }

    const releases = (await releasesResponse.json()) as Array<{
      tag_name: string;
      assets: Array<{
        id: number;
        name: string;
        browser_download_url: string;
      }>;
    }>;

    console.log(`取得したリリース数: ${releases.length}`);

    // 最新のリリースからマニフェストファイルを探す
    let asset: { id: number; name: string; browser_download_url: string } | undefined;
    let foundRelease: string | undefined;

    for (const release of releases) {
      asset = release.assets.find((a) => a.name === manifestFileName);
      if (asset) {
        foundRelease = release.tag_name;
        break;
      }
    }

    if (!asset || !foundRelease) {
      console.error(`マニフェストファイルが見つかりません: ${manifestFileName}`);
      return c.json({ error: "Manifest file not found in any release" }, 404);
    }

    console.log(`マニフェストアセットを取得: ${asset.name} (ID: ${asset.id}) from ${foundRelease}`);

    // アセットをダウンロード
    const assetUrl = `https://api.github.com/repos/${owner}/${repo}/releases/assets/${asset.id}`;
    const response = await fetch(assetUrl, {
      headers: {
        Authorization: `Bearer ${githubToken}`,
        Accept: "application/octet-stream",
        "User-Agent": "Orano-Keihi-Updater",
        "X-GitHub-Api-Version": "2022-11-28",
      },
    });

    if (!response.ok) {
      console.error(`GitHub APIエラー: ${response.status} ${response.statusText}`);
      return c.json(
        { error: "Failed to fetch manifest from GitHub" },
        response.status as 404 | 500,
      );
    }

    const manifestData = (await response.json()) as UpdateManifest;
    console.log(`マニフェスト取得成功: version=${manifestData.version}`);

    // URLをAPI Server経由のURLに書き換え
    const modifiedManifest = { ...manifestData };
    for (const platform in modifiedManifest.platforms) {
      const platformData = modifiedManifest.platforms[platform];
      if (platformData && platformData.url) {
        // GitHub URLからファイル名を抽出
        const urlParts = platformData.url.split("/");
        const fileName = urlParts[urlParts.length - 1];
        const version = urlParts[urlParts.length - 2];

        // API Server経由のURLに書き換え
        const baseUrl = new URL(c.req.url).origin;
        platformData.url = `${baseUrl}/api/updater/download/${version}/${fileName}`;
        console.log(`URL書き換え: ${platform} -> ${platformData.url}`);
      }
    }

    // マニフェストデータを返す
    return c.json(modifiedManifest);
  } catch (error) {
    console.error("マニフェスト取得エラー:", error);
    return c.json({ error: "Internal server error" }, 500);
  }
});

/**
 * GitHubプライベートリポジトリからインストーラーファイルをダウンロードするプロキシエンドポイント
 * パス: /api/updater/download/{version}/{filename}
 */
updaterApp.get("/download/:version/:filename", async (c) => {
  try {
    const version = c.req.param("version");
    const filename = c.req.param("filename");

    console.log(`インストーラーダウンロード: version=${version}, filename=${filename}`);

    // GitHub Personal Access Token（環境変数から取得）
    const githubToken = c.env?.GITHUB_TOKEN;

    if (!githubToken) {
      console.error("GITHUB_TOKENが設定されていません");
      return c.json({ error: "Server configuration error" }, 500);
    }

    const owner = "tsucchinoko";
    const repo = "orano-keihi";

    // リリース情報を取得
    const releaseUrl = `https://api.github.com/repos/${owner}/${repo}/releases/tags/${version}`;
    console.log(`リリース情報を取得: ${releaseUrl}`);

    const releaseResponse = await fetch(releaseUrl, {
      headers: {
        Authorization: `Bearer ${githubToken}`,
        Accept: "application/vnd.github+json",
        "User-Agent": "Orano-Keihi-Updater",
        "X-GitHub-Api-Version": "2022-11-28",
      },
    });

    if (!releaseResponse.ok) {
      console.error(`リリース取得エラー: ${releaseResponse.status} ${releaseResponse.statusText}`);
      return c.json(
        { error: "Failed to fetch release from GitHub" },
        releaseResponse.status as 404 | 500,
      );
    }

    const release = (await releaseResponse.json()) as {
      assets: Array<{
        id: number;
        name: string;
        size: number;
        content_type: string;
      }>;
    };

    // 指定されたファイル名のアセットを探す
    const asset = release.assets.find((a) => a.name === filename);

    if (!asset) {
      console.error(`アセットが見つかりません: ${filename}`);
      return c.json({ error: "Asset not found in release" }, 404);
    }

    console.log(`アセットを取得: ${asset.name} (ID: ${asset.id}, Size: ${asset.size} bytes)`);

    // アセットをダウンロード
    const assetUrl = `https://api.github.com/repos/${owner}/${repo}/releases/assets/${asset.id}`;
    const response = await fetch(assetUrl, {
      headers: {
        Authorization: `Bearer ${githubToken}`,
        Accept: "application/octet-stream",
        "User-Agent": "Orano-Keihi-Updater",
        "X-GitHub-Api-Version": "2022-11-28",
      },
    });

    if (!response.ok) {
      console.error(`GitHub APIエラー: ${response.status} ${response.statusText}`);
      return c.json(
        { error: "Failed to download asset from GitHub" },
        response.status as 404 | 500,
      );
    }

    console.log(`ダウンロード成功: ${filename}`);

    // ファイルをストリーミングで返す
    return new Response(response.body, {
      headers: {
        "Content-Type": asset.content_type || "application/octet-stream",
        "Content-Length": asset.size.toString(),
        "Content-Disposition": `attachment; filename="${filename}"`,
        "Cache-Control": "public, max-age=31536000", // 1年間キャッシュ
      },
    });
  } catch (error) {
    console.error("ダウンロードエラー:", error);
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
  });
});

export { updaterApp };

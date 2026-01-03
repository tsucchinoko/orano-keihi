import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig, loadEnv } from 'vite';

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
  // 環境変数を読み込み（src-tauriディレクトリから）
  const env = loadEnv(mode, './src-tauri', '');

  return {
    plugins: [tailwindcss(), sveltekit()],

    // 環境変数をクライアントサイドで使用可能にする
    define: {
      'import.meta.env.R2_ACCOUNT_ID': JSON.stringify(env.R2_ACCOUNT_ID),
      'import.meta.env.R2_ACCESS_KEY': JSON.stringify(env.R2_ACCESS_KEY),
      'import.meta.env.R2_SECRET_KEY': JSON.stringify(env.R2_SECRET_KEY),
      'import.meta.env.R2_BUCKET_NAME': JSON.stringify(env.R2_BUCKET_NAME),
      'import.meta.env.R2_REGION': JSON.stringify(env.R2_REGION),
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      hmr: host
        ? {
            protocol: 'ws',
            host,
            port: 1421,
          }
        : undefined,
      watch: {
        // 3. tell Vite to ignore watching `src-tauri`
        ignored: ['**/src-tauri/**'],
      },
    },
  };
});

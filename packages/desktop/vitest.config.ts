import path from 'path';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [svelte()],
  test: {
    globals: true,
    environment: 'jsdom',
  },
  resolve: {
    alias: {
      $lib: path.resolve('./src/lib'),
    },
  },
});

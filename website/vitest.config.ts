import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

// Vitest config for the website. Astro's own Vite config isn't reused here;
// component tests don't go through Astro's pipeline, just plain Vite + React.
// The `@/*` alias mirrors tsconfig.json so imports resolve identically in
// tests and Astro builds.
export default defineConfig({
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  test: {
    environment: 'jsdom',
    globals: false,
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
  },
});

import react from '@astrojs/react';
import { defineConfig } from 'astro/config';

// Tailwind 4 is wired in via PostCSS (`postcss.config.mjs`) rather than
// `@tailwindcss/vite`, because the Vite plugin (4.3.0) is incompatible with
// the Rolldown-based Vite 8 that Astro 6 ships. See postcss.config.mjs.
export default defineConfig({
  output: 'static',
  integrations: [react()],
  vite: {
    // Dev-only proxy: astro dev serves on :4321, the Rust backend on :8080.
    // The ConverterForm posts to /api/v2/convert on same-origin in production
    // (Rust serves both Astro static + the API), so we forward /api/* during
    // local dev to match that contract. The backend's port is overridable via
    // BACKEND_URL for non-default setups.
    server: {
      proxy: {
        '/api': {
          target: process.env.BACKEND_URL ?? 'http://localhost:8080',
          changeOrigin: true,
        },
      },
    },
  },
});

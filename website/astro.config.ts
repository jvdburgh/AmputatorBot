import react from '@astrojs/react';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'astro/config';

// Tailwind 4 is wired in via `@tailwindcss/vite` (the Tailwind-recommended
// integration). It used to go through PostCSS because the Vite plugin (4.3.0)
// was incompatible with Rolldown-based Vite 8 in Astro 6; with Astro 7 the
// situation inverted — Vite's native CSS pipeline stopped resolving the bare
// `@import "tailwindcss"` under PostCSS, while the plugin gained Rolldown
// support.
export default defineConfig({
  output: 'static',
  integrations: [react()],
  vite: {
    plugins: [tailwindcss()],
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

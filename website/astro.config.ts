import react from '@astrojs/react';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'astro/config';

// Note: `@tailwindcss/vite`'s Plugin type doesn't perfectly match Astro's
// bundled Vite version yet (Astro 5.18 + Tailwind 4.3 + Vite 7). The build
// works fine; the type cast silences `astro check` until those align.
export default defineConfig({
  output: 'static',
  integrations: [react()],
  vite: {
    plugins: [
      // biome-ignore lint/suspicious/noExplicitAny: vite plugin type mismatch, see note above
      tailwindcss() as any,
    ],
  },
});

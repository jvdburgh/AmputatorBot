import { devvit } from '@devvit/start/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    // Per Devvit docs: the plugin must come last.
    devvit(),
  ],
});

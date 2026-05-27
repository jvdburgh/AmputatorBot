// Tailwind 4 is wired in via PostCSS here (rather than `@tailwindcss/vite`)
// because that Vite plugin is currently incompatible with the Rolldown-based
// Vite 8 that Astro 6 ships. The @tailwindcss/postcss plugin handles both
// the @import "tailwindcss" resolution and the source-scan that generates
// the utility-class layer.
export default {
  plugins: {
    '@tailwindcss/postcss': {},
  },
};

import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://piducan.dev',
  base: '/buffer-uppercut',
  output: 'static',
  trailingSlash: 'always',
});

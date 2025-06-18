// @ts-check
import { defineConfig } from 'astro/config';

import sentry from '@sentry/astro';
import spotlightjs from '@spotlightjs/astro';

import tailwindcss from '@tailwindcss/vite';

// https://astro.build/config
export default defineConfig({
  integrations: [sentry(), spotlightjs()],

  vite: {
    plugins: [tailwindcss()]
  }
});
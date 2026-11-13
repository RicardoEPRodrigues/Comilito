import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// vitePreprocess lets <script lang="ts"> in .svelte files work.
export default {
  preprocess: vitePreprocess(),
};

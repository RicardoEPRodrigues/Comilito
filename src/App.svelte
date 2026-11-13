<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  // Asking Rust for the version proves the UI -> Rust command bridge works.
  let version = $state("…");

  invoke<string>("app_version")
    .then((v) => (version = v))
    .catch((err) => (version = `error: ${err}`));
</script>

<main>
  <h1>Comilito</h1>
  <p>Version {version}</p>
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: system-ui, sans-serif;
    background: #1b1b1f;
    color: #e6e6e6;
  }

  main {
    padding: 2rem;
  }
</style>

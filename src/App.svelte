<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import VolumeSlider from "./VolumeSlider.svelte";

  // Must match the Rust `Category` enum's JSON names (src-tauri/src/project.rs).
  type Category = "music" | "ambience" | "effects";

  const categories: { id: Category; label: string }[] = [
    { id: "music", label: "Music" },
    { id: "ambience", label: "Ambience" },
    { id: "effects", label: "Effects" },
  ];

  // Same shape as the Rust `Volumes` struct.
  interface Volumes {
    master: number;
    music: number;
    ambience: number;
    effects: number;
  }

  // Asking Rust for the version proves the UI -> Rust command bridge works.
  let version = $state("…");

  invoke<string>("app_version")
    .then((v) => (version = v))
    .catch((err) => (version = `error: ${err}`));

  // Rust owns the real volumes; we fetch them once and then send every change back.
  // `null` until the first answer arrives, so the sliders don't flash a wrong position.
  let volumes = $state<Volumes | null>(null);

  invoke<Volumes>("get_volumes")
    .then((v) => (volumes = v))
    .catch((err) => (error = String(err)));

  function setMaster(v: number) {
    volumes!.master = v;
    run("set_master_volume", { volume: v });
  }

  function setCategory(category: Category, v: number) {
    volumes![category] = v;
    run("set_category_volume", { category, volume: v });
  }

  // Temporary test controls until there's a real sound library.
  let path = $state("");
  let playCategory = $state<Category>("music");
  let error = $state("");

  // When a Rust command returns Err(message), invoke() rejects with that message.
  async function run(command: string, args?: Record<string, unknown>) {
    error = "";
    try {
      await invoke(command, args);
    } catch (err) {
      error = String(err);
    }
  }
</script>

<main>
  <h1>Comilito</h1>
  <p class="version">Version {version}</p>

  <section>
    <h2>Audio test</h2>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        run("play_file", { path, category: playCategory });
      }}
    >
      <input bind:value={path} placeholder="/path/to/sound.ogg" />
      <select bind:value={playCategory}>
        {#each categories as c (c.id)}
          <option value={c.id}>{c.label}</option>
        {/each}
      </select>
      <button type="submit" disabled={!path}>Play</button>
      <button type="button" onclick={() => run("stop_all")}>Stop all</button>
    </form>
    {#if error}
      <p class="error">{error}</p>
    {/if}
  </section>

  {#if volumes}
    <section class="mixer">
      <h2>Mixer</h2>
      <VolumeSlider label="Master" value={volumes.master} onchange={setMaster} />
      {#each categories as c (c.id)}
        <VolumeSlider
          label={c.label}
          value={volumes[c.id]}
          onchange={(v) => setCategory(c.id, v)}
        />
      {/each}
    </section>
  {/if}
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

  .version {
    color: #888;
  }

  form {
    display: flex;
    gap: 0.5rem;
  }

  form input {
    flex: 1;
    max-width: 40rem;
  }

  .mixer {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .error {
    color: #ff7b7b;
  }
</style>

<script lang="ts">
  // The project's sounds, one tab per category. Preview with ▶; add/edit/remove in Edit mode.
  import { api } from "../lib/api";
  import { addSoundFiles, removeSound } from "../lib/actions";
  import { run, ui } from "../lib/store.svelte";
  import { CATEGORIES, type Category } from "../lib/types";

  let category = $state<Category>("music");

  const project = $derived(ui.snapshot!.project!);
  const missing = $derived(new Set(ui.snapshot!.missing));
  const preview = $derived(ui.snapshot!.playback.preview);
  const sounds = $derived(
    project.sounds
      .filter((s) => s.category === category)
      .sort((a, b) => a.name.localeCompare(b.name)),
  );
</script>

<div class="cats" role="tablist">
  {#each CATEGORIES as c (c.id)}
    <button role="tab" class:active={category === c.id} onclick={() => (category = c.id)}>
      {c.icon} {c.label}
    </button>
  {/each}
</div>

<div class="scroll">
  {#if sounds.length === 0}
    <p class="muted">No {category} sounds yet.{ui.mode === "edit" ? "" : " Switch to Edit mode to add some."}</p>
  {/if}
  <ul>
    {#each sounds as sound (sound.id)}
      <li class:missing={missing.has(sound.id)}>
        <button
          class="icon"
          onclick={() => run(api.previewSound(sound.id))}
          disabled={missing.has(sound.id)}
          title="Preview"
        >
          {preview === sound.id ? "⏹" : "▶"}
        </button>
        <span class="name" title={sound.file.path}>
          {#if missing.has(sound.id)}<span title="Audio file is missing">⚠</span>{/if}
          {sound.name}
        </span>
        {#if ui.mode === "edit"}
          <button class="icon" title="Edit" onclick={() => (ui.editing = { kind: "sound", id: sound.id })}>✎</button>
          <button class="icon danger" title="Remove" onclick={() => removeSound(sound.id)}>🗑</button>
        {/if}
      </li>
    {/each}
  </ul>
</div>

{#if ui.mode === "edit"}
  <div class="bottom">
    <button onclick={() => addSoundFiles(category)}>+ Add {category} files…</button>
  </div>
{/if}

<style>
  .cats {
    display: flex;
    gap: 0.2rem;
    padding: 0.5rem 0.5rem 0;
  }
  .cats button {
    flex: 1;
    font-size: 0.8rem;
    padding: 0.25rem 0.3rem;
  }
  .cats button.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  .scroll {
    flex: 1;
    overflow: auto;
    padding: 0.5rem;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  li {
    display: flex;
    align-items: center;
    gap: 0.2rem;
    padding: 0.1rem 0.2rem;
    border-radius: 6px;
  }
  li:hover {
    background: var(--panel-2);
  }
  li.missing .name {
    color: var(--warn);
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bottom {
    padding: 0.5rem;
    border-top: 1px solid var(--border);
  }
</style>

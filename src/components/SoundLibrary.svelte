<script lang="ts">
  // The project's sounds, one tab per category. Preview with ▶; ✎ edits, 🗑 removes.
  // Drag a sound onto another category's tab to move it there, or onto a scene in the stack
  // to add it to that scene.
  import { api } from "../lib/api";
  import { addSoundFiles, changeSoundCategory, relinkOne, removeSound } from "../lib/actions";
  import { run, ui } from "../lib/store.svelte";
  import { CATEGORIES, type Category } from "../lib/types";

  let category = $state<Category>("music");
  /** The tab a dragged sound is over, to highlight it. */
  let tabTarget = $state<Category | null>(null);

  function startDrag(e: DragEvent, soundId: string) {
    ui.drag = { kind: "sound", id: soundId };
    e.dataTransfer?.setData("text/plain", soundId);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "copyMove";
  }

  function onTabDragOver(e: DragEvent, target: Category) {
    // Only sounds can be dropped on a tab, and not on their own category's tab.
    const drag = ui.drag;
    if (drag?.kind !== "sound") return;
    if (project.sounds.find((s) => s.id === drag.id)?.category === target) return;
    e.preventDefault();
    tabTarget = target;
  }

  async function onTabDrop(e: DragEvent, target: Category) {
    e.preventDefault();
    const drag = ui.drag;
    tabTarget = null;
    ui.drag = null;
    if (drag?.kind === "sound") await changeSoundCategory(drag.id, target);
  }

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
    <button
      role="tab"
      class:active={category === c.id}
      class:target={tabTarget === c.id}
      onclick={() => (category = c.id)}
      ondragover={(e) => onTabDragOver(e, c.id)}
      ondragleave={() => (tabTarget = null)}
      ondrop={(e) => onTabDrop(e, c.id)}
    >
      {c.icon} {c.label}
    </button>
  {/each}
</div>

<div class="scroll">
  {#if sounds.length === 0}
    <p class="muted">No {category} sounds yet.</p>
  {/if}
  <ul>
    {#each sounds as sound (sound.id)}
      <li
        class:missing={missing.has(sound.id)}
        draggable="true"
        ondragstart={(e) => startDrag(e, sound.id)}
        ondragend={() => {
          ui.drag = null;
          tabTarget = null;
        }}
        title="Drag to another category's tab, or onto a scene in the stack"
      >
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
        {#if missing.has(sound.id)}
          <button class="icon" title="Locate the audio file" onclick={() => relinkOne(sound.id)}>📂</button>
        {/if}
        <button class="icon" title="Edit" onclick={() => (ui.editing = { kind: "sound", id: sound.id })}>✎</button>
        <button class="icon danger" title="Remove" onclick={() => removeSound(sound.id)}>🗑</button>
      </li>
    {/each}
  </ul>
</div>

<div class="bottom">
  <button onclick={() => addSoundFiles(category)}>+ Add {category} files…</button>
</div>

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
  .cats button.target {
    background: color-mix(in srgb, var(--accent) 25%, var(--panel-2));
    border-color: var(--accent);
  }
  li[draggable="true"] {
    cursor: grab;
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

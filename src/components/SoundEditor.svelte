<script lang="ts">
  // Edits a sound's name and settings. Changes are kept in a local draft until Save.
  import { api } from "../lib/api";
  import { changeSoundCategory } from "../lib/actions";
  import { project, run } from "../lib/store.svelte";
  import { closeEditor, fingerprint } from "../lib/unsaved";
  import { formatDb, normalizationGainDb } from "../lib/loudness";
  import { CATEGORIES, type Sound } from "../lib/types";
  import Modal from "./Modal.svelte";
  import OverrideField from "./OverrideField.svelte";

  let { original, onclose }: { original: Sound; onclose: () => void } = $props();

  // A deep copy, so editing doesn't touch the snapshot until we save.
  // `$state.snapshot` turns Svelte's reactive proxy back into a plain object first.
  // Copying `original` once is the point: the draft must not follow later changes.
  // svelte-ignore state_referenced_locally
  let draft = $state<Sound>(structuredClone($state.snapshot(original)));

  const defaults = $derived(project()!.defaults);
  const catDefaults = $derived(defaults.categories[draft.category]);

  // svelte-ignore state_referenced_locally
  const initial = fingerprint($state.snapshot(original));
  const dirty = $derived(fingerprint($state.snapshot(draft)) !== initial);

  async function save() {
    if (!draft.name.trim()) draft.name = original.name;
    // The category change asks first if scenes would lose the sound; stop if cancelled.
    if (!(await changeSoundCategory(draft.id, draft.category))) return;
    // `update_sound` leaves the category alone, so this order is safe.
    await run(api.updateSound($state.snapshot(draft)));
    onclose();
  }
</script>

<!-- ✕ and Esc go through closeEditor (may ask about unsaved changes); Cancel just closes. -->
<Modal title="Edit sound" onclose={() => closeEditor(dirty, save, onclose)}>
  <label class="row">
    <span>Name</span>
    <input bind:value={draft.name} />
  </label>
  <p class="muted file">{draft.file.path}</p>
  <label class="row">
    <span>Category</span>
    <select bind:value={draft.category}>
      {#each CATEGORIES as c (c.id)}
        <option value={c.id}>{c.icon} {c.label}</option>
      {/each}
    </select>
  </label>

  <p class="muted loudness">
    {#if draft.file.loudness}
      Measured loudness {draft.file.loudness.lufs.toFixed(1)} LUFS ·
      {#if normalizationGainDb(project()!, original) !== null}
        normalized by {formatDb(normalizationGainDb(project()!, original)!)}
      {:else}
        not normalized (off for {original.category} in Project settings)
      {/if}
    {:else}
      Loudness not measured yet.
    {/if}
  </p>

  <OverrideField
    label="Volume"
    kind="percent"
    value={draft.volume}
    inherited={defaults.soundVolume}
    onchange={(v) => (draft.volume = v as number | undefined)}
  />
  <OverrideField
    label="Loop"
    kind="toggle"
    value={draft.loop}
    inherited={catDefaults.loop}
    onchange={(v) => (draft.loop = v as boolean | undefined)}
  />
  <OverrideField
    label="Fade in"
    kind="seconds"
    value={draft.fadeInMs}
    inherited={catDefaults.fadeInMs}
    onchange={(v) => (draft.fadeInMs = v as number | undefined)}
  />
  <OverrideField
    label="Fade out"
    kind="seconds"
    value={draft.fadeOutMs}
    inherited={catDefaults.fadeOutMs}
    onchange={(v) => (draft.fadeOutMs = v as number | undefined)}
  />
  <p class="muted">
    In scenes, an ambience layer's own volume replaces this one. Effects that loop act as
    on/off toggles on the pad.
  </p>

  {#snippet footer()}
    <button onclick={onclose}>Cancel</button>
    <button class="primary" onclick={save}>Save</button>
  {/snippet}
</Modal>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .row span {
    width: 8rem;
  }
  .row input,
  .row select {
    flex: 1;
  }
  .loudness {
    margin: 0 0 0 8.5rem;
    font-size: 0.85rem;
  }
  .file {
    margin: -0.5rem 0 0 8.5rem;
    font-size: 0.8rem;
    user-select: text;
  }
  p {
    margin: 0;
  }
</style>

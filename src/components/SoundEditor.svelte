<script lang="ts">
  // Edits a sound's name and settings. Changes are kept in a local draft until Save.
  import { api } from "../lib/api";
  import { ask, project, run } from "../lib/store.svelte";
  import { CATEGORIES, type Category, type Sound } from "../lib/types";
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

  /** Scenes that use this sound in a slot the new category doesn't fit (it will leave them). */
  function scenesLosingSound(category: Category): string[] {
    return project()!
      .scenes.filter(
        (s) =>
          (category !== "music" && s.music?.tracks.includes(draft.id)) ||
          (category !== "ambience" && s.ambience.some((l) => l.sound === draft.id)) ||
          (category !== "effects" && s.effects.includes(draft.id)),
      )
      .map((s) => s.name);
  }

  async function save() {
    if (!draft.name.trim()) draft.name = original.name;
    const newCategory = draft.category;

    if (newCategory !== original.category) {
      const losing = scenesLosingSound(newCategory);
      if (losing.length > 0) {
        const answer = await ask({
          title: "Change category?",
          message: `"${draft.name}" will be removed from: ${losing.join(", ")}.`,
          choices: [
            { label: "Cancel", value: "cancel" },
            { label: "Change category", value: "ok", primary: true },
          ],
        });
        if (answer?.choice !== "ok") return;
      }
    }

    // Settings first (Rust ignores the category there), then the move, which also moves the file.
    await run(api.updateSound({ ...$state.snapshot(draft), category: original.category }));
    if (newCategory !== original.category) await run(api.setSoundCategory(draft.id, newCategory));
    onclose();
  }
</script>

<Modal title="Edit sound" {onclose}>
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
  .file {
    margin: -0.5rem 0 0 8.5rem;
    font-size: 0.8rem;
    user-select: text;
  }
  p {
    margin: 0;
  }
</style>

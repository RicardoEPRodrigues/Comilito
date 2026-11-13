<script lang="ts">
  // Editing several sounds at once (decision 0018): favourite, tags, move, and playback
  // settings. Each change applies to all selected sounds straight away. Fields where the
  // sounds differ show "mixed" until set.
  import { FolderInput, Star, Trash2, X } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { moveSounds } from "../lib/actions";
  import { confirmAction, run, ui } from "../lib/store.svelte";
  import { CATEGORIES, type Category, type Sound } from "../lib/types";
  import OverrideField from "./OverrideField.svelte";

  let { sounds }: { sounds: Sound[] } = $props();

  const project = $derived(ui.snapshot!.project!);
  const categories = $derived([...new Set(sounds.map((s) => s.category))]);
  const oneCategory = $derived(categories.length === 1 ? categories[0] : null);
  const allFavourite = $derived(sounds.every((s) => s.favourite));
  const tagCounts = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const s of sounds) for (const t of s.tags ?? []) counts.set(t, (counts.get(t) ?? 0) + 1);
    return [...counts.entries()].sort((a, b) => b[1] - a[1]);
  });

  let newTag = $state("");
  let targetCategory = $state<Category>("ambience");
  let targetFolder = $state("");
  $effect(() => {
    targetCategory = oneCategory ?? "ambience";
  });

  /** All sounds' value of a field, or undefined if they differ (or none set it). */
  function common<K extends keyof Sound>(key: K): { value: Sound[K] | undefined; mixed: boolean } {
    const values = sounds.map((s) => s[key]);
    const same = values.every((v) => v === values[0]);
    return { value: same ? values[0] : undefined, mixed: !same };
  }

  function update(change: (s: Sound) => Partial<Sound>) {
    run(api.updateSounds(sounds.map((s) => ({ ...$state.snapshot(s), ...change(s) }))));
  }

  function addTag() {
    const t = newTag.trim().replace(/^#/, "").toLowerCase();
    newTag = "";
    if (t) update((s) => ({ tags: [...new Set([...(s.tags ?? []), t])] }));
  }

  async function removeAll() {
    const ok = await confirmAction(
      `Remove ${sounds.length} sounds?`,
      "They're removed from every scene, and their files go to the system trash.",
      "Move to trash",
      true,
    );
    if (!ok) return;
    for (const s of sounds) await run(api.removeSound(s.id, false));
    ui.library.selected = [];
  }

  const folders = $derived(ui.snapshot!.folders[targetCategory]);
  const defaults = $derived(oneCategory ? project.defaults.categories[oneCategory] : null);
</script>

<div class="editor">
  <h2 class="display">{sounds.length} sounds</h2>
  <span class="caps muted">{categories.map((c) => CATEGORIES.find((x) => x.id === c)?.label).join(" · ")} · changes apply to all of them</span>

  <section>
    <div class="section-head"><span class="caps">Favourite and tags</span></div>
    <div class="line">
      <button onclick={() => update(() => ({ favourite: !allFavourite }))}>
        <Star size={13} fill={allFavourite ? "currentColor" : "none"} />{allFavourite ? "Remove from favourites" : "Add to favourites"}
      </button>
    </div>
    <div class="line wrap">
      {#each tagCounts as [tag, count] (tag)}
        <span class="chip">
          #{tag}{count < sounds.length ? ` (${count})` : ""}
          <button class="icon x" title="Remove from all" onclick={() => update((s) => ({ tags: (s.tags ?? []).filter((t) => t !== tag) }))}><X size={11} /></button>
        </span>
      {/each}
      <input class="tag-input" bind:value={newTag} placeholder="+ tag for all" onkeydown={(e) => e.key === "Enter" && addTag()} />
    </div>
  </section>

  <section>
    <div class="section-head"><span class="caps">Move</span><span class="hint">Moves the files into another category or folder</span></div>
    <div class="line">
      <select bind:value={targetCategory}>
        {#each CATEGORIES as c (c.id)}<option value={c.id}>{c.label}</option>{/each}
      </select>
      <select bind:value={targetFolder}>
        <option value="">(category folder)</option>
        {#each folders as f (f)}<option value={f}>{f}</option>{/each}
      </select>
      <button onclick={() => moveSounds(sounds.map((s) => s.id), targetCategory, targetFolder)}><FolderInput size={13} />Move</button>
    </div>
  </section>

  <section>
    <div class="section-head">
      <span class="caps">Playback</span>
      <span class="hint">{oneCategory ? "Grey values follow the project defaults" : "Select sounds of one category to see their defaults"}</span>
    </div>
    <div class="grid">
      {#each [{ key: "volume", label: "Volume", kind: "percent" }, { key: "loop", label: "Loop", kind: "toggle" }, { key: "fadeInMs", label: "Fade in", kind: "seconds" }, { key: "fadeOutMs", label: "Fade out", kind: "seconds" }] as f (f.key)}
        {@const c = common(f.key as keyof Sound)}
        {@const inherited =
          f.key === "volume"
            ? project.defaults.soundVolume
            : f.key === "loop"
              ? (defaults?.loop ?? false)
              : f.key === "fadeInMs"
                ? (defaults?.fadeInMs ?? 0)
                : (defaults?.fadeOutMs ?? 0)}
        <OverrideField
          label={f.label}
          kind={f.kind as "percent" | "toggle" | "seconds"}
          value={c.value as number | boolean | undefined}
          mixed={c.mixed}
          {inherited}
          onchange={(v) => update(() => ({ [f.key]: v }))}
        />
      {/each}
    </div>
  </section>

  <section>
    <div class="line">
      <span class="grow"></span>
      <button class="danger" onclick={removeAll}><Trash2 size={13} />Remove {sounds.length} sounds…</button>
    </div>
  </section>
</div>

<style>
  .editor {
    flex: 1;
    overflow: auto;
    padding: 1.5rem 2rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
  }
  h2 {
    font-size: 2.3rem;
  }
  .caps {
    font-size: 0.6rem;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .wrap {
    flex-wrap: wrap;
  }
  .grow {
    flex: 1;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.8rem;
  }
  .x {
    padding: 0.05rem;
  }
  .tag-input {
    width: 8rem;
    padding: 0.25rem 0.5rem;
    border-radius: 999px;
    border-style: dashed;
    font-size: 0.8rem;
  }
</style>

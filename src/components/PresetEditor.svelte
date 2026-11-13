<script lang="ts">
  // Edits a preset in place (decision 0017), the way the tapestry itself is edited, without
  // touching the live tapestry: reorder, enable/disable, remove, and drag scenes in from the
  // Scenes tab. Clicking a scene opens it in the scene editor, with the preset in the
  // breadcrumbs. Changes are kept in a draft until Save (in the context bar).
  import { GripVertical, Pencil, Replace, X } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { loadPreset } from "../lib/actions";
  import { focusOn, presetById, project, run, sceneById, ui } from "../lib/store.svelte";
  import type { PresetEntry } from "../lib/types";
  import { confirmLeave, fingerprint } from "../lib/unsaved";
  import Capital from "./Capital.svelte";
  import Watermark from "./Watermark.svelte";

  let { id }: { id: string } = $props();

  // svelte-ignore state_referenced_locally
  const original = presetById(id)!;
  const snap = (pr: typeof original) => ({ name: pr.name, group: pr.group ?? "", stack: structuredClone($state.snapshot(pr.stack)) });
  let draft = $state(snap(original));
  let saved = $state(fingerprint(snap(original)));
  const dirty = $derived(fingerprint($state.snapshot(draft)) !== saved);
  const groups = $derived(ui.snapshot?.presetGroups ?? []);
  const p = $derived(project()!);

  // Shown top first; stored bottom first like the tapestry.
  const displayed = $derived([...draft.stack].map((e, i) => ({ entry: e, index: i })).reverse());
  let dropIndex = $state<number | null>(null);

  async function save() {
    const current = presetById(id);
    if (!current) return;
    const d = $state.snapshot(draft);
    if (d.name.trim() && d.name.trim() !== current.name) await run(api.presetRename(id, d.name.trim()));
    if ((d.group.trim() || null) !== (current.group ?? null)) await run(api.presetSetGroup(id, d.group.trim() || null));
    await run(api.presetSetEntries(id, d.stack));
    saved = fingerprint(d);
  }

  function discard() {
    const current = presetById(id) ?? original;
    draft = snap(current);
    saved = fingerprint(snap(current));
  }

  $effect(() => {
    ui.editorActions = { dirty, save, discard, saveLabel: "Save preset" };
  });
  $effect(() => {
    const guard = () => confirmLeave(dirty, save);
    ui.guard = guard;
    return () => {
      if (ui.guard === guard) ui.guard = null;
    };
  });

  function dragOverEntry(e: DragEvent, index: number) {
    if (ui.drag?.kind !== "scene" && ui.drag?.kind !== "presetEntry") return;
    e.preventDefault();
    e.stopPropagation();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    // Upper half: above this entry (a higher index, since higher = nearer the top).
    dropIndex = e.clientY < rect.top + rect.height / 2 ? index + 1 : index;
  }

  function drop(e: DragEvent) {
    e.preventDefault();
    const drag = ui.drag;
    let index = dropIndex ?? draft.stack.length;
    dropIndex = null;
    ui.drag = null;
    if (drag?.kind === "scene") {
      draft.stack.splice(index, 0, { scene: drag.id, enabled: true });
    } else if (drag?.kind === "presetEntry") {
      const [moved] = draft.stack.splice(drag.index, 1);
      if (drag.index < index) index--;
      draft.stack.splice(index, 0, moved);
    }
  }

  function openScene(entry: PresetEntry) {
    focusOn({ kind: "scene", id: entry.scene, from: id });
  }
</script>

<div
  class="editor"
  role="list"
  ondragover={(e) => {
    if (ui.drag?.kind !== "scene" && ui.drag?.kind !== "presetEntry") return;
    e.preventDefault();
    if (dropIndex === null) dropIndex = 0;
  }}
  ondrop={drop}
>
  <Watermark />
  <div class="head">
    <div class="title">
      <input class="name display" bind:value={draft.name} aria-label="Preset name" />
      <span class="caps muted">Preset · {draft.stack.length} scene{draft.stack.length === 1 ? "" : "s"} · editing doesn't change the live tapestry</span>
    </div>
    <label class="field group">
      <span class="caps">Group</span>
      <input bind:value={draft.group} list="preset-groups-edit" placeholder="Ungrouped" />
      <datalist id="preset-groups-edit">
        {#each groups as g (g)}<option value={g}></option>{/each}
      </datalist>
    </label>
    <button class="primary" onclick={() => loadPreset(id)} disabled={dirty} title={dirty ? "Save first" : "Replace the tapestry with this preset"}>
      <Replace size={14} />Load
    </button>
  </div>
  <div class="ornament"><i></i></div>

  <div class="entries">
    {#if draft.stack.length === 0}
      <div class="empty">Drag scenes here from the Scenes tab.</div>
    {:else}
      <span class="caps edge">Top</span>
    {/if}
    {#each displayed as { entry, index } (index)}
      {@const scene = sceneById(entry.scene)}
      <div
        class="entry"
        class:off={!entry.enabled}
        class:drop-above={dropIndex === index + 1}
        class:drop-below={index === 0 && dropIndex === 0}
        role="listitem"
        draggable="true"
        ondragstart={(e) => {
          ui.drag = { kind: "presetEntry", index };
          e.dataTransfer?.setData("text/plain", String(index));
        }}
        ondragend={() => (ui.drag = null)}
        ondragover={(e) => dragOverEntry(e, index)}
      >
        <span class="handle"><GripVertical size={18} /></span>
        <input type="checkbox" checked={entry.enabled} onchange={(e) => (draft.stack[index].enabled = e.currentTarget.checked)} title="Enabled in this preset" />
        <Capital name={scene?.name ?? "?"} size={38} />
        <button class="text" onclick={() => openScene(entry)} title="Edit this scene">
          <span class="sname">{scene?.name ?? "?"}</span>
          <span class="meta">{entry.enabled ? "" : "Disabled · "}click to edit the scene</span>
        </button>
        <button class="icon" onclick={() => openScene(entry)} title="Edit scene"><Pencil size={16} /></button>
        <button class="icon" onclick={() => draft.stack.splice(index, 1)} title="Remove from this preset"><X size={18} /></button>
      </div>
    {/each}
    {#if draft.stack.length}<span class="caps edge">Bottom</span>{/if}
  </div>
  {#if p.stack.length === 0 && draft.stack.length > 0}
    <p class="muted tip">Tip: Load puts this preset in the tapestry.</p>
  {/if}
</div>

<style>
  .editor {
    flex: 1;
    overflow: auto;
    padding: 1.5rem 2.4rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    position: relative;
  }
  .editor > :not(:global(.watermark)) {
    position: relative;
    z-index: 1;
  }
  .head {
    display: flex;
    align-items: flex-end;
    gap: 1rem;
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .name {
    font-size: 2.6rem;
    background: transparent;
    border: 1px solid transparent;
    padding: 0 0.2rem;
    margin-left: -0.25rem;
    color: var(--text);
  }
  .name:hover,
  .name:focus {
    border-color: var(--rule);
  }
  .title .caps {
    font-size: 0.62rem;
  }
  .group {
    width: 12rem;
  }
  .entries {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  .edge {
    color: var(--muted);
    font-size: 0.6rem;
    letter-spacing: 0.25em;
  }
  .empty {
    border: 1.5px dashed var(--rule);
    border-radius: 6px;
    padding: 2.5rem;
    text-align: center;
    color: var(--muted);
  }
  .entry {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0.6rem 0.7rem;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
  }
  .entry.off {
    opacity: 0.55;
  }
  .entry.drop-above {
    box-shadow: 0 -3px 0 0 var(--accent);
  }
  .entry.drop-below {
    box-shadow: 0 3px 0 0 var(--accent);
  }
  .handle {
    cursor: grab;
    color: var(--muted);
    display: inline-flex;
  }
  .text {
    flex: 1;
    min-width: 0;
    border: none;
    padding: 0;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.1rem;
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
  }
  .text:hover {
    background: none !important;
  }
  .sname {
    font-size: 1rem;
    font-weight: 600;
  }
  .meta {
    font-size: 0.75rem;
    color: var(--muted);
  }
  .icon {
    color: var(--muted);
  }
  .tip {
    font-size: 0.8rem;
  }
</style>

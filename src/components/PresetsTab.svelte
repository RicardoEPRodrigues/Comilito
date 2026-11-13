<script lang="ts">
  // The Presets tab: saved tapestries, in groups (decision 0017). Presets can hold disabled
  // scenes. Load replaces the tapestry (asking to save it first if needed), Edit opens the
  // preset in place, and the ⋯ menu renames, duplicates, moves to a group or deletes.
  import {
    ArrowUpToLine,
    BookmarkPlus,
    Check,
    ChevronDown,
    ChevronRight,
    Copy,
    Ellipsis,
    FolderInput,
    FolderPlus,
    Layers,
    Pencil,
    Replace,
    TextCursorInput,
    Trash2,
  } from "@lucide/svelte";
  import { api } from "../lib/api";
  import {
    deletePreset,
    deletePresetGroup,
    loadPreset,
    movePresetToNewGroup,
    newPresetGroup,
    renamePreset,
    renamePresetGroup,
    saveTapestryAsPreset,
  } from "../lib/actions";
  import { focusOn, project, run, sceneById, ui } from "../lib/store.svelte";
  import type { StackPreset } from "../lib/types";
  import { openMenu, type MenuItem } from "./ContextMenu.svelte";
  import GroupHeading from "./GroupHeading.svelte";

  const p = $derived(project()!);
  const groups = $derived(ui.snapshot?.presetGroups ?? []);
  const ungrouped = $derived(p.stackPresets.filter((pr) => !pr.group));
  const origin = $derived(p.stackOrigin);
  let expanded = $state<string | null>(null);

  function meta(preset: StackPreset): string {
    const n = preset.stack.length;
    const off = preset.stack.filter((e) => !e.enabled).length;
    return `${n} scene${n === 1 ? "" : "s"}${off ? ` · ${off} disabled` : ""}`;
  }

  function menuFor(preset: StackPreset): MenuItem[] {
    return [
      { label: "Load (replace tapestry)", icon: Replace, action: () => loadPreset(preset.id) },
      { label: "Add on top of the tapestry", icon: ArrowUpToLine, action: () => run(api.presetInsert(preset.id, p.stack.length)) },
      { label: "Edit", icon: Pencil, action: () => focusOn({ kind: "preset", id: preset.id }) },
      { label: "Rename", icon: TextCursorInput, action: () => renamePreset(preset.id) },
      { label: "Duplicate", icon: Copy, action: () => run(api.presetDuplicate(preset.id)) },
      {
        label: "Move to group",
        icon: FolderInput,
        submenu: [
          ...groups.map((g) => ({ label: g, checked: preset.group === g, action: () => run(api.presetSetGroup(preset.id, g)) })),
          { label: "Ungrouped", checked: !preset.group, action: () => run(api.presetSetGroup(preset.id, null)) },
          "separator",
          { label: "New group…", icon: FolderPlus, action: () => movePresetToNewGroup(preset.id) },
        ],
      },
      "separator",
      { label: "Delete…", icon: Trash2, danger: true, action: () => deletePreset(preset.id) },
    ];
  }

  function groupMenu(group: string): MenuItem[] {
    return [
      { label: "Rename group", icon: TextCursorInput, action: () => renamePresetGroup(group) },
      { label: "Delete group…", icon: Trash2, danger: true, action: () => deletePresetGroup(group) },
    ];
  }

  function startDrag(e: DragEvent, id: string) {
    ui.drag = { kind: "preset", id };
    e.dataTransfer?.setData("text/plain", id);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "copyMove";
  }

  function dropOnGroup(group: string | null) {
    if (ui.drag?.kind === "preset") run(api.presetSetGroup(ui.drag.id, group));
    ui.drag = null;
  }
</script>

{#snippet card(preset: StackPreset)}
  {@const open = expanded === preset.id}
  <div
    class="card"
    class:open
    class:origin={origin === preset.id}
    role="listitem"
    draggable="true"
    ondragstart={(e) => startDrag(e, preset.id)}
    ondragend={() => (ui.drag = null)}
    oncontextmenu={(e) => openMenu(e, menuFor(preset), preset.name)}
  >
    <div class="head">
      <button class="fold icon" onclick={() => (expanded = open ? null : preset.id)} aria-expanded={open}>
        {#if open}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
      </button>
      <Layers size={14} />
      <button class="text" onclick={() => (expanded = open ? null : preset.id)} ondblclick={() => loadPreset(preset.id)}>
        <span class="name">{preset.name}</span>
        <span class="meta">{meta(preset)}{origin === preset.id ? " · loaded" : ""}</span>
      </button>
      <button class="icon" title="Preset options" onclick={(e) => openMenu(e, menuFor(preset), preset.name)}><Ellipsis size={16} /></button>
    </div>
    {#if open}
      <div class="entries">
        {#each [...preset.stack].reverse() as entry, i (i)}
          <div class="entry" class:off={!entry.enabled}>
            <span class="box" class:on={entry.enabled}>{#if entry.enabled}<Check size={10} />{/if}</span>
            <span class="ename">{sceneById(entry.scene)?.name ?? "?"}</span>
            {#if !entry.enabled}<span class="meta">disabled</span>{/if}
          </div>
        {/each}
      </div>
      <div class="actions">
        <button class="primary" onclick={() => loadPreset(preset.id)}><Replace size={13} />Load</button>
        <button onclick={() => focusOn({ kind: "preset", id: preset.id })}><Pencil size={13} />Edit</button>
      </div>
    {/if}
  </div>
{/snippet}

<p class="hint">Starting points for a session. Loading one replaces the tapestry.</p>
<div class="list" role="list">
  {#if p.stackPresets.length === 0}
    <p class="hint">No presets yet. Build a tapestry, then save it from its ⋯ menu.</p>
  {/if}
  {#each ungrouped as preset (preset.id)}{@render card(preset)}{/each}
  {#each groups as group (group)}
    {@const presets = p.stackPresets.filter((pr) => pr.group === group)}
    <GroupHeading
      id={`preset:${group}`}
      name={group}
      count={presets.length}
      onmenu={(e) => openMenu(e, groupMenu(group), group)}
      droppable={ui.drag?.kind === "preset"}
      ondrop={() => dropOnGroup(group)}
    >
      {#each presets as preset (preset.id)}{@render card(preset)}{/each}
      {#if presets.length === 0}<p class="hint small">Empty. Drag a preset here.</p>{/if}
    </GroupHeading>
  {/each}
</div>
<div class="foot">
  <button class="ghost link" onclick={() => newPresetGroup()}><FolderPlus size={14} />New group</button>
  <button onclick={() => saveTapestryAsPreset()} disabled={p.stack.length === 0}><BookmarkPlus size={14} />Save tapestry as preset</button>
</div>

<style>
  .hint {
    margin: 0.2rem 1.1rem 0.4rem;
    color: var(--side-muted);
    font-size: 0.8rem;
    line-height: 1.4;
  }
  .hint.small {
    margin: 0.1rem 0 0.3rem 1.2rem;
    font-size: 0.75rem;
  }
  .list {
    flex: 1;
    overflow: auto;
    padding: 0 1.1rem 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .card {
    border: 1px solid color-mix(in srgb, var(--side-muted) 70%, transparent);
    border-radius: var(--radius);
    color: var(--side-text);
  }
  .card.open {
    background: var(--surface);
    color: var(--text);
    border-color: var(--rule);
  }
  .card.origin {
    border-color: var(--accent);
    border-width: 1.5px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.35rem 0.4rem 0.3rem;
    cursor: grab;
  }
  .fold {
    padding: 0.15rem;
    color: inherit;
    opacity: 0.7;
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
    font-size: 0.875rem;
    color: inherit;
  }
  .text:hover {
    background: none !important;
  }
  .name {
    font-weight: 600;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    font-size: 0.72rem;
    opacity: 0.7;
    white-space: normal;
    text-align: left;
  }
  .icon {
    color: inherit;
    opacity: 0.75;
  }
  .entries {
    padding: 0 0.7rem 0.4rem 2.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .entry {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }
  .entry.off {
    opacity: 0.5;
  }
  .ename {
    flex: 1;
  }
  .box {
    width: 0.8rem;
    height: 0.8rem;
    border: 1.2px solid currentColor;
    border-radius: 2px;
    display: grid;
    place-items: center;
  }
  .box.on {
    background: var(--text);
    border-color: var(--text);
    color: var(--surface);
  }
  .actions {
    display: flex;
    gap: 0.4rem;
    padding: 0.2rem 0.7rem 0.7rem 2.1rem;
  }
  .actions button {
    padding: 0.3rem 0.65rem;
  }
  .foot {
    padding: 0.6rem 1.1rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .foot button {
    justify-content: center;
    border-color: var(--side-text);
  }
  .link {
    justify-content: flex-start !important;
    color: var(--side-muted);
    padding: 0.2rem 0;
  }
</style>

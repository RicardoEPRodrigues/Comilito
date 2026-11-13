<script lang="ts">
  // One sound in the Library drawer or quick search: drag it onto a scene, the tapestry or the
  // scene editor; ▶ previews it; ★ marks it a favourite; double-click opens it in the Library.
  import { FolderOpen, Pencil, Play, Square, Star, Trash2, TriangleAlert } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { removeSound } from "../lib/actions";
  import { soundDragStart } from "../lib/library";
  import { durationOf, formatDuration, openSoundInLibrary, run, ui } from "../lib/store.svelte";
  import type { Sound } from "../lib/types";
  import { openMenu, type MenuItem } from "./ContextMenu.svelte";

  let { sound, detail = "" }: { sound: Sound; detail?: string } = $props();

  const previewing = $derived(ui.snapshot?.playback.preview === sound.id);
  const missing = $derived(ui.snapshot?.missing.includes(sound.id) ?? false);

  function toggleFavourite() {
    run(api.updateSounds([{ ...$state.snapshot(sound), favourite: !sound.favourite }]));
  }

  const menu = (): MenuItem[] => [
    { label: previewing ? "Stop preview" : "Preview", icon: Play, action: () => run(api.previewSound(sound.id)) },
    { label: "Edit in the Library", icon: Pencil, action: () => openSoundInLibrary(sound.id) },
    { label: sound.favourite ? "Remove from favourites" : "Add to favourites", icon: Star, action: toggleFavourite },
    { label: "Show in file manager", icon: FolderOpen, disabled: missing, action: () => run(api.revealSound(sound.id)) },
    "separator",
    { label: "Remove from project…", icon: Trash2, danger: true, action: () => removeSound(sound.id) },
  ];
</script>

<div
  class="row"
  class:missing
  role="button"
  tabindex="0"
  draggable="true"
  ondragstart={(e) => soundDragStart(e, sound.id, (id) => {
    ui.drag = { kind: "sound", id };
  })}
  ondragend={() => (ui.drag = null)}
  ondblclick={() => openSoundInLibrary(sound.id)}
  oncontextmenu={(e) => openMenu(e, menu(), sound.name)}
>
  <button class="icon play" title={previewing ? "Stop" : "Preview"} disabled={missing} onclick={() => run(api.previewSound(sound.id))}>
    {#if previewing}<Square size={11} />{:else}<Play size={11} />{/if}
  </button>
  {#if missing}<span class="warn" title="The audio file is missing"><TriangleAlert size={13} /></span>{/if}
  <span class="name">{sound.name}</span>
  {#if detail}<span class="detail">{detail}</span>{:else if durationOf(sound) !== null}<span class="detail">{formatDuration(durationOf(sound)!)}</span>{/if}
  <button class="icon star" class:on={sound.favourite} title={sound.favourite ? "Favourite" : "Add to favourites"} onclick={toggleFavourite}>
    <Star size={12} fill={sound.favourite ? "currentColor" : "none"} />
  </button>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.18rem 0.3rem 0.18rem 0.2rem;
    border-radius: 3px;
    cursor: grab;
    min-width: 0;
  }
  .row:hover {
    background: color-mix(in srgb, var(--text) 7%, transparent);
  }
  .row.missing .name {
    color: var(--warn);
  }
  .play {
    padding: 0.2rem;
    color: var(--muted);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.85rem;
  }
  .detail {
    font-size: 0.72rem;
    color: var(--muted);
    white-space: nowrap;
  }
  .warn {
    color: var(--warn);
    display: inline-flex;
  }
  .star {
    padding: 0.15rem;
    color: var(--rule);
  }
  .star.on {
    color: var(--accent);
  }
</style>

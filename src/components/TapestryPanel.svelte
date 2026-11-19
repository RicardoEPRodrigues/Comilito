<script lang="ts">
  // The Tapestry: the scene stack, top of the stack shown at the top (docs/scenes.md).
  //
  // Drag and drop uses the browser's built-in HTML5 events (decision 0014):
  //   dragstart (on the dragged item) → dragover (on whatever it's over, many times a second)
  //   → drop (on the target) → dragend (on the dragged item, always, even if cancelled).
  // A target must call `preventDefault()` in dragover, or the browser won't allow a drop.
  //
  // The stack is stored bottom to top, so displayed position `p` (0 = top) of `n` entries is
  // stack index `n - 1 - p`. "Insert above displayed item p" means stack index `n - p`.
  import { fold } from "../lib/motion";
  import {
    BookmarkPlus,
    ChevronDown,
    ChevronUp,
    Ellipsis,
    Eraser,
    GripVertical,
    Layers,
    Music,
    Pause,
    Pencil,
    Plus,
    Save,
    Undo2,
    VolumeX,
    X,
    AudioLines,
    Zap,
    Film,
  } from "@lucide/svelte";
  import { videoEnabled } from "../lib/video";
  import { api } from "../lib/api";
  import { clearTapestry, insertScene, revertToOrigin, saveTapestryAsPreset, updateOriginPreset } from "../lib/actions";
  import { focusOn, percent, presetById, project as currentProject, run, sceneById, soundById, ui } from "../lib/store.svelte";
  import { videoById } from "../lib/video";
  import { ROLES, roleLabel, type Role, type Scene, type StackEntry } from "../lib/types";
  import Capital from "./Capital.svelte";
  import { openMenu, type MenuItem } from "./ContextMenu.svelte";
  import Watermark from "./Watermark.svelte";

  const snapshot = $derived(ui.snapshot!);
  const project = $derived(snapshot.project!);
  const displayed = $derived([...project.stack].reverse());
  const n = $derived(project.stack.length);
  const origin = $derived(project.stackOrigin ? presetById(project.stackOrigin) : undefined);
  const topEnabled = $derived(displayed.find((e) => e.enabled)?.id);

  /** Where a drop would insert (stack index, 0 = bottom), or null when not dragging over us. */
  let dropIndex = $state<number | null>(null);
  /** True while an entry is dragged outside the panel, so releasing it removes it. */
  let draggedOutside = $state(false);
  /** The entry a dragged sound is over (sounds are added to a scene, not inserted). */
  let soundTarget = $state<string | null>(null);
  let panel: HTMLElement;
  let expanded = $state<Set<string>>(new Set());

  function toggleExpanded(id: string) {
    const next = new Set(expanded);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expanded = next;
  }

  // When any drag ends (here or elsewhere, dropped or cancelled), clear the drop indicators.
  $effect(() => {
    if (!ui.drag) {
      dropIndex = null;
      soundTarget = null;
    }
  });

  function onItemDragOver(e: DragEvent, p: number, entry: StackEntry) {
    if (!ui.drag || ui.drag.kind === "presetEntry") return;
    e.preventDefault();
    e.stopPropagation();
    if (ui.drag.kind === "sound" || ui.drag.kind === "video") {
      if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
      soundTarget = entry.id;
      dropIndex = null;
      return;
    }
    // Upper half of the item: insert above it; lower half: below it.
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const upper = e.clientY < rect.top + rect.height / 2;
    dropIndex = upper ? n - p : n - p - 1;
  }

  function onPanelDragOver(e: DragEvent) {
    if (!ui.drag || ui.drag.kind === "sound" || ui.drag.kind === "video" || ui.drag.kind === "presetEntry") return;
    e.preventDefault();
    // Over empty space: insert at the top when the stack is empty, else at the bottom.
    if (dropIndex === null) dropIndex = n === 0 ? 0 : 0;
  }

  async function onDrop(e: DragEvent) {
    e.preventDefault();
    const drag = ui.drag;
    const index = dropIndex ?? 0;
    // The entry under the pointer, read from the drop itself: WebKitGTK sends `dragleave`
    // without a relatedTarget, so `soundTarget` may already have been cleared.
    const under = (e.target as HTMLElement | null)?.closest<HTMLElement>("[data-entry-id]")?.dataset.entryId;
    const target = under ?? soundTarget;
    dropIndex = null;
    soundTarget = null;
    ui.drag = null;
    if (!drag) return;

    if (drag.kind === "sound" || drag.kind === "video") {
      const entry = project.stack.find((en) => en.id === target);
      if (entry) await run(drag.kind === "video" ? api.addVideoToScene(entry.scene, drag.id) : api.addSoundToScene(entry.scene, drag.id));
      return;
    }
    if (drag.kind === "scene") await insertScene(drag.id, index);
    else if (drag.kind === "preset") await run(api.presetInsert(drag.id, index));
    else if (drag.kind === "entry") await run(api.stackMove(drag.id, index));
  }

  function onEntryDragStart(e: DragEvent, entry: StackEntry) {
    ui.drag = { kind: "entry", id: entry.id };
    draggedOutside = false;
    e.dataTransfer?.setData("text/plain", entry.id);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }

  function onEntryDragEnd(entry: StackEntry) {
    // Released outside the panel: "drag it out of the tapestry" removes it.
    if (draggedOutside) run(api.stackRemove(entry.id));
    draggedOutside = false;
    dropIndex = null;
    ui.drag = null;
  }

  /** Watches drags anywhere in the window to know when an entry leaves the panel. */
  function onWindowDragOver(e: DragEvent) {
    if (ui.drag?.kind !== "entry" || !panel) return;
    const r = panel.getBoundingClientRect();
    const inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    draggedOutside = !inside;
    if (!inside) dropIndex = null;
  }

  type MusicState = "playing" | "paused" | "covered" | null;
  function musicState(entry: StackEntry, scene: Scene | undefined): MusicState {
    if (!scene?.music?.tracks.length || !entry.enabled) return null;
    if (snapshot.mix.musicEntry === entry.id) return snapshot.playback.transport === "playing" ? "playing" : "paused";
    return "covered";
  }

  function meta(entry: StackEntry, scene: Scene | undefined): string {
    if (!scene) return "";
    if (!entry.enabled) return "Disabled · keeps its place in the tapestry";
    const parts: string[] = [];
    const tracks = scene.music?.tracks ?? [];
    if (tracks.length === 1) parts.push(soundById(tracks[0])?.name ?? "Music");
    else if (tracks.length > 1) parts.push(`Playlist · ${tracks.length} tracks`);
    else if (scene.blocksMusic) parts.push("Silences the music below");
    if (scene.ambience.length) parts.push(`${scene.ambience.length} ambience layer${scene.ambience.length === 1 ? "" : "s"}`);
    if (scene.effects.length) parts.push(`${scene.effects.length} effect${scene.effects.length === 1 ? "" : "s"}`);
    if (videoEnabled()) {
      const videos = (scene.video?.videos.length ?? 0) + (scene.videoEffects?.length ?? 0);
      if (videos) parts.push(`${videos} video${videos === 1 ? "" : "s"}`);
    }
    const duck = scene.duckLevel ?? project.defaults.duckLevel;
    // A scene only ducks the roles it has layers for, unless it ducks everything.
    const ducks = scene.ducksAll || scene.ambience.length > 0;
    if (ducks && duck < 1 && entry.id !== displayed[displayed.length - 1]?.id) {
      parts.push(`${scene.ducksAll ? "ducks all ambience below" : "ducks below"} to ${percent(duck)}`);
    }
    return parts.join(" · ") || "No sounds yet";
  }

  function entryMenu(entry: StackEntry, scene: Scene | undefined): MenuItem[] {
    return [
      { label: "Edit scene", icon: Pencil, action: () => scene && focusOn({ kind: "scene", id: scene.id }) },
      { label: entry.enabled ? "Disable" : "Enable", action: () => run(api.stackSetEnabled(entry.id, !entry.enabled)) },
      "separator",
      { label: "Remove from tapestry", icon: X, danger: true, action: () => run(api.stackRemove(entry.id)) },
    ];
  }

  const tapestryMenu = $derived.by((): MenuItem[] => {
    const items: MenuItem[] = [];
    if (snapshot.stackStatus === "modified" && origin) {
      items.push({ label: `Update “${origin.name}”`, icon: Save, hint: "Ctrl+S", action: updateOriginPreset });
    }
    items.push({ label: "Save as new preset…", icon: BookmarkPlus, hint: "Ctrl+Shift+S", disabled: n === 0, action: saveTapestryAsPreset });
    if (snapshot.stackStatus === "modified" && origin) {
      items.push({ label: `Revert to “${origin.name}”`, icon: Undo2, action: revertToOrigin });
    }
    items.push("separator", { label: "Clear tapestry…", icon: Eraser, danger: true, disabled: n === 0, action: clearTapestry });
    return items;
  });

  function onKeydown(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "s" || ui.editing || ui.question || ui.focus) return;
    if (ui.view !== "tapestry" || !currentProject()) return;
    e.preventDefault();
    if (e.shiftKey || !(snapshot.stackStatus === "modified" && origin)) saveTapestryAsPreset();
    else updateOriginPreset();
  }
</script>

<!-- In an expanded entry, a ✕ that takes a sound or video out of the scene (it stays in the library). -->
{#snippet removeButton(sceneId: string, soundId: string)}
  <button class="icon small" title="Remove from this scene" onclick={() => run(api.removeSoundFromScene(sceneId, soundId))}>
    <X size={13} />
  </button>
{/snippet}

<svelte:window ondragover={onWindowDragOver} onkeydown={onKeydown} />

<section
  bind:this={panel}
  class="tapestry"
  class:sound-drag={ui.drag?.kind === "sound" || ui.drag?.kind === "video"}
  ondragover={onPanelDragOver}
  ondragleave={(e) => {
    // Only clear when really leaving the panel. WebKitGTK gives no relatedTarget, so check
    // the pointer position instead of which element it moved to.
    const r = panel.getBoundingClientRect();
    const outside = e.clientX <= r.left || e.clientX >= r.right || e.clientY <= r.top || e.clientY >= r.bottom;
    if (outside) {
      dropIndex = null;
      soundTarget = null;
    }
  }}
  ondrop={onDrop}
  role="list"
  aria-label="Tapestry"
>
  <Watermark />
  <div class="head">
    <div class="title-block">
      <div class="title-row">
        <h2 class="display">The Tapestry</h2>
        {#if snapshot.stackStatus === "saved" && origin}
          <span class="pill"><Layers size={11} />From {origin.name}</span>
        {:else if snapshot.stackStatus === "modified" && origin}
          <span class="pill"><Layers size={11} />From {origin.name} <i class="warn-dot"></i><span class="warn">Modified</span></span>
        {:else if snapshot.stackStatus === "unsaved"}
          <span class="pill"><i class="warn-dot"></i>Not saved in a preset</span>
        {/if}
      </div>
      <p class="hint">
        {#if snapshot.playback.transport === "stopped" && n > 0}
          Press <kbd>Space</kbd> to start. Higher scenes play over the ones below.
        {:else}
          Scenes woven together, top over bottom. Drag scenes or sounds here.
        {/if}
      </p>
    </div>
    <button class="icon menu-btn" title="Tapestry options" onclick={(e) => openMenu(e, tapestryMenu, "Tapestry")}>
      <Ellipsis size={18} />
    </button>
  </div>
  <div class="ornament"><i></i></div>

  <div class="entries">
    {#if n === 0}
      <div class="empty" class:target={dropIndex !== null}>
        Drag scenes or presets here, or double-click a scene.
      </div>
    {/if}

    {#each displayed as entry, p (entry.id)}
      {@const scene = sceneById(entry.scene)}
      {@const layerVolumes = snapshot.mix.layerVolumes[entry.id] ?? []}
      {@const music = musicState(entry, scene)}
      {@const live = entry.id === topEnabled}
      <div
        class="entry"
        class:disabled={!entry.enabled}
        class:live
        class:drop-above={dropIndex === n - p}
        class:drop-below={p === n - 1 && dropIndex === 0}
        class:drop-into={soundTarget === entry.id}
        role="listitem"
        data-entry-id={entry.id}
        ondragover={(e) => onItemDragOver(e, p, entry)}
        oncontextmenu={(e) => openMenu(e, entryMenu(entry, scene), scene?.name)}
      >
        <!-- Only the header row is draggable: if the whole entry were, dragging a volume
             slider in the expanded details would drag the entry instead. -->
        <div
          class="row"
          role="presentation"
          draggable="true"
          ondragstart={(e) => onEntryDragStart(e, entry)}
          ondragend={() => onEntryDragEnd(entry)}
          onmousedown={(e) => e.button === 1 && e.preventDefault()}
          onauxclick={(e) => {
            // Middle click enables or disables the scene, like its checkbox. (Mousedown is
            // cancelled above so Linux doesn't paste or start autoscrolling.)
            if (e.button !== 1) return;
            e.preventDefault();
            run(api.stackSetEnabled(entry.id, !entry.enabled));
          }}
        >
          <span class="handle" title="Drag to reorder, or out of the tapestry to remove"><GripVertical size={18} /></span>
          <input
            type="checkbox"
            checked={entry.enabled}
            onchange={(e) => run(api.stackSetEnabled(entry.id, e.currentTarget.checked))}
            title="Enable / disable (or middle-click the scene)"
          />
          <Capital name={scene?.name ?? "?"} live={live} />
          <button class="text" onclick={() => toggleExpanded(entry.id)}>
            <span class="name">{scene?.name ?? "?"}</span>
            <span class="meta">{meta(entry, scene)}</span>
          </button>
          {#if music === "playing"}
            <span title="Its music is playing" class="status playing"><Music size={11} /><span class="label">Playing</span></span>
          {:else if music === "paused"}
            <span title="Its music is paused" class="status"><Pause size={11} /><span class="label">Paused</span></span>
          {:else if music === "covered"}
            <span title="A scene above plays its own music" class="status"><Pause size={11} /><span class="label">Covered</span></span>
          {:else if scene?.blocksMusic && entry.enabled && !scene.music?.tracks.length}
            <span class="status" title="Silences the music of the scenes below"><VolumeX size={11} /><span class="label">Silence</span></span>
          {/if}
          {#if videoEnabled() && entry.enabled && scene}
            <!-- The video follows the same rule as music (decision 0035). -->
            {#if snapshot.mix.video?.entryId === entry.id}
              <span class="status playing" title="Its video shows in the video window"><Film size={11} /><span class="label">Video</span></span>
            {:else if scene.video?.videos.length}
              <span class="status" title="A scene above shows its own video, or hides it"><Film size={11} /><span class="label">Covered</span></span>
            {:else if scene.hidesVideo}
              <span class="status" title="Hides the video of the scenes below"><Film size={11} /><span class="label">Hides video</span></span>
            {/if}
          {/if}
          <button class="icon" title={expanded.has(entry.id) ? "Collapse" : "Show sounds"} onclick={() => toggleExpanded(entry.id)}>
            {#if expanded.has(entry.id)}<ChevronUp size={18} />{:else}<ChevronDown size={18} />{/if}
          </button>
          <button class="icon" title="Remove from tapestry" onclick={() => run(api.stackRemove(entry.id))}><X size={18} /></button>
        </div>

        {#if soundTarget === entry.id && (ui.drag?.kind === "sound" || ui.drag?.kind === "video")}
          {@const dragged = ui.drag.kind === "video" ? videoById(ui.drag.id)?.name : soundById(ui.drag.id)?.name}
          <div class="drop-hint"><Plus size={14} />Release to add “{dragged}” to {scene?.name}</div>
        {/if}

        {#if expanded.has(entry.id) && scene}
          <div class="details" transition:fold>
            {#each scene.music?.tracks ?? [] as trackId (trackId)}
              {@const playing = snapshot.mix.musicEntry === entry.id && snapshot.playback.musicSound === trackId}
              <div class="sound" class:playing>
                <Music size={14} />
                <span class="sname">{soundById(trackId)?.name ?? "?"}</span>
                <span class="role"></span>
                <span class="bar-space">{playing ? "playing" : ""}</span>
                {@render removeButton(scene.id, trackId)}
              </div>
            {/each}
            {#each scene.ambience as layer, i (i)}
              {@const sound = soundById(layer.sound)}
              {@const base = layer.volume ?? sound?.volume ?? project.defaults.soundVolume}
              {@const effective = entry.enabled ? (layerVolumes[i] ?? 0) : 0}
              <div class="sound">
                <AudioLines size={14} />
                <span class="sname">{sound?.name ?? "?"}</span>
                <span class="role">
                  <select
                    class="tag"
                    value={layer.role}
                    title="Role: only a scene above with a layer in the same role ducks this one"
                    aria-label="Role in this scene"
                    onchange={(e) => run(api.setLayerRole(scene.id, i, e.currentTarget.value as Role))}
                  >
                    {#each ROLES as r (r)}<option value={r}>{roleLabel(r)}</option>{/each}
                  </select>
                </span>
                <span class="bars" title="Gold: volume in this scene · colour: what's heard after ducking">
                  <span class="base" style:width="{base * 100}%"></span>
                  <span class="eff" style:width="{effective * 100}%"></span>
                  <input
                    type="range"
                    min="0"
                    max="100"
                    value={Math.round(base * 100)}
                    oninput={(e) => run(api.setLayerVolume(scene.id, i, e.currentTarget.valueAsNumber / 100))}
                    aria-label="Volume in this scene"
                  />
                </span>
                <span class="vol">{entry.enabled ? percent(effective) : "—"}</span>
                {#if entry.enabled && effective < base - 0.005}<span class="note">ducked from {percent(base)}</span>{/if}
                {@render removeButton(scene.id, layer.sound)}
              </div>
            {/each}
            {#each scene.effects as effectId (effectId)}
              <div class="sound">
                <Zap size={14} />
                <span class="sname">{soundById(effectId)?.name ?? "?"}</span>
                <span class="role"></span>
                <span class="bar-space">on the effects pad</span>
                {@render removeButton(scene.id, effectId)}
              </div>
            {/each}
            {#if videoEnabled()}
              {#each scene.video?.videos ?? [] as videoId (videoId)}
                {@const playing = snapshot.mix.video?.entryId === entry.id && ui.videoStatus?.videoId === videoId}
                <div class="sound" class:playing>
                  <Film size={14} />
                  <span class="sname">{videoById(videoId)?.name ?? "?"}</span>
                  <span class="role"></span>
                  <span class="bar-space">{playing ? "showing" : ""}</span>
                  {@render removeButton(scene.id, videoId)}
                </div>
              {/each}
              {#each scene.videoEffects ?? [] as videoId (videoId)}
                <div class="sound">
                  <Zap size={14} />
                  <span class="sname">{videoById(videoId)?.name ?? "?"}</span>
                  <span class="role"></span>
                  <span class="bar-space">on the video effects pad</span>
                  {@render removeButton(scene.id, videoId)}
                </div>
              {/each}
            {/if}
            {#if !scene.music?.tracks.length && scene.ambience.length === 0 && scene.effects.length === 0 && !scene.video?.videos.length && !scene.videoEffects?.length}
              <span class="muted">This scene has no sounds yet. Drag some here from the Library.</span>
            {/if}
            <button class="edit" onclick={() => focusOn({ kind: "scene", id: scene.id })}><Pencil size={13} />Edit scene</button>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</section>

<style>
  .tapestry {
    /* Lets the rules at the end ask how wide the panel itself is (container queries). */
    container-type: inline-size;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 1.6rem 2.4rem 0.6rem;
    gap: 0.9rem;
    position: relative;
    overflow: hidden;
  }
  /* While a sound is dragged, show where it can go. */
  .tapestry.sound-drag .entry {
    border-style: dashed;
    border-color: var(--accent);
  }
  .head {
    display: flex;
    align-items: flex-end;
    gap: 1rem;
    position: relative;
    z-index: 1;
  }
  .title-block {
    flex: 1;
    min-width: 0;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    flex-wrap: wrap;
  }
  h2 {
    font-size: 2.6rem;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.6rem;
    border: 1px solid var(--muted);
    border-radius: 999px;
    color: var(--muted);
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  .warn {
    color: var(--warn);
  }
  .warn-dot {
    width: 0.4rem;
    height: 0.4rem;
    border-radius: 50%;
    background: var(--warn);
  }
  .hint {
    margin: 0.2rem 0 0;
    color: var(--muted);
    font-size: 0.82rem;
  }
  .menu-btn {
    border: 1px solid var(--text) !important;
  }
  .ornament {
    position: relative;
    z-index: 1;
  }
  .entries {
    flex: 1;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.1rem 0.2rem 1rem;
    position: relative;
    z-index: 1;
  }
  .empty {
    border: 1.5px dashed var(--rule);
    border-radius: 6px;
    padding: 2.5rem;
    text-align: center;
    color: var(--muted);
    background: color-mix(in srgb, var(--surface) 60%, transparent);
  }
  .empty.target {
    border-color: var(--accent);
    color: var(--accent);
  }
  .entry {
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    position: relative;
    box-shadow: 0 0 0 0 transparent;
  }
  .entry.live::before {
    content: "";
    position: absolute;
    left: -1px;
    top: -1px;
    bottom: -1px;
    width: 4px;
    background: var(--accent);
    border-radius: var(--radius) 0 0 var(--radius);
  }
  .entry.drop-above {
    box-shadow: 0 -3px 0 0 var(--accent);
  }
  .entry.drop-below {
    box-shadow: 0 3px 0 0 var(--accent);
  }
  .entry.drop-into {
    border-color: var(--accent);
    border-width: 2px;
  }
  .entry.disabled {
    opacity: 0.5;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0.7rem 0.8rem 0.7rem 0.7rem;
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
    gap: 0.15rem;
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
    font-size: 0.875rem;
  }
  .text:hover {
    background: none !important;
  }
  .name {
    font-size: 1.05rem;
    font-weight: 600;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    color: var(--muted);
    font-size: 0.78rem;
    text-align: left;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.25rem 0.6rem;
    border: 1px solid var(--muted);
    border-radius: 999px;
    color: var(--muted);
    font-size: 0.6rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .status.playing {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }
  .icon {
    color: var(--muted);
  }
  .drop-hint {
    margin: 0 0.8rem 0.7rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.7rem;
    background: var(--accent);
    color: var(--accent-text);
    border-radius: 3px;
    font-size: 0.8rem;
    font-weight: 600;
  }
  .details {
    border-top: 1px solid var(--rule);
    margin: 0 0.8rem;
    padding: 0.7rem 0 0.8rem 5.2rem;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  .sound {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .sound.playing .sname {
    color: var(--accent);
    font-weight: 600;
  }
  .sname {
    width: 11rem;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .role {
    width: 7rem;
  }
  .tag {
    font-size: 0.6rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    border: 1px solid var(--rule);
    border-radius: 999px;
    padding: 0.1rem 0.5rem;
    background: transparent;
    color: inherit;
    cursor: pointer;
    max-width: 100%;
    /* No native arrow: it's drawn as the same pill the role used to be shown in. */
    appearance: none;
    -webkit-appearance: none;
    text-align: center;
  }
  .tag:hover {
    border-color: var(--accent);
  }
  .bars,
  .bar-space {
    width: 12rem;
    font-size: 0.75rem;
  }
  .bars {
    position: relative;
    height: 6px;
    background: var(--track);
    border-radius: 3px;
  }
  .bars .base,
  .bars .eff {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    border-radius: 3px;
  }
  .bars .base {
    background: var(--gold);
  }
  .bars .eff {
    background: var(--accent);
  }
  .bars input {
    position: absolute;
    inset: -8px 0;
    width: 100%;
    opacity: 0;
    cursor: pointer;
  }
  .vol {
    width: 2.6rem;
    color: var(--text);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .note {
    font-size: 0.75rem;
  }
  .small {
    padding: 0.15rem;
  }
  .edit {
    align-self: flex-start;
    margin-top: 0.2rem;
    padding: 0.3rem 0.65rem;
  }
  /* A narrow panel (a narrow window, before it turns into the mini player): less padding, no
     capitals, and the status pills show only their icon, so the scene's name keeps its room. */
  /* (A container query can't restyle the container itself, so its padding goes by the window.) */
  @media (max-width: 1100px) {
    .tapestry {
      padding: 1.1rem 1rem 0.4rem;
    }
  }
  @container (max-width: 560px) {
    h2 {
      font-size: 2rem;
    }
    .row {
      gap: 0.5rem;
      padding: 0.55rem 0.5rem;
    }
    .row :global(.capital) {
      display: none;
    }
    .status {
      padding: 0.3rem;
    }
    .status .label {
      display: none;
    }
    .details {
      padding-left: 1rem;
    }
  }
</style>

<script lang="ts">
  // Edits one sound in the Library workspace (decisions 0018–0020): name, tags, playback
  // overrides, loudness, file, source, where it's used. Name/levels/tags are kept in a draft
  // until Save (in the context bar); category and folder changes move the file right away.
  import {
    Copy,
    ExternalLink,
    FileMusic,
    FolderOpen,
    FolderSearch,
    Play,
    Plus,
    RefreshCw,
    Scissors,
    Scroll,
    Square,
    Star,
    Trash2,
    X,
  } from "@lucide/svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onDestroy, onMount } from "svelte";
  import { api } from "../lib/api";
  import { duplicateSound, moveSounds, relinkOne, removeSound, stopPreviewOnLeave } from "../lib/actions";
  import { formatDb, normalizationGainDb } from "../lib/loudness";
  import { askText, focusOn, folderOf, formatBytes, formatDuration, run, scenesUsing, soundById, ui } from "../lib/store.svelte";
  import { CATEGORIES, type Category, type Sound } from "../lib/types";
  import { confirmLeave, fingerprint } from "../lib/unsaved";
  import Capital from "./Capital.svelte";
  import OverrideField from "./OverrideField.svelte";
  import Waveform from "./Waveform.svelte";
  import { followPreview, wheelView, type View } from "../lib/wave";

  let { id }: { id: string } = $props();
  onDestroy(() => stopPreviewOnLeave(id));

  // svelte-ignore state_referenced_locally
  const original = soundById(id)!;
  const editable = (s: Sound) => ({
    name: s.name,
    volume: s.volume,
    loop: s.loop,
    fadeInMs: s.fadeInMs,
    fadeOutMs: s.fadeOutMs,
    normalize: s.normalize,
    tags: [...(s.tags ?? [])],
    favourite: s.favourite ?? false,
  });
  let draft = $state(editable(original));
  let saved = $state(fingerprint(editable(original)));
  const dirty = $derived(fingerprint($state.snapshot(draft)) !== saved);

  const sound = $derived(soundById(id) ?? original);
  const project = $derived(ui.snapshot!.project!);
  const d = $derived(project.defaults.categories[sound.category]);
  const missing = $derived(ui.snapshot!.missing.includes(id));
  const previewing = $derived(ui.snapshot!.playback.preview === id);
  const usedIn = $derived(scenesUsing(id));
  const folders = $derived(ui.snapshot!.folders[sound.category]);
  const ext = $derived(sound.file.path.split(".").pop()?.toUpperCase() ?? "");
  // From the draft, so flipping the switch shows its effect before saving.
  const gain = $derived(normalizationGainDb(project, { ...sound, normalize: draft.normalize }));

  let peaks = $state<number[]>([]);
  let durationMs = $state<number | null>(null);
  let waveError = $state("");
  /** The part of the sound the waveform shows (Ctrl + scroll zooms). */
  let view = $state<View>({ start: 0, end: 0 });
  let waveWidth = $state(600);
  /** Where the preview is playing (ms), for the cursor. */
  let position = $state<number | null>(null);

  onMount(() => {
    if (missing) return;
    api
      .soundWaveform(id, 10)
      .then((w) => {
        durationMs = w.durationMs;
        view = { start: 0, end: w.durationMs };
      })
      .catch((e) => (waveError = String(e)));
    return followPreview(id, (ms) => (position = ms));
  });

  // Peaks for the visible part, refetched when zooming or resizing.
  $effect(() => {
    const { start, end } = view;
    if (end <= start) return;
    const buckets = Math.max(60, Math.round(waveWidth / 3));
    api.soundWaveform(id, buckets, start, end).then((w) => {
      if (view.start === start && view.end === end) peaks = w.peaks;
    });
  });

  const cursor = $derived(position !== null && view.end > view.start ? (position - view.start) / (view.end - view.start) : null);

  /** Click on the waveform: play the sound from there. */
  function seek(fraction: number) {
    const from = view.start + fraction * (view.end - view.start);
    if (durationMs) run(api.previewRange(id, from, durationMs, false));
  }

  async function save() {
    const current = soundById(id);
    if (!current) return;
    const snapshot = $state.snapshot(draft);
    await run(api.updateSounds([{ ...$state.snapshot(current), ...snapshot }]));
    saved = fingerprint(snapshot);
  }

  function discard() {
    const current = soundById(id) ?? original;
    draft = editable(current);
    saved = fingerprint(editable(current));
  }

  $effect(() => {
    ui.editorActions = { dirty, save, discard, saveLabel: "Save sound" };
  });
  $effect(() => {
    const guard = () => confirmLeave(dirty, save);
    ui.guard = guard;
    return () => {
      if (ui.guard === guard) ui.guard = null;
      if (ui.editorActions?.saveLabel === "Save sound") ui.editorActions = null;
    };
  });

  let newTag = $state("");
  function addTag() {
    const t = newTag.trim().replace(/^#/, "").toLowerCase();
    if (t && !draft.tags.includes(t)) draft.tags.push(t);
    newTag = "";
  }

  async function changeCategory(category: Category) {
    if (await moveSounds([id], category, "")) ui.library.category = ui.library.category ? category : null;
  }

  async function changeFolder(folder: string) {
    await moveSounds([id], sound.category, folder);
  }

  /** "Download again": the URL dialog, filled in with the original download's settings. */
  function downloadAgain() {
    const src = sound.source!;
    ui.editing = {
      kind: "addUrl",
      prefill: {
        url: src.url,
        name: sound.name,
        category: sound.category,
        folder: folderOf(sound),
        range: src.range ?? null,
        replace: id,
      },
    };
  }

  async function newFolderHere() {
    const name = await askText("New folder", "Name");
    if (!name) return;
    if (await run(api.createFolder(sound.category, name).then(() => true))) await changeFolder(name);
  }
</script>

<div class="editor">
  <div class="head">
    <Capital name={draft.name} size={62} live />
    <div class="title">
      <input class="name display" bind:value={draft.name} aria-label="Sound name" />
      <span class="caps muted">
        {CATEGORIES.find((c) => c.id === sound.category)?.label}{durationMs !== null ? ` · ${formatDuration(durationMs)}` : ""} · {ext} · {formatBytes(sound.file.size)}
      </span>
    </div>
    <!-- Favourite, duplicate and remove, as on a video's page (VideoPage.svelte). -->
    <div class="head-actions">
      <button class="icon fav" class:on={draft.favourite} title="Favourite" onclick={() => (draft.favourite = !draft.favourite)}>
        <Star size={20} fill={draft.favourite ? "currentColor" : "none"} />
      </button>
      <!-- Copies the saved sound; leaving for the copy asks about unsaved changes here. -->
      <button class="icon" disabled={missing} title="Duplicate: a copy of the file with the same settings, in no scene" onclick={() => duplicateSound(id, true)}>
        <Copy size={17} />
      </button>
      <button class="icon danger" title="Remove from the project…" onclick={() => removeSound(id)}><Trash2 size={17} /></button>
    </div>
  </div>

  <div class="wave-card">
    <button class="play" disabled={missing} onclick={() => run(api.previewSound(id))} title={previewing ? "Stop" : "Preview"}>
      {#if previewing}<Square size={14} />{:else}<Play size={15} />{/if}
    </button>
    {#if missing}
      <span class="muted grow">The audio file is missing.</span>
    {:else if waveError}
      <span class="muted grow">Couldn't draw the waveform: {waveError}</span>
    {:else}
      <div
        class="grow"
        bind:clientWidth={waveWidth}
        role="presentation"
        title="Click to play from there · Ctrl + scroll to zoom"
        onwheel={(e) => {
          if (!durationMs) return;
          const next = wheelView(e, view, durationMs);
          if (next) view = next;
        }}
      >
        {#if peaks.length === 0}
          <span class="muted reading">Reading the audio…</span>
        {:else}
          <Waveform {peaks} height={52} playhead={cursor} progress={cursor} onseek={seek} />
        {/if}
      </div>
      <!-- Always present (empty when not playing), so the waveform doesn't jump when it appears. -->
      <span class="pos">{position !== null ? formatDuration(position) : ""}</span>
    {/if}
    <button disabled={missing} onclick={() => (ui.library.cropping = true)}><Scissors size={13} />Crop</button>
  </div>

  <section>
    <div class="section-head"><span class="caps">General</span></div>
    <div class="grid three">
      <label class="field">
        <span class="caps">Category</span>
        <select value={sound.category} onchange={(e) => changeCategory(e.currentTarget.value as Category)}>
          {#each CATEGORIES as c (c.id)}<option value={c.id}>{c.label}</option>{/each}
        </select>
      </label>
      <label class="field">
        <span class="caps">Folder</span>
        <select
          value={folderOf(sound)}
          onchange={(e) => {
            const v = e.currentTarget.value;
            if (v === "__new__") {
              e.currentTarget.value = folderOf(sound);
              newFolderHere();
            } else changeFolder(v);
          }}
        >
          <option value="">({CATEGORIES.find((c) => c.id === sound.category)?.label} folder)</option>
          {#each folders as f (f)}<option value={f}>{f}</option>{/each}
          <option value="__new__">+ New folder…</option>
        </select>
      </label>
      <div class="field">
        <span class="caps">Tags</span>
        <div class="tags">
          {#each draft.tags as t, i (t)}
            <span class="chip">#{t}<button class="icon x" onclick={() => draft.tags.splice(i, 1)} title="Remove"><X size={11} /></button></span>
          {/each}
          <input
            class="tag-input"
            bind:value={newTag}
            placeholder="+ tag"
            onkeydown={(e) => (e.key === "Enter" || e.key === ",") && (e.preventDefault(), addTag())}
            onblur={addTag}
          />
        </div>
      </div>
    </div>
  </section>

  <section>
    <div class="section-head"><span class="caps">Playback</span><span class="hint">Grey values follow the project defaults for this category</span></div>
    <div class="grid four">
      <OverrideField label="Volume" kind="percent" value={draft.volume} inherited={project.defaults.soundVolume} onchange={(v) => (draft.volume = v as number | undefined)} />
      <OverrideField label="Loop" kind="toggle" value={draft.loop} inherited={d.loop} onchange={(v) => (draft.loop = v as boolean | undefined)} />
      <OverrideField label="Fade in" kind="seconds" value={draft.fadeInMs} inherited={d.fadeInMs} onchange={(v) => (draft.fadeInMs = v as number | undefined)} />
      <OverrideField label="Fade out" kind="seconds" value={draft.fadeOutMs} inherited={d.fadeOutMs} onchange={(v) => (draft.fadeOutMs = v as number | undefined)} />
    </div>
  </section>

  <section>
    <div class="section-head"><span class="caps">Loudness</span><span class="hint">Evens out quiet and loud files</span></div>
    <div class="grid four">
      <OverrideField
        label="Normalize"
        kind="toggle"
        value={draft.normalize}
        inherited={project.defaults.normalize[sound.category]}
        onchange={(v) => (draft.normalize = v as boolean | undefined)}
      />
    </div>
    <div class="line">
      {#if sound.file.loudness}
        <span>Measured <strong>{sound.file.loudness.lufs.toFixed(1)} LUFS</strong></span>
        {#if gain !== null}
          <span class="muted">→ plays at {project.defaults.normalize.targetLufs} LUFS ({formatDb(gain)})</span>
        {:else if draft.normalize === false}
          <span class="muted">· normalization is off for this sound</span>
        {:else}
          <span class="muted">· normalization is off for {sound.category} (Project settings)</span>
        {/if}
      {:else}
        <span class="muted">{missing ? "Can't be measured while the file is missing." : "Not measured yet; it happens in the background."}</span>
      {/if}
    </div>
  </section>

  <section>
    <div class="section-head"><span class="caps">File</span></div>
    <div class="line">
      <FileMusic size={16} />
      <code class="grow">{sound.file.path}</code>
      <button onclick={() => run(api.revealSound(id))} disabled={missing}><FolderSearch size={13} />Show in folder</button>
      <button onclick={() => relinkOne(id)}><FolderOpen size={13} />Replace file…</button>
    </div>
  </section>

  {#if sound.source}
    {@const src = sound.source}
    <section>
      <div class="section-head"><span class="caps">Source</span><span class="hint">Kept so the file can be fetched again if it goes missing</span></div>
      <div class="line">
        <div class="grow">
          <div><strong>“{src.title ?? sound.name}”</strong></div>
          <div class="muted small">
            {src.site ?? ""} · downloaded {new Date(src.downloadedAt).toLocaleDateString()}{src.range
              ? ` · ${formatDuration(src.range.startMs)}–${formatDuration(src.range.endMs)}`
              : ""}
          </div>
        </div>
        <button onclick={() => openUrl(src.url)}><ExternalLink size={13} />Open page</button>
        <button onclick={downloadAgain}><RefreshCw size={13} />Download again</button>
      </div>
    </section>
  {/if}

  <section>
    <div class="section-head"><span class="caps">Used in</span></div>
    <div class="line wrap">
      {#each usedIn as scene (scene.id)}
        <button class="chip" onclick={() => focusOn({ kind: "scene", id: scene.id })}><Scroll size={12} />{scene.name}</button>
      {:else}
        <span class="muted">No scene uses this sound yet.</span>
      {/each}
    </div>
  </section>
  <p class="muted small tip"><Plus size={11} /> Drag sounds onto a scene in the Tapestry view (the Library drawer) to use them.</p>
</div>

<style>
  .editor {
    flex: 1;
    overflow: auto;
    padding: 1.3rem 2rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.15rem;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .name {
    font-size: 2.3rem;
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
    font-size: 0.6rem;
  }
  .fav {
    color: var(--rule);
  }
  .fav.on {
    color: var(--accent);
  }
  .wave-card {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    padding: 0.7rem 0.8rem;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
  }
  .play {
    width: 2.4rem;
    height: 2.4rem;
    border-radius: 50%;
    justify-content: center;
    padding: 0;
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
    flex: none;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .reading {
    display: block;
    line-height: 52px;
    font-size: 0.85rem;
  }
  .pos {
    width: 3.2rem;
    text-align: right;
    flex: none;
    font-size: 0.8rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .grid {
    display: grid;
    gap: 0.8rem;
  }
  .grid.three {
    grid-template-columns: 1fr 1fr 1.4fr;
  }
  .grid.four {
    grid-template-columns: repeat(4, 1fr);
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    align-items: center;
    min-height: 2.3rem;
  }
  .x {
    padding: 0.05rem;
  }
  .tag-input {
    width: 6rem;
    padding: 0.25rem 0.5rem;
    border-radius: 999px;
    border-style: dashed;
    font-size: 0.8rem;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 0.7rem;
  }
  .line.wrap {
    flex-wrap: wrap;
  }
  code {
    font-size: 0.8rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }
  .small {
    font-size: 0.78rem;
  }
  .tip {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
</style>

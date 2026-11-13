<script lang="ts">
  // App preferences (this computer, docs/preferences.md): levels for new projects, playback,
  // projects folder and download tools, behaviour, and appearance. Never changes existing
  // projects. Appearance changes preview live and are undone if you cancel.
  import { FolderOpen, MousePointerClick, Palette, Play, SlidersHorizontal } from "@lucide/svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { Component } from "svelte";
  import { api } from "../lib/api";
  import { confirmAction, formatBytes, notify, openLogs, refreshTools, run, ui } from "../lib/store.svelte";
  import type { Preferences } from "../lib/types";
  import { CATEGORIES } from "../lib/types";
  import { closeEditor, fingerprint } from "../lib/unsaved";
  import AppearancePage from "./AppearancePage.svelte";
  import DefaultsForm from "./DefaultsForm.svelte";
  import Modal from "./Modal.svelte";
  import ToolsInstall from "./ToolsInstall.svelte";

  let { onclose, page: startPage }: { onclose: () => void; page?: string } = $props();

  const PAGES: { id: string; label: string; icon: Component }[] = [
    { id: "levels", label: "Levels", icon: SlidersHorizontal },
    { id: "playback", label: "Playback", icon: Play },
    { id: "projects", label: "Projects & tools", icon: FolderOpen },
    { id: "behaviour", label: "Behaviour", icon: MousePointerClick },
    { id: "appearance", label: "Appearance", icon: Palette },
  ];
  // svelte-ignore state_referenced_locally
  let page = $state(startPage ?? "appearance");

  let draft = $state<Preferences | null>(ui.prefs ? structuredClone($state.snapshot(ui.prefs)) : null);
  const initial = fingerprint(ui.prefs);
  const dirty = $derived(fingerprint($state.snapshot(draft)) !== initial);

  // Show appearance changes live while this window is open.
  $effect(() => {
    if (draft) ui.previewAppearance = $state.snapshot(draft.appearance);
    return () => (ui.previewAppearance = null);
  });

  async function chooseFolder() {
    const folder = await open({ directory: true, title: "Projects folder" });
    if (typeof folder === "string" && draft) draft.projectsFolder = folder;
  }

  async function save() {
    if (!draft) return;
    const prefs = $state.snapshot(draft);
    try {
      await api.setPreferences(prefs);
      ui.prefs = prefs;
      onclose();
    } catch (err) {
      notify(String(err));
    }
  }

  async function removeTools() {
    if (!(await confirmAction("Remove the download tools?", "yt-dlp and ffmpeg are deleted. They're downloaded again the next time you need them.", "Remove", true))) return;
    await run(api.removeTools());
    await refreshTools();
  }

  /** Removes preferences, themes, tools, logs and caches (not projects), then quits. */
  async function forgetEverything() {
    const ok = await confirmAction(
      "Remove all Comilito data from this computer?",
      "Deletes your preferences, your own themes, the downloaded tools, logs and caches, then " +
        "closes Comilito. Your projects and their backups are kept. Use this before uninstalling, " +
        "or to start over with a clean slate.",
      "Remove and quit",
      true,
    );
    if (ok) await run(api.forgetAllData());
  }

  const pct = (v: number) => Math.round(v * 100);
  const seconds = (ms: number) => ms / 1000;
  const toMs = (s: number) => Math.round(Math.max(0, s || 0) * 1000);
</script>

<Modal title="Preferences" onclose={() => closeEditor(dirty, save, onclose)} wide tall bare>
  {#if draft}
    <div class="layout">
      <nav>
        {#each PAGES as p (p.id)}
          <button class:active={page === p.id} onclick={() => (page = p.id)}><p.icon size={15} />{p.label}</button>
        {/each}
      </nav>
      <div class="content">
        <h2 class="display">{PAGES.find((p) => p.id === page)?.label}</h2>

        {#if page === "levels"}
          <p class="muted">The levels new projects start with. Existing projects keep their own (Project settings).</p>
          <section>
            <div class="section-head"><span class="caps">Volumes</span></div>
            <div class="volumes">
              <label class="field">
                <span class="caps">Master {pct(draft.newProjectDefaults.masterVolume)}%</span>
                <input type="range" min="0" max="100" value={pct(draft.newProjectDefaults.masterVolume)}
                  oninput={(e) => (draft!.newProjectDefaults.masterVolume = e.currentTarget.valueAsNumber / 100)} />
              </label>
              {#each CATEGORIES as c (c.id)}
                {@const cat = draft.newProjectDefaults.categories[c.id]}
                <label class="field">
                  <span class="caps">{c.label} {pct(cat.volume)}%</span>
                  <input type="range" min="0" max="100" value={pct(cat.volume)} oninput={(e) => (cat.volume = e.currentTarget.valueAsNumber / 100)} />
                </label>
              {/each}
            </div>
          </section>
          <DefaultsForm bind:defaults={draft.newProjectDefaults.defaults} bind:backups={draft.newProjectDefaults.backups} />
        {:else if page === "playback"}
          <section class="form">
            <label class="row">
              <span>Pause / resume fade (Space)</span>
              <input type="number" min="0" max="10" step="0.1" value={seconds(draft.playback.pauseFadeMs)}
                onchange={(e) => (draft!.playback.pauseFadeMs = toMs(e.currentTarget.valueAsNumber))} />
              <span class="muted">s · a short fade avoids clicks</span>
            </label>
            <label class="row">
              <span>Stop all fade (Esc)</span>
              <input type="number" min="0" max="10" step="0.1" value={seconds(draft.playback.stopFadeMs)}
                onchange={(e) => (draft!.playback.stopFadeMs = toMs(e.currentTarget.valueAsNumber))} />
              <span class="muted">s</span>
            </label>
          </section>
        {:else if page === "projects"}
          <section>
            <div class="section-head"><span class="caps">Projects folder</span></div>
            <div class="row">
              <code class="grow">{draft.projectsFolder ?? "Default (Documents/Comilito)"}</code>
              <button onclick={chooseFolder}>Choose…</button>
              {#if draft.projectsFolder}<button onclick={() => (draft!.projectsFolder = null)}>Use default</button>{/if}
            </div>
          </section>
          <section>
            <div class="section-head"><span class="caps">Download tools</span><span class="hint">yt-dlp and ffmpeg, for downloads and cropping</span></div>
            {#if ui.tools?.ytDlp || ui.tools?.ffmpeg}
              <div class="row">
                <span class="grow">
                  yt-dlp {ui.tools.ytDlp ?? "not installed"} · ffmpeg {ui.tools.ffmpeg ? "installed" : "not installed"} · {formatBytes(ui.tools.size)} on disk
                </span>
                <button onclick={() => run(api.installTools().then((t) => (ui.tools = t)))}>Update</button>
                <button class="danger" onclick={removeTools}>Remove</button>
              </div>
            {:else}
              <ToolsInstall />
            {/if}
          </section>
          <section>
            <div class="section-head"><span class="caps">Logs</span><span class="hint">Errors and background events, for troubleshooting</span></div>
            <div class="row"><button onclick={openLogs}>See logs</button></div>
          </section>
          <section>
            <div class="section-head"><span class="caps">Remove Comilito's data</span></div>
            <p class="muted">
              Preferences, themes, downloaded tools, logs and caches are kept outside your projects.
              Remove them before uninstalling, or to start over. Projects are never touched.
            </p>
            <div class="row"><button class="danger" onclick={forgetEverything}>Remove all data and quit…</button></div>
          </section>
        {:else if page === "behaviour"}
          <section class="form">
            <label class="row">
              <span>A scene that's already in the tapestry</span>
              <select bind:value={draft.behaviour.duplicateInStack}>
                <option value="ask">Ask</option>
                <option value="move">Always move it</option>
                <option value="duplicate">Always add a duplicate</option>
              </select>
            </label>
            <label class="row">
              <span>Removing a sound</span>
              <select bind:value={draft.behaviour.removeSound}>
                <option value="ask">Ask</option>
                <option value="trash">Move to trash (still asks if scenes use it)</option>
              </select>
            </label>
            <label class="row">
              <span>Leaving an editor with unsaved changes</span>
              <select bind:value={draft.behaviour.unsavedChanges}>
                <option value="ask">Ask</option>
                <option value="save">Save the changes</option>
                <option value="discard">Discard the changes</option>
              </select>
            </label>
            <label class="row">
              <span>Loading a preset over an unsaved tapestry</span>
              <select bind:value={draft.behaviour.replaceUnsavedTapestry}>
                <option value="ask">Ask</option>
                <option value="replace">Replace without saving</option>
              </select>
            </label>
          </section>
        {:else if page === "appearance"}
          <AppearancePage bind:appearance={draft.appearance} />
        {/if}
      </div>
    </div>
  {/if}

  {#snippet footer()}
    <button class="ghost" onclick={onclose}>Cancel</button>
    <button class="primary" onclick={save} disabled={!dirty}>Save</button>
  {/snippet}
</Modal>

<style>
  .layout {
    display: flex;
    height: 100%;
    min-height: 0;
  }
  nav {
    width: 13rem;
    flex: none;
    background: var(--side-bg);
    padding: 1rem 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  nav button {
    border: none;
    justify-content: flex-start;
    gap: 0.6rem;
    padding: 0.55rem 0.7rem;
    text-transform: none;
    letter-spacing: 0;
    font-size: 0.875rem;
    font-weight: 450;
    color: var(--side-text);
  }
  nav button.active {
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }
  .content {
    flex: 1;
    overflow: auto;
    padding: 1.4rem 1.8rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.3rem;
  }
  h2 {
    font-size: 2.2rem;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }
  p {
    margin: 0;
  }
  .volumes {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
  }
  .form {
    gap: 0.9rem;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.7rem;
  }
  .row > span:first-child {
    width: 18rem;
  }
  .row input[type="number"] {
    width: 5.5rem;
  }
  .row select {
    width: auto;
  }
  .grow {
    flex: 1;
  }
  code {
    font-size: 0.82rem;
    user-select: text;
  }
</style>

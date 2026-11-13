<script lang="ts">
  // App preferences (this computer): playback, behaviour, appearance, projects folder, and
  // the defaults that new projects start with. Never changes existing projects.
  import { open } from "@tauri-apps/plugin-dialog";
  import { api } from "../lib/api";
  import { notify, ui } from "../lib/store.svelte";
  import type { Preferences } from "../lib/types";
  import { CATEGORIES } from "../lib/types";
  import DefaultsForm from "./DefaultsForm.svelte";
  import { closeEditor, fingerprint } from "../lib/unsaved";
  import Modal from "./Modal.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let draft = $state<Preferences | null>(ui.prefs ? structuredClone($state.snapshot(ui.prefs)) : null);
  const initial = fingerprint(ui.prefs);
  const dirty = $derived(fingerprint($state.snapshot(draft)) !== initial);

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

  const pct = (v: number) => Math.round(v * 100);
</script>

<Modal title="Preferences" onclose={() => closeEditor(dirty, save, onclose)}>
  {#if draft}
    <section>
      <h2>Playback</h2>
      <label class="row">
        <span>Pause / resume fade</span>
        <input type="number" min="0" max="10" step="0.1" value={draft.playback.pauseFadeMs / 1000}
          onchange={(e) => (draft!.playback.pauseFadeMs = Math.round(Math.max(0, e.currentTarget.valueAsNumber || 0) * 1000))} />
        <span class="muted">s</span>
      </label>
      <label class="row">
        <span>Stop all fade</span>
        <input type="number" min="0" max="10" step="0.1" value={draft.playback.stopFadeMs / 1000}
          onchange={(e) => (draft!.playback.stopFadeMs = Math.round(Math.max(0, e.currentTarget.valueAsNumber || 0) * 1000))} />
        <span class="muted">s</span>
      </label>
    </section>

    <section>
      <h2>Behaviour</h2>
      <label class="row">
        <span>Scene already in stack</span>
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
        <span>Closing a window with unsaved changes</span>
        <select bind:value={draft.behaviour.unsavedChanges}>
          <option value="ask">Ask</option>
          <option value="save">Save the changes</option>
          <option value="discard">Discard the changes</option>
        </select>
      </label>
    </section>

    <section>
      <h2>Appearance</h2>
      <label class="row">
        <span>Theme</span>
        <select bind:value={draft.appearance.theme}>
          <option value="system">Follow system</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </label>
    </section>

    <section>
      <h2>Projects folder</h2>
      <div class="row">
        <span class="path">{draft.projectsFolder ?? "Default (Documents/Comilito)"}</span>
        <button onclick={chooseFolder}>Choose…</button>
        {#if draft.projectsFolder}
          <button onclick={() => (draft!.projectsFolder = null)}>Use default</button>
        {/if}
      </div>
    </section>

    <h3>Defaults for new projects</h3>
    <section>
      <h2>Volumes</h2>
      <label class="row">
        <span>Master</span>
        <input type="range" min="0" max="100" value={pct(draft.newProjectDefaults.masterVolume)}
          oninput={(e) => (draft!.newProjectDefaults.masterVolume = e.currentTarget.valueAsNumber / 100)} />
        <span class="muted">{pct(draft.newProjectDefaults.masterVolume)}%</span>
      </label>
      {#each CATEGORIES as c (c.id)}
        {@const cat = draft.newProjectDefaults.categories[c.id]}
        <label class="row">
          <span>{c.label}</span>
          <input type="range" min="0" max="100" value={pct(cat.volume)}
            oninput={(e) => (cat.volume = e.currentTarget.valueAsNumber / 100)} />
          <span class="muted">{pct(cat.volume)}%</span>
        </label>
      {/each}
    </section>
    <DefaultsForm bind:defaults={draft.newProjectDefaults.defaults} bind:backups={draft.newProjectDefaults.backups} />
  {/if}

  {#snippet footer()}
    <button onclick={onclose}>Cancel</button>
    <button class="primary" onclick={save}>Save</button>
  {/snippet}
</Modal>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  h3 {
    border-top: 1px solid var(--border);
    padding-top: 0.8rem;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .row > span:first-child {
    width: 11rem;
  }
  input[type="range"] {
    width: 10rem;
  }
  input[type="number"] {
    width: 5rem;
  }
  .path {
    flex: 1;
    width: auto !important;
    overflow-wrap: anywhere;
    user-select: text;
  }
</style>

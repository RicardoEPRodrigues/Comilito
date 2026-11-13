<script lang="ts">
  // The first screen: pick a project from the projects folder, or create one.
  import { FolderInput, History, Info, Plus, RefreshCw, SlidersHorizontal, Trash2 } from "@lucide/svelte";
  import { onMount } from "svelte";
  import icon from "../../src-tauri/icons/128x128@2x.png";
  import Watermark from "./Watermark.svelte";
  import { api } from "../lib/api";
  import { importProject, restoreBackup } from "../lib/actions";
  import { ask, run, ui } from "../lib/store.svelte";
  import type { ProjectSummary } from "../lib/types";

  let projects = $state<ProjectSummary[]>([]);
  let folder = $state("");
  let version = $state("");

  async function refresh() {
    projects = (await run(api.listProjects())) ?? [];
    folder = (await run(api.projectsFolder())) ?? "";
  }

  onMount(async () => {
    version = (await run(api.appVersion())) ?? "";
    await refresh();
  });

  async function remove(p: ProjectSummary) {
    const answer = await ask({
      title: `Delete "${p.name}"?`,
      message:
        "Moves it to the system trash (you can restore it from there). Backups are kept.\n\n" +
        "Keeping the audio deletes only the project's settings and scenes, and leaves its " +
        "audio files in the folder.",
      choices: [
        { label: "Cancel", value: "cancel" },
        { label: "Delete, keep audio files", value: "keep" },
        { label: "Delete project and audio", value: "all", danger: true, primary: true },
      ],
    });
    if (!answer || answer.choice === "cancel") return;
    await run(api.deleteProject(p.path, answer.choice === "all"));
    await refresh();
  }

  async function create() {
    const answer = await ask({
      title: "New project",
      message:
        "Start empty, or from the sample: a few sounds and three scenes (music, rain, combat) " +
        "already in the stack, to try things out.",
      input: { label: "Name (for example the campaign)", value: "" },
      choices: [
        { label: "Cancel", value: "cancel" },
        { label: "Start from the sample", value: "sample" },
        { label: "Empty project", value: "empty", primary: true },
      ],
    });
    if (!answer || answer.choice === "cancel") return;
    const name = answer.text.trim() || (answer.choice === "sample" ? "Sample project" : "");
    if (!name) return;
    await run(api.createProject(name, answer.choice === "sample"));
  }
</script>

<div class="picker">
  <Watermark />
  <div class="card">
    <header>
      <img src={icon} alt="" width="72" height="72" />
      <div class="grow">
        <h1 class="display">Comilito</h1>
        <span class="caps muted">A soundboard for Dungeon Masters · v{version}</span>
      </div>
      <button class="icon" title="Preferences" onclick={() => (ui.editing = { kind: "preferences" })}><SlidersHorizontal size={17} /></button>
      <button class="icon" title="About" onclick={() => (ui.editing = { kind: "about" })}><Info size={17} /></button>
    </header>
    <div class="ornament"><i></i></div>

    <div class="section-head"><span class="caps">Projects</span>
      <button class="icon refresh" onclick={refresh} title="Refresh"><RefreshCw size={14} /></button>
    </div>
    {#if projects.length === 0}
      <p class="muted">No projects yet. Create one (or start from the sample) to begin.</p>
    {:else}
      <ul>
        {#each projects as p (p.path)}
          <li>
            <button class="project" onclick={() => run(api.openProject(p.path))}>
              <span class="cap display">{p.name.trim()[0]?.toUpperCase() ?? "?"}</span>
              <span>{p.name}</span>
            </button>
            <button class="icon" title="Delete project…" onclick={() => remove(p)}><Trash2 size={15} /></button>
          </li>
        {/each}
      </ul>
    {/if}

    <div class="actions">
      <button class="primary" onclick={create}><Plus size={14} />New project</button>
      <button onclick={importProject}><FolderInput size={14} />Import…</button>
      <button onclick={restoreBackup}><History size={14} />Restore backup…</button>
    </div>
    <p class="folder muted">Projects folder: {folder}</p>
  </div>
</div>

<style>
  .picker {
    height: 100%;
    display: grid;
    place-items: center;
    position: relative;
    overflow: hidden;
    background: var(--bg);
  }
  .card {
    position: relative;
    z-index: 1;
    width: min(36rem, 92vw);
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: 8px;
    box-shadow: var(--shadow);
    padding: 1.6rem 1.8rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  header {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  header img {
    border-radius: 14px;
  }
  h1 {
    font-size: 3rem;
  }
  header .caps {
    font-size: 0.6rem;
  }
  .refresh {
    margin-left: auto;
    color: var(--muted);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    max-height: 18rem;
    overflow: auto;
  }
  li {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  li .icon {
    color: var(--muted);
  }
  .project {
    flex: 1;
    justify-content: flex-start;
    gap: 0.8rem;
    padding: 0.45rem 0.6rem;
    border-color: var(--rule);
    text-transform: none;
    letter-spacing: 0;
    font-size: 1rem;
    font-weight: 600;
  }
  .project:hover {
    border-color: var(--accent);
  }
  .cap {
    width: 2rem;
    height: 2rem;
    display: grid;
    place-items: center;
    background: var(--gold);
    border-radius: 2px;
    font-size: 1.4rem;
    font-weight: 400;
  }
  :global(:root.no-capitals) .cap {
    display: none;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }
  .grow {
    flex: 1;
    align-self: flex-start;
  }
  header .icon {
    align-self: flex-start;
    color: var(--muted);
  }
  .folder {
    margin: 0;
    font-size: 0.72rem;
    overflow-wrap: anywhere;
    user-select: text;
  }
  p {
    margin: 0;
  }
</style>

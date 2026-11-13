<script lang="ts">
  // The first screen: pick a project from the projects folder, or create one.
  import { onMount } from "svelte";
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
  <div class="card">
    <header>
      <h1>Comilito</h1>
      <span class="muted">v{version}</span>
    </header>

    <h2>Projects</h2>
    {#if projects.length === 0}
      <p class="muted">No projects yet. Create one to start adding sounds.</p>
    {:else}
      <ul>
        {#each projects as p (p.path)}
          <li>
            <button class="project" onclick={() => run(api.openProject(p.path))}>
              {p.name}
            </button>
          </li>
        {/each}
      </ul>
    {/if}

    <div class="actions">
      <button class="primary" onclick={create}>+ New project</button>
      <button onclick={importProject}>Import…</button>
      <button onclick={restoreBackup}>Restore backup…</button>
      <button onclick={() => (ui.editing = { kind: "preferences" })}>Preferences</button>
      <button onclick={() => (ui.editing = { kind: "about" })}>About</button>
      <button class="icon" onclick={refresh} title="Refresh">⟳</button>
    </div>
    <p class="muted small">Projects folder: {folder}</p>
  </div>
</div>

<style>
  .picker {
    height: 100%;
    display: grid;
    place-items: center;
  }
  .card {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1.5rem;
    width: min(30rem, 92vw);
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    max-height: 50vh;
    overflow: auto;
  }
  .project {
    width: 100%;
    text-align: left;
    padding: 0.6rem 0.8rem;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
  .small {
    font-size: 0.8rem;
    margin: 0;
    user-select: text;
  }
</style>

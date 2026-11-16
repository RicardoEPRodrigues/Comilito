<script lang="ts">
  // The home screen (design "H · Home — projects" in pencil/comilito.pen): the brand column
  // with New project, the rarer actions under More, and "open the last project at start";
  // on the right, the most recent project as "Continue", then every other project. Projects
  // opened from outside the projects folder (decision 0042) are listed too, with their folder.
  import { fitText } from "../lib/fit";
  import {
    ArrowRight,
    Copy,
    Ellipsis,
    ChevronDown,
    FolderInput,
    FolderOpen,
    FolderSearch,
    History,
    ListX,
    Info,
    Plus,
    RefreshCw,
    SlidersHorizontal,
    Trash2,
  } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { open as pickFolder } from "@tauri-apps/plugin-dialog";
  import icon from "../../src-tauri/icons/128x128@2x.png";
  import Watermark from "./Watermark.svelte";
  import { openMenu, type MenuItem } from "./ContextMenu.svelte";
  import { api } from "../lib/api";
  import { importProject, restoreBackup } from "../lib/actions";
  import { ask, notify, run, savePrefs, ui } from "../lib/store.svelte";
  import type { ProjectSummary } from "../lib/types";

  let projects = $state<ProjectSummary[]>([]);
  let folder = $state("");
  let version = $state("");

  async function refresh() {
    projects = (await run(api.listProjects())) ?? [];
    folder = (await run(api.projectsFolder())) ?? "";
    // Rust updates the recent list when a project opens; fetch it for the ordering.
    const prefs = await run(api.getPreferences());
    if (prefs && ui.prefs) ui.prefs.recentProjects = prefs.recentProjects;
  }

  onMount(async () => {
    version = (await run(api.appVersion())) ?? "";
  });

  // Reload when the projects folder changes (Preferences › Projects can move it).
  $effect(() => {
    void ui.prefs?.projectsFolder;
    refresh();
  });

  /** Position in the recent list (never-opened projects last). */
  function recency(p: ProjectSummary): number {
    const i = ui.prefs?.recentProjects.indexOf(p.path) ?? -1;
    return i < 0 ? Infinity : i;
  }
  const sorted = $derived([...projects].sort((a, b) => recency(a) - recency(b) || a.name.localeCompare(b.name)));
  // "Continue" is the last project opened, if it's still in the folder.
  const last = $derived(sorted.length && recency(sorted[0]) !== Infinity ? sorted[0] : null);
  const others = $derived(last ? sorted.slice(1) : sorted);

  /** "edited yesterday · 6 scenes · 3 presets · 24 sounds" */
  function meta(p: ProjectSummary): string {
    if (p.sounds === null) return "Can't read this project's file";
    const parts = [edited(p.modified)];
    const count = (n: number | null, one: string) => (n ? `${n} ${one}${n === 1 ? "" : "s"}` : "");
    parts.push(count(p.scenes, "scene"), count(p.presets, "preset"), count(p.sounds, "sound"));
    return parts.filter(Boolean).join(" · ") || "Empty";
  }

  function edited(iso: string | null): string {
    if (!iso) return "";
    const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
    if (days <= 0) return "Edited today";
    if (days === 1) return "Edited yesterday";
    if (days < 14) return `Edited ${days} days ago`;
    if (days < 60) return `Edited ${Math.round(days / 7)} weeks ago`;
    if (days < 365) return `Edited ${Math.round(days / 30)} months ago`;
    return `Edited ${Math.round(days / 365)} year${days < 730 ? "" : "s"} ago`;
  }

  /** The last two folders of the projects folder ("…/Documents/Comilito"); the tooltip has all of it. */
  const shortFolder = $derived.by(() => {
    const parts = folder.split(/[\\/]/).filter(Boolean);
    return parts.length > 2 ? `…/${parts.slice(-2).join("/")}` : folder;
  });

  const capital = (name: string) => name.trim()[0]?.toUpperCase() ?? "?";

  /** Where a project from elsewhere lives: the last two folders above it ("…/Drive/Campaigns"). */
  function shortParent(path: string): string {
    const parts = path.split(/[\\/]/).filter(Boolean).slice(0, -1);
    return parts.length > 2 ? `…/${parts.slice(-2).join("/")}` : parts.join("/");
  }

  /** "Open a project folder…": a project anywhere, opened in place (decision 0042). Rust
   *  remembers it, so it's listed here from then on. */
  async function openFolder() {
    const dir = await pickFolder({ directory: true, title: "Open a project folder (one with a comilito.json)" });
    if (typeof dir === "string") await run(api.openProject(dir));
  }

  async function forget(p: ProjectSummary) {
    await run(api.forgetProject(p.path));
    notify(`Removed “${p.name}” from the list. Its folder is untouched.`);
    await refresh();
  }

  function open(p: ProjectSummary) {
    run(api.openProject(p.path));
  }

  async function duplicate(p: ProjectSummary) {
    const copy = await run(api.duplicateProject(p.path));
    if (!copy) return;
    notify(`Duplicated “${p.name}”.`);
    await refresh();
  }

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

  function projectMenu(p: ProjectSummary): MenuItem[] {
    return [
      { label: "Open", icon: ArrowRight, action: () => open(p) },
      { label: "Duplicate", icon: Copy, action: () => duplicate(p) },
      { label: "Show in file manager", icon: FolderOpen, action: () => run(api.showProjectsFolder(p.path)) },
      ...(p.elsewhere ? [{ label: "Remove from list", icon: ListX, action: () => forget(p) }] : []),
      "separator",
      { label: "Delete…", icon: Trash2, danger: true, action: () => remove(p) },
    ];
  }

  const moreMenu: MenuItem[] = [
    { label: "Open a project folder…", icon: FolderSearch, action: openFolder },
    { label: "Import a project…", icon: FolderInput, action: () => importProject() },
    { label: "Restore a backup…", icon: History, action: () => restoreBackup() },
    "separator",
    { label: "Open the projects folder", icon: FolderOpen, action: () => run(api.showProjectsFolder()) },
  ];

  function setOpenLast(on: boolean) {
    if (ui.prefs) savePrefs({ ...ui.prefs, openLastProject: on });
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

<!-- For a project from elsewhere: which folder it's in (the whole path in the tooltip). -->
{#snippet where(p: ProjectSummary)}
  {#if p.elsewhere}
    <span class="meta where" title={p.path}><FolderOpen size={12} />{shortParent(p.path)}</span>
  {/if}
{/snippet}

{#snippet row(p: ProjectSummary)}
  <li>
    <button class="project" onclick={() => open(p)} oncontextmenu={(e) => openMenu(e, projectMenu(p), p.name)}>
      <span class="cap display">{capital(p.name)}</span>
      <span class="texts">
        <span class="name">{p.name}</span>
        <span class="meta">{meta(p)}</span>
        {@render where(p)}
      </span>
    </button>
    <button class="icon" title="Project options" onclick={(e) => openMenu(e, projectMenu(p), p.name)}><Ellipsis size={18} /></button>
  </li>
{/snippet}

<div class="home">
  <aside>
    <div class="brand">
      <img src={icon} alt="" width="96" height="96" />
      <h1 class="display" use:fitText>Comilito</h1>
      <span class="caps tagline">A soundboard for Dungeon Masters</span>
    </div>
    <p class="intro">
      Each campaign is a project: its sounds, scenes and presets, kept together in one folder you
      can back up or carry to another computer.
    </p>
    <div class="actions">
      <button class="primary" onclick={create}><Plus size={15} />New project</button>
      <button onclick={(e) => openMenu(e, moreMenu)}><Ellipsis size={15} />More<ChevronDown size={14} /></button>
    </div>

    <span class="grow"></span>

    <label class="switch start">
      <input type="checkbox" checked={ui.prefs?.openLastProject ?? false} onchange={(e) => setOpenLast(e.currentTarget.checked)} />
      <span>
        <strong>Open the last project at start</strong>
        <small>Skips this screen. Close the project to come back here.</small>
      </span>
    </label>
    <footer>
      <button class="ghost link" onclick={() => (ui.editing = { kind: "preferences" })}><SlidersHorizontal size={15} />Preferences</button>
      <button class="ghost link" onclick={() => (ui.editing = { kind: "about" })}><Info size={15} />About</button>
      <span class="grow"></span>
      <span class="version">v{version}</span>
    </footer>
  </aside>

  <main>
    <Watermark />
    {#if last}
      <span class="caps label">Continue where you left off</span>
      <div class="continue" role="button" tabindex="0"
        ondblclick={() => open(last)}
        onkeydown={(e) => e.key === "Enter" && open(last)}
        oncontextmenu={(e) => openMenu(e, projectMenu(last), last.name)}>
        <span class="cap big display">{capital(last.name)}</span>
        <span class="texts">
          <span class="name display" use:fitText>{last.name}</span>
          <span class="meta">{meta(last)}</span>
          {@render where(last)}
        </span>
        <button class="primary" onclick={() => open(last)}>Open<ArrowRight size={15} /></button>
        <button class="icon" title="Project options" onclick={(e) => openMenu(e, projectMenu(last), last.name)}><Ellipsis size={18} /></button>
      </div>
    {/if}

    <div class="section-head list-head">
      <span class="caps">{last ? "Other projects" : "Projects"}</span>
      <span class="count">{others.length}</span>
      <span class="grow"></span>
      <button class="ghost link path" title={`${folder}\nOpen the projects folder`} onclick={() => run(api.showProjectsFolder())}>
        <FolderOpen size={14} /><span>{shortFolder}</span>
      </button>
      <button class="icon" onclick={refresh} title="Refresh"><RefreshCw size={14} /></button>
    </div>
    {#if projects.length === 0}
      <p class="empty">No projects yet. Create one (or start from the sample) to begin.</p>
    {:else if others.length === 0}
      <p class="empty">No other projects in this folder.</p>
    {:else}
      <ul>
        {#each others as p (p.path)}{@render row(p)}{/each}
      </ul>
    {/if}
  </main>
</div>

<style>
  .home {
    height: 100%;
    display: flex;
    background: var(--bg);
    overflow: hidden;
  }
  aside {
    width: clamp(19rem, 30vw, 26rem);
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 1.6rem;
    padding: 3.2rem clamp(1.6rem, 3vw, 2.8rem) 2rem;
    background: var(--side-bg);
    color: var(--side-text);
    border-right: 1px solid var(--rule);
    overflow: auto;
  }
  .brand {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .brand img {
    border-radius: 20px;
  }
  h1 {
    font-size: 4.4rem;
    line-height: 1;
    color: var(--side-text);
  }
  .tagline {
    font-size: 0.66rem;
    color: var(--side-muted);
  }
  .intro {
    margin: 0;
    line-height: 1.55;
  }
  .actions {
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  .actions button {
    padding: 0.7rem 1.1rem;
  }
  .actions button:not(.primary) {
    border-color: var(--side-text);
    color: var(--side-text);
  }
  .grow {
    flex: 1;
  }
  .start {
    align-items: flex-start;
  }
  .start span {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .start small {
    color: var(--side-muted);
    font-size: 0.78rem;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding-top: 1rem;
    border-top: 1px solid var(--rule);
  }
  footer .link {
    color: var(--side-muted);
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
    padding: 0.2rem 0;
  }
  .version {
    font-size: 0.75rem;
    color: var(--side-muted);
  }

  main {
    flex: 1;
    min-width: 0;
    position: relative;
    overflow: auto;
    padding: 4rem clamp(2rem, 5vw, 4.5rem) 3rem;
    display: flex;
    flex-direction: column;
    gap: 1.3rem;
  }
  main > :not(:global(.watermark)) {
    position: relative;
    z-index: 1;
  }
  .label {
    color: var(--accent);
    font-size: 0.7rem;
  }
  .continue {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    padding: 1.5rem;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: 6px;
    box-shadow: var(--shadow);
    margin-bottom: 1rem;
  }
  .continue .name {
    font-size: clamp(2rem, 3.4vw, 2.8rem);
    line-height: 1;
  }
  .continue .primary {
    padding: 0.8rem 1.3rem;
  }
  .texts {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    text-align: left;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    font-size: 0.82rem;
    color: var(--muted);
    font-weight: 400;
  }
  .where {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
  }
  .cap {
    width: 2.75rem;
    height: 2.75rem;
    flex: none;
    display: grid;
    place-items: center;
    background: var(--track);
    color: var(--text);
    border-radius: 4px;
    font-size: 1.7rem;
  }
  .cap.big {
    width: 5.5rem;
    height: 5.5rem;
    font-size: 3.4rem;
    background: var(--accent);
    color: var(--accent-text);
  }
  :global(:root.no-capitals) .cap {
    display: none;
  }
  .list-head .count {
    font-size: 0.75rem;
    color: var(--muted);
  }
  .path {
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
    font-size: 0.75rem;
    color: var(--muted);
    min-width: 0;
  }
  .path span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .path :global(svg) {
    flex: none;
  }
  ul {
    list-style: none;
    margin: -0.6rem 0 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    border-bottom: 1px solid var(--rule);
    padding-right: 0.4rem;
  }
  li:hover {
    background: var(--surface);
  }
  .project {
    flex: 1;
    min-width: 0;
    justify-content: flex-start;
    gap: 1rem;
    padding: 0.85rem 0.5rem;
    border: none;
    background: none !important;
    text-transform: none;
    letter-spacing: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text);
  }
  li .icon,
  .continue .icon,
  .list-head .icon {
    color: var(--muted);
  }
  .empty {
    margin: 0;
    color: var(--muted);
  }
</style>

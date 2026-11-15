<script lang="ts">
  // The main screen for an open project (designs D–G in pencil/comilito.pen):
  //
  //   header
  //   context bar (breadcrumbs, while editing something or in the Library)
  //   Scenes/Presets | Tapestry or an in-place editor | Effects     ← "tapestry" view
  //   …or the full-screen Library workspace                        ← "library" view
  //   Library drawer (tapestry view only)
  //   mixer bar
  import { Download, Info, Library, Settings2, SlidersHorizontal, X } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { exportProject, relinkFromFolder } from "../lib/actions";
  import { askText, confirmAction, run, showView, ui } from "../lib/store.svelte";
  import ContextBar from "./ContextBar.svelte";
  import EffectsPad from "./EffectsPad.svelte";
  import LeftPanel from "./LeftPanel.svelte";
  import LibraryDrawer from "./LibraryDrawer.svelte";
  import LibraryWorkspace from "./LibraryWorkspace.svelte";
  import MixerBar from "./MixerBar.svelte";
  import PresetEditor from "./PresetEditor.svelte";
  import PresetOutline from "./PresetOutline.svelte";
  import SceneEditor from "./SceneEditor.svelte";
  import TapestryPanel from "./TapestryPanel.svelte";

  const snapshot = $derived(ui.snapshot!);
  const project = $derived(snapshot.project!);
  const missingCount = $derived(snapshot.missing.length);
  const running = $derived(ui.downloads.filter((d) => d.status === "running" || d.status === "queued").length);
  const convertingIds = $derived(ui.conversions.filter((c) => c.status === "running" || c.status === "queued").map((c) => c.id));
  const converting = $derived(convertingIds.length);

  async function removeMissing() {
    const ok = await confirmAction(
      "Remove sounds with missing files?",
      `${missingCount} sound(s) or video(s) will be removed from the library and from every scene.`,
      "Remove",
      true,
    );
    if (ok) run(api.removeMissingSounds());
  }

  async function rename() {
    const name = await askText("Rename project", "Name", project.name);
    if (name) run(api.renameProject(name));
  }

  async function closeProject() {
    if (ui.view === "library") await showView("tapestry");
    run(api.closeProject());
  }
</script>

<div class="workspace">
  <header>
    <button class="title display" onclick={rename} title="Rename project">{project.name}</button>
    <span class="spacer"></span>
    {#if running > 0}
      <span class="status">Downloading {running}…</span>
    {/if}
    {#if converting > 0}
      <button class="status link" onclick={() => (ui.convertOffer = { ids: convertingIds, origin: "library" })}>
        Converting {converting} video{converting === 1 ? "" : "s"}…
      </button>
    {/if}
    {#if snapshot.analyzingRemaining > 0}
      <span class="status" title="Measuring loudness for normalization">Measuring loudness · {snapshot.analyzingRemaining} left</span>
    {:else if snapshot.hashingRemaining > 0}
      <span class="status" title="Fingerprinting audio files to find them again if they go missing">
        Fingerprinting · {snapshot.hashingRemaining} left
      </span>
    {/if}
    <nav>
      <button class="icon" class:on={ui.view === "library"} title="Library workspace" onclick={() => showView(ui.view === "library" ? "tapestry" : "library")}>
        <Library size={18} />
      </button>
      <button class="icon" title="Export…" onclick={exportProject}><Download size={18} /></button>
      <button class="icon" title="Project settings" onclick={() => (ui.editing = { kind: "projectSettings" })}><Settings2 size={18} /></button>
      <button class="icon" title="Preferences" onclick={() => (ui.editing = { kind: "preferences" })}><SlidersHorizontal size={18} /></button>
      <button class="icon" title="About" onclick={() => (ui.editing = { kind: "about" })}><Info size={18} /></button>
      <button class="icon" title="Close project" onclick={closeProject}><X size={18} /></button>
    </nav>
  </header>

  {#if snapshot.changedOnDisk}
    <div class="banner">
      <span>comilito.json was changed by another program (a sync tool?).</span>
      <button onclick={() => run(api.reloadProject())}>Reload from disk</button>
      <button onclick={() => run(api.keepMyVersion())}>Keep mine</button>
    </div>
  {/if}
  {#if missingCount > 0}
    <div class="banner">
      <span>
        {missingCount} file{missingCount === 1 ? "" : "s"} can't be found (missing sounds are listed under Missing
        files in the Library, missing videos are marked in its Video shelf).
      </span>
      <button onclick={relinkFromFolder}>Find in a folder…</button>
      <button class="danger" onclick={removeMissing}>Remove them</button>
    </div>
  {/if}

  {#if ui.focus || ui.view === "library"}
    <ContextBar />
  {/if}

  {#if ui.view === "library"}
    <main class="library">
      <LibraryWorkspace />
    </main>
  {:else}
    <main>
      <LeftPanel />
      <section class="centre">
        {#if ui.focus?.kind === "scene"}
          {#key ui.focus.id}<SceneEditor id={ui.focus.id} />{/key}
        {:else if ui.focus?.kind === "preset"}
          {#key ui.focus.id}<PresetEditor id={ui.focus.id} />{/key}
        {:else}
          <TapestryPanel />
        {/if}
      </section>
      {#if ui.focus?.kind === "preset" || (ui.focus?.kind === "scene" && ui.focus.from)}
        <PresetOutline />
      {:else}
        <EffectsPad />
      {/if}
    </main>
    <LibraryDrawer />
  {/if}

  <MixerBar />
</div>

<style>
  .workspace {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0 1rem 0 1.4rem;
    height: 3.6rem;
    flex: none;
    background: var(--header-bg);
    color: var(--header-text);
  }
  .title {
    border: none;
    padding: 0;
    font-size: 1.9rem;
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
  }
  .title:hover:not(:disabled) {
    background: none;
    text-decoration: underline;
    text-decoration-thickness: 1px;
  }
  .spacer {
    flex: 1;
  }
  .status {
    font-size: 0.78rem;
    opacity: 0.8;
  }
  button.status {
    background: none;
    border: none;
    color: inherit;
    padding: 0;
    text-transform: none;
    letter-spacing: 0;
    text-decoration: underline dotted;
  }
  nav {
    display: flex;
    gap: 0.15rem;
  }
  nav .on {
    background: color-mix(in srgb, currentColor 18%, transparent);
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.45rem 1rem;
    background: color-mix(in srgb, var(--warn) 18%, var(--bg));
    border-bottom: 1px solid var(--rule);
    font-size: 0.85rem;
  }
  .banner span {
    flex: 1;
  }
  .banner button {
    padding: 0.3rem 0.6rem;
  }
  main {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(15rem, 19rem) 1fr minmax(13rem, 17rem);
  }
  main.library {
    display: flex;
  }
  .centre {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    position: relative;
  }
</style>

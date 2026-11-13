<script lang="ts">
  // The main screen for an open project (layout from docs/scenes.md "UI layout"):
  //   header / library | stack | effects / mixer bar
  import { api } from "../lib/api";
  import { exportProject, relinkFromFolder } from "../lib/actions";
  import { ask, askText, run, ui } from "../lib/store.svelte";
  import ContextMenu from "./ContextMenu.svelte";
  import EffectsPad from "./EffectsPad.svelte";
  import Library from "./Library.svelte";
  import MixerBar from "./MixerBar.svelte";
  import StackPanel from "./StackPanel.svelte";

  // `$derived` recomputes when what it reads changes. The `!` tells TypeScript the project
  // exists: App only shows this component when it does.
  const project = $derived(ui.snapshot!.project!);
  const missingCount = $derived(ui.snapshot!.missing.length);

  async function removeMissing() {
    const answer = await ask({
      title: "Remove sounds with missing files?",
      message: `${missingCount} sound(s) will be removed from the library and from every scene.`,
      choices: [
        { label: "Cancel", value: "cancel" },
        { label: "Remove", value: "remove", danger: true, primary: true },
      ],
    });
    if (answer?.choice === "remove") run(api.removeMissingSounds());
  }

  async function rename() {
    const name = await askText("Rename project", "Name", project.name);
    if (name) run(api.renameProject(name));
  }
</script>

<div class="workspace">
  <header>
    <button class="title" onclick={rename} title="Rename project">
      <h1>{project.name}</h1>
    </button>


    <div class="spacer"></div>
    {#if ui.snapshot!.analyzingRemaining > 0}
      <span class="muted" title="Measuring loudness for normalization">
        Measuring loudness: {ui.snapshot!.analyzingRemaining} left…
      </span>
    {:else if ui.snapshot!.hashingRemaining > 0}
      <span class="muted" title="Fingerprinting audio files to find them again if they go missing">
        Hashing {ui.snapshot!.hashingRemaining} file{ui.snapshot!.hashingRemaining === 1 ? "" : "s"}…
      </span>
    {/if}
    <button onclick={exportProject}>Export…</button>
    <button onclick={() => (ui.editing = { kind: "projectSettings" })}>Project settings</button>
    <button onclick={() => (ui.editing = { kind: "preferences" })}>Preferences</button>
    <button onclick={() => (ui.editing = { kind: "about" })}>About</button>
    <button onclick={() => run(api.closeProject())}>Close</button>
  </header>

  {#if ui.snapshot!.changedOnDisk}
    <div class="banner">
      <span>comilito.json was changed by another program (a sync tool?).</span>
      <button onclick={() => run(api.reloadProject())}>Reload from disk</button>
      <button onclick={() => run(api.keepMyVersion())}>Keep mine</button>
    </div>
  {/if}
  {#if missingCount > 0}
    <div class="banner">
      <span>
        {missingCount} sound{missingCount === 1 ? "" : "s"} can't find {missingCount === 1 ? "its" : "their"}
        audio file. They're marked ⚠ in the Sounds tab.
      </span>
      <button onclick={relinkFromFolder}>Find in a folder…</button>
      <button onclick={removeMissing}>Remove them</button>
    </div>
  {/if}

  <main>
    <Library />
    <StackPanel />
    <EffectsPad />
  </main>

  <MixerBar />
</div>

<ContextMenu />

<style>
  .workspace {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.8rem;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }
  .title {
    background: none;
    border: none;
    padding: 0;
  }
  h1 {
    font-size: 1.1rem;
  }
  .spacer {
    flex: 1;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0.8rem;
    background: color-mix(in srgb, var(--warn) 18%, var(--panel));
    border-bottom: 1px solid var(--border);
  }
  main {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(15rem, 20rem) 1fr minmax(12rem, 16rem);
  }
</style>

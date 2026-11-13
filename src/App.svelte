<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import { init, project, rememberedMode, run, ui } from "./lib/store.svelte";
  import ProjectPicker from "./components/ProjectPicker.svelte";
  import Workspace from "./components/Workspace.svelte";
  import Dialog from "./components/Dialog.svelte";
  import Editors from "./components/Editors.svelte";
  import Notices from "./components/Notices.svelte";

  let ready = $state(false);

  onMount(async () => {
    await init();
    ready = true;
  });

  // Theme: "system" follows the OS; otherwise force light or dark via a data attribute
  // that app.css reads. `$effect` re-runs whenever something it reads changes.
  $effect(() => {
    const theme = ui.prefs?.appearance.theme ?? "system";
    if (theme === "system") delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = theme;
  });

  // Whenever a different project opens, restore the mode it was last left in.
  let lastProjectDir: string | null = null;
  $effect(() => {
    const dir = ui.snapshot?.projectDir ?? null;
    if (dir && dir !== lastProjectDir) ui.mode = rememberedMode(dir);
    lastProjectDir = dir;
  });

  /** Global shortcuts: Space, Esc and scene hotkeys (docs/scenes.md "Controls"). */
  function onKeydown(e: KeyboardEvent) {
    // Already handled by a dialog (see Dialog.svelte).
    if (e.defaultPrevented) return;
    // Never steal keys while typing in a field.
    const target = e.target as HTMLElement;
    if (target.closest("input, textarea, select, [contenteditable]")) return;
    // While a dialog or editor is open, keys belong to it.
    if (ui.question || ui.editing) return;
    const p = project();
    if (!p) return;

    if (e.key === " ") {
      e.preventDefault();
      run(api.togglePlay());
    } else if (e.key === "Escape") {
      run(api.stopAll());
    } else if (!e.ctrlKey && !e.altKey && !e.metaKey) {
      const scene = p.scenes.find((s) => s.hotkey && s.hotkey.toLowerCase() === e.key.toLowerCase());
      if (scene) {
        e.preventDefault();
        run(api.sceneHotkey(scene.id));
      }
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if ready}
  {#if ui.snapshot?.project}
    <Workspace />
  {:else}
    <ProjectPicker />
  {/if}
{/if}

<Editors />
<Dialog />
<Notices />

<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import { init, leaveFocus, project, run, ui } from "./lib/store.svelte";
  import { activeTheme, applyTheme } from "./lib/themes";
  import ProjectPicker from "./components/ProjectPicker.svelte";
  import Workspace from "./components/Workspace.svelte";
  import Dialog from "./components/Dialog.svelte";
  import Editors from "./components/Editors.svelte";
  import Notices from "./components/Notices.svelte";
  import ContextMenu from "./components/ContextMenu.svelte";
  import SaveTapestryDialog from "./components/SaveTapestryDialog.svelte";

  let ready = $state(false);
  /** Bumped when the OS light/dark preference changes, to re-run the theme effect. */
  let systemScheme = $state(0);

  onMount(() => {
    init().then(() => (ready = true));
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => systemScheme++;
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  });

  // Apply the theme whenever anything it depends on changes: the preferences, the desktop's
  // accent or light/dark, the user's themes. `$effect` tracks what it reads and re-runs.
  $effect(() => {
    void systemScheme;
    if (!ui.prefs) return;
    // While Preferences is open, show what's being edited.
    const appearance = ui.previewAppearance ?? ui.prefs.appearance;
    const theme = ui.previewTheme ?? activeTheme(appearance, ui.desktop, ui.userThemes);
    applyTheme(theme, appearance, ui.desktop);
  });

  /** Global shortcuts: Space, Esc, / and scene hotkeys (docs/scenes.md "Controls"). */
  function onKeydown(e: KeyboardEvent) {
    // Already handled by a dialog or menu.
    if (e.defaultPrevented) return;
    // Never steal keys while typing in a field.
    const target = e.target as HTMLElement;
    if (target.closest("input, textarea, select, [contenteditable]")) return;
    // While a dialog or settings window is open, keys belong to it.
    if (ui.question || ui.editing) return;
    const p = project();
    if (!p) return;

    if (e.key === " ") {
      e.preventDefault();
      run(api.togglePlay());
    } else if (e.key === "Escape") {
      // Esc first leaves an editor; with nothing open it stops everything.
      if (ui.library.cropping) ui.library.cropping = false;
      else if (ui.focus) leaveFocus();
      else if (ui.view === "library") ui.view = "tapestry";
      else run(api.stopAll());
    } else if (e.key === "/") {
      e.preventDefault();
      document.querySelector<HTMLInputElement>("[data-library-search]")?.focus();
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
<SaveTapestryDialog />
<Dialog />
<ContextMenu />
<Notices />

<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import { init, leaveFocus, notify, project, run, ui } from "./lib/store.svelte";
  import { activeTheme, applyTheme, loadThemeFiles } from "./lib/themes";
  import ProjectPicker from "./components/ProjectPicker.svelte";
  import Workspace from "./components/Workspace.svelte";
  import Dialog from "./components/Dialog.svelte";
  import Editors from "./components/Editors.svelte";
  import Notices from "./components/Notices.svelte";
  import ContextMenu from "./components/ContextMenu.svelte";
  import SaveTapestryDialog from "./components/SaveTapestryDialog.svelte";
  import FileDropImport from "./components/FileDropImport.svelte";
  import MovingProjects from "./components/MovingProjects.svelte";

  let ready = $state(false);
  /** Bumped when the OS light/dark preference changes, to re-run the theme effect. */
  let systemScheme = $state(0);

  onMount(() => {
    init()
      .then(openLastProject)
      .then(() => (ready = true));
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => systemScheme++;
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  });

  /** Counts theme-file loads, so only the latest one is used. A plain variable (not $state):
   *  the effect below must not re-run when it changes. */
  let themeFilesRequest = 0;

  // Apply the theme whenever anything it depends on changes: the preferences, the desktop's
  // accent or light/dark, the user's themes. `$effect` tracks what it reads and re-runs.
  $effect(() => {
    void systemScheme;
    if (!ui.prefs) return;
    // While Preferences is open, show what's being edited.
    const appearance = ui.previewAppearance ?? ui.prefs.appearance;
    const theme = ui.previewTheme ?? activeTheme(appearance, ui.desktop, ui.userThemes);
    applyTheme(theme, appearance, ui.desktop);
    // Fonts and the watermark load in the background. If the theme changes again before
    // they arrive, the older result is dropped.
    const request = ++themeFilesRequest;
    loadThemeFiles(theme).then((url) => {
      if (request === themeFilesRequest) ui.themeWatermark = url;
    });
  });

  /** "Open the last project at start" (the home screen's switch). If it fails (moved or
   *  deleted), the home screen shows instead, with the reason. */
  async function openLastProject() {
    const last = ui.prefs?.recentProjects[0];
    if (!ui.prefs?.openLastProject || !last || ui.snapshot?.project) return;
    try {
      await api.openProject(last);
    } catch (e) {
      notify(`Couldn't open the last project: ${e}`);
    }
  }

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
<FileDropImport />
<MovingProjects />
<Dialog />
<ContextMenu />
<Notices />

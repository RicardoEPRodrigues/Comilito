<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import { init, leaveFocus, notify, project, restartInterface, run, savePrefs, setMiniPlayer, ui } from "./lib/store.svelte";
  import { activeTheme, applyTheme, loadThemeFiles } from "./lib/themes";
  import { applyScale, stepScale } from "./lib/scale";
  import ProjectPicker from "./components/ProjectPicker.svelte";
  import Workspace from "./components/Workspace.svelte";
  import Dialog from "./components/Dialog.svelte";
  import Editors from "./components/Editors.svelte";
  import Notices from "./components/Notices.svelte";
  import ContextMenu from "./components/ContextMenu.svelte";
  import SaveTapestryDialog from "./components/SaveTapestryDialog.svelte";
  import FileDropImport from "./components/FileDropImport.svelte";
  import MovingProjects from "./components/MovingProjects.svelte";
  import ConvertVideos from "./components/ConvertVideos.svelte";
  import AppMenuOffer from "./components/AppMenuOffer.svelte";

  let ready = $state(false);
  /** Bumped when the OS light/dark preference changes, to re-run the theme effect. */
  let systemScheme = $state(0);

  onMount(() => {
    init()
      .then(openLastProject)
      .then(() => (ready = true))
      .then(() => {
        offerAppMenu();
        checkForUpdate();
      });
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

  // The interface scale (decision 0048), live while Preferences is open, like the theme.
  $effect(() => {
    if (!ui.prefs) return;
    const scale = (ui.previewAppearance ?? ui.prefs.appearance).scale;
    applyScale(scale).catch((e) => notify(`Couldn't scale the interface: ${e}`));
  });

  /** Ctrl + / Ctrl − / Ctrl 0 change the interface scale and save it. With Preferences open
   *  they do nothing: its draft would overwrite the change on Save. */
  function scaleShortcut(e: KeyboardEvent): boolean {
    if (!e.ctrlKey || e.altKey || e.metaKey || !ui.prefs || ui.previewAppearance) return false;
    const now = ui.prefs.appearance.scale;
    // "=" is the unshifted "+" on US keyboards; "+" covers the keypad and other layouts.
    let next: number;
    if (e.key === "+" || e.key === "=") next = stepScale(now, 1);
    else if (e.key === "-") next = stepScale(now, -1);
    else if (e.key === "0") next = 1;
    else return false;
    if (next !== now) savePrefs({ ...ui.prefs, appearance: { ...ui.prefs.appearance, scale: next } });
    return true;
  }

  // The mini player (decision 0049) is for a project. Started in it with none open (it was on
  // when the app quit, and "open the last project" is off), or the project closed: back to the
  // full window, or the home screen would be stuck small and on top.
  $effect(() => {
    if (ready && ui.mini.on && !ui.snapshot?.project) setMiniPlayer(false);
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

  /** The AppImage not in the app menu, or its entry out of date: ask (decision 0065), unless
   *  that's turned off. Only looks; anything changes only if the user says yes. */
  async function offerAppMenu() {
    try {
      ui.appMenu = await api.appMenuStatus();
    } catch {
      return;
    }
    const asks = ["missing", "moved", "otherCopy", "changed"].includes(ui.appMenu.state);
    if (asks && ui.prefs?.behaviour.askAppMenu) ui.appMenuOffer = "start";
  }

  /** Once per run, if allowed: a newer release on GitHub (decision 0066) gets a notice with a
   *  button to the website's downloads. Offline or any failure: nothing. */
  async function checkForUpdate() {
    if (!ui.prefs?.behaviour.checkForUpdates) return;
    try {
      const update = await api.checkForUpdate();
      if (update) {
        notify(`Comilito ${update.version} is available.`, {
          sticky: true,
          link: { label: "Download", url: update.downloadPage },
        });
      }
    } catch (e) {
      api.logMessage("info", String(e)).catch(() => {});
    }
  }

  /** True if `e` is the key set for `shortcut` (letters match either case). */
  function isShortcut(e: KeyboardEvent, shortcut: string | undefined): boolean {
    return !!shortcut && !e.ctrlKey && !e.altKey && !e.metaKey && e.key.toLowerCase() === shortcut.toLowerCase();
  }

  /** Global shortcuts: Space, Esc, /, the configurable ones and scene hotkeys (docs/scenes.md "Controls"). */
  function onKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    const typing = !!target.closest("input, textarea, select, [contenteditable]");
    // Restarting the interface is the way out when something is stuck, so it comes first and
    // works with a dialog open, or in a field when the key doesn't type anything (F5).
    if (isShortcut(e, ui.prefs?.shortcuts.restartInterface) && (e.key.length > 1 || !typing)) {
      e.preventDefault();
      restartInterface();
      return;
    }
    // Already handled by a dialog or menu.
    if (e.defaultPrevented) return;
    // Scaling works in text fields too: none of its keys type anything with Ctrl held.
    if (scaleShortcut(e)) {
      e.preventDefault();
      return;
    }
    // Never steal keys while typing in a field.
    if (typing) return;
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
    } else if (isShortcut(e, ui.prefs?.shortcuts.removeTopScene)) {
      // The stack is stored bottom to top, so the top scene is the last entry.
      e.preventDefault();
      const top = p.stack[p.stack.length - 1];
      if (top) run(api.stackRemove(top.id));
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
{#if ui.convertOffer && ui.snapshot?.project}
  <!-- Keyed by the first video, so a new offer starts with fresh ticks. -->
  {#key ui.convertOffer.ids[0]}<ConvertVideos />{/key}
{/if}
<MovingProjects />
{#if ui.appMenuOffer && ui.appMenu}<AppMenuOffer />{/if}
<Dialog />
<ContextMenu />
<Notices />

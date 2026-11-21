<script lang="ts">
  // App preferences (this computer, docs/preferences.md): levels for new projects, playback,
  // projects folder and download tools, behaviour, and appearance. Never changes existing
  // projects. Appearance changes preview live and are undone if you cancel.
  import { Film, FolderOpen, LoaderCircle, MousePointerClick, Palette, Play, RefreshCw, RotateCcw, SlidersHorizontal } from "@lucide/svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { Component } from "svelte";
  import { api } from "../lib/api";
  import { ask, confirmAction, formatBytes, moveProjects, notify, openLogs, refreshTools, restartInterface, run, ui } from "../lib/store.svelte";
  import type { OutputDevice, Preferences, Screen, VideoEncoder } from "../lib/types";
  import { CATEGORIES } from "../lib/types";
  import { closeEditor, fingerprint } from "../lib/unsaved";
  import AppearancePage from "./AppearancePage.svelte";
  import DefaultsForm from "./DefaultsForm.svelte";
  import KeyInput from "./KeyInput.svelte";
  import Modal from "./Modal.svelte";
  import ToolsInstall from "./ToolsInstall.svelte";

  let { onclose, page: startPage }: { onclose: () => void; page?: string } = $props();

  const PAGES: { id: string; label: string; icon: Component }[] = [
    { id: "levels", label: "Levels", icon: SlidersHorizontal },
    { id: "playback", label: "Playback", icon: Play },
    { id: "video", label: "Video", icon: Film },
    { id: "projects", label: "Projects & tools", icon: FolderOpen },
    { id: "behaviour", label: "Behaviour", icon: MousePointerClick },
    { id: "appearance", label: "Appearance", icon: Palette },
  ];
  // svelte-ignore state_referenced_locally
  let page = $state(startPage ?? "appearance");

  let draft = $state<Preferences | null>(ui.prefs ? structuredClone($state.snapshot(ui.prefs)) : null);
  const initial = fingerprint(ui.prefs);
  const dirty = $derived(fingerprint($state.snapshot(draft)) !== initial);

  // Show appearance changes live while this window is open.
  $effect(() => {
    if (draft) ui.previewAppearance = $state.snapshot(draft.appearance);
    return () => (ui.previewAppearance = null);
  });

  // GPU encoders (decision 0051), looked for when the Video page is first shown: the first
  // time takes a few seconds of test encodes.
  let encoders = $state<VideoEncoder[] | null>(null);
  $effect(() => {
    if (page === "video" && encoders === null) api.videoEncoders().then((e) => (encoders = e)).catch(() => (encoders = []));
  });
  /** An ffmpeg from before decision 0051, whose GPU encoders may not work with current drivers. */
  const oldFfmpeg = $derived(!!ui.tools?.ffmpeg && !ui.tools.ffmpegVersion);

  async function chooseFolder() {
    const folder = await open({ directory: true, title: "Projects folder" });
    if (typeof folder === "string" && draft) draft.projectsFolder = folder;
  }

  /** Moves every project into another folder, right away (not on Save). Backups stay in the
   *  app's data folder (decision 0026). */
  async function moveToFolder() {
    const to = await open({ directory: true, title: "Move the projects to…" });
    if (typeof to !== "string" || !draft) return;
    const from = (await run(api.projectsFolder())) ?? "";
    const ok = await confirmAction(
      "Move all projects?",
      `Every project moves from ${from} to ${to}, which becomes the projects ` +
        "folder. Other files in the old folder stay where they are. An open project is closed and " +
        "reopened from its new place.",
      "Move projects",
    );
    if (!ok) return;
    try {
      const moved = await moveProjects(to);
      notify(`Moved ${moved} folder${moved === 1 ? "" : "s"} to ${to}.`);
      draft.projectsFolder = ui.prefs?.projectsFolder ?? null;
    } catch (e) {
      notify(`Couldn't move the projects: ${e}`);
    }
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

  /** Update: downloads only what's out of date (Rust checks), with its progress shown here. */
  let updatingTools = $state(false);
  async function updateTools() {
    updatingTools = true;
    ui.toolsProgress = null;
    try {
      const tools = await api.installTools();
      ui.tools = tools;
      // A new ffmpeg may find other GPU encoders.
      if (tools.updated.includes("ffmpeg")) encoders = null;
      const versions = `yt-dlp ${tools.ytDlp ?? "?"}, ffmpeg ${tools.ffmpegVersion ?? "?"}`;
      notify(tools.updated.length ? `Updated ${tools.updated.join(" and ")} (${versions}).` : `The download tools are up to date (${versions}).`);
    } catch (err) {
      notify(`Couldn't update the download tools: ${err}`);
    } finally {
      updatingTools = false;
      ui.toolsProgress = null;
    }
  }

  // Decoding videos on the GPU (decision 0052): Linux only, read when Comilito starts.
  let gpuDecode = $state<{ available: boolean; active: boolean } | null>(null);
  $effect(() => {
    if (page === "video" && gpuDecode === null) api.gpuDecode().then((g) => (gpuDecode = g)).catch(() => {});
  });

  async function removeTools() {
    if (!(await confirmAction("Remove the download tools?", "yt-dlp and ffmpeg are deleted. They're downloaded again the next time you need them.", "Remove", true))) return;
    await run(api.removeTools());
    await refreshTools();
  }

  /** Removes preferences, themes, tools, logs, caches and (if asked) backups, never projects,
   *  then quits. Backups live in the app's data folder (decision 0026), so they're asked about. */
  // The app menu entry (decision 0065), looked at when this window opens.
  $effect(() => {
    api.appMenuStatus().then((s) => (ui.appMenu = s)).catch(() => {});
  });

  async function removeFromAppMenu() {
    const answer = await ask({
      title: "Remove Comilito from the app menu?",
      message: "Removes its launcher and icons. The AppImage itself stays where it is, and still runs.",
      choices: [
        { label: "Cancel", value: "cancel" },
        { label: "Remove", value: "remove", danger: true, primary: true },
      ],
    });
    if (answer?.choice === "remove") {
      const s = await run(api.removeFromAppMenu());
      if (s) ui.appMenu = s;
    }
  }

  async function forgetEverything() {
    const answer = await ask({
      title: "Remove all Comilito data from this computer?",
      message:
        "Deletes your preferences, your own themes, the downloaded tools, logs and caches, then " +
        "closes Comilito. Your projects are kept. Use this before uninstalling, or to start over " +
        "with a clean slate.\n\nBackups are kept with this data. Keep them, or delete them too?",
      choices: [
        { label: "Cancel", value: "cancel" },
        { label: "Delete backups too", value: "all", danger: true },
        { label: "Keep backups", value: "keep", primary: true },
      ],
    });
    if (answer?.choice === "keep" || answer?.choice === "all") await run(api.forgetAllData(answer.choice === "keep"));
  }

  // Audio outputs, listed when the Playback page is shown (and on Refresh, after plugging
  // something in).
  let outputs = $state<{ devices: OutputDevice[]; defaultId: string | null } | null>(null);
  async function refreshOutputs() {
    outputs = (await run(api.listOutputDevices())) ?? { devices: [], defaultId: null };
  }
  $effect(() => {
    if (page === "playback" && outputs === null) refreshOutputs();
  });
  // Screens for the video window, listed when the Video page is shown.
  let screens = $state<Screen[] | null>(null);
  $effect(() => {
    if (page === "video" && screens === null) run(api.listScreens()).then((s) => (screens = s ?? []));
  });
  const defaultName = $derived(outputs?.devices.find((d) => d.id === outputs?.defaultId)?.name);
  /** The saved output, when it isn't connected right now (so it's still shown as chosen). */
  const missingOutput = $derived(
    draft?.playback.outputDevice && outputs && !outputs.devices.some((d) => d.id === draft?.playback.outputDevice)
      ? draft.playback.outputDevice
      : null,
  );

  const pct = (v: number) => Math.round(v * 100);
  const seconds = (ms: number) => ms / 1000;
  const toMs = (s: number) => Math.round(Math.max(0, s || 0) * 1000);
</script>

<Modal title="Preferences" onclose={() => closeEditor(dirty, save, onclose)} wide tall bare>
  {#if draft}
    <div class="layout">
      <nav>
        {#each PAGES as p (p.id)}
          <button class:active={page === p.id} onclick={() => (page = p.id)}><p.icon size={15} />{p.label}</button>
        {/each}
      </nav>
      <div class="content">
        <h2 class="display">{PAGES.find((p) => p.id === page)?.label}</h2>

        {#if page === "levels"}
          <p class="muted">The levels new projects start with. Existing projects keep their own (Project settings).</p>
          <section>
            <div class="section-head"><span class="caps">Volumes</span></div>
            <div class="volumes">
              <label class="field">
                <span class="caps">Master {pct(draft.newProjectDefaults.masterVolume)}%</span>
                <input type="range" min="0" max="100" value={pct(draft.newProjectDefaults.masterVolume)}
                  oninput={(e) => (draft!.newProjectDefaults.masterVolume = e.currentTarget.valueAsNumber / 100)} />
              </label>
              {#each CATEGORIES as c (c.id)}
                {@const cat = draft.newProjectDefaults.categories[c.id]}
                <label class="field">
                  <span class="caps">{c.label} {pct(cat.volume)}%</span>
                  <input type="range" min="0" max="100" value={pct(cat.volume)} oninput={(e) => (cat.volume = e.currentTarget.valueAsNumber / 100)} />
                </label>
              {/each}
            </div>
          </section>
          <DefaultsForm bind:defaults={draft.newProjectDefaults.defaults} bind:backups={draft.newProjectDefaults.backups} />
        {:else if page === "playback"}
          <section class="form">
            <div class="row">
              <span>Audio output</span>
              <select
                value={draft.playback.outputDevice ?? ""}
                onchange={(e) => (draft!.playback.outputDevice = e.currentTarget.value || null)}
              >
                <option value="">System default{defaultName ? ` (${defaultName})` : ""}</option>
                {#each outputs?.devices ?? [] as d (d.id)}<option value={d.id}>{d.name}</option>{/each}
                {#if missingOutput}<option value={missingOutput}>{missingOutput} (not connected)</option>{/if}
              </select>
              <button class="icon" title="Look for outputs again" onclick={refreshOutputs}><RefreshCw size={15} /></button>
            </div>
            <p class="muted small">
              The system default follows your desktop's sound settings. A chosen output that's
              unplugged falls back to the system default. Changing it restarts what's playing.
            </p>
            <label class="row">
              <span>Pause / resume fade (Space)</span>
              <input type="number" min="0" max="10" step="0.1" value={seconds(draft.playback.pauseFadeMs)}
                onchange={(e) => (draft!.playback.pauseFadeMs = toMs(e.currentTarget.valueAsNumber))} />
              <span class="muted">s · a short fade avoids clicks</span>
            </label>
            <label class="row">
              <span>Stop all fade (Esc)</span>
              <input type="number" min="0" max="10" step="0.1" value={seconds(draft.playback.stopFadeMs)}
                onchange={(e) => (draft!.playback.stopFadeMs = toMs(e.currentTarget.valueAsNumber))} />
              <span class="muted">s</span>
            </label>
          </section>
        {:else if page === "video"}
          <!-- Decision 0035. The project's own video defaults are under Levels (new projects)
               and in Project settings. -->
          <section class="form">
            <label class="switch">
              <input type="checkbox" bind:checked={draft.video.enabled} />
              <span>Use videos</span>
            </label>
            <p class="muted small">
              Off hides the video window, the Video sections, the video effects and the Library's
              Video shelf on this computer. Projects keep their videos and video settings, and
              they come back when it's turned on.
            </p>
          </section>
          {#if draft.video.enabled}
            <section class="form">
              <div class="section-head"><span class="caps">Video window</span></div>
              <label class="row">
                <span>Screen</span>
                <select value={draft.video.screen ?? ""} onchange={(e) => (draft!.video.screen = e.currentTarget.value || null)}>
                  <option value="">The main screen</option>
                  {#each screens ?? [] as s (s.name)}<option value={s.name}>{s.name} · {s.width}×{s.height}{s.primary ? " (main)" : ""}</option>{/each}
                  {#if draft.video.screen && screens && !screens.some((s) => s.name === draft!.video.screen)}
                    <option value={draft.video.screen}>{draft.video.screen} (not connected)</option>
                  {/if}
                </select>
              </label>
              <p class="muted small">On Wayland apps can't move their windows: fullscreen goes to this screen, but a window that isn't fullscreen has to be dragged there.</p>
              <label class="row">
                <span>Picture</span>
                <select bind:value={draft.video.fit}>
                  <option value="cover">Fill the window (crop the edges)</option>
                  <option value="contain">Show the whole picture (black bars)</option>
                </select>
              </label>
              <label class="row">
                <span>Crossfade between videos</span>
                <input type="number" min="0" max="10" step="0.1" value={seconds(draft.video.crossfadeMs)}
                  onchange={(e) => (draft!.video.crossfadeMs = toMs(e.currentTarget.valueAsNumber))} />
                <span class="muted">s</span>
              </label>
              <label class="switch">
                <input type="checkbox" bind:checked={draft.video.fullscreen} />
                <span>Open the video window in fullscreen</span>
              </label>
              <label class="switch">
                <input type="checkbox" bind:checked={draft.video.keepPlayingWhenPaused} />
                <span>Keep the video playing while the sound is paused (Space); Esc still blanks it</span>
              </label>
              {#if gpuDecode?.available}
                <label class="switch">
                  <input type="checkbox" bind:checked={draft.video.gpuDecode} />
                  <span>Decode videos on the GPU (NVIDIA)</span>
                </label>
                <p class="muted small">
                  Lets NVIDIA's GStreamer decoders play videos, which takes work off the CPU. Needs
                  <code>gstreamer1.0-plugins-bad</code>; VP8 and VP9 on GStreamer 1.20, AV1 too on newer
                  ones. Intel and AMD GPUs are used without this setting. Turn it off if videos freeze
                  after jumping to another point or resuming.
                  {#if draft.video.gpuDecode !== gpuDecode.active}<strong class="warn-note">Applies the next time Comilito starts.</strong>{/if}
                </p>
              {/if}
            </section>
            <section class="form">
              <div class="section-head"><span class="caps">Downloads and import</span></div>
              <label class="row">
                <span>Video resolution when downloading</span>
                <select bind:value={draft.video.downloadHeight}>
                  <option value={720}>Up to 720p</option>
                  <option value={1080}>Up to 1080p</option>
                  <option value={1440}>Up to 1440p</option>
                  <option value={0}>Best available</option>
                </select>
              </label>
              <label class="row">
                <span>Videos that may not play</span>
                <select bind:value={draft.video.convert}>
                  <option value="ask">Ask (convert or not)</option>
                  <option value="always">Always convert to WebM</option>
                  <option value="never">Never convert</option>
                </select>
              </label>
              <label class="row">
                <span>Convert videos with</span>
                <select bind:value={draft.video.encoder} disabled={encoders === null}>
                  <option value="auto">
                    {encoders === null ? "Looking for GPUs…" : encoders.length ? `Automatic (${encoders[0].label})` : "Automatic (no GPU found: the CPU)"}
                  </option>
                  {#each encoders ?? [] as e (e.id)}<option value={e.id}>{e.label}</option>{/each}
                  <option value="cpu">CPU only · VP9 (slower, any computer)</option>
                  <!-- A GPU chosen earlier that isn't found now: kept, and "auto" is used meanwhile. -->
                  {#if encoders && !["auto", "cpu"].includes(draft.video.encoder) && !encoders.some((e) => e.id === draft!.video.encoder)}
                    <option value={draft.video.encoder}>A GPU not found now (Automatic meanwhile)</option>
                  {/if}
                </select>
              </label>
              <p class="muted small">
                WebM videos play everywhere. Others (MP4, MOV…) may need extra codecs on Linux, so
                Comilito can convert them with its ffmpeg: on a GPU when one can (AV1, many times
                faster), else on the CPU (VP9). If a GPU fails, the CPU takes over. Defaults for new
                projects' video playlists are under Levels.
              </p>
              {#if oldFfmpeg}
                <p class="muted small warn-note">
                  Comilito's ffmpeg is an older build that may not use your GPU. Update it in Projects &amp; tools › Download tools.
                </p>
              {/if}
            </section>
          {/if}
        {:else if page === "projects"}
          <section>
            <div class="section-head"><span class="caps">Projects folder</span></div>
            <div class="row">
              <code class="grow">{draft.projectsFolder ?? "Default (Documents/Comilito)"}</code>
              <button onclick={moveToFolder} title="Move every project to another folder">Move to…</button>
              <button onclick={chooseFolder} title="Look for projects in another folder, without moving any">Choose…</button>
              {#if draft.projectsFolder}<button onclick={() => (draft!.projectsFolder = null)}>Use default</button>{/if}
            </div>
            <p class="muted small">
              <strong>Move to…</strong> moves your projects there. <strong>Choose…</strong>
              only changes where Comilito looks (for example a folder you synced from another computer).
              Backups are kept separately, in Comilito's data folder.
            </p>
            <label class="switch">
              <input type="checkbox" bind:checked={draft.openLastProject} />
              <span>Open the last project at start</span>
            </label>
          </section>
          <section>
            <div class="section-head"><span class="caps">Download tools</span><span class="hint">yt-dlp and ffmpeg, for downloads and cropping</span></div>
            {#if ui.tools?.ytDlp || ui.tools?.ffmpeg}
              <div class="row">
                <span class="grow">
                  yt-dlp {ui.tools.ytDlp ?? "not installed"} · ffmpeg {ui.tools.ffmpeg ? (ui.tools.ffmpegVersion ?? "(older build: Update for GPU video conversion)") : "not installed"} · {formatBytes(ui.tools.size)} on disk
                </span>
                <button onclick={updateTools} disabled={updatingTools}>
                  {#if updatingTools}<LoaderCircle size={13} class="spin" />Updating…{:else}Update{/if}
                </button>
                <button class="danger" onclick={removeTools} disabled={updatingTools}>Remove</button>
              </div>
              {#if updatingTools}
                {@const p = ui.toolsProgress}
                <p class="muted small tools-progress">
                  {#if p?.total}Downloading {p.step} · {formatBytes(p.done)} of {formatBytes(p.total)}{:else if p}{p.step === "yt-dlp" ? "Updating yt-dlp" : p.step}…{:else}Checking for newer versions…{/if}
                </p>
                {#if p?.total}<div class="tools-track"><div style:width="{(p.done / p.total) * 100}%"></div></div>{/if}
              {/if}
            {:else}
              <ToolsInstall />
            {/if}
          </section>
          {#if ui.appMenu && ui.appMenu.state !== "unavailable"}
            {@const m = ui.appMenu}
            <section>
              <div class="section-head"><span class="caps">App menu</span><span class="hint">Start the AppImage from your desktop's menu</span></div>
              <div class="row">
                <span class="grow">
                  {#if m.state === "current"}In the app menu.
                  {:else if m.state === "missing"}Not in the app menu.
                  {:else if m.state === "elsewhere"}In the app menu, added by another tool (<code>{m.otherEntry}</code>).
                  {:else if m.state === "moved"}In the app menu, but its launcher starts a file that isn't there any more.
                  {:else if m.state === "otherCopy"}In the app menu, starting another copy (<code>{m.pointsAt}</code>).
                  {:else}In the app menu; this version brings a different launcher or icon.{/if}
                </span>
                {#if m.state === "missing"}
                  <button onclick={() => (ui.appMenuOffer = "preferences")}>Add…</button>
                {:else if m.state !== "current" && m.state !== "elsewhere"}
                  <button onclick={() => (ui.appMenuOffer = "preferences")}>Update…</button>
                {/if}
                {#if m.state !== "missing" && m.state !== "elsewhere"}
                  <button class="danger" onclick={removeFromAppMenu}>Remove…</button>
                {/if}
              </div>
              <p class="muted small">This copy: <code>{m.appimage}</code></p>
              <label class="switch">
                <input type="checkbox" bind:checked={draft.behaviour.askAppMenu} />
                <span>At start, offer to add Comilito to the app menu, or to update it</span>
              </label>
            </section>
          {/if}
          <section>
            <div class="section-head"><span class="caps">Troubleshooting</span><span class="hint">Logs, and a fresh start for the interface</span></div>
            <div class="row">
              <button title="Errors and background events (downloads, backups, tools)" onclick={() => openLogs("events")}>See logs</button>
              <button title="What Comilito and its libraries printed, crashes included (Linux, when not started from a terminal)" onclick={() => openLogs("errors")}>See error log</button>
              <button title="The folder with all the log files" onclick={() => openLogs("folder")}><FolderOpen size={13} />Open logs folder</button>
            </div>
            <div class="row">
              <!-- A restart would drop this window's unsaved changes, so it waits for Save. -->
              <button
                disabled={dirty}
                title={dirty
                  ? "Save or cancel your changes here first"
                  : "Reloads the windows, as if Comilito had just started. The project stays open and the sound keeps playing."}
                onclick={restartInterface}><RotateCcw size={13} />Restart the interface</button>
              <span class="muted small">{draft.shortcuts.restartInterface ? `Shortcut: ${draft.shortcuts.restartInterface}` : ""}</span>
            </div>
          </section>
          <section>
            <div class="section-head"><span class="caps">Remove Comilito's data</span></div>
            <p class="muted">
              Preferences, themes, downloaded tools, logs and caches are kept outside your projects.
              Remove them before uninstalling, or to start over. Projects are never touched.
            </p>
            <div class="row"><button class="danger" onclick={forgetEverything}>Remove all data and quit…</button></div>
          </section>
        {:else if page === "behaviour"}
          <section class="form">
            <label class="row">
              <span>A scene that's already in the tapestry</span>
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
              <span>Leaving an editor with unsaved changes</span>
              <select bind:value={draft.behaviour.unsavedChanges}>
                <option value="ask">Ask</option>
                <option value="save">Save the changes</option>
                <option value="discard">Discard the changes</option>
              </select>
            </label>
            <label class="row">
              <span>Loading a preset over an unsaved tapestry</span>
              <select bind:value={draft.behaviour.replaceUnsavedTapestry}>
                <option value="ask">Ask</option>
                <option value="replace">Replace without saving</option>
              </select>
            </label>
            <label class="switch">
              <input type="checkbox" bind:checked={draft.behaviour.warnOnQuit} />
              <span>Warn when quitting during downloads, fingerprinting or a backup</span>
            </label>
            <label class="switch">
              <input type="checkbox" bind:checked={draft.behaviour.checkForUpdates} />
              <span>Check for a new version at start (asks GitHub; nothing is downloaded)</span>
            </label>
          </section>
          <section class="form">
            <div class="section-head"><span class="caps">Shortcuts</span><span class="hint">Click, then press a key</span></div>
            <div class="row">
              <span>Remove the top scene from the tapestry</span>
              <KeyInput value={draft.shortcuts.removeTopScene} onchange={(k) => (draft!.shortcuts.removeTopScene = k)} />
            </div>
            <div class="row">
              <span>Restart the interface (if something gets stuck)</span>
              <KeyInput value={draft.shortcuts.restartInterface} onchange={(k) => (draft!.shortcuts.restartInterface = k)} />
            </div>
            <p class="muted small">Space (pause), Esc (stop all) and / (search) are fixed. A scene hotkey on the same key is ignored.</p>
          </section>
        {:else if page === "appearance"}
          <AppearancePage bind:appearance={draft.appearance} />
        {/if}
      </div>
    </div>
  {/if}

  {#snippet footer()}
    <button class="ghost" onclick={onclose}>Cancel</button>
    <button class="primary" onclick={save} disabled={!dirty}>Save</button>
  {/snippet}
</Modal>

<style>
  .layout {
    display: flex;
    height: 100%;
    min-height: 0;
  }
  nav {
    width: 13rem;
    flex: none;
    background: var(--side-bg);
    padding: 1rem 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  nav button {
    border: none;
    justify-content: flex-start;
    gap: 0.6rem;
    padding: 0.55rem 0.7rem;
    text-transform: none;
    letter-spacing: 0;
    font-size: 0.875rem;
    font-weight: 450;
    color: var(--side-text);
  }
  nav button.active {
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }
  .content {
    flex: 1;
    overflow: auto;
    padding: 1.4rem 1.8rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.3rem;
  }
  h2 {
    font-size: 2.2rem;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }
  p {
    margin: 0;
  }
  .volumes {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
  }
  .form {
    gap: 0.9rem;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.7rem;
  }
  .row > span:first-child {
    width: 18rem;
  }
  .row input[type="number"] {
    width: 5.5rem;
  }
  .row select {
    width: auto;
  }
  .grow {
    flex: 1;
  }
  code {
    font-size: 0.82rem;
    user-select: text;
  }
  .tools-track {
    height: 5px;
    background: var(--track);
    border-radius: 3px;
    overflow: hidden;
  }
  .tools-track div {
    height: 100%;
    background: var(--accent);
  }
  .tools-progress {
    margin: 0;
  }
  .warn-note {
    color: var(--warn);
  }
  .small {
    font-size: 0.78rem;
    margin: 0.4rem 0 0.6rem;
  }
</style>

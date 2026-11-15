<script lang="ts">
  // Dropping audio files (or videos, decision 0035) from the file manager onto the window
  // imports copies of them, after
  // asking where they should go. Dropped on a Library folder or category (anything marked with
  // `data-drop-category` / `data-drop-folder`), that's where they go; anywhere else, to the
  // folder the Library is showing. The native drop handler is off (decision 0014), so
  // Where the files come from depends on the platform (decision 0022):
  //   - Linux: WebKitGTK hides dropped files from the page, so Tauri's native drop event is on
  //     (tauri.linux.conf.json) and gives their paths; `add_sounds` copies them, like "Add files…".
  //   - Windows and macOS: the native handler is off (decision 0014) and the browser's drop
  //     event gives File objects with the contents but no path; each is sent to Rust as raw
  //     bytes (`api.importDroppedFile`).
  import { FileAudio, FileVideo, Upload } from "@lucide/svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { askText, formatBytes, notify, project, run, ui } from "../lib/store.svelte";
  import { CATEGORIES, type Shelf } from "../lib/types";
  import { isVideoName, VIDEO_EXTENSIONS, videoEnabled } from "../lib/video";
  import Modal from "./Modal.svelte";

  // Keep in step with AUDIO_EXTENSIONS in storage.rs.
  const AUDIO = ["mp3", "ogg", "oga", "flac", "wav", "m4a", "aac"];
  const isAudio = (name: string) => AUDIO.includes(name.split(".").pop()?.toLowerCase() ?? "");
  const isVideo = (name: string) => videoEnabled() && isVideoName(name);

  /** A dropped file: either its path (Linux) or its contents. */
  type Dropped = { name: string; size: number | null; path?: string; file?: File };

  const fromPath = (path: string): Dropped => ({ name: path.split(/[\\/]/).pop() ?? path, size: null, path });

  type Target = { category: Shelf; folder: string };

  /** The Library folder or category under a point of the window (CSS pixels), if any. */
  function targetAt(x: number, y: number): Target | null {
    const el = document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-drop-category]");
    if (!el) return null;
    return { category: el.dataset.dropCategory as Shelf, folder: el.dataset.dropFolder ?? "" };
  }

  /** Lights up the folder under the pointer while files are dragged (see `ui.fileDropTarget`). */
  function point(x: number, y: number) {
    const t = targetAt(x, y);
    target = t;
    ui.fileDropTarget = t ? `${t.category}:${t.folder}` : null;
  }

  /** The page point of a native (Linux) drag event. Tauri calls it a `PhysicalPosition`, but on
   *  Linux wry passes on GTK's widget coordinates unchanged, and those are logical pixels: the
   *  same as the page's CSS pixels at any display scale (the app never zooms the webview).
   *  Dividing by the scale put the point too high on scaled displays. */
  const css = (p: { x: number; y: number }) => [p.x, p.y] as const;

  const SHELVES = $derived<{ id: Shelf; label: string }[]>([...CATEGORIES, ...(videoEnabled() ? [{ id: "video" as const, label: "Video" }] : [])]);
  /** True while files from outside are dragged over the window (shows the overlay). */
  let hovering = $state(false);
  /** Where the files would go if dropped now (null: the folder the Library is showing). */
  let target = $state<Target | null>(null);
  const targetLabel = $derived(
    target ? [SHELVES.find((c) => c.id === target!.category)?.label, ...target.folder.split("/").filter(Boolean)].join(" › ") : "",
  );

  // Hovering ends (dropped, left the window): nothing is lit up any more.
  $effect(() => {
    if (!hovering) {
      ui.fileDropTarget = null;
      target = null;
    }
  });
  let files = $state<Dropped[]>([]);
  let skipped = $state<string[]>([]);
  let category = $state<Shelf>("music");
  let folder = $state("");
  let importing = $state<number | null>(null);
  const folders = $derived(category === "video" ? (ui.snapshot?.videoFolders ?? []) : (ui.snapshot?.folders[category] ?? []));
  // Audio and videos dropped together: the audio goes where chosen, the videos to the Video
  // shelf (its top level, unless Video is the shelf chosen).
  const videos = $derived(files.filter((f) => isVideo(f.name)));
  const audio = $derived(files.filter((f) => !isVideo(f.name)));
  const mixed = $derived(videos.length > 0 && audio.length > 0);

  /** Only drags of files from outside the app count; our own drags (sounds, scenes) set `ui.drag`.
   *  WebKitGTK lists dropped files as "text/uri-list" rather than "Files". */
  function fromOutside(e: DragEvent) {
    const types = Array.from(e.dataTransfer?.types ?? []);
    return !ui.drag && (types.includes("Files") || types.includes("text/uri-list"));
  }

  onMount(() => {
    // Every drag of files is stopped, even when it can't be imported here: the webview's
    // default for a dropped file is to open it, which would replace the whole app with the
    // file (and play it in the webview).
    // Set once Tauri's native drag events arrive (only on Linux, see the top of this file).
    // WebKitGTK still sends the page its own drag events too, but with wrong pointer
    // positions; acting on both made the outline flicker and the hover end too early. From
    // then on the page's events only stop the webview from opening the file.
    let nativeEvents = false;

    function over(e: DragEvent) {
      if (!fromOutside(e)) return;
      e.preventDefault();
      if (nativeEvents) return;
      const accept = !!project() && !files.length;
      e.dataTransfer!.dropEffect = accept ? "copy" : "none";
      hovering = accept;
      if (accept) point(e.clientX, e.clientY);
    }
    function leave(e: DragEvent) {
      if (nativeEvents) return;
      // WebKitGTK sends dragleave without relatedTarget, so leaving the window is detected
      // by the pointer being outside it.
      if (e.clientX <= 0 || e.clientY <= 0 || e.clientX >= window.innerWidth || e.clientY >= window.innerHeight) hovering = false;
    }
    function drop(e: DragEvent) {
      if (!fromOutside(e)) return;
      e.preventDefault();
      if (nativeEvents) return;
      hovering = false;
      if (!project() || files.length) return;
      receive(
        Array.from(e.dataTransfer!.files).map((file) => ({ name: file.name, size: file.size, file })),
        targetAt(e.clientX, e.clientY),
      );
    }
    // Linux: the native drop event (see the top of this file).
    const native = getCurrentWebview().onDragDropEvent(({ payload }) => {
      nativeEvents = true;
      if (payload.type === "leave") hovering = false;
      else if (payload.type === "enter") hovering = !!project() && !files.length && payload.paths.length > 0;
      else if (payload.type === "over") {
        if (hovering) point(...css(payload.position));
      } else if (payload.type === "drop") {
        hovering = false;
        if (project() && !files.length) receive(payload.paths.map(fromPath), targetAt(...css(payload.position)));
      }
    });
    function receive(all: Dropped[], at: Target | null) {
      if (!all.length) return;
      const usable = (name: string) => isAudio(name) || isVideo(name);
      files = all.filter((f) => usable(f.name));
      skipped = all.filter((f) => !usable(f.name)).map((f) => f.name);
      if (!files.length) {
        const kinds = videoEnabled() ? [...AUDIO, ...VIDEO_EXTENSIONS] : AUDIO;
        notify(`Comilito can't use ${skipped.length === 1 ? `“${skipped[0]}”` : "those files"}. It takes ${kinds.join(", ")}.`);
        skipped = [];
        return;
      }
      // Where they were dropped, or else where the library is pointing, if anywhere.
      if (at) {
        category = at.category;
        folder = at.folder;
      } else {
        category = ui.library.category ?? "music";
        folder = ui.library.category && ui.library.folder ? ui.library.folder : "";
      }
      // Only videos: they can only go to the Video shelf. Only audio: never to it.
      const onlyVideos = files.every((f) => isVideo(f.name));
      if (onlyVideos && category !== "video") [category, folder] = ["video", ""];
      if (!onlyVideos && category === "video") [category, folder] = ["music", ""];
    }
    window.addEventListener("dragover", over);
    window.addEventListener("dragleave", leave);
    window.addEventListener("drop", drop);
    return () => {
      native.then((unlisten) => unlisten());
      window.removeEventListener("dragover", over);
      window.removeEventListener("dragleave", leave);
      window.removeEventListener("drop", drop);
    };
  });

  async function newFolder() {
    const created = await askText("New folder", "Name");
    if (!created) return;
    if (await run(api.createFolder(category, created).then(() => true))) folder = created;
  }

  function close() {
    if (importing !== null) return;
    files = [];
    skipped = [];
  }

  async function confirm() {
    const added: string[] = [];
    const videoFolder = category === "video" ? folder : "";
    // Files with a path are copied by Rust in one go per kind (problems arrive as notices).
    const audioPaths = audio.flatMap((f) => (f.path ? [f.path] : []));
    const videoPaths = videos.flatMap((f) => (f.path ? [f.path] : []));
    importing = 0;
    if (audioPaths.length && category !== "video") added.push(...((await run(api.addSounds(category, audioPaths, folder))) ?? []));
    if (videoPaths.length) added.push(...((await run(api.addVideos(videoPaths, videoFolder))) ?? []));
    // The others are sent one by one, as their contents.
    const contents = files.filter((f) => f.file);
    const done = audioPaths.length + videoPaths.length;
    for (let i = 0; i < contents.length; i++) {
      importing = done + i;
      const file = contents[i].file!;
      const [shelf, into] = isVideo(file.name) ? (["video", videoFolder] as const) : ([category, folder] as const);
      const id = await run(file.arrayBuffer().then((bytes) => api.importDroppedFile(shelf, into, file.name, bytes)));
      if (id) added.push(id);
    }
    importing = null;
    if (added.length) {
      notify(added.length === 1 ? `Imported “${files[0].name}”.` : `Imported ${added.length} files.`);
      // Show them in the library (the videos, if only videos were dropped).
      const showVideos = videos.length > 0 && audio.length === 0;
      ui.library = {
        ...ui.library,
        smart: "all",
        category: showVideos ? "video" : category,
        folder: showVideos ? videoFolder : folder,
        search: "",
        selected: added,
        cropping: false,
      };
      ui.view = "library";
    }
    close();
  }

  const total = $derived(files.every((f) => f.size !== null) ? files.reduce((sum, f) => sum + (f.size ?? 0), 0) : null);
</script>

{#if hovering}
  <!-- Over a Library folder, the overlay steps aside so the lit-up folder stays visible. -->
  <div class="overlay" class:aside={target}>
    <div class="target">
      <Upload size={target ? 18 : 34} />
      {#if target}
        <span>Drop into <strong>{targetLabel}</strong></span>
      {:else}
        <span class="display">Drop to import</span>
        <span class="muted">Copies of the files are added to the project · drop on a Library folder to put them there</span>
      {/if}
    </div>
  </div>
{/if}

{#if files.length}
  <Modal title={files.length === 1 ? (videos.length ? "Import a video" : "Import a sound") : `Import ${files.length} files`} onclose={close}>
    <div class="body">
      <p class="muted">Copies go into the project folder; the original files stay where they are.</p>
      <ul class="list">
        {#each files as f, i (i)}
          <li class:done={importing !== null && i < importing} class:now={importing === i}>
            {#if isVideo(f.name)}<FileVideo size={14} />{:else}<FileAudio size={14} />{/if}<span class="name">{f.name}</span>{#if f.size !== null}<span class="muted">{formatBytes(f.size)}</span>{/if}
          </li>
        {/each}
      </ul>
      {#if skipped.length}
        <p class="muted small">Skipped (not audio or video Comilito can use): {skipped.join(", ")}</p>
      {/if}
      {#if mixed}
        <p class="muted small">The videos go to the Video shelf; the sounds to the category below.</p>
      {/if}
      <div class="save-to">
        <label class="field">
          <span class="caps">Category</span>
          <select bind:value={category} onchange={() => (folder = "")} disabled={importing !== null}>
            <!-- Only videos can go to Video; sounds only to the audio categories. -->
            {#each SHELVES.filter((c) => (audio.length ? c.id !== "video" : c.id === "video")) as c (c.id)}<option value={c.id}>{c.label}</option>{/each}
          </select>
        </label>
        <label class="field grow">
          <span class="caps">Folder</span>
          <select
            value={folder}
            disabled={importing !== null}
            onchange={(e) => {
              if (e.currentTarget.value === "__new__") {
                e.currentTarget.value = folder;
                newFolder();
              } else folder = e.currentTarget.value;
            }}
          >
            <option value="">(category folder)</option>
            {#each folders as f (f)}<option value={f}>{f}</option>{/each}
            <option value="__new__">+ New folder…</option>
          </select>
        </label>
      </div>
    </div>
    {#snippet footer()}
      {#if total !== null}<span class="muted small">{formatBytes(total)}</span>{/if}
      <span class="grow"></span>
      <button class="ghost" onclick={close} disabled={importing !== null}>Cancel</button>
      <button class="primary" onclick={confirm} disabled={importing !== null}>
        <Upload size={14} />{importing !== null ? `Importing ${importing + 1} of ${files.length}…` : "Import"}
      </button>
    {/snippet}
  </Modal>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: rgb(0 0 0 / 0.45);
    display: grid;
    place-items: center;
    pointer-events: none;
  }
  .target {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
    padding: 2.2rem 3.4rem;
    border: 2px dashed var(--accent);
    border-radius: 10px;
    background: var(--surface);
    color: var(--accent);
  }
  .overlay.aside {
    background: transparent;
    place-items: end center;
    padding-bottom: 2rem;
  }
  .overlay.aside .target {
    flex-direction: row;
    padding: 0.6rem 1.2rem;
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.3);
  }
  .target .display {
    font-size: 2rem;
    color: var(--text);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  p {
    margin: 0;
  }
  .small {
    font-size: 0.78rem;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0.5rem 0.7rem;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    max-height: 14rem;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }
  li.done {
    opacity: 0.5;
  }
  li.now {
    color: var(--accent);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .save-to {
    display: flex;
    gap: 0.7rem;
  }
  .grow {
    flex: 1;
  }
</style>

<script lang="ts">
  // Dropping audio files from the file manager onto the window imports copies of them, after
  // asking where they should go (TODO.md). The native drop handler is off (decision 0014), so
  // Where the files come from depends on the platform (decision 0022):
  //   - Linux: WebKitGTK hides dropped files from the page, so Tauri's native drop event is on
  //     (tauri.linux.conf.json) and gives their paths; `add_sounds` copies them, like "Add files…".
  //   - Windows and macOS: the native handler is off (decision 0014) and the browser's drop
  //     event gives File objects with the contents but no path; each is sent to Rust as raw
  //     bytes (`api.importDroppedFile`).
  import { FileAudio, Upload } from "@lucide/svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { askText, formatBytes, notify, project, run, ui } from "../lib/store.svelte";
  import { CATEGORIES, type Category } from "../lib/types";
  import Modal from "./Modal.svelte";

  // Keep in step with AUDIO_EXTENSIONS in storage.rs.
  const AUDIO = ["mp3", "ogg", "oga", "flac", "wav", "m4a", "aac"];
  const isAudio = (name: string) => AUDIO.includes(name.split(".").pop()?.toLowerCase() ?? "");

  /** A dropped file: either its path (Linux) or its contents. */
  type Dropped = { name: string; size: number | null; path?: string; file?: File };

  const fromPath = (path: string): Dropped => ({ name: path.split(/[\\/]/).pop() ?? path, size: null, path });

  /** True while files from outside are dragged over the window (shows the overlay). */
  let hovering = $state(false);
  let files = $state<Dropped[]>([]);
  let skipped = $state<string[]>([]);
  let category = $state<Category>("music");
  let folder = $state("");
  let importing = $state<number | null>(null);
  const folders = $derived(ui.snapshot?.folders[category] ?? []);

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
    function over(e: DragEvent) {
      if (!fromOutside(e)) return;
      e.preventDefault();
      const accept = !!project() && !files.length;
      e.dataTransfer!.dropEffect = accept ? "copy" : "none";
      hovering = accept;
    }
    function leave(e: DragEvent) {
      // WebKitGTK sends dragleave without relatedTarget, so leaving the window is detected
      // by the pointer being outside it.
      if (e.clientX <= 0 || e.clientY <= 0 || e.clientX >= window.innerWidth || e.clientY >= window.innerHeight) hovering = false;
    }
    function drop(e: DragEvent) {
      hovering = false;
      if (!fromOutside(e)) return;
      e.preventDefault();
      if (!project() || files.length) return;
      receive(Array.from(e.dataTransfer!.files).map((file) => ({ name: file.name, size: file.size, file })));
    }
    // Linux: the native drop event (see the top of this file).
    const native = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "leave") hovering = false;
      else if (payload.type === "enter") hovering = !!project() && !files.length && payload.paths.length > 0;
      else if (payload.type === "drop") {
        hovering = false;
        if (project() && !files.length) receive(payload.paths.map(fromPath));
      }
    });
    function receive(all: Dropped[]) {
      if (!all.length) return;
      files = all.filter((f) => isAudio(f.name));
      skipped = all.filter((f) => !isAudio(f.name)).map((f) => f.name);
      if (!files.length) {
        notify(`Comilito can't play ${skipped.length === 1 ? `“${skipped[0]}”` : "those files"}. It plays ${AUDIO.join(", ")}.`);
        skipped = [];
        return;
      }
      // Start where the library is pointing, if anywhere.
      category = ui.library.category ?? "music";
      folder = ui.library.category && ui.library.folder ? ui.library.folder : "";
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
    // Files with a path are copied by Rust in one go (problems arrive as notices).
    const paths = files.flatMap((f) => (f.path ? [f.path] : []));
    if (paths.length) {
      importing = 0;
      added.push(...((await run(api.addSounds(category, paths, folder))) ?? []));
    }
    // The others are sent one by one, as their contents.
    const contents = files.filter((f) => f.file);
    for (let i = 0; i < contents.length; i++) {
      importing = paths.length + i;
      const file = contents[i].file!;
      const id = await run(file.arrayBuffer().then((bytes) => api.importDroppedFile(category, folder, file.name, bytes)));
      if (id) added.push(id);
    }
    importing = null;
    if (added.length) {
      notify(added.length === 1 ? `Imported “${files[0].name}”.` : `Imported ${added.length} sounds.`);
      // Show them in the library.
      ui.library = { ...ui.library, smart: "all", category, folder, search: "", selected: added, cropping: false };
      ui.view = "library";
    }
    close();
  }

  const total = $derived(files.every((f) => f.size !== null) ? files.reduce((sum, f) => sum + (f.size ?? 0), 0) : null);
</script>

{#if hovering}
  <div class="overlay">
    <div class="target">
      <Upload size={34} />
      <span class="display">Drop to import</span>
      <span class="muted">Copies of the files are added to the project</span>
    </div>
  </div>
{/if}

{#if files.length}
  <Modal title={files.length === 1 ? "Import a sound" : `Import ${files.length} sounds`} onclose={close}>
    <div class="body">
      <p class="muted">Copies go into the project folder; the original files stay where they are.</p>
      <ul class="list">
        {#each files as f, i (i)}
          <li class:done={importing !== null && i < importing} class:now={importing === i}>
            <FileAudio size={14} /><span class="name">{f.name}</span>{#if f.size !== null}<span class="muted">{formatBytes(f.size)}</span>{/if}
          </li>
        {/each}
      </ul>
      {#if skipped.length}
        <p class="muted small">Skipped (not audio Comilito can play): {skipped.join(", ")}</p>
      {/if}
      <div class="save-to">
        <label class="field">
          <span class="caps">Category</span>
          <select bind:value={category} onchange={() => (folder = "")} disabled={importing !== null}>
            {#each CATEGORIES as c (c.id)}<option value={c.id}>{c.label}</option>{/each}
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

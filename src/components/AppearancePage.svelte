<script lang="ts">
  // Preferences › Appearance (decision 0016): theme, light/dark pairing, accent colour and
  // illumination. Edits `appearance` (part of the Preferences draft) and shows it live.
  import { CircleCheck, Copy, CopyPlus, Download, Pencil, Trash2, Upload } from "@lucide/svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { api } from "../lib/api";
  import { confirmAction, refreshUserThemes, run, ui } from "../lib/store.svelte";
  import { BUILT_IN_THEMES, activeTheme, findTheme } from "../lib/themes";
  import type { Appearance, Theme } from "../lib/types";
  import ThemeEditor from "./ThemeEditor.svelte";

  let { appearance = $bindable() }: { appearance: Appearance } = $props();

  const themes = $derived([...BUILT_IN_THEMES, ...ui.userThemes]);
  const current = $derived(activeTheme(appearance, ui.desktop, ui.userThemes));
  let editing = $state<Theme | null>(null);

  /** Picking a theme: the fixed theme, and the light or dark one when following the desktop. */
  function pick(theme: Theme) {
    appearance.theme = theme.id;
    if (theme.dark) appearance.darkTheme = theme.id;
    else appearance.lightTheme = theme.id;
  }

  function duplicate(theme: Theme) {
    editing = {
      ...structuredClone($state.snapshot(theme)),
      id: `user-${crypto.randomUUID().replaceAll("-", "")}`,
      name: `${theme.name} (my copy)`,
    };
  }

  function edit(theme: Theme) {
    if (theme.id.startsWith("user-")) editing = structuredClone($state.snapshot(theme));
    else duplicate(theme);
  }

  async function remove(theme: Theme) {
    if (!(await confirmAction(`Delete “${theme.name}”?`, "Projects aren't affected; the app falls back to Codex if it was in use.", "Delete", true))) return;
    await run(api.deleteUserTheme(theme.id));
    await refreshUserThemes();
    for (const key of ["theme", "lightTheme", "darkTheme"] as const) {
      if (appearance[key] === theme.id) appearance[key] = key === "darkTheme" ? "ironwood" : "codex";
    }
  }

  async function exportTheme(theme: Theme) {
    const dest = await save({ title: "Export theme", defaultPath: `${theme.name}.comilito-theme.json`, filters: [{ name: "Comilito theme", extensions: ["json"] }] });
    if (dest) await run(api.exportTheme($state.snapshot(theme), dest));
  }

  async function importTheme() {
    const path = await open({ title: "Import a theme", filters: [{ name: "Comilito theme", extensions: ["json"] }] });
    if (typeof path !== "string") return;
    const theme = await run(api.importTheme(path));
    if (theme) {
      await refreshUserThemes();
      pick(theme);
    }
  }

  function saved(theme: Theme) {
    editing = null;
    pick(theme);
  }
</script>

{#if editing}
  <ThemeEditor theme={editing} onsave={saved} oncancel={() => (editing = null)} />
{:else}
  <section>
    <div class="section-head"><span class="caps">Theme</span><span class="hint">Built-in themes can be duplicated and edited; your own can be shared as files</span></div>
    <div class="cards">
      {#each themes as t (t.id)}
        {@const colors = { ...findTheme(t.dark ? "ironwood" : "codex", []).colors, ...t.colors }}
        <button class="card" class:selected={current.id === t.id} onclick={() => pick(t)}>
          <span class="preview" style:background={colors.bg}>
            <span class="p-head" style:background={colors["header-bg"]}></span>
            <span class="p-body">
              <span class="p-side" style:background={colors["side-bg"]}></span>
              <span class="p-main">
                {#each [0, 1, 2] as i (i)}
                  <span class="p-row" style:background={colors.surface}>
                    <i style:background={i === 0 ? colors.accent : colors.gold}></i>
                    <b style:background={colors.text}></b>
                  </span>
                {/each}
              </span>
            </span>
          </span>
          <span class="label">
            <span class="tname" style:font-family={`"${t.fonts.display}"`}>{t.name}</span>
            {#if t.id.startsWith("user-")}<span class="badge">Custom</span>{/if}
            {#if current.id === t.id}<CircleCheck size={16} />{/if}
          </span>
        </button>
      {/each}
      <button class="card new" onclick={() => duplicate(current)}>
        <CopyPlus size={22} />
        <span>Duplicate {current.name}</span>
        <small>to make your own</small>
      </button>
    </div>
    <div class="actions">
      <button onclick={() => edit(current)}><Pencil size={13} />{current.id.startsWith("user-") ? "Edit theme" : "Edit a copy"}</button>
      <button onclick={() => duplicate(current)}><Copy size={13} />Duplicate</button>
      <button onclick={() => exportTheme(current)}><Upload size={13} />Export…</button>
      <button onclick={importTheme}><Download size={13} />Import theme file…</button>
      {#if current.id.startsWith("user-")}<button class="danger" onclick={() => remove(current)}><Trash2 size={13} />Delete</button>{/if}
    </div>
  </section>

  <section>
    <div class="section-head"><span class="caps">Light and dark</span></div>
    <label class="radio">
      <input type="radio" bind:group={appearance.mode} value="system" />
      Follow the desktop: when light use
      <select bind:value={appearance.lightTheme}>
        {#each themes as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
      </select>
      and when dark use
      <select bind:value={appearance.darkTheme}>
        {#each themes as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
      </select>
    </label>
    <label class="radio"><input type="radio" bind:group={appearance.mode} value="fixed" />Always use the selected theme</label>
  </section>

  <section>
    <div class="section-head"><span class="caps">Accent colour</span><span class="hint">Used for what's live: the top scene, playing music, active effects</span></div>
    <div class="accents">
      <label class="accent" class:selected={appearance.accent === "theme"}>
        <input type="radio" bind:group={appearance.accent} value="theme" />
        <span class="swatch" style:background={current.colors.accent}></span>
        <span><strong>Theme colour</strong><small>{current.name}'s own accent</small></span>
      </label>
      <label class="accent" class:selected={appearance.accent === "desktop"}>
        <input type="radio" bind:group={appearance.accent} value="desktop" />
        <span class="swatch" style:background={ui.desktop?.accent ?? "transparent"} class:none={!ui.desktop?.accent}></span>
        <span>
          <strong>Desktop accent</strong>
          <small>{ui.desktop?.accent ? "Detected from your desktop · follows it live" : "Your desktop doesn't share one; the theme colour is used"}</small>
        </span>
      </label>
      <label class="accent" class:selected={appearance.accent === "custom"}>
        <input type="radio" bind:group={appearance.accent} value="custom" />
        <input type="color" bind:value={appearance.customAccent} onclick={() => (appearance.accent = "custom")} />
        <span><strong>Custom</strong><small>Pick any colour</small></span>
      </label>
    </div>
    <p class="muted small">If an accent would make text hard to read, Comilito darkens or lightens it just enough for the theme.</p>
  </section>

  <section>
    <div class="section-head"><span class="caps">Illumination</span><span class="hint">Decoration only; turn it down for a plainer look</span></div>
    <div class="toggles">
      <label class="switch"><input type="checkbox" bind:checked={appearance.capitals} />Decorated capitals</label>
      <label class="switch"><input type="checkbox" bind:checked={appearance.watermark} />Woodcut watermark</label>
      <label class="switch"><input type="checkbox" bind:checked={appearance.rules} />Ornamental rules</label>
      <label class="switch"><input type="checkbox" bind:checked={appearance.blackletter} />Blackletter headings</label>
    </div>
  </section>
{/if}

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(10.5rem, 1fr));
    gap: 0.8rem;
  }
  .card {
    flex-direction: column;
    align-items: stretch;
    gap: 0.5rem;
    padding: 0.55rem;
    border: 1px solid var(--rule);
    background: var(--surface);
    border-radius: 6px;
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
    color: var(--text);
  }
  .card.selected {
    border: 2px solid var(--accent);
  }
  .preview {
    height: 5.6rem;
    border-radius: 3px;
    border: 1px solid rgb(0 0 0 / 0.12);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .p-head {
    height: 0.9rem;
  }
  .p-body {
    flex: 1;
    display: flex;
  }
  .p-side {
    width: 2.2rem;
  }
  .p-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.4rem;
  }
  .p-row {
    height: 0.8rem;
    border-radius: 2px;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0 0.2rem;
  }
  .p-row i {
    width: 0.5rem;
    height: 0.5rem;
  }
  .p-row b {
    width: 45%;
    height: 0.15rem;
    opacity: 0.5;
  }
  .label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .label :global(svg) {
    color: var(--accent);
  }
  .tname {
    flex: 1;
    font-size: 1.15rem;
    text-align: left;
  }
  .badge {
    font-size: 0.55rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    border: 1px solid var(--rule);
    border-radius: 3px;
    padding: 0.05rem 0.3rem;
    color: var(--muted);
  }
  .card.new {
    justify-content: center;
    align-items: center;
    border-style: dashed;
    border-color: var(--gold);
    background: transparent;
    color: var(--muted);
    gap: 0.3rem;
    font-weight: 600;
  }
  .card.new small {
    font-weight: 400;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .radio {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  .radio select {
    width: auto;
  }
  .accents {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.7rem;
  }
  .accent {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.7rem;
    border: 1px solid var(--rule);
    border-radius: 6px;
    background: var(--surface);
    cursor: pointer;
  }
  .accent.selected {
    border: 2px solid var(--accent);
  }
  .accent > span:last-child {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
  }
  .swatch {
    width: 1.6rem;
    height: 1.6rem;
    border-radius: 50%;
    border: 1px solid rgb(0 0 0 / 0.15);
    flex: none;
  }
  .swatch.none {
    background: repeating-linear-gradient(45deg, var(--rule) 0 3px, transparent 3px 6px) !important;
  }
  small {
    font-size: 0.72rem;
    color: var(--muted);
  }
  .small {
    font-size: 0.8rem;
    margin: 0;
  }
  .toggles {
    display: flex;
    gap: 1.6rem;
    flex-wrap: wrap;
  }
</style>

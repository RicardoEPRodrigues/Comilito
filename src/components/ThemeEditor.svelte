<script lang="ts">
  // Editing a user theme (decisions 0016 and 0025): name, light or dark, fonts, every colour
  // token, and the files the theme brings with it (its woodcut watermark, font files and their
  // licences). The whole app previews the theme live while it's edited. Built-in themes are
  // never edited directly; the Appearance page makes a copy first.
  import { Check, FileText, ImageUp, Link, Plus, RotateCcw, Trash2, Type } from "@lucide/svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import horse from "../assets/skeletal-horse.svg";
  import { api } from "../lib/api";
  import { askText, refreshUserThemes, run, ui } from "../lib/store.svelte";
  import {
    ACCENT_FOLLOWERS,
    ACCENT_PAIRS,
    BODY_FONTS,
    BUILT_IN_THEMES,
    COLOR_TOKENS,
    DISPLAY_FONTS,
    contrast,
    loadFont,
    resolveColors,
    watermarkUrl,
  } from "../lib/themes";
  import type { Theme } from "../lib/types";

  let { theme, onsave, oncancel }: { theme: Theme; onsave: (t: Theme) => void; oncancel: () => void } = $props();

  // svelte-ignore state_referenced_locally
  let draft = $state<Theme>(structuredClone($state.snapshot(theme)));
  const base = $derived(BUILT_IN_THEMES.find((t) => t.id === (draft.dark ? "ironwood" : "codex"))!);

  // Preview the draft everywhere while editing.
  $effect(() => {
    ui.previewTheme = $state.snapshot(draft);
    return () => (ui.previewTheme = null);
  });

  // The colours as shown, with "accent" tokens turned into the accent in use (decision 0050).
  const shown = $derived(resolveColors(draft, ui.previewAppearance ?? ui.prefs!.appearance, ui.desktop));
  function colour(key: string) {
    return shown[key];
  }

  /** The token's own value: a colour, or "accent". */
  const valueOf = (key: string) => draft.colors[key] ?? base.colors[key];
  const follows = (key: string) => valueOf(key) === "accent";
  /** A text token whose background follows the accent: it follows too (worked out to read well). */
  const followsBackground = (key: string) => Object.entries(ACCENT_PAIRS).some(([bg, texts]) => texts.includes(key) && follows(bg));

  /** Turns "follow the accent" on or off; off, the token keeps the colour it showed. */
  function toggleFollow(key: string) {
    draft.colors[key] = follows(key) ? colour(key) : "accent";
  }

  // Fonts the theme brought with it (decision 0025) are offered next to the bundled ones,
  // so editing an imported theme doesn't lose them.
  const ownFonts = $derived(Object.keys(draft.fontFiles ?? {}));
  const displayFonts = $derived([...ownFonts, ...DISPLAY_FONTS.filter((f) => !ownFonts.includes(f))]);
  const bodyFonts = $derived([...ownFonts, ...BODY_FONTS.filter((f) => !ownFonts.includes(f))]);

  const readability = $derived(contrast(colour("text"), colour("bg")));

  // ---- Files the theme brings (decision 0025) ----
  // A picked file is stored by Rust straight away (under its hash) and the draft refers to it
  // by that name. Saving keeps it; files no saved theme uses are cleaned up on the next save.

  /** Asks for a file and stores it for `role`; null if cancelled or refused. */
  async function pickFile(role: "watermark" | "font" | "licence", title: string, extensions: string[]) {
    const path = await open({ title, filters: [{ name: title, extensions }] });
    if (typeof path !== "string") return null;
    return { path, name: await run(api.addThemeFile(path, role)) };
  }

  async function chooseWatermark() {
    const picked = await pickFile("watermark", "Woodcut image", ["png", "webp", "svg"]);
    if (picked?.name) draft.watermark = picked.name;
  }

  /** "NewRocker-Regular.woff2" → "New Rocker": a starting guess for the font's name. */
  function guessFamily(path: string): string {
    const file = path.split(/[\\/]/).pop() ?? "";
    return file
      .replace(/\.[^.]+$/, "")
      .replace(/[-_ ]?(Regular|Italic|Bold|Medium|Light|Black|SemiBold|Variable|VF)$/i, "")
      .replace(/([a-z])([A-Z])/g, "$1 $2")
      .replace(/[-_]+/g, " ")
      .trim();
  }

  async function addFont() {
    const picked = await pickFile("font", "Font file", ["woff2", "woff", "ttf", "otf"]);
    if (!picked?.name) return;
    const answer = await askText("Font name", "Shown in the font lists as", guessFamily(picked.path));
    // Only characters a theme accepts in a font name (Rust checks too).
    const family = answer?.replace(/[^\p{L}\p{N} _-]/gu, "").trim().slice(0, 60);
    if (!family) return;
    draft.fontFiles = { ...(draft.fontFiles ?? {}), [family]: picked.name };
    // Use it for headings straight away; that's what theme fonts are usually for.
    draft.fonts.display = family;
    void loadFont(family, picked.name);
  }

  function removeFont(family: string) {
    const { [family]: _, ...rest } = draft.fontFiles ?? {};
    draft.fontFiles = rest;
    if (draft.fonts.display === family) draft.fonts.display = base.fonts.display;
    if (draft.fonts.body === family) draft.fonts.body = base.fonts.body;
  }

  async function addLicence() {
    const picked = await pickFile("licence", "Licence text", ["txt"]);
    if (picked?.name && !draft.licences?.includes(picked.name)) draft.licences = [...(draft.licences ?? []), picked.name];
  }

  // The woodcut preview, shown the way the app shows it: its shape painted in the gold.
  let watermarkPreview = $state<string | null>(null);
  $effect(() => {
    const file = draft.watermark;
    watermarkPreview = null;
    if (file) watermarkUrl(file).then((url) => draft.watermark === file && (watermarkPreview = url)).catch(() => {});
  });

  async function save() {
    const t = $state.snapshot(draft);
    // Store every token, so the theme looks the same even if built-in defaults change later.
    t.colors = { ...base.colors, ...t.colors };
    if (await run(api.saveUserTheme(t).then(() => true))) {
      await refreshUserThemes();
      onsave(t);
    }
  }
</script>

<section>
  <div class="section-head"><span class="caps">Edit theme</span><span class="hint">The whole app previews your changes</span></div>
  <div class="row">
    <label class="field grow"><span class="caps">Name</span><input bind:value={draft.name} /></label>
    <label class="field">
      <span class="caps">Display font</span>
      <select bind:value={draft.fonts.display}>
        {#each displayFonts as f (f)}<option value={f}>{f}</option>{/each}
      </select>
    </label>
    <label class="field">
      <span class="caps">Body font</span>
      <select bind:value={draft.fonts.body}>
        {#each bodyFonts as f (f)}<option value={f}>{f === "system-ui" ? "System" : f}</option>{/each}
      </select>
    </label>
    <label class="switch dark"><input type="checkbox" bind:checked={draft.dark} />Dark theme</label>
  </div>
  <label class="switch" title="For themes whose colours were chosen around one accent and clash with others">
    <input type="checkbox" checked={!!draft.fixedAccent} onchange={(e) => (draft.fixedAccent = e.currentTarget.checked || undefined)} />
    Always use this theme's accent (ignore the desktop's or a custom one)
  </label>
  <p class="sample" style:font-family={`"${draft.fonts.display}"`}>The Tapestry</p>

  <div class="colours">
    {#each COLOR_TOKENS as token (token.key)}
      {@const linked = follows(token.key) || followsBackground(token.key)}
      <label class="colour" title={followsBackground(token.key) ? "Follows its background, which follows the accent" : undefined}>
        <input type="color" value={colour(token.key)} disabled={linked} oninput={(e) => (draft.colors[token.key] = e.currentTarget.value)} />
        <span>{token.label}</span>
        {#if ACCENT_FOLLOWERS.includes(token.key)}
          <button
            class="icon follow"
            class:on={follows(token.key)}
            title={follows(token.key) ? "Follows the accent · click to give it its own colour" : "Follow the accent (the theme's, the desktop's or your own)"}
            onclick={(e) => {
              e.preventDefault();
              toggleFollow(token.key);
            }}
          >
            <Link size={12} />
          </button>
        {/if}
        {#if draft.colors[token.key] && draft.colors[token.key] !== base.colors[token.key]}
          <button class="icon" title="Use the default" onclick={() => delete draft.colors[token.key]}><RotateCcw size={12} /></button>
        {/if}
      </label>
    {/each}
  </div>
  <div class="files">
    <div class="file-block">
      <span class="caps">Woodcut</span>
      <div class="woodcut-row">
        <div class="woodcut" style:background={colour("bg")}>
          {#if watermarkPreview}
            <div class="mask" style:--woodcut="url({watermarkPreview})" style:background={colour("gold")}></div>
          {:else}
            <img src={horse} alt="" />
          {/if}
        </div>
        <div class="file-actions">
          <button onclick={chooseWatermark}><ImageUp size={13} />{draft.watermark ? "Change image…" : "Choose image…"}</button>
          {#if draft.watermark}<button class="ghost" onclick={() => (draft.watermark = undefined)}><RotateCcw size={13} />Use the horse</button>{/if}
          <small>A one-colour PNG, WebP or SVG on a transparent background. Only its shape is used, painted in the theme's gold.</small>
        </div>
      </div>
    </div>

    <div class="file-block">
      <span class="caps">Font files</span>
      {#each ownFonts as family (family)}
        <div class="font-row">
          <Type size={14} />
          <span class="font-name" style:font-family={`"${family}"`}>{family}</span>
          <button class="icon" title="Remove this font" onclick={() => removeFont(family)}><Trash2 size={13} /></button>
        </div>
      {:else}
        <small>None: the theme uses fonts that come with Comilito.</small>
      {/each}
      <div class="file-actions inline">
        <button onclick={addFont}><Plus size={13} />Add font file…</button>
        <button onclick={addLicence}><FileText size={13} />Add licence…</button>
      </div>
      {#each draft.licences ?? [] as licence, i (licence)}
        <div class="font-row">
          <FileText size={14} /><span class="licence">Licence {i + 1}</span>
          <button class="icon" title="Remove this licence" onclick={() => (draft.licences = draft.licences!.filter((l) => l !== licence))}><Trash2 size={13} /></button>
        </div>
      {/each}
      <small>Fonts usually ask for their licence (e.g. OFL.txt) to be passed on with them; it's shared when the theme is exported.</small>
    </div>
  </div>

  {#if readability < 4.5}
    <p class="warn">Text on the page has low contrast ({readability.toFixed(1)}:1). Aim for at least 4.5:1 so it's easy to read.</p>
  {/if}

  <div class="actions">
    <button class="ghost" onclick={oncancel}>Cancel</button>
    <button class="primary" onclick={save} disabled={!draft.name.trim()}><Check size={14} />Save theme</button>
  </div>
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  .row {
    display: flex;
    gap: 0.8rem;
    align-items: flex-end;
  }
  .grow {
    flex: 1;
  }
  .dark {
    padding-bottom: 0.5rem;
  }
  .sample {
    margin: 0;
    font-size: 2rem;
  }
  .colours {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.4rem 1rem;
  }
  .colour {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    font-size: 0.82rem;
    cursor: pointer;
  }
  .colour span {
    flex: 1;
  }
  .colour input:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .follow {
    color: var(--muted);
    padding: 0.15rem;
  }
  .follow.on {
    color: var(--accent);
  }
  .files {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.2rem;
  }
  .file-block {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    min-width: 0;
  }
  .woodcut-row {
    display: flex;
    gap: 0.8rem;
  }
  .woodcut {
    flex: none;
    width: 7rem;
    height: 7rem;
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    display: grid;
    padding: 0.4rem;
  }
  .woodcut img,
  .woodcut .mask {
    width: 100%;
    height: 100%;
    object-fit: contain;
    opacity: 0.8;
  }
  .woodcut .mask {
    -webkit-mask: var(--woodcut) center / contain no-repeat;
    mask: var(--woodcut) center / contain no-repeat;
  }
  .file-actions {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.4rem;
  }
  .file-actions.inline {
    flex-direction: row;
    flex-wrap: wrap;
  }
  .font-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .font-name {
    flex: 1;
    font-size: 1.15rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .licence {
    flex: 1;
    font-size: 0.85rem;
  }
  small {
    color: var(--muted);
    font-size: 0.75rem;
  }
  .warn {
    color: var(--warn);
    font-size: 0.82rem;
    margin: 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
</style>

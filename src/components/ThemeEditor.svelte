<script lang="ts">
  // Editing a user theme (decision 0016): name, light or dark, fonts and every colour token.
  // The whole app previews the theme live while it's edited. Built-in themes are never
  // edited directly; the Appearance page makes a copy first.
  import { Check, RotateCcw } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { refreshUserThemes, run, ui } from "../lib/store.svelte";
  import { BODY_FONTS, BUILT_IN_THEMES, COLOR_TOKENS, DISPLAY_FONTS, contrast } from "../lib/themes";
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

  function colour(key: string) {
    return draft.colors[key] ?? base.colors[key];
  }

  // Fonts the theme brought with it (decision 0025) are offered next to the bundled ones,
  // so editing an imported theme doesn't lose them.
  const ownFonts = $derived(Object.keys(draft.fontFiles ?? {}));
  const displayFonts = $derived([...ownFonts, ...DISPLAY_FONTS.filter((f) => !ownFonts.includes(f))]);
  const bodyFonts = $derived([...ownFonts, ...BODY_FONTS.filter((f) => !ownFonts.includes(f))]);

  const readability =$derived(contrast(colour("text"), colour("bg")));

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
  <p class="sample" style:font-family={`"${draft.fonts.display}"`}>The Tapestry</p>

  <div class="colours">
    {#each COLOR_TOKENS as token (token.key)}
      <label class="colour">
        <input type="color" value={colour(token.key)} oninput={(e) => (draft.colors[token.key] = e.currentTarget.value)} />
        <span>{token.label}</span>
        {#if draft.colors[token.key] && draft.colors[token.key] !== base.colors[token.key]}
          <button class="icon" title="Use the default" onclick={() => delete draft.colors[token.key]}><RotateCcw size={12} /></button>
        {/if}
      </label>
    {/each}
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

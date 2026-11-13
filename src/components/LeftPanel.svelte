<script lang="ts">
  // Left panel: Scenes and Presets tabs (decision 0017). Presets have their own tab: they're
  // mostly for preparing a session, while scenes are used all the time.
  import { remember, ui } from "../lib/store.svelte";
  import PresetsTab from "./PresetsTab.svelte";
  import ScenesTab from "./ScenesTab.svelte";

  function select(tab: "scenes" | "presets") {
    ui.leftTab = tab;
    remember("leftTab", tab);
  }
</script>

<aside>
  <div class="tabs" role="tablist">
    <button role="tab" class:active={ui.leftTab === "scenes"} aria-selected={ui.leftTab === "scenes"} onclick={() => select("scenes")}>Scenes</button>
    <button role="tab" class:active={ui.leftTab === "presets"} aria-selected={ui.leftTab === "presets"} onclick={() => select("presets")}>Presets</button>
  </div>
  {#if ui.leftTab === "scenes"}
    <ScenesTab />
  {:else}
    <PresetsTab />
  {/if}
</aside>

<style>
  aside {
    background: var(--side-bg);
    color: var(--side-text);
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }
  .tabs {
    display: flex;
    gap: 1.4rem;
    margin: 1rem 1.1rem 0.4rem;
    border-bottom: 1px solid color-mix(in srgb, var(--side-muted) 60%, transparent);
  }
  .tabs button {
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    padding: 0 0 0.5rem;
    color: var(--side-muted);
  }
  .tabs button:hover {
    background: none !important;
    color: var(--side-text);
  }
  .tabs button.active {
    color: var(--side-text);
    border-bottom-color: var(--side-text);
  }
</style>

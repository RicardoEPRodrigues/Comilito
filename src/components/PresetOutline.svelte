<script lang="ts">
  // Right panel while editing a preset, or a scene opened from one: the preset's scenes as an
  // outline (like Unity's hierarchy), with the scene being edited highlighted.
  import { ChevronDown, Eye, EyeOff, Info, Layers, Scroll } from "@lucide/svelte";
  import { focusOn, presetById, project, sceneById, ui } from "../lib/store.svelte";

  const presetId = $derived(ui.focus?.kind === "preset" ? ui.focus.id : ui.focus?.kind === "scene" ? ui.focus.from : undefined);
  const preset = $derived(presetId ? presetById(presetId) : undefined);
  const editingScene = $derived(ui.focus?.kind === "scene" ? ui.focus.id : undefined);
  const presetsUsing = $derived(
    editingScene ? (project()?.stackPresets.filter((p) => p.stack.some((e) => e.scene === editingScene)) ?? []) : [],
  );
  const inTapestry = $derived(editingScene ? (project()?.stack.some((e) => e.scene === editingScene) ?? false) : false);
</script>

<aside>
  <h2 class="display">Preset</h2>
  {#if preset}
    <span class="caps muted">{preset.name} · {preset.stack.length} scenes</span>
    <div class="tree">
      <button class="node root" onclick={() => focusOn({ kind: "preset", id: preset.id })}>
        <ChevronDown size={14} /><Layers size={14} /><span>{preset.name}</span>
      </button>
      {#each [...preset.stack].reverse() as entry, i (i)}
        {@const scene = sceneById(entry.scene)}
        <button
          class="node"
          class:current={entry.scene === editingScene}
          class:off={!entry.enabled}
          onclick={() => focusOn({ kind: "scene", id: entry.scene, from: preset.id })}
        >
          <Scroll size={14} /><span>{scene?.name ?? "?"}</span>
          {#if entry.enabled}<Eye size={13} />{:else}<EyeOff size={13} />{/if}
        </button>
      {/each}
    </div>
  {/if}
  {#if editingScene}
    <div class="note">
      <span class="caps"><Info size={13} />Shared scene</span>
      <p>
        Saving changes this scene everywhere it's used{inTapestry ? ": the tapestry" : ""}{presetsUsing.length
          ? `${inTapestry ? " and" : ":"} ${presetsUsing.map((p) => p.name).join(", ")}`
          : ""}. Changing the order or toggles of a preset only affects that preset.
      </p>
    </div>
  {/if}
</aside>

<style>
  aside {
    background: var(--fx-bg);
    border-left: 1px solid var(--rule);
    padding: 1.4rem 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    min-height: 0;
    overflow: auto;
  }
  h2 {
    font-size: 1.9rem;
  }
  .caps {
    font-size: 0.6rem;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }
  .tree {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .node {
    border: none;
    justify-content: flex-start;
    gap: 0.5rem;
    padding: 0.45rem 0.5rem 0.45rem 1.8rem;
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
    font-size: 0.85rem;
    color: var(--text);
  }
  .node span {
    flex: 1;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .node.root {
    padding-left: 0.4rem;
    font-weight: 600;
  }
  .node.current {
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }
  .node.off {
    opacity: 0.55;
  }
  .note {
    margin-top: 0.6rem;
    padding: 0.8rem;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
  }
  .note .caps {
    color: var(--accent);
  }
  .note p {
    margin: 0.4rem 0 0;
    font-size: 0.8rem;
    line-height: 1.45;
  }
</style>

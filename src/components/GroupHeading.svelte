<script lang="ts">
  // A collapsible group heading: ▾ Name ……… count  [⋯]. Folded groups are remembered.
  import { ChevronDown, ChevronRight, Ellipsis } from "@lucide/svelte";
  import type { Snippet } from "svelte";
  import { remember, ui } from "../lib/store.svelte";

  interface Props {
    /** Unique key for remembering whether it's folded, e.g. "scene:Wilderness". */
    id: string;
    name: string;
    count: number;
    onmenu?: (e: MouseEvent) => void;
    /** Drop target (e.g. presets dropped on a group move into it). */
    ondrop?: () => void;
    droppable?: boolean;
    children: Snippet;
  }
  let { id, name, count, onmenu, ondrop, droppable = false, children }: Props = $props();

  const folded = $derived(ui.collapsed.includes(id));
  let over = $state(false);

  function toggle() {
    ui.collapsed = folded ? ui.collapsed.filter((c) => c !== id) : [...ui.collapsed, id];
    remember("collapsed", ui.collapsed);
  }
</script>

<div
  class="heading"
  class:over
  role="group"
  ondragover={(e) => {
    if (!droppable) return;
    e.preventDefault();
    over = true;
  }}
  ondragleave={() => (over = false)}
  ondrop={(e) => {
    if (!droppable) return;
    e.preventDefault();
    over = false;
    ondrop?.();
  }}
>
  <button class="fold" onclick={toggle} aria-expanded={!folded}>
    {#if folded}<ChevronRight size={14} />{:else}<ChevronDown size={14} />{/if}
    <span class="name display">{name}</span>
  </button>
  <span class="count">{count}</span>
  {#if onmenu}
    <button class="icon" title="Group options" onclick={onmenu}><Ellipsis size={16} /></button>
  {/if}
</div>
{#if !folded}
  {@render children()}
{/if}

<style>
  .heading {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.55rem 0 0.15rem;
    border-radius: var(--radius);
  }
  .heading.over {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
  }
  .fold {
    flex: 1;
    border: none;
    padding: 0;
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
    color: var(--side-muted);
    min-width: 0;
    justify-content: flex-start;
  }
  .fold:hover {
    background: none !important;
  }
  .name {
    font-size: 1.35rem;
    color: var(--side-text);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .count {
    font-size: 0.72rem;
    color: var(--side-muted);
  }
  .icon {
    color: var(--side-muted);
  }
</style>

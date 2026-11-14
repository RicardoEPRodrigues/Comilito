<script lang="ts" module>
  // `module` script: runs once and is shared, so any component can open the menu with
  // `openMenu(event, items)` and a single <ContextMenu /> (in Workspace) shows it.
  // Used for right-click menus and for the "⋯" buttons.
  import { pop } from "../lib/motion";
  import type { Component } from "svelte";

  export type MenuItem =
    | {
        label: string;
        action?: () => void;
        icon?: Component;
        danger?: boolean;
        checked?: boolean;
        disabled?: boolean;
        /** A keyboard shortcut shown on the right. */
        hint?: string;
        /** Opens a second menu to the side instead of running an action. */
        submenu?: MenuItem[];
      }
    | "separator";

  export const menu = $state({ x: 0, y: 0, items: [] as MenuItem[], title: "" });

  /** Opens at the mouse (right-click) or under the clicked button ("⋯"). */
  export function openMenu(e: MouseEvent, items: MenuItem[], title = "") {
    e.preventDefault();
    e.stopPropagation();
    const target = e.currentTarget as HTMLElement | null;
    if (e.type === "click" && target?.getBoundingClientRect) {
      const r = target.getBoundingClientRect();
      menu.x = r.left;
      menu.y = r.bottom + 4;
    } else {
      menu.x = e.clientX;
      menu.y = e.clientY;
    }
    menu.items = items;
    menu.title = title;
  }
</script>

<script lang="ts">
  import { Check, ChevronRight } from "@lucide/svelte";

  let openSub = $state<number | null>(null);
  let box = $state<HTMLElement>();
  let pos = $state({ x: 0, y: 0 });

  function close() {
    menu.items = [];
    openSub = null;
  }

  // Keep the menu on screen: measure it once it's drawn and nudge it in.
  $effect(() => {
    if (!box || menu.items.length === 0) return;
    const r = box.getBoundingClientRect();
    pos = {
      x: Math.min(menu.x, window.innerWidth - r.width - 8),
      y: Math.min(menu.y, window.innerHeight - r.height - 8),
    };
  });
</script>

<svelte:window onclick={close} onkeydown={(e) => e.key === "Escape" && menu.items.length && (e.preventDefault(), close())} onblur={close} />

{#snippet items(list: MenuItem[], nested: boolean)}
  {#each list as item, i (i)}
    {#if item === "separator"}
      <hr />
    {:else}
      <div class="item-wrap">
        <button
          role="menuitem"
          class:danger={item.danger}
          disabled={item.disabled}
          onmouseenter={() => !nested && (openSub = item.submenu ? i : null)}
          onclick={(e) => {
            if (item.submenu) {
              e.stopPropagation();
              openSub = openSub === i ? null : i;
              return;
            }
            close();
            item.action?.();
          }}
        >
          <span class="ic">
            {#if item.checked}<Check size={14} />{:else if item.icon}<item.icon size={14} />{/if}
          </span>
          <span class="label">{item.label}</span>
          {#if item.hint}<span class="hint">{item.hint}</span>{/if}
          {#if item.submenu}<ChevronRight size={14} />{/if}
        </button>
        {#if item.submenu && openSub === i && !nested}
          <div class="menu sub" role="menu" in:pop>
            {@render items(item.submenu, true)}
          </div>
        {/if}
      </div>
    {/if}
  {/each}
{/snippet}

{#if menu.items.length > 0}
  <div
    bind:this={box}
    class="menu"
    in:pop
    style:left="{pos.x || menu.x}px"
    style:top="{pos.y || menu.y}px"
    role="menu"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={() => {}}
  >
    {#if menu.title}<div class="title caps">{menu.title}</div>{/if}
    {@render items(menu.items, false)}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 90;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--rule);
    border-radius: 6px;
    box-shadow: var(--shadow);
    padding: 0.3rem;
    display: flex;
    flex-direction: column;
    min-width: 14rem;
  }
  .menu.sub {
    position: absolute;
    left: calc(100% + 0.3rem);
    top: -0.3rem;
    min-width: 12rem;
  }
  .item-wrap {
    position: relative;
  }
  .title {
    color: var(--muted);
    font-size: 0.6rem;
    padding: 0.35rem 0.6rem 0.25rem;
  }
  button {
    width: 100%;
    border: none;
    border-radius: 4px;
    padding: 0.45rem 0.6rem;
    text-transform: none;
    letter-spacing: 0;
    font-weight: 450;
    font-size: 0.85rem;
    gap: 0.6rem;
    color: var(--text);
  }
  button:hover:not(:disabled) {
    background: var(--accent);
    color: var(--accent-text);
  }
  button.danger {
    color: var(--danger);
  }
  button.danger:hover:not(:disabled) {
    color: var(--accent-text);
  }
  .ic {
    width: 14px;
    display: inline-flex;
    opacity: 0.8;
  }
  .label {
    flex: 1;
    text-align: left;
  }
  .hint {
    font-size: 0.7rem;
    opacity: 0.65;
  }
  hr {
    border: none;
    border-top: 1px solid var(--rule);
    margin: 0.25rem 0.2rem;
  }
</style>

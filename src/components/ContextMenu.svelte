<script lang="ts" module>
  // `module` script: runs once and is shared, so any component can open the menu with
  // `openMenu(event, items)` and a single <ContextMenu /> (in Workspace) shows it.
  export interface MenuItem {
    label: string;
    action: () => void;
    danger?: boolean;
  }

  export const menu = $state({ x: 0, y: 0, items: [] as MenuItem[] });

  export function openMenu(e: MouseEvent, items: MenuItem[]) {
    e.preventDefault();
    menu.x = e.clientX;
    menu.y = e.clientY;
    menu.items = items;
  }
</script>

<script lang="ts">
  function close() {
    menu.items = [];
  }
</script>

<svelte:window onclick={close} onkeydown={(e) => e.key === "Escape" && close()} onblur={close} />

{#if menu.items.length > 0}
  <div class="menu" style:left="{menu.x}px" style:top="{menu.y}px" role="menu">
    {#each menu.items as item (item.label)}
      <button
        role="menuitem"
        class:danger={item.danger}
        onclick={() => {
          close();
          item.action();
        }}
      >
        {item.label}
      </button>
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 90;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: var(--shadow);
    padding: 0.3rem;
    display: flex;
    flex-direction: column;
    min-width: 12rem;
  }
  button {
    text-align: left;
    background: transparent;
    border: none;
    border-radius: 4px;
  }
  button:hover {
    background: var(--panel-2);
  }
</style>

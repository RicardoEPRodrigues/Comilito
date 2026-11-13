<script lang="ts">
  // Error and info messages in the corner. They disappear after a few seconds (see notify()).
  import { ui } from "../lib/store.svelte";
</script>

<div class="notices">
  {#each ui.notices as n (n.id)}
    <div class="notice">
      <span>{n.message}</span>
      <button class="icon" onclick={() => (ui.notices = ui.notices.filter((x) => x.id !== n.id))}>✕</button>
    </div>
  {/each}
</div>

<style>
  .notices {
    position: fixed;
    right: 1rem;
    bottom: 5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    z-index: 200;
    max-width: 28rem;
  }
  .notice {
    display: flex;
    align-items: start;
    gap: 0.5rem;
    background: var(--panel);
    border: 1px solid var(--border);
    border-left: 4px solid var(--warn);
    border-radius: 6px;
    padding: 0.5rem 0.6rem;
    box-shadow: var(--shadow);
    user-select: text;
  }
  span {
    flex: 1;
  }
</style>

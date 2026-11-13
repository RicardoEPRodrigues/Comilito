<script lang="ts">
  // Shows `ui.question` (set by `ask()` in the store) and resolves it with the answer.
  import { ui } from "../lib/store.svelte";

  let text = $state("");

  // Reset the text field whenever a new question appears.
  $effect(() => {
    text = ui.question?.input?.value ?? "";
  });

  function answer(choice: string | null) {
    const q = ui.question;
    ui.question = null;
    q?.resolve(choice === null ? null : { choice, text });
  }

  function onKeydown(e: KeyboardEvent) {
    if (!ui.question) return;
    if (e.key === "Escape") {
      // Marks the key as handled, so the editor underneath (and the global Esc = stop all)
      // ignore it. They all listen on the window, so the order they run in isn't guaranteed.
      e.preventDefault();
      answer(null);
    } else if (e.key === "Enter") {
      const primary = ui.question.choices.find((c) => c.primary);
      if (primary) {
        e.preventDefault();
        answer(primary.value);
      }
    }
  }

  /** Focuses an element when it appears. Used as `use:focus` (a Svelte "action"). */
  function focus(node: HTMLElement) {
    node.focus();
    if (node instanceof HTMLInputElement) node.select();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if ui.question}
  <div class="backdrop">
    <div class="dialog" role="dialog" aria-modal="true" aria-label={ui.question.title}>
      <h3>{ui.question.title}</h3>
      {#if ui.question.message}
        <p>{ui.question.message}</p>
      {/if}
      {#if ui.question.input}
        <label>
          <span class="muted">{ui.question.input.label}</span>
          <input bind:value={text} use:focus />
        </label>
      {/if}
      <div class="buttons">
        {#each ui.question.choices as c (c.value)}
          <button class:primary={c.primary} class:danger={c.danger} onclick={() => answer(c.value)}>
            {c.label}
          </button>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.4);
    display: grid;
    place-items: center;
    z-index: 100;
  }
  .dialog {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 1.2rem;
    width: min(26rem, 90vw);
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  p {
    margin: 0;
    white-space: pre-line;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
</style>

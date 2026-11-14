<script lang="ts">
  // A shortcut field: click it, press a key, and that key is the new shortcut. Esc cancels.
  // `value` is a `KeyboardEvent.key` ("Backspace", "Delete", "x"); "" means off.
  // Keys the app already uses everywhere (Space, Esc, /) can't be chosen.
  import { X } from "@lucide/svelte";

  let { value, onchange }: { value: string; onchange: (key: string) => void } = $props();

  const RESERVED: Record<string, string> = { " ": "Space", Escape: "Esc", "/": "/" };
  let listening = $state(false);
  let refused = $state("");

  const label = (key: string) => (key === "" ? "None" : key.length === 1 ? key.toUpperCase() : key);

  function onkeydown(e: KeyboardEvent) {
    if (!listening) return;
    // Taken here, so neither the window's shortcuts nor the dialog's Esc see it.
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      listening = false;
      return;
    }
    // A modifier on its own isn't a shortcut; wait for the real key.
    if (["Shift", "Control", "Alt", "Meta"].includes(e.key)) return;
    if (e.key in RESERVED) {
      refused = `${RESERVED[e.key]} is already used`;
      return;
    }
    refused = "";
    listening = false;
    onchange(e.key);
  }
</script>

<span class="key-input">
  <button
    type="button"
    class="key"
    class:listening
    onclick={() => ((listening = !listening), (refused = ""))}
    onblur={() => (listening = false)}
    {onkeydown}
  >
    {listening ? "Press a key…" : label(value)}
  </button>
  {#if value && !listening}
    <button type="button" class="icon" title="No shortcut" onclick={() => onchange("")}><X size={14} /></button>
  {/if}
  {#if refused}<span class="refused small">{refused}</span>{/if}
</span>

<style>
  .key-input {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }
  .key {
    min-width: 7rem;
    justify-content: center;
    text-transform: none;
    letter-spacing: 0;
    font-variant-numeric: tabular-nums;
  }
  .key.listening {
    border-color: var(--accent);
    color: var(--accent);
  }
  .refused {
    color: var(--warn);
  }
</style>

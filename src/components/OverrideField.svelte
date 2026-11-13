<script lang="ts">
  // A setting that inherits a default unless overridden (docs/preferences.md):
  // shows the inherited value greyed out, and a "reset" button when it's overridden.
  //
  // `kind` picks the input: "percent" (0–1 shown as 0–100%), "seconds" (stored as ms),
  // or "toggle" (on/off).
  interface Props {
    label: string;
    kind: "percent" | "seconds" | "toggle";
    /** The override, or undefined to inherit. Numbers for percent/seconds, boolean for toggle. */
    value: number | boolean | undefined;
    inherited: number | boolean;
    onchange: (value: number | boolean | undefined) => void;
  }
  let { label, kind, value, inherited, onchange }: Props = $props();

  const overridden = $derived(value !== undefined);
  const shown = $derived(value ?? inherited);
</script>

<div class="field" class:inherited={!overridden}>
  <span class="label">{label}</span>

  {#if kind === "percent"}
    <input
      type="range"
      min="0"
      max="100"
      value={Math.round((shown as number) * 100)}
      oninput={(e) => onchange(e.currentTarget.valueAsNumber / 100)}
    />
    <span class="value">{Math.round((shown as number) * 100)}%</span>
  {:else if kind === "seconds"}
    <input
      type="number"
      min="0"
      max="60"
      step="0.1"
      value={(shown as number) / 1000}
      onchange={(e) => {
        const s = e.currentTarget.valueAsNumber;
        if (!Number.isNaN(s)) onchange(Math.round(Math.max(0, s) * 1000));
      }}
    />
    <span class="value">s</span>
  {:else}
    <input type="checkbox" checked={shown as boolean} onchange={(e) => onchange(e.currentTarget.checked)} />
    <span class="value"></span>
  {/if}

  {#if overridden}
    <button class="icon" title="Reset to the project default" onclick={() => onchange(undefined)}>↺</button>
  {:else}
    <span class="muted note">default</span>
  {/if}
</div>

<style>
  .field {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .label {
    width: 8rem;
  }
  input[type="range"] {
    width: 10rem;
  }
  input[type="number"] {
    width: 5rem;
  }
  .value {
    width: 2.5rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .inherited input {
    opacity: 0.55;
  }
  .note {
    font-size: 0.8rem;
    width: 2.2rem;
  }
</style>

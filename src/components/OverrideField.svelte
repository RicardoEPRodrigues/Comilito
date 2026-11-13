<script lang="ts">
  // A setting that inherits a default unless overridden (docs/preferences.md): shows the
  // inherited value greyed out with a link icon, and a reset button when it's overridden.
  //
  // `kind` picks the input: "percent" (0–1 shown as 0–100%), "seconds" (stored as ms),
  // or "toggle" (on/off).
  import { Link, RotateCcw } from "@lucide/svelte";

  interface Props {
    label: string;
    kind: "percent" | "seconds" | "toggle";
    /** The override, or undefined to inherit. Numbers for percent/seconds, boolean for toggle. */
    value: number | boolean | undefined;
    inherited: number | boolean;
    onchange: (value: number | boolean | undefined) => void;
    /** For bulk editing: several sounds with different values. */
    mixed?: boolean;
  }
  let { label, kind, value, inherited, onchange, mixed = false }: Props = $props();

  const overridden = $derived(value !== undefined);
  const shown = $derived(value ?? inherited);
</script>

<div class="field override">
  {#if label}<span class="caps">{label}</span>{/if}
  <div class="box" class:inherited={!overridden && !mixed}>
    {#if kind === "percent"}
      <input
        type="range"
        min="0"
        max="100"
        value={Math.round((shown as number) * 100)}
        oninput={(e) => onchange(e.currentTarget.valueAsNumber / 100)}
        aria-label={label}
      />
      <span class="value">{mixed ? "mixed" : `${Math.round((shown as number) * 100)}%`}</span>
    {:else if kind === "seconds"}
      <input
        class="num"
        type="number"
        min="0"
        max="60"
        step="0.1"
        value={mixed ? "" : (shown as number) / 1000}
        placeholder={mixed ? "mixed" : ""}
        onchange={(e) => {
          const s = e.currentTarget.valueAsNumber;
          if (!Number.isNaN(s)) onchange(Math.round(Math.max(0, s) * 1000));
        }}
        aria-label={label}
      />
      <span class="value">s</span>
    {:else}
      <label class="switch">
        <input type="checkbox" checked={shown as boolean} onchange={(e) => onchange(e.currentTarget.checked)} aria-label={label} />
        <span class="value">{mixed ? "mixed" : shown ? "On" : "Off"}</span>
      </label>
    {/if}
    {#if overridden}
      <button class="icon" title="Reset to the default" onclick={() => onchange(undefined)}><RotateCcw size={13} /></button>
    {:else}
      <span class="link" title="Follows the project default"><Link size={13} /></span>
    {/if}
  </div>
</div>

<style>
  .box {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    padding: 0.3rem 0.4rem 0.3rem 0.6rem;
    min-height: 2.3rem;
  }
  .box input[type="range"] {
    flex: 1;
    min-width: 4rem;
  }
  .num {
    flex: 1;
    border: none;
    background: transparent;
    padding: 0.1rem 0;
    font-weight: 600;
    width: 3rem;
  }
  .value {
    font-weight: 600;
    font-size: 0.85rem;
    font-variant-numeric: tabular-nums;
  }
  .switch {
    flex: 1;
  }
  .inherited .value,
  .inherited .num {
    color: var(--muted);
    font-weight: 400;
  }
  .inherited input[type="range"] {
    opacity: 0.55;
  }
  .link {
    color: var(--muted);
    display: inline-flex;
    padding: 0.3rem;
  }
  .icon {
    color: var(--muted);
  }
</style>

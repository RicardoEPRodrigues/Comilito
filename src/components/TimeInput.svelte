<script lang="ts">
  // A time typed as hh:mm:ss in one field (long videos need hours). Shorter forms are accepted
  // too: "1:30" is a minute and a half, "90" is 90 seconds. `value` is in milliseconds;
  // `onchange` gets the new total once the field is left or Enter is pressed.
  let { value, onchange, label = "" }: { value: number; onchange: (ms: number) => void; label?: string } = $props();

  const pad = (n: number) => String(n).padStart(2, "0");

  function format(ms: number): string {
    const total = Math.max(0, Math.round(ms / 1000));
    return `${pad(Math.floor(total / 3600))}:${pad(Math.floor((total % 3600) / 60))}:${pad(total % 60)}`;
  }

  /** Seconds from "h:m:s", "m:s" or "s"; null if it isn't a time. */
  function parse(text: string): number | null {
    const parts = text.trim().split(":");
    if (parts.length > 3 || parts.some((p) => !/^\d+$/.test(p))) return null;
    // Read right to left: seconds, minutes, hours.
    return parts.reverse().reduce((sum, p, i) => sum + Number(p) * 60 ** i, 0);
  }

  let invalid = $state(false);

  function commit(input: HTMLInputElement) {
    const seconds = parse(input.value);
    invalid = seconds === null;
    if (seconds === null) return;
    onchange(seconds * 1000);
    // Normalise what was typed ("90" becomes 00:01:30), even when the value didn't change.
    input.value = format(seconds * 1000);
  }
</script>

<input
  class="time"
  class:invalid
  value={format(value)}
  aria-label={label}
  title="Hours:minutes:seconds"
  inputmode="numeric"
  spellcheck="false"
  onchange={(e) => commit(e.currentTarget)}
  onkeydown={(e) => e.key === "Enter" && commit(e.currentTarget)}
/>

<style>
  .time {
    width: 6.2rem;
    padding: 0.3rem 0.45rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    text-align: center;
  }
  .invalid {
    border-color: var(--warn);
  }
</style>

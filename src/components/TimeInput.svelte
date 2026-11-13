<script lang="ts">
  // A time as three small fields: hours, minutes, seconds (TODO.md: long videos need hours).
  // `value` is in milliseconds; `onchange` gets the new total whenever a field changes.
  let { value, onchange, label = "" }: { value: number; onchange: (ms: number) => void; label?: string } = $props();

  const total = $derived(Math.max(0, Math.round(value / 1000)));
  const h = $derived(Math.floor(total / 3600));
  const m = $derived(Math.floor((total % 3600) / 60));
  const s = $derived(total % 60);

  function set(part: "h" | "m" | "s", raw: number) {
    const n = Number.isFinite(raw) ? Math.max(0, Math.floor(raw)) : 0;
    const hours = part === "h" ? n : h;
    const minutes = part === "m" ? n : m;
    const seconds = part === "s" ? n : s;
    onchange((hours * 3600 + minutes * 60 + seconds) * 1000);
  }
</script>

<span class="time" role="group" aria-label={label}>
  <input type="number" min="0" value={h} onchange={(e) => set("h", e.currentTarget.valueAsNumber)} aria-label="Hours" />
  <span>h</span>
  <input type="number" min="0" max="59" value={m} onchange={(e) => set("m", e.currentTarget.valueAsNumber)} aria-label="Minutes" />
  <span>m</span>
  <input type="number" min="0" max="59" value={s} onchange={(e) => set("s", e.currentTarget.valueAsNumber)} aria-label="Seconds" />
  <span>s</span>
</span>

<style>
  .time {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
  }
  input {
    width: 3.4rem;
    padding: 0.3rem 0.35rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  span {
    color: var(--muted);
    font-size: 0.8rem;
    margin-right: 0.25rem;
  }
</style>

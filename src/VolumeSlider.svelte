<script lang="ts">
  // A labelled 0–100% slider. The parent passes props in like function arguments:
  //   <VolumeSlider label="Music" value={0.8} onchange={(v) => ...} />
  // `$props()` is how a Svelte 5 component receives them.
  interface Props {
    label: string;
    /** 0.0–1.0 */
    value: number;
    /** Called continuously while dragging, with the new 0.0–1.0 value. */
    onchange: (value: number) => void;
  }

  let { label, value, onchange }: Props = $props();

  // The slider works in whole percent; the rest of the app uses 0.0–1.0.
  const percent = $derived(Math.round(value * 100));
</script>

<label>
  <span class="name">{label}</span>
  <input
    type="range"
    min="0"
    max="100"
    value={percent}
    oninput={(e) => onchange(e.currentTarget.valueAsNumber / 100)}
  />
  <span class="value">{percent}%</span>
</label>

<style>
  label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .name {
    width: 5rem;
  }

  input {
    width: 12rem;
  }

  .value {
    width: 3rem;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: #aaa;
  }
</style>

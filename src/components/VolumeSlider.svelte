<script lang="ts">
  // A labelled 0–100% slider for the mixer bar. The parent passes props in like function
  // arguments:  <VolumeSlider label="Music" value={0.8} onchange={(v) => ...} />
  interface Props {
    label: string;
    /** 0.0–1.0 */
    value: number;
    /** Called continuously while dragging, with the new 0.0–1.0 value. */
    onchange: (value: number) => void;
  }
  let { label, value, onchange }: Props = $props();
  const percent = $derived(Math.round(value * 100));
</script>

<label>
  <span class="top"><span class="name">{label}</span><span class="value">{percent}%</span></span>
  <input type="range" min="0" max="100" value={percent} oninput={(e) => onchange(e.currentTarget.valueAsNumber / 100)} />
</label>

<style>
  label {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    width: 9.5rem;
  }
  .top {
    display: flex;
    justify-content: space-between;
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }
  .name {
    opacity: 0.7;
  }
  input {
    width: 100%;
    accent-color: var(--accent);
  }
</style>

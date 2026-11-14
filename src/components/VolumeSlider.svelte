<script lang="ts">
  // A labelled 0–100% slider for the mixer bar. The parent passes props in like function
  // arguments:  <VolumeSlider label="Music" value={0.8} onchange={(v) => ...} />
  //
  // With `glideMs`, a new value that comes from elsewhere (another scene on top of the
  // tapestry) slides there over that time, in step with the sound's own fade.
  import { Tween } from "svelte/motion";

  interface Props {
    label: string;
    /** 0.0–1.0 */
    value: number;
    /** Called continuously while dragging, with the new 0.0–1.0 value. */
    onchange: (value: number) => void;
    /** How long to glide to a new `value` that wasn't set by dragging this slider. */
    glideMs?: number;
  }
  let { label, value, onchange, glideMs = 0 }: Props = $props();

  // `Tween` holds a number that moves towards whatever it's `set` to, over a duration.
  // svelte-ignore state_referenced_locally
  const shown = new Tween(value);
  let dragging = false;
  $effect(() => {
    // While dragging, the slider follows the hand exactly.
    shown.set(value, { duration: dragging ? 0 : glideMs });
  });
  const percent = $derived(Math.round(shown.current * 100));
</script>

<label>
  <span class="top"><span class="name">{label}</span><span class="value">{percent}%</span></span>
  <input
    type="range"
    min="0"
    max="100"
    value={percent}
    onpointerdown={() => (dragging = true)}
    onpointerup={() => (dragging = false)}
    onpointercancel={() => (dragging = false)}
    oninput={(e) => onchange(e.currentTarget.valueAsNumber / 100)}
  />
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

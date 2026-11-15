<script lang="ts">
  // The "Levels", "Sound defaults per category", "Video" and "Backups" sections, shared by
  // Project settings and Preferences (docs/preferences.md).
  //
  // The parent passes draft objects with `bind:defaults={...}`; `$bindable()` allows this
  // component to change them in place.
  import type { BackupSettings, ProjectDefaults } from "../lib/types";
  import { CATEGORIES } from "../lib/types";
  import { videoEnabled } from "../lib/video";

  let {
    defaults = $bindable(),
    backups = $bindable(),
  }: { defaults: ProjectDefaults; backups: BackupSettings } = $props();

  const pct = (v: number) => Math.round(v * 100);
</script>

<section>
  <h2>Levels</h2>
  <label class="row">
    <span>Ducking level</span>
    <input type="range" min="0" max="100" value={pct(defaults.duckLevel)}
      oninput={(e) => (defaults.duckLevel = e.currentTarget.valueAsNumber / 100)} />
    <span class="v">{pct(defaults.duckLevel)}%</span>
  </label>
  <label class="row">
    <span>New sound volume</span>
    <input type="range" min="0" max="100" value={pct(defaults.soundVolume)}
      oninput={(e) => (defaults.soundVolume = e.currentTarget.valueAsNumber / 100)} />
    <span class="v">{pct(defaults.soundVolume)}%</span>
  </label>
  <label class="row">
    <span>Scene transition</span>
    <input type="number" min="0" max="60" step="0.1" value={defaults.transitionMs / 1000}
      onchange={(e) => (defaults.transitionMs = Math.round(Math.max(0, e.currentTarget.valueAsNumber || 0) * 1000))} />
    <span class="v">s</span>
  </label>
</section>

<section>
  <h2>Sound defaults per category</h2>
  <table>
    <thead>
      <tr><th></th><th>Loop</th><th>Fade in (s)</th><th>Fade out (s)</th><th>Shuffle</th></tr>
    </thead>
    <tbody>
      {#each CATEGORIES as c (c.id)}
        {@const d = defaults.categories[c.id]}
        <tr>
          <td>{c.label}</td>
          <td><input type="checkbox" bind:checked={d.loop} /></td>
          <td><input type="number" min="0" max="60" step="0.1" value={d.fadeInMs / 1000}
            onchange={(e) => (d.fadeInMs = Math.round(Math.max(0, e.currentTarget.valueAsNumber || 0) * 1000))} /></td>
          <td><input type="number" min="0" max="60" step="0.1" value={d.fadeOutMs / 1000}
            onchange={(e) => (d.fadeOutMs = Math.round(Math.max(0, e.currentTarget.valueAsNumber || 0) * 1000))} /></td>
          <td>
            {#if c.id === "music"}
              <input type="checkbox" checked={d.shuffle ?? false} onchange={(e) => (d.shuffle = e.currentTarget.checked)} />
            {:else}—{/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</section>

<section>
  <h2>Loudness normalization</h2>
  <p class="muted help">
    Files from different sources are recorded at different volumes. Normalization measures each
    file once and evens them out, so switching from one track or ambience to another doesn't
    jump in volume. Quiet files are only turned up as far as they can go without distorting.
  </p>
  <div class="row">
    <span>Normalize</span>
    {#each CATEGORIES as c (c.id)}
      <label class="check">
        <input type="checkbox" bind:checked={defaults.normalize[c.id]} />
        {c.label}
      </label>
    {/each}
  </div>
  <label class="row">
    <span>Target loudness</span>
    <input type="number" min="-40" max="-6" step="1" bind:value={defaults.normalize.targetLufs} />
    <span class="v">LUFS (−18 is a good start; lower is quieter)</span>
  </label>
</section>

{#if videoEnabled() && defaults.video}
  <section>
    <h2>Video</h2>
    <p class="muted help">What scenes' video playlists use unless they set their own (decision 0035).</p>
    <label class="row">
      <span>Switch videos every</span>
      <input type="number" min="0" max="240" step="1" bind:value={defaults.video.switchMinutes} />
      <span class="v">min (0 = play each to its end)</span>
    </label>
    <label class="row">
      <span>Coming back to a video</span>
      <select bind:value={defaults.video.resume}>
        <option value={true}>Resume where it left off</option>
        <option value={false}>Start it over</option>
      </select>
    </label>
    <label class="row">
      <span>Shuffle</span>
      <input type="checkbox" bind:checked={defaults.video.shuffle} />
    </label>
  </section>
{/if}

<section>
  <h2>Backups</h2>
  <label class="row">
    <span>Every</span>
    <input type="number" min="1" max="1440" bind:value={backups.intervalMinutes} />
    <span class="v">min</span>
  </label>
  <label class="row">
    <span>Backups to keep</span>
    <input type="number" min="1" max="100" bind:value={backups.keep} />
  </label>
  <label class="row">
    <span>Include audio</span>
    <input type="checkbox" bind:checked={backups.includeAudio} />
  </label>
  {#if videoEnabled()}
    <label class="row">
      <span>Include videos</span>
      <input type="checkbox" checked={backups.includeVideo ?? false} onchange={(e) => (backups.includeVideo = e.currentTarget.checked)} />
      <span class="v">videos are big</span>
    </label>
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .row span:first-child {
    width: 9rem;
  }
  input[type="range"] {
    width: 10rem;
  }
  input[type="number"] {
    width: 5rem;
  }
  .v {
    color: var(--muted);
  }
  .help {
    margin: 0;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    margin-right: 0.6rem;
  }
  table {
    border-collapse: collapse;
  }
  th {
    font-weight: normal;
    color: var(--muted);
    text-align: left;
    padding: 0 0.5rem 0.2rem 0;
  }
  td {
    padding: 0.15rem 0.5rem 0.15rem 0;
  }
</style>

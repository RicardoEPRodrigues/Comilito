<script lang="ts">
  // About Comilito: author links and acknowledgements. Keep in sync with CREDITS.md.
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "../lib/api";
  import { run } from "../lib/store.svelte";
  import Modal from "./Modal.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let version = $state("");
  onMount(async () => {
    version = (await run(api.appVersion())) ?? "";
  });

  const CC_BY_3 = "https://creativecommons.org/licenses/by/3.0/";
  const PDM_1 = "https://creativecommons.org/publicdomain/mark/1.0/";

  /** Links open in the system browser; a plain <a href> would try to navigate the app itself. */
  function open(e: MouseEvent, url: string) {
    e.preventDefault();
    run(openUrl(url));
  }
</script>

<!-- A snippet is a reusable piece of markup inside this component, like a small local function. -->
{#snippet link(url: string, text: string)}
  <a href={url} onclick={(e) => open(e, url)}>{text}</a>
{/snippet}

<Modal title="About Comilito" {onclose}>
  <section class="intro">
    <h1>Comilito</h1>
    <p class="muted">Version {version}</p>
    <p>A soundboard and mixer for Dungeons &amp; Dragons Dungeon Masters.</p>
    <p>
      Made by Ricardo Rodrigues ·
      {@render link("https://colorcrow.me/", "colorcrow.me")} ·
      {@render link("https://github.com/RicardoEPRodrigues", "GitHub")}
    </p>
    <p>Source code: {@render link("https://github.com/RicardoEPRodrigues/Comilito", "github.com/RicardoEPRodrigues/Comilito")}</p>
  </section>

  <section>
    <h2>Made with AI</h2>
    <p class="ai">
      Comilito was created with the help of AI: most of its code and documentation were written
      by {@render link("https://www.anthropic.com/claude", "Claude")} (Anthropic), through Claude Code,
      with Ricardo Rodrigues directing the design, making the decisions and testing the app.
    </p>
  </section>

  <section>
    <h2>Acknowledgements</h2>

    <h3>Sample audio</h3>
    <ul>
      <li>
        <em>Nocturne in E-flat major, Op. 9 No. 2</em> by Frédéric Chopin, recorded by the
        {@render link("https://musopen.org/", "Musopen")} Kickstarter Project “Set Chopin Free”, via
        {@render link("https://www.classicals.de/", "Classicals.de")}.
        {@render link(PDM_1, "Public Domain Mark 1.0")}.
      </li>
      <li>
        {@render link("https://soundbible.com/2217-Heavy-Rain-Wind.html", "Heavy Rain Wind")} by Daniel Simion,
        from SoundBible. {@render link(CC_BY_3, "CC BY 3.0")}.
      </li>
      <li>
        {@render link("https://soundbible.com/2206-Tolling-Bell.html", "Tolling Bell")} by Daniel Simion,
        from SoundBible. {@render link(CC_BY_3, "CC BY 3.0")}.
      </li>
      <li>
        {@render link("https://soundbible.com/1512-Civil-War-Drummer-.html", "Civil War Drummer")} by DrumM8,
        from SoundBible. {@render link(CC_BY_3, "CC BY 3.0")}.
      </li>
    </ul>
    <p class="muted note">
      Files were renamed, and the Chopin recording, Heavy Rain Wind and Tolling Bell were
      re-encoded to a lower bitrate to keep the app small. Nothing else was changed.
    </p>

    <h3>Images</h3>
    <ul>
      <li>
        The app icon, a skeletal pale horse in the style of medieval manuscript illumination, is original
        artwork generated with AI in {@render link("https://pen.dev", "Pencil")}.
      </li>
      <li>Interface symbols are Unicode emoji, drawn by your system's fonts.</li>
    </ul>

    <h3>Inspiration</h3>
    <ul>
      <li>
        {@render link("https://github.com/ilyaly/ascapes-mixer", "Ascapes Mixer")} by ilyaly (MIT).
        Comilito is an independent implementation, not a fork.
      </li>
    </ul>

    <h3>Built with</h3>
    <ul>
      <li>
        {@render link("https://tauri.app", "Tauri")},
        {@render link("https://github.com/tesselode/kira", "kira")} (audio),
        {@render link("https://github.com/pdeljanov/Symphonia", "Symphonia")} (decoding),
        {@render link("https://svelte.dev", "Svelte")}, and other open-source libraries listed in
        <code>Cargo.toml</code> and <code>package.json</code>, each under its own licence.
      </li>
    </ul>
  </section>
</Modal>

<style>
  .intro {
    text-align: center;
  }
  .intro p {
    margin: 0.3rem 0;
  }
  h3 {
    font-size: 0.95rem;
    margin-top: 0.8rem;
  }
  ul {
    margin: 0.3rem 0;
    padding-left: 1.2rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    user-select: text;
  }
  a {
    color: var(--accent);
  }
  .ai {
    margin: 0.3rem 0 0;
    user-select: text;
  }
  .note {
    margin: 0.2rem 0 0;
    font-size: 0.85rem;
  }
</style>

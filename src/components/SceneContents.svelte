<script lang="ts">
  // What a scene holds, as the Library's category icons with a count each:
  //   ♪ 2   ≋ 3   ⚡ 1   ⛰ 1   ✦ 2
  // (music tracks, ambience layers, effects, ambience videos, video effects). Shorter than
  // spelling it out, so it fits the scene cards, the Tapestry and the mini player. Each icon
  // has a tooltip saying what it counts. `children` adds more at the end (the Tapestry's ducking).
  import { VolumeX } from "@lucide/svelte";
  import type { Snippet } from "svelte";
  import { SHELF_ICONS } from "../lib/icons";
  import type { Scene, Shelf } from "../lib/types";
  import { videoEnabled } from "../lib/video";

  let { scene, size = 12, children }: { scene: Scene; size?: number; children?: Snippet } = $props();

  const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

  const items = $derived.by(() => {
    const list: { shelf: Shelf; count: number; title: string }[] = [];
    const tracks = scene.music?.tracks.length ?? 0;
    if (tracks) list.push({ shelf: "music", count: tracks, title: plural(tracks, "music track") });
    if (scene.ambience.length) {
      const title = plural(scene.ambience.length, "ambience layer") + (scene.ducksAll ? " · ducks all ambience below" : "");
      list.push({ shelf: "ambience", count: scene.ambience.length, title });
    }
    if (scene.effects.length) list.push({ shelf: "effects", count: scene.effects.length, title: plural(scene.effects.length, "effect") });
    if (videoEnabled()) {
      const videos = scene.video?.videos.length ?? 0;
      const videoEffects = scene.videoEffects?.length ?? 0;
      if (videos) list.push({ shelf: "ambienceVideo", count: videos, title: plural(videos, "video") });
      if (videoEffects) list.push({ shelf: "videoEffect", count: videoEffects, title: plural(videoEffects, "video effect") });
    }
    return list;
  });
  // A scene without music that silences the music below says so (it has no count to show).
  const silences = $derived(!scene.music?.tracks.length && scene.blocksMusic);
</script>

<span class="contents">
  {#if silences}
    <span class="item" title="Silences the music below"><VolumeX {size} /></span>
  {/if}
  {#each items as item (item.shelf)}
    {@const Icon = SHELF_ICONS[item.shelf]}
    <span class="item" title={item.title}><Icon {size} />{item.count}</span>
  {/each}
  {#if !items.length && !silences}<span class="empty">Empty</span>{/if}
  {@render children?.()}
</span>

<style>
  .contents {
    display: inline-flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.15rem 0.6rem;
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
    line-height: 1.3;
  }
  .contents :global(.item) {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    white-space: nowrap;
  }
  .contents :global(svg) {
    flex: none;
  }
</style>

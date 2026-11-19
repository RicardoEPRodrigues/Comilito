// The icon for each Library shelf, shared by the Library and by the scene summaries
// (SceneContents.svelte), so a category looks the same everywhere.
import { AudioLines, Mountain, Music, Sparkles, Zap } from "@lucide/svelte";
import type { Component } from "svelte";
import type { Shelf } from "./types";

export const SHELF_ICONS: Record<Shelf, Component> = {
  music: Music,
  ambience: AudioLines,
  effects: Zap,
  ambienceVideo: Mountain,
  videoEffect: Sparkles,
};

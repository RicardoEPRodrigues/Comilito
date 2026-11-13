// Loudness normalization numbers for display. Mirrors `normalization_gain_db` in
// src-tauri/src/project.rs (Rust is what actually applies the gain); keep them in sync.

import type { Project, Sound } from "./types";

const MAX_BOOST_DB = 12;

/** The gain (dB) normalization applies to a sound, or null if it doesn't apply. */
export function normalizationGainDb(project: Project, sound: Sound): number | null {
  const n = project.defaults.normalize;
  const l = sound.file.loudness;
  if (!l || !n[sound.category] || l.lufs < -60) return null;
  const wanted = n.targetLufs - l.lufs;
  if (wanted <= 0) return wanted;
  const headroom = l.peak > 0 ? Math.max(0, -20 * Math.log10(l.peak)) : 0;
  return Math.min(wanted, headroom, MAX_BOOST_DB);
}

export function formatDb(db: number): string {
  return `${db >= 0 ? "+" : "−"}${Math.abs(db).toFixed(1)} dB`;
}

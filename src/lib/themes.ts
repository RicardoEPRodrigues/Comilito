// Themes (decision 0016): every colour and font is a named token, set as a CSS variable on
// <html>. Components only ever use `var(--token)`, so switching theme is just setting new
// values. The three built-in themes come from the designs in pencil/comilito.pen.
//
// User themes have the same shape and are stored by Rust (src-tauri/src/themes.rs).

import type { Appearance, DesktopAppearance, Theme } from "./types";

/** Every colour token, with what it's for (shown in the theme editor). */
export const COLOR_TOKENS: { key: string; label: string }[] = [
  { key: "bg", label: "Page" },
  { key: "surface", label: "Cards and fields" },
  { key: "text", label: "Text" },
  { key: "muted", label: "Secondary text" },
  { key: "rule", label: "Borders" },
  { key: "gold", label: "Gold (ornaments, capitals)" },
  { key: "accent", label: "Accent (what's live)" },
  { key: "header-bg", label: "Header" },
  { key: "header-text", label: "Header text" },
  { key: "side-bg", label: "Side panel" },
  { key: "side-text", label: "Side panel text" },
  { key: "side-muted", label: "Side panel secondary text" },
  { key: "fx-bg", label: "Effects and drawer" },
  { key: "mixer-bg", label: "Mixer bar" },
  { key: "mixer-text", label: "Mixer bar text" },
  { key: "track", label: "Slider tracks" },
  { key: "warn", label: "Warnings" },
  { key: "danger", label: "Danger" },
];

/** Display fonts bundled with the app (src/main.ts imports them). */
export const DISPLAY_FONTS = ["UnifrakturMaguntia", "Grenze Gotisch", "Pirata One", "MedievalSharp", "Inter"];
export const BODY_FONTS = ["Inter", "system-ui"];

export const BUILT_IN_THEMES: Theme[] = [
  {
    id: "codex",
    name: "Codex",
    dark: false,
    fonts: { display: "UnifrakturMaguntia", body: "Inter" },
    colors: {
      bg: "#f4f2e9",
      surface: "#fbfaf4",
      text: "#281d1b",
      muted: "#7a6a5e",
      rule: "#c6b88f",
      gold: "#c6b88f",
      accent: "#dd4b2e",
      "header-bg": "#dd4b2e",
      "header-text": "#fbf4e8",
      "side-bg": "#e8e3c8",
      "side-text": "#281d1b",
      "side-muted": "#76665a",
      "fx-bg": "#ede8d2",
      "mixer-bg": "#281d1b",
      "mixer-text": "#f4f2e9",
      track: "#e3dcc4",
      warn: "#a8620e",
      danger: "#b3261e",
    },
  },
  {
    id: "ironwood",
    name: "Ironwood",
    dark: true,
    fonts: { display: "Grenze Gotisch", body: "Inter" },
    colors: {
      bg: "#1e1614",
      surface: "#2a1f1c",
      text: "#efe8d6",
      muted: "#a3937f",
      rule: "#4a3a33",
      gold: "#8c7a55",
      accent: "#e2583b",
      "header-bg": "#120d0c",
      "header-text": "#efe8d6",
      "side-bg": "#171110",
      "side-text": "#efe8d6",
      "side-muted": "#a3937f",
      "fx-bg": "#171110",
      "mixer-bg": "#0e0a09",
      "mixer-text": "#efe8d6",
      track: "#3a2c28",
      warn: "#e0a43a",
      danger: "#ff7a6b",
    },
  },
  {
    id: "hearth",
    name: "Hearth",
    dark: false,
    fonts: { display: "Pirata One", body: "Inter" },
    colors: {
      bg: "#f4f2e9",
      surface: "#fbfaf4",
      text: "#281d1b",
      muted: "#7a6a5e",
      rule: "#c6b88f",
      gold: "#c6b88f",
      accent: "#dd4b2e",
      "header-bg": "#281d1b",
      "header-text": "#f4f2e9",
      "side-bg": "#dd4b2e",
      "side-text": "#fbf4e8",
      "side-muted": "#f8cdbf",
      "fx-bg": "#e8e3c8",
      "mixer-bg": "#281d1b",
      "mixer-text": "#f4f2e9",
      track: "#e3dcc4",
      warn: "#a8620e",
      danger: "#b3261e",
    },
  },
];

export function findTheme(id: string, userThemes: Theme[]): Theme {
  return [...BUILT_IN_THEMES, ...userThemes].find((t) => t.id === id) ?? BUILT_IN_THEMES[0];
}

/** The theme to show now: fixed, or the light/dark pair following the desktop. */
export function activeTheme(appearance: Appearance, desktop: DesktopAppearance | null, userThemes: Theme[]): Theme {
  if (appearance.mode === "fixed") return findTheme(appearance.theme, userThemes);
  const dark = desktop?.dark ?? window.matchMedia("(prefers-color-scheme: dark)").matches;
  return findTheme(dark ? appearance.darkTheme : appearance.lightTheme, userThemes);
}

/** Sets the CSS variables and switches for a theme. */
export function applyTheme(theme: Theme, appearance: Appearance, desktop: DesktopAppearance | null) {
  const root = document.documentElement;
  // Fill any token a (user) theme lacks from the built-in theme of the same lightness.
  const base = BUILT_IN_THEMES.find((t) => t.id === (theme.dark ? "ironwood" : "codex"))!;
  const colors = { ...base.colors, ...theme.colors };

  let accent = colors.accent;
  if (appearance.accent === "desktop" && desktop?.accent) accent = desktop.accent;
  if (appearance.accent === "custom" && /^#[0-9a-f]{6}$/i.test(appearance.customAccent)) accent = appearance.customAccent;
  // Keep the accent readable against the page (3:1, the guideline for UI elements).
  colors.accent = ensureContrast(accent, colors.bg, 3);
  colors["accent-text"] = bestTextOn(colors.accent);

  for (const [key, value] of Object.entries(colors)) root.style.setProperty(`--${key}`, value);
  root.style.setProperty("--font-display", `${fontStack(theme.fonts.display)}, serif`);
  root.style.setProperty("--font-body", `${fontStack(theme.fonts.body)}, system-ui, sans-serif`);
  root.style.colorScheme = theme.dark ? "dark" : "light";
  root.dataset.dark = String(theme.dark);
  root.classList.toggle("no-capitals", !appearance.capitals);
  root.classList.toggle("no-watermark", !appearance.watermark);
  root.classList.toggle("no-rules", !appearance.rules);
  root.classList.toggle("no-blackletter", !appearance.blackletter);
}

/** CSS font-family for a font name. The bundled Inter registers itself as "Inter Variable". */
function fontStack(name: string): string {
  if (name === "Inter") return `"Inter Variable", "Inter"`;
  if (name === "system-ui") return "system-ui";
  return `"${name}"`;
}

// ---- Colour maths -----------------------------------------------------------------------------
// Contrast follows WCAG: compare the relative luminance of two colours. To fix a low-contrast
// accent we change only its lightness in OKLCH (a colour space where lightness steps look even),
// so it keeps its hue.

type Rgb = [number, number, number];

function hexToRgb(hex: string): Rgb {
  const h = hex.replace("#", "");
  const full = h.length === 3 ? h.split("").map((c) => c + c).join("") : h;
  const n = parseInt(full, 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => v / 255) as Rgb;
}

function rgbToHex([r, g, b]: Rgb): string {
  const to = (v: number) => Math.round(Math.min(1, Math.max(0, v)) * 255).toString(16).padStart(2, "0");
  return `#${to(r)}${to(g)}${to(b)}`;
}

const toLinear = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const fromLinear = (c: number) => (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);

function luminance(hex: string): number {
  const [r, g, b] = hexToRgb(hex).map(toLinear);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/** Near-white or near-black, whichever reads better on `bg`. */
export function bestTextOn(bg: string): string {
  return contrast(bg, "#ffffff") >= contrast(bg, "#1a1412") ? "#ffffff" : "#1a1412";
}

function toOklch(hex: string): [number, number, number] {
  const [r, g, b] = hexToRgb(hex).map(toLinear);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  return [L, Math.hypot(A, B), Math.atan2(B, A)];
}

function fromOklch([L, C, h]: [number, number, number]): string {
  const A = C * Math.cos(h);
  const B = C * Math.sin(h);
  const l = (L + 0.3963377774 * A + 0.2158037573 * B) ** 3;
  const m = (L - 0.1055613458 * A - 0.0638541728 * B) ** 3;
  const s = (L - 0.0894841775 * A - 1.291485548 * B) ** 3;
  const r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
  const g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
  const b = -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s;
  return rgbToHex([fromLinear(r), fromLinear(g), fromLinear(b)]);
}

/** `color`, lightened or darkened only as much as needed to reach `ratio` against `bg`. */
export function ensureContrast(color: string, bg: string, ratio: number): string {
  if (contrast(color, bg) >= ratio) return color;
  const [L, C, h] = toOklch(color);
  const direction = luminance(bg) > 0.5 ? -1 : 1; // darken on light pages, lighten on dark
  for (let step = 1; step <= 40; step++) {
    const candidate = fromOklch([Math.min(1, Math.max(0, L + direction * step * 0.015)), C, h]);
    if (contrast(candidate, bg) >= ratio) return candidate;
  }
  return direction < 0 ? "#000000" : "#ffffff";
}

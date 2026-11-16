// `npm run tauri …` runs this instead of the Tauri CLI directly (decision 0041).
//
// On Linux, `tauri build` makes an AppImage, which must carry GStreamer's plugins or WebKit
// can't play any video. Tauri's bundler copies them (`bundleMediaFramework` in
// tauri.linux.conf.json) from the folder named by GSTREAMER_PLUGINS_DIR, or else every plugin
// installed on this computer, FFmpeg and the patented codecs of plugins-ugly included. So for a
// build this fills a temporary folder with only the plugins below and points the bundler at it.
// Every other command (dev, icon, …) and every other OS goes straight to the Tauri CLI.
//
// Node rather than a shell script so `npm run tauri` keeps working on Windows.
import { spawn, spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdtempSync, rmSync } from "node:fs";
import { createRequire } from "node:module";
import { arch, tmpdir } from "node:os";
import { join } from "node:path";

// Without these no video plays: the pipeline's basics, WebM, VP8/VP9, OpenGL and an audio sink
// (WebKit insists on one even though our videos are muted).
// An array lists alternatives: the first one installed is used.
const REQUIRED = [
  // GStreamer core and plugins-base
  "coreelements", "app", "playback", "typefindfunctions", "pbtypes",
  // GStreamer 1.22 merged videoconvert and videoscale into videoconvertscale; Ubuntu 22.04
  // (what `npm run package` builds on, decision 0046) has 1.20, with the two separate ones.
  ["videoconvertscale", ["videoconvert", "videoscale"]], "videorate", "audioconvert", "audioresample", "volume",
  // gstreamer1.0-gl
  "opengl",
  // plugins-good
  "matroska", "vpx", "autodetect",
];
// Nice to have: AV1, Opus/Vorbis audio tracks, the parsers WebKit's MediaSource may plug, the
// usual audio outputs, and a few video filters WebKit uses for odd formats.
const OPTIONAL = [
  // plugins-base
  "audiomixer", "audiorate", "compositor", "overlaycomposition", "gio", "opus", "vorbis", "ogg",
  // gstreamer1.0-alsa
  "alsa",
  // plugins-good
  "pulseaudio", "pipewire", "audioparsers", "audiofx", "videofilter", "deinterlace", "interleave",
  "alpha", "alphacolor", "videocrop", "videobox", "imagefreeze",
  // plugins-bad, only ones without patent trouble
  "videoparsersbad", "aom", "codecalpha",
];

const args = process.argv.slice(2);
const env = { ...process.env };
let pluginsDir = null;

if (process.platform === "linux" && args[0] === "build" && bundlesAppImage(args)) {
  pluginsDir = choosePlugins();
  env.GSTREAMER_PLUGINS_DIR = pluginsDir;
}

const cli = createRequire(import.meta.url).resolve("@tauri-apps/cli/tauri.js");
const child = spawn(process.execPath, [cli, ...args], { stdio: "inherit", env });
// Ctrl+C reaches the CLI too (same terminal); wait for it to finish rather than dying first.
process.on("SIGINT", () => {});
child.on("exit", (code, signal) => {
  if (pluginsDir) rmSync(pluginsDir, { recursive: true, force: true });
  process.exit(code ?? (signal ? 1 : 0));
});

/** Whether this build makes an AppImage: not with `--no-bundle`, nor when `--bundles` / `-b`
 *  lists only other formats (e.g. `-b deb`). */
function bundlesAppImage(args) {
  if (args.includes("--no-bundle")) return false;
  const listed = [];
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a.startsWith("--bundles=")) listed.push(...a.slice("--bundles=".length).split(","));
    else if (a === "--bundles" || a === "-b") {
      // Values follow until the next flag; each may itself be comma-separated.
      while (args[i + 1] && !args[i + 1].startsWith("-")) listed.push(...args[++i].split(","));
    }
  }
  return listed.length === 0 || listed.includes("appimage");
}

/** Copies the chosen plugins into a temporary folder; exits if a required one is missing. */
function choosePlugins() {
  // The bundler's GStreamer step rewrites each plugin's library search path with patchelf, and
  // without it fails late with only "failed to run linuxdeploy".
  if (spawnSync("patchelf", ["--version"], { stdio: "ignore" }).error) {
    fail("patchelf is needed to bundle GStreamer into the AppImage: sudo apt install patchelf");
  }
  const multiarch = { x64: "x86_64", arm64: "aarch64" }[arch()] ?? arch();
  const src = [`/usr/lib/${multiarch}-linux-gnu/gstreamer-1.0`, "/usr/lib/gstreamer-1.0"].find(existsSync);
  if (!src) fail("GStreamer's plugin folder wasn't found (is gstreamer1.0-plugins-base installed?)");

  const has = (p) => existsSync(join(src, `libgst${p}.so`));
  // Each alternative is one plugin name or a group of names that are only usable together.
  const installed = (alt) => (Array.isArray(alt) ? alt.every(has) : has(alt));
  const chosen = [];
  const missing = [];
  for (const entry of REQUIRED) {
    const alts = Array.isArray(entry) ? entry : [entry];
    const found = alts.find(installed);
    if (found) chosen.push(...[found].flat());
    else missing.push(alts.map((a) => [a].flat().join("+")).join(" or "));
  }
  if (missing.length) {
    fail(`these GStreamer plugins aren't installed (in ${src}): ${missing.join(", ")}`);
  }
  const dir = mkdtempSync(join(tmpdir(), "comilito-gst-"));
  for (const p of [...chosen, ...OPTIONAL]) {
    const file = join(src, `libgst${p}.so`);
    if (existsSync(file)) copyFileSync(file, join(dir, `libgst${p}.so`));
    else console.warn(`note: optional GStreamer plugin '${p}' isn't installed; leaving it out`);
  }
  return dir;
}

function fail(message) {
  console.error(`error: ${message}`);
  process.exit(1);
}

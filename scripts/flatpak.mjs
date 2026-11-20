// `npm run flatpak`: builds the Flatpak (decisions 0059, 0061). Linux only.
//
//   npm run flatpak                    build this folder (the local manifest) and make a .flatpak file
//   npm run flatpak -- --install       …and install it for your user (`flatpak run me.colorcrow.comilito`)
//   npm run flatpak -- --flathub       build the Flathub manifest instead (the tagged release, offline)
//   npm run flatpak -- sources         regenerate cargo-sources.json and node-sources.json
//   npm run flatpak -- uninstall       remove the installed Flatpak (your user's); its data stays
//   npm run flatpak -- uninstall --delete-data   …and its data too (~/.var/app/me.colorcrow.comilito)
//
// `npm run package` runs the first one after the AppImage. Building needs Flathub's
// flatpak-builder (`flatpak install --user flathub org.flatpak.Builder`); the runtime, the SDK
// and its extensions are installed from Flathub the first time, by flatpak-builder itself.
// `sources` needs flatpak-builder-tools, in ~/src/flatpak-builder-tools or wherever
// FLATPAK_BUILDER_TOOLS points (docs/flatpak.md).
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const APP_ID = "me.colorcrow.comilito";
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const dir = join(root, "flatpak");

// Run directly (not imported by package.mjs or release.mjs).
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  process.exit(main(process.argv.slice(2)));
}

/** Picks the command from the arguments. Anything it doesn't know stops with the usage instead
 *  of falling through to a build, which takes minutes. */
function main(args) {
  const usage =
    "usage: npm run flatpak -- [--install] [--flathub]\n" +
    "       npm run flatpak -- sources\n" +
    "       npm run flatpak -- uninstall [--delete-data]";
  // The command is the one word without dashes; `--uninstall` and `--sources` work too.
  const words = args.filter((a) => !a.startsWith("--"));
  const flags = args.filter((a) => a.startsWith("--"));
  let command = words[0] ?? "build";
  for (const c of ["sources", "uninstall"]) {
    if (flags.includes(`--${c}`)) command = c;
  }
  const allowed = {
    build: ["--install", "--flathub"],
    sources: ["--sources"],
    uninstall: ["--uninstall", "--delete-data"],
  }[command];
  const unknown = [...words.slice(1), ...flags.filter((f) => !allowed?.includes(f))];
  if (!allowed || unknown.length > 0) {
    console.error(`error: don't know ${!allowed ? `"${command}"` : unknown.join(" ")}\n${usage}`);
    return 1;
  }
  if (command === "sources") return generateSources();
  if (command === "uninstall") return uninstall({ deleteData: flags.includes("--delete-data") });
  return build({ flathub: flags.includes("--flathub"), install: flags.includes("--install") });
}

/** Builds a manifest into flatpak/repo and exports it as a single .flatpak file. Returns an exit code. */
export function build({ flathub = false, install = false } = {}) {
  if (process.platform !== "linux") {
    console.error("error: Flatpaks are only built on Linux.");
    return 1;
  }
  const builder = findBuilder();
  if (!builder) {
    console.error(
      "error: flatpak-builder isn't installed. Install Flathub's with\n" +
        "  flatpak install --user flathub org.flatpak.Builder",
    );
    return 1;
  }
  const manifest = join(dir, flathub ? `${APP_ID}.yml` : `${APP_ID}.local.yml`);
  const repo = join(dir, "repo");
  const code = run(builder[0], [
    ...builder.slice(1),
    "--user",
    // The runtime, SDK and extensions the manifest names, from Flathub, if they're missing.
    "--install-deps-from=flathub",
    "--force-clean",
    // Kept between builds, so only what changed is rebuilt.
    `--state-dir=${join(dir, ".flatpak-builder")}`,
    `--repo=${repo}`,
    ...(install ? ["--install"] : []),
    join(dir, "build"),
    manifest,
  ]);
  if (code !== 0) return code;

  // One file to hand to others: `flatpak install comilito.flatpak`. The runtime comes from
  // Flathub, which the file names, so it stays small.
  const out = join(root, "src-tauri", "target", "release", "bundle", "flatpak");
  mkdirSync(out, { recursive: true });
  const file = join(out, `Comilito_${appVersion()}_${flatpakArch()}.flatpak`);
  const bundled = run("flatpak", [
    "build-bundle",
    "--runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo",
    repo,
    file,
    APP_ID,
  ]);
  if (bundled !== 0) return bundled;
  console.log(`\nFlatpak: ${file}${install ? `\nInstalled: flatpak run ${APP_ID}` : ""}`);
  return 0;
}

/** Removes the Flatpak `--install` (or `flatpak install`) put in the user's installation. Its
 *  data (settings, backups, downloaded tools) stays for a later install, unless `deleteData`.
 *  Your projects are never touched: they're in your own folders. Returns an exit code. */
export function uninstall({ deleteData = false } = {}) {
  const installed = spawnSync("flatpak", ["info", "--user", APP_ID], { stdio: "ignore" });
  if (installed.error) {
    console.error("error: Flatpak isn't installed.");
    return 1;
  }
  if (installed.status !== 0) {
    console.log(`${APP_ID} isn't installed for your user; nothing to do.`);
    return 0;
  }
  const code = run("flatpak", ["uninstall", "--user", "--noninteractive", ...(deleteData ? ["--delete-data"] : []), APP_ID]);
  if (code === 0) {
    console.log(
      deleteData
        ? `Uninstalled ${APP_ID} and deleted its data.`
        : `Uninstalled ${APP_ID}. Its data is kept in ~/.var/app/${APP_ID}/ (--delete-data removes it).`,
    );
  }
  return code;
}

/** Regenerates the crate and npm package lists the Flathub manifest builds from. */
export function generateSources() {
  const tools = process.env.FLATPAK_BUILDER_TOOLS ?? join(homedir(), "src", "flatpak-builder-tools");
  const python = join(tools, "venv", "bin", "python");
  const cargoGenerator = join(tools, "cargo", "flatpak-cargo-generator.py");
  const nodeGenerator = join(tools, "venv", "bin", "flatpak-node-generator");
  for (const needed of [python, cargoGenerator, nodeGenerator]) {
    if (!existsSync(needed)) {
      console.error(
        `error: ${needed} is missing. Set up flatpak-builder-tools as docs/flatpak.md shows, or\n` +
          "point FLATPAK_BUILDER_TOOLS at where they are.",
      );
      return 1;
    }
  }
  let code = run(python, [cargoGenerator, join(root, "src-tauri", "Cargo.lock"), "-o", join(dir, "cargo-sources.json")]);
  if (code !== 0) return code;
  code = run(nodeGenerator, ["npm", join(root, "package-lock.json"), "-o", join(dir, "node-sources.json")]);
  return code;
}

/** Flathub's flatpak-builder when installed (it's what Flathub builds with), else one on PATH. */
function findBuilder() {
  const flatpakApp = spawnSync("flatpak", ["info", "org.flatpak.Builder"], { stdio: "ignore" });
  if (flatpakApp.status === 0) {
    // It runs in its own sandbox, so it's given this folder.
    return ["flatpak", "run", `--filesystem=${root}`, "org.flatpak.Builder"];
  }
  if (spawnSync("flatpak-builder", ["--version"], { stdio: "ignore" }).status === 0) {
    return ["flatpak-builder"];
  }
  return null;
}

function appVersion() {
  return JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8")).version;
}

/** Flatpak's name for this machine's architecture. */
function flatpakArch() {
  const r = spawnSync("flatpak", ["--default-arch"], { encoding: "utf8" });
  return r.stdout?.trim() || "x86_64";
}

function run(cmd, cmdArgs) {
  const r = spawnSync(cmd, cmdArgs, { stdio: "inherit", cwd: root });
  if (r.error) {
    console.error(`error: couldn't run ${cmd}: ${r.error.message}`);
    return 1;
  }
  return r.status ?? 1;
}

// `npm run flatpak`: builds the Flatpak (decisions 0059, 0061). Linux only.
//
//   npm run flatpak                    build this folder (the local manifest) and make a .flatpak file
//   npm run flatpak -- --install       …and install it for your user (`flatpak run me.colorcrow.comilito`)
//   npm run flatpak -- --flathub       build the Flathub manifest instead (the tagged release, offline)
//   npm run flatpak -- sources         regenerate cargo-sources.json and node-sources.json
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
  const args = process.argv.slice(2);
  if (args[0] === "sources") {
    process.exit(generateSources());
  }
  process.exit(build({ flathub: args.includes("--flathub"), install: args.includes("--install") }));
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

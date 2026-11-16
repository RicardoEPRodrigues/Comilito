// `npm run package`: builds Comilito's release packages (decision 0046).
//
// On Linux it makes an AppImage. glibc can't be bundled, and a program only runs where glibc is
// at least as new as the one it was built against, so when Docker is available the build runs
// in an Ubuntu 22.04 container (scripts/package/Dockerfile) and the AppImage runs on 22.04 and
// anything newer. Without Docker it builds on this computer, and the AppImage then needs this
// computer's glibc or newer; a warning says so.
//
//   npm run package            Docker if it's running, otherwise this computer
//   npm run package -- --local always this computer
//   npm run package -- --docker always Docker (fails if it isn't running)
//
// On other OSes it runs `tauri build` (Windows: the installers from tauri.conf.json).
import { spawnSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const forceLocal = args.includes("--local");
const forceDocker = args.includes("--docker");

if (process.platform !== "linux") {
  process.exit(tauriBuild());
}

if (!forceLocal && dockerRunning()) {
  process.exit(buildInDocker());
}
if (forceDocker) {
  fail("Docker isn't available (is it installed and running? `docker info` should work)");
}
if (!forceLocal) {
  console.warn("note: Docker isn't running, so building on this computer.");
}
console.warn(
  `warning: this AppImage needs glibc ${hostGlibc()} or newer to run. Start Docker and run ` +
    "`npm run package` again to build one that runs on Ubuntu 22.04 and newer.",
);
process.exit(tauriBuild());

function buildInDocker() {
  const image = "comilito-package";
  // Cached after the first time, so this is quick unless the Dockerfile changed.
  let code = run("docker", ["build", "-t", image, join(root, "scripts", "package")]);
  if (code !== 0) return code;

  // Same place a local build puts it, so the README can name one path.
  const out = join(root, "src-tauri", "target", "release", "bundle", "appimage");
  mkdirSync(out, { recursive: true });

  code = run("docker", [
    "run", "--rm",
    // The project read-only: the container copies it and builds the copy.
    "-v", `${root}:/src:ro`,
    "-v", `${out}:/out`,
    // Volumes that survive between runs, so a rebuild only compiles what changed.
    "-v", "comilito-package-cargo:/usr/local/cargo/registry",
    "-v", "comilito-package-target:/target",
    "-v", "comilito-package-node-modules:/work/node_modules",
    "-v", "comilito-package-cache:/root/.cache",
    "-e", "CARGO_TARGET_DIR=/target",
    // So the AppImage it copies out belongs to you, not root.
    "-e", `HOST_UID=${process.getuid()}`,
    "-e", `HOST_GID=${process.getgid()}`,
    image, "comilito-build",
  ]);
  if (code === 0) console.log(`\nAppImage (runs on glibc 2.35+, e.g. Ubuntu 22.04): ${out}`);
  return code;
}

// Through scripts/tauri.mjs, which picks the GStreamer plugins (decision 0041). Called with
// Node itself rather than `npm` so it works on Windows, where npm is a .cmd file.
function tauriBuild() {
  return run(process.execPath, [join(root, "scripts", "tauri.mjs"), "build"]);
}

function dockerRunning() {
  return spawnSync("docker", ["info"], { stdio: "ignore" }).status === 0;
}

/** This computer's glibc version, e.g. "2.39". */
function hostGlibc() {
  const r = spawnSync("getconf", ["GNU_LIBC_VERSION"], { encoding: "utf8" });
  return r.stdout?.trim().split(" ")[1] ?? "(this computer's)";
}

function run(cmd, cmdArgs) {
  const r = spawnSync(cmd, cmdArgs, { stdio: "inherit", cwd: root });
  if (r.error) fail(`couldn't run ${cmd}: ${r.error.message}`);
  return r.status ?? 1;
}

function fail(message) {
  console.error(`error: ${message}`);
  process.exit(1);
}

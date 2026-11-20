// `npm run release -- v1.1.0`: makes a release (decision 0061). It
//
//   1. sets the version in package.json (and its lock file), src-tauri/Cargo.toml (and
//      Cargo.lock) and tauri.conf.json, adds a <release> to the metainfo, and starts the
//      version's section in CHANGELOG.md (everything under Unreleased goes into it);
//   2. commits that ("Release v1.1.0") and tags the commit v1.1.0;
//   3. regenerates the Flathub sources (cargo-sources.json, node-sources.json), points the
//      Flathub manifest at the tag and its commit, and commits that too.
//
// It never pushes: check the two commits, then `git push origin main v1.1.0`, and copy the
// Flathub manifest and the two source files to the Flathub repository (docs/flatpak.md).
//
//   npm run release -- v1.1.0 --dry-run    says what it would do, changes nothing
import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { APP_ID, generateSources } from "./flatpak.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const input = args.find((a) => !a.startsWith("--"));

const match = /^v?(\d+\.\d+\.\d+)$/.exec(input ?? "");
if (!match) fail("give the version to release, like: npm run release -- v1.1.0");
const version = match[1];
const tag = `v${version}`;
const today = new Date().toISOString().slice(0, 10);

const files = {
  cargoToml: "src-tauri/Cargo.toml",
  cargoLock: "src-tauri/Cargo.lock",
  tauriConf: "src-tauri/tauri.conf.json",
  changelog: "CHANGELOG.md",
  metainfo: `flatpak/${APP_ID}.metainfo.xml`,
  flathubManifest: `flatpak/${APP_ID}.yml`,
  cargoSources: "flatpak/cargo-sources.json",
  nodeSources: "flatpak/node-sources.json",
};

// ---- Checks first, so a release never stops halfway ----
const current = JSON.parse(read(files.tauriConf)).version;
if (git(["tag", "--list", tag]).trim()) fail(`the tag ${tag} already exists`);
if (!isNewer(version, current)) fail(`${version} isn't newer than the current version, ${current}`);
// Only tracked files: the release commits exactly the files it changes, so untracked ones are
// left alone, but a file with changes of its own would end up half in the release.
const dirty = git(["status", "--porcelain", "--untracked-files=no"]).trim();
if (dirty) fail(`commit or stash your changes first:\n${dirty}`);
// Flathub builds from the tag, so everything its manifest installs must be in git by then.
for (const f of [`flatpak/${APP_ID}.desktop`, files.metainfo, files.flathubManifest]) {
  if (!git(["ls-files", f]).trim()) fail(`${f} isn't committed; Flathub builds from the tag, so it must be`);
}

console.log(`Releasing ${current} → ${version} (${tag}, ${today})${dryRun ? " — dry run, nothing changes" : ""}`);

// ---- 1. The version, everywhere it's written ----
step("package.json and package-lock.json", () => {
  // npm keeps both files' formatting; no tag or commit of its own.
  runOrFail("npm", ["version", version, "--no-git-tag-version", "--allow-same-version"]);
});
step(files.cargoToml, () => edit(files.cargoToml, /^version = "[^"]*"/m, `version = "${version}"`));
step(files.cargoLock, () =>
  edit(files.cargoLock, /(\[\[package\]\]\nname = "comilito"\nversion = )"[^"]*"/, `$1"${version}"`),
);
step(files.tauriConf, () => edit(files.tauriConf, /"version": "[^"]*"/, `"version": "${version}"`));
step(`${files.metainfo} (a <release> for ${version})`, () =>
  edit(
    files.metainfo,
    /<releases>\n/,
    `<releases>\n    <release version="${version}" date="${today}">\n` +
      `      <url type="details">https://github.com/RicardoEPRodrigues/Comilito/blob/${tag}/CHANGELOG.md</url>\n` +
      `    </release>\n`,
  ),
);
step(`${files.changelog} (Unreleased becomes ${version})`, () => {
  let text = read(files.changelog);
  // The note that only made sense before the first release.
  text = text.replace(/\nNothing has been released yet;[^\n]*\n/, "");
  if (!text.includes("\n## Unreleased\n")) fail(`${files.changelog} has no "## Unreleased" heading`);
  // Everything that was under Unreleased now sits under the new version's heading.
  text = text.replace("\n## Unreleased\n", `\n## Unreleased\n\n## [${version}] - ${today}\n`);
  write(files.changelog, text);
});

// ---- 2. Commit and tag ----
step(`commit "Release ${tag}" and tag it ${tag}`, () => {
  runOrFail("git", ["add", "package.json", "package-lock.json", files.cargoToml, files.cargoLock, files.tauriConf, files.metainfo, files.changelog]);
  runOrFail("git", ["commit", "-m", `Release ${tag}`]);
  runOrFail("git", ["tag", "-a", tag, "-m", `Comilito ${version}`]);
});

// ---- 3. The Flathub manifest follows the tag ----
step("regenerate cargo-sources.json and node-sources.json", () => {
  if (generateSources() !== 0) fail(`the release is tagged, but the sources couldn't be generated; fix that, then run\n  npm run flatpak -- sources\nand update ${files.flathubManifest} by hand`);
});
step(`point ${files.flathubManifest} at ${tag} and commit it`, () => {
  const commit = git(["rev-list", "-n", "1", tag]).trim();
  edit(files.flathubManifest, /^(\s*)tag: .*$/m, `$1tag: ${tag}`);
  edit(files.flathubManifest, /^(\s*)commit: .*$/m, `$1commit: '${commit}'`);
  runOrFail("git", ["add", files.flathubManifest, files.cargoSources, files.nodeSources]);
  runOrFail("git", ["commit", "-m", `Point the Flathub manifest at ${tag}`]);
});

console.log(
  dryRun
    ? "\nDry run: nothing was changed."
    : `\nReleased ${tag}. Check the two commits (git log -2 -p), then:\n` +
        `  git push origin HEAD ${tag}\n` +
        `and copy ${files.flathubManifest}, ${files.cargoSources} and ${files.nodeSources} to the Flathub repository.`,
);

// ---- Helpers ----

/** Runs one step, or only names it in a dry run. */
function step(name, action) {
  console.log(`• ${name}`);
  if (!dryRun) action();
}

/** Replaces the first match of `pattern` in a file; fails if there's none, so nothing is missed. */
function edit(file, pattern, replacement) {
  const text = read(file);
  if (!pattern.test(text)) fail(`couldn't find ${pattern} in ${file}`);
  write(file, text.replace(pattern, replacement));
}

/** Whether version a (x.y.z) comes after b. */
function isNewer(a, b) {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    if (pa[i] !== pb[i]) return pa[i] > pb[i];
  }
  return false;
}

function read(file) {
  return readFileSync(join(root, file), "utf8");
}

function write(file, text) {
  writeFileSync(join(root, file), text);
}

function git(gitArgs) {
  const r = spawnSync("git", gitArgs, { cwd: root, encoding: "utf8" });
  if (r.status !== 0) fail(`git ${gitArgs.join(" ")} failed: ${r.stderr}`);
  return r.stdout;
}

function runOrFail(cmd, cmdArgs) {
  // `shell` on Windows, where npm is a .cmd file.
  const r = spawnSync(cmd, cmdArgs, { cwd: root, stdio: "inherit", shell: process.platform === "win32" });
  if (r.status !== 0) fail(`${cmd} ${cmdArgs.join(" ")} failed`);
}

function fail(message) {
  console.error(`error: ${message}`);
  process.exit(1);
}

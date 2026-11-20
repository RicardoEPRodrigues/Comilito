// `npm run release -- v1.1.0`: makes a release, or builds this computer's package for one
// (decision 0061). Run it on Linux and on Windows: each builds its own package.
//
// A new version (no tag v1.1.0 yet):
//   1. sets the version in package.json (and its lock file), src-tauri/Cargo.toml (and
//      Cargo.lock) and tauri.conf.json, adds a <release> to the metainfo, and starts the
//      version's section in CHANGELOG.md (everything under Unreleased goes into it);
//   2. builds this computer's packages: the AppImage and the Flatpak on Linux, the MSI on
//      Windows. If that fails, the version changes are undone and nothing is committed;
//   3. commits ("Release v1.1.0") and tags the commit v1.1.0;
//   4. on Linux: regenerates the Flathub sources (cargo-sources.json, node-sources.json), points
//      the Flathub manifest at the tag and its commit, and commits that.
//
// A released version (the tag exists): a tag never moves, so nothing is re-tagged. If this
// working copy is the tagged one (apart from the Flathub manifest and sources, which are
// updated after tagging), it builds this computer's package, and on Linux points the Flathub
// manifest at the tag if it doesn't yet. So: release on one OS, then run the same command on
// the other to build its package.
//
// It never pushes: check the commits, then `git push origin HEAD v1.1.0`, and put the packages
// on the GitHub release with `npm run publish-release -- v1.1.0` (decision 0063).
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
const linux = process.platform === "linux";
const windows = process.platform === "win32";

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
// What a release commit changes (step 1), so a failed build can put it back.
const versionFiles = ["package.json", "package-lock.json", files.cargoToml, files.cargoLock, files.tauriConf, files.metainfo, files.changelog];
// Updated after tagging, so they may differ from the tag.
const flathubFiles = [files.flathubManifest, files.cargoSources, files.nodeSources];

// ---- Checks first, so a release never stops halfway ----
const released = git(["tag", "--list", tag]).trim() !== "";
// Only tracked files: the release commits exactly the files it changes, so untracked ones are
// left alone, but a file with changes of its own would end up half in the release.
const dirty = git(["status", "--porcelain", "--untracked-files=no"]).trim();
if (dirty) fail(`commit or stash your changes first:\n${dirty}`);

if (released) {
  // The package must be built from exactly what was tagged.
  const exclude = flathubFiles.map((f) => `:(exclude)${f}`);
  const differs = spawnSync("git", ["diff", "--quiet", tag, "HEAD", "--", ".", ...exclude], { cwd: root }).status !== 0;
  if (differs) {
    fail(`${tag} is already released, and this working copy isn't it. To build its package, check it out first:\n  git checkout ${tag}`);
  }
  console.log(`${tag} is already released: building this computer's package for it${dryRun ? " — dry run, nothing changes" : ""}`);
} else {
  const current = JSON.parse(read(files.tauriConf)).version;
  if (!isNewer(version, current)) fail(`${version} isn't newer than the current version, ${current}`);
  // Flathub builds from the tag, so everything its manifest installs must be in git by then.
  for (const f of [`flatpak/${APP_ID}.desktop`, files.metainfo, files.flathubManifest]) {
    if (!git(["ls-files", f]).trim()) fail(`${f} isn't committed; Flathub builds from the tag, so it must be`);
  }
  console.log(`Releasing ${current} → ${version} (${tag}, ${today})${dryRun ? " — dry run, nothing changes" : ""}`);
  setVersion();
}

step(`build the ${packageName()}`, () => {
  if (buildPackage() === 0) return;
  if (!released) {
    // Nothing is committed yet: put the version files back as they were.
    runOrFail("git", ["checkout", "--", ...versionFiles]);
    fail(`the ${packageName()} didn't build, so the version changes were undone. Fix it and run this again.`);
  }
  fail(`the ${packageName()} didn't build`);
});

if (!released) {
  step(`commit "Release ${tag}" and tag it ${tag}`, () => {
    runOrFail("git", ["add", ...versionFiles]);
    runOrFail("git", ["commit", "-m", `Release ${tag}`]);
    runOrFail("git", ["tag", "-a", tag, "-m", `Comilito ${version}`]);
  });
}

// Whether the Flathub manifest ends up naming this tag (Linux only).
const flathubReady = linux && pointFlathubAtTag();

console.log(dryRun ? "\nDry run: nothing was changed." : `\n${summary()}`);

// ---- The steps ----

/** Step 1: the version, everywhere it's written. */
function setVersion() {
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
}

/** This computer's package, through the same scripts as `npm run package`. Returns an exit code. */
function buildPackage() {
  const node = process.execPath;
  // On Linux the AppImage (in the Ubuntu 22.04 container when Docker runs, decision 0046) and
  // the Flatpak, both published on the GitHub release (decision 0063). On Windows, the MSI.
  return run(node, [join(root, "scripts", "package.mjs")]);
}

function packageName() {
  return linux ? "AppImage and Flatpak" : windows ? "MSI installer" : "packages";
}

/** Step 4 (Linux): the Flathub manifest names the tag and its commit, with fresh sources.
 *  Returns whether it does now. */
function pointFlathubAtTag() {
  const commit = released || !dryRun ? git(["rev-list", "-n", "1", tag]).trim() : "(the new commit)";
  const manifest = read(files.flathubManifest);
  if (released && manifest.includes(`tag: ${tag}\n`) && manifest.includes(`commit: '${commit}'`)) {
    console.log(`• the Flathub manifest already points at ${tag}`);
    return true;
  }
  // A commit made on a checked-out tag (a detached HEAD) belongs to no branch and gets lost.
  if (spawnSync("git", ["symbolic-ref", "-q", "HEAD"], { cwd: root }).status !== 0) {
    console.warn(`warning: not on a branch, so the Flathub manifest wasn't pointed at ${tag}. Switch to your branch and run this again.`);
    return false;
  }
  step("regenerate cargo-sources.json and node-sources.json", () => {
    if (generateSources() !== 0) {
      fail(`${tag} is tagged, but the sources couldn't be generated. Fix that and run this again.`);
    }
  });
  step(`point ${files.flathubManifest} at ${tag} and commit it`, () => {
    edit(files.flathubManifest, /^(\s*)tag: .*$/m, `$1tag: ${tag}`);
    edit(files.flathubManifest, /^(\s*)commit: .*$/m, `$1commit: '${commit}'`);
    runOrFail("git", ["add", ...flathubFiles]);
    runOrFail("git", ["commit", "-m", `Point the Flathub manifest at ${tag}`]);
  });
  return true;
}

function summary() {
  const out = linux
    ? "src-tauri/target/release/bundle/appimage/ and …/flatpak/"
    : "src-tauri/target/release/bundle/msi/";
  const lines = [`${released ? "Built" : "Released"} ${tag}. The ${packageName()}: ${out}`];
  if (!released) lines.push(`Check the commits (git log -p), then: git push origin HEAD ${tag}`);
  lines.push(`Then put the ${packageName()} on the GitHub release: npm run publish-release -- ${tag}`);
  if (flathubReady) {
    lines.push(`For Flathub, copy ${flathubFiles.join(", ")} to the Flathub repository.`);
  } else if (!linux) {
    lines.push(`On Linux, run \`npm run release -- ${tag}\` too: it builds the AppImage and the Flatpak.`);
  }
  return lines.join("\n");
}

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

function run(cmd, cmdArgs) {
  // npm is a .cmd file on Windows, which only a shell runs. Nothing else goes through the shell:
  // it would split paths with spaces (Node's own, in C:\Program Files).
  const r = spawnSync(cmd, cmdArgs, { cwd: root, stdio: "inherit", shell: windows && cmd === "npm" });
  return r.status ?? 1;
}

function runOrFail(cmd, cmdArgs) {
  if (run(cmd, cmdArgs) !== 0) fail(`${cmd} ${cmdArgs.join(" ")} failed`);
}

function fail(message) {
  console.error(`error: ${message}`);
  process.exit(1);
}

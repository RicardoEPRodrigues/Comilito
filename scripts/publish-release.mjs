// `npm run publish-release -- v1.1.0`: puts this computer's packages for a version on its
// GitHub release (decision 0063), with the GitHub CLI (`gh`). Run it on Linux (the AppImage) and
// on Windows (the MSI), after `npm run release` built them and the tag is pushed. The Flatpak
// isn't published any more (decision 0064); --flatpak uploads it too, if one was built.
//
// The first run creates the release, with the version's CHANGELOG section as its notes; later
// runs (the other OS, or a rebuilt package) add their files to it, replacing any of the same name.
//
// The files are uploaded without the version in their names (Comilito_amd64.AppImage, not
// Comilito_1.1.0_amd64.AppImage), so the website can link to
// https://github.com/<owner>/<repo>/releases/latest/download/<name>, which always gives the
// newest release's file.
//
//   npm run publish-release -- v1.1.0 --dry-run    says what it would do, changes nothing
//   npm run publish-release -- v1.1.0 --flatpak    the Flatpak too
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const bundle = join(root, "src-tauri", "target", "release", "bundle");
const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const withFlatpak = args.includes("--flatpak");
const input = args.find((a) => !a.startsWith("--"));

const match = /^v?(\d+\.\d+\.\d+)$/.exec(input ?? "");
if (!match) fail("give the version to publish, like: npm run publish-release -- v1.1.0");
const version = match[1];
const tag = `v${version}`;

// ---- Checks first ----
if (spawnSync("gh", ["--version"], { stdio: "ignore" }).error) {
  fail("the GitHub CLI (gh) isn't installed: https://cli.github.com");
}
if (spawnSync("gh", ["auth", "status"], { stdio: "ignore" }).status !== 0) {
  fail("the GitHub CLI isn't logged in: run `gh auth login`");
}
// `gh release create` makes the tag from the default branch when GitHub doesn't have it, which
// would release the wrong code. So the tag must be pushed, and be the same as here.
const localTag = capture("git", ["rev-parse", "--verify", "--quiet", `refs/tags/${tag}^{commit}`]);
if (!localTag) fail(`there's no tag ${tag}; make the release first: npm run release -- ${tag}`);
const remote = capture("git", ["ls-remote", "--tags", "origin", `refs/tags/${tag}^{}`, `refs/tags/${tag}`]);
// An annotated tag is listed twice: the tag object, and (with ^{}) the commit it points at.
const remoteCommit = (remote.split("\n").find((l) => l.endsWith("^{}")) ?? remote.split("\n")[0] ?? "").split("\t")[0];
if (!remoteCommit) fail(`${tag} isn't on GitHub yet: git push origin HEAD ${tag}`);
if (remoteCommit !== localTag) fail(`${tag} on GitHub isn't the same commit as here (${remoteCommit} vs ${localTag})`);

const packages = findPackages();
if (packages.length === 0) {
  fail(`no packages for ${version} in ${bundle}; build them first: npm run release -- ${tag}`);
}

// ---- Upload ----
const exists = spawnSync("gh", ["release", "view", tag], { cwd: root, stdio: "ignore" }).status === 0;
console.log(`${exists ? "Adding to" : "Creating"} the GitHub release ${tag}${dryRun ? " — dry run, nothing changes" : ""}`);
for (const p of packages) console.log(`• ${p.from} → ${p.name}`);
if (dryRun) process.exit(0);

// Copies with the names to upload (gh uploads a file under its own name).
const staging = join(bundle, "github");
rmSync(staging, { recursive: true, force: true });
mkdirSync(staging, { recursive: true });
const uploads = packages.map((p) => {
  const to = join(staging, p.name);
  copyFileSync(p.path, to);
  return to;
});

let code;
if (exists) {
  code = run("gh", ["release", "upload", tag, "--clobber", ...uploads]);
} else {
  const notes = join(staging, "notes.md");
  writeFileSync(notes, releaseNotes());
  code = run("gh", ["release", "create", tag, "--title", `Comilito ${version}`, "--notes-file", notes, ...uploads]);
}
if (code !== 0) fail("gh failed (see above)");
console.log(`\nPublished: https://github.com/${repoName()}/releases/tag/${tag}`);

// ---- Helpers ----

/** This version's packages in the bundle folders, each with its version-free upload name. */
function findPackages() {
  const kinds = [
    { folder: "appimage", ext: ".AppImage" },
    ...(withFlatpak ? [{ folder: "flatpak", ext: ".flatpak" }] : []),
    { folder: "msi", ext: ".msi" },
  ];
  const found = [];
  for (const { folder, ext } of kinds) {
    const dir = join(bundle, folder);
    if (!existsSync(dir)) continue;
    for (const file of readdirSync(dir)) {
      // Tauri and scripts/flatpak.mjs name them Comilito_<version>_<arch>[_<lang>].<ext>.
      if (file.startsWith(`Comilito_${version}_`) && file.endsWith(ext)) {
        found.push({
          path: join(dir, file),
          from: `${folder}/${file}`,
          name: file.replace(`_${version}_`, "_"),
        });
      }
    }
  }
  return found;
}

/** The version's section of CHANGELOG.md, with a word on which file to pick. */
function releaseNotes() {
  const changelog = readFileSync(join(root, "CHANGELOG.md"), "utf8");
  const start = changelog.indexOf(`\n## [${version}]`);
  let section = "";
  if (start >= 0) {
    const body = changelog.slice(changelog.indexOf("\n", start + 1) + 1);
    const end = body.search(/^## /m);
    section = (end >= 0 ? body.slice(0, end) : body).trim();
  }
  return [
    "## Downloads",
    "",
    "- **Linux, AppImage** (`Comilito_amd64.AppImage`): make it executable and run it. Ubuntu 22.04 and newer.",
    ...(withFlatpak
      ? [
          "- **Linux, Flatpak** (`Comilito_x86_64.flatpak`): `flatpak install --user Comilito_x86_64.flatpak`. " +
            "It doesn't update itself: install a newer file over it.",
        ]
      : []),
    "- **Windows** (`Comilito_x64_en-US.msi`): the installer, for Windows 10 and 11.",
    "",
    section ? `## What changed\n\n${section}` : "",
  ].join("\n");
}

function repoName() {
  return capture("gh", ["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"]) || "RicardoEPRodrigues/Comilito";
}

/** A command's output, trimmed, or "" if it failed. */
function capture(cmd, cmdArgs) {
  const r = spawnSync(cmd, cmdArgs, { cwd: root, encoding: "utf8" });
  return r.status === 0 ? r.stdout.trim() : "";
}

function run(cmd, cmdArgs) {
  const r = spawnSync(cmd, cmdArgs, { cwd: root, stdio: "inherit" });
  return r.status ?? 1;
}

function fail(message) {
  console.error(`error: ${message}`);
  process.exit(1);
}

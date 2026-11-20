#!/usr/bin/env python3
"""Builds the website into build/website/ (decision 0060).

website/ holds the hand-written pages. This adds what comes from the rest of the repository, so
the site never falls behind the app and nothing is kept twice in git:

- the screenshot (docs/images/), the skeletal horse woodcut (src/assets/) and the app icon;
- the extra themes (extras/themes/), packed with extras/themes/build.sh, each with its woodcut
  and display font for the preview, and themes.json describing them for site.js.

Run it, then preview with:  python3 -m http.server -d build/website
(the page fetches themes.json, which browsers refuse to do from a file:// page).
"""

import json
import re
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "build" / "website"
THEMES = ROOT / "extras" / "themes"

# Fan themes based on trademarks we don't own: kept in the repository for personal use, never
# published (see extras/themes/README.md). Anything not listed here goes on the website.
NOT_ON_WEBSITE = {}


def theme_descriptions() -> dict[str, str]:
    """The "Look" column of the table in extras/themes/README.md, by theme name, so the
    descriptions are written once."""
    descriptions = {}
    for line in (THEMES / "README.md").read_text(encoding="utf-8").splitlines():
        # | **Horde** (dark) | charred iron, ... | New Rocker | a tribal war sigil ... |
        match = re.match(r"\|\s*\*\*(.+?)\*\*[^|]*\|([^|]+)\|", line)
        if match:
            descriptions[match.group(1)] = match.group(2).strip()
    return descriptions


def build_themes(out: Path) -> list[dict]:
    subprocess.run([str(THEMES / "build.sh")], check=True)
    descriptions = theme_descriptions()
    entries = []
    for folder in sorted(p for p in THEMES.iterdir() if (p / "theme.json").is_file()):
        slug = folder.name
        if slug in NOT_ON_WEBSITE:
            continue
        theme = json.loads((folder / "theme.json").read_text(encoding="utf-8"))
        name = theme["name"]
        target = out / slug
        target.mkdir(parents=True)

        # The download keeps a URL without spaces; the page's download attribute restores the name.
        package = THEMES / "dist" / f"{name}.comilito-theme"
        shutil.copy(package, out / f"{slug}.comilito-theme")

        # Only what the preview card needs: the woodcut and the display font.
        watermark = None
        if theme.get("watermark"):
            shutil.copy(folder / theme["watermark"], target / Path(theme["watermark"]).name)
            watermark = f"themes/{slug}/{Path(theme['watermark']).name}"
        display = theme["fonts"]["display"]
        font_file = None
        if display in theme.get("fontFiles", {}):
            source = folder / theme["fontFiles"][display]
            shutil.copy(source, target / source.name)
            font_file = f"themes/{slug}/{source.name}"

        colors = dict(theme["colors"])
        # A token set to "accent" follows the accent in use; on the website that's the theme's own.
        for key, value in colors.items():
            if value == "accent":
                colors[key] = colors["accent"]

        entries.append({
            "slug": slug,
            "name": name,
            "dark": theme["dark"],
            "description": descriptions.get(name, ""),
            "colors": colors,
            "displayFont": display,
            "fontFile": font_file,
            "watermark": watermark,
            "fixedAccent": bool(theme.get("fixedAccent")),
            "followsAccent": "accent" in theme["colors"].values(),
            "download": f"themes/{slug}.comilito-theme",
            "fileName": package.name,
            "size": package.stat().st_size,
        })
    return entries


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    shutil.copytree(ROOT / "website", OUT)

    assets = OUT / "assets"
    assets.mkdir()
    shutil.copy(ROOT / "docs" / "images" / "screenshot.png", assets / "screenshot.png")
    shutil.copy(ROOT / "src" / "assets" / "skeletal-horse.svg", assets / "skeletal-horse.svg")
    shutil.copy(ROOT / "src-tauri" / "icons" / "128x128@2x.png", assets / "icon.png")

    themes = OUT / "themes"
    themes.mkdir()
    entries = build_themes(themes)
    (OUT / "themes.json").write_text(json.dumps(entries, indent=2), encoding="utf-8")

    print(f"built {OUT.relative_to(ROOT)} with {len(entries)} themes: {', '.join(e['name'] for e in entries)}")


if __name__ == "__main__":
    main()

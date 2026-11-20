// Fills the Themes section from themes.json, which scripts/build-website.py writes from
// extras/themes/. Each card is a small sketch of the app in that theme, so people can see a theme
// before downloading it.

const list = document.getElementById("theme-list");

/** Near white or near black, whichever reads better on `hex` (as bestTextOn in src/lib/themes.ts). */
function textOn(hex) {
  const channel = (i) => {
    const c = parseInt(hex.slice(1 + i * 2, 3 + i * 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  const luminance = 0.2126 * channel(0) + 0.7152 * channel(1) + 0.0722 * channel(2);
  return luminance > 0.4 ? "#1a1412" : "#ffffff";
}

function formatSize(bytes) {
  return bytes < 1024 * 1024 ? `${Math.max(1, Math.round(bytes / 1024))} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

/** Creates an element with a class and optional text (text, never HTML). */
function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text) node.textContent = text;
  return node;
}

/** The font family the miniature uses: the theme's own file, or one the site already loads. */
async function themeFont(theme) {
  if (!theme.fontFile) return `"${theme.displayFont}", serif`;
  const family = `Theme ${theme.slug}`;
  try {
    const face = new FontFace(family, `url("${theme.fontFile}")`);
    document.fonts.add(await face.load());
  } catch {
    // The card still works in the fallback font.
  }
  return `"${family}", serif`;
}

function miniature(theme, font) {
  const mini = el("div", "mini");
  mini.setAttribute("aria-hidden", "true");
  for (const [key, value] of Object.entries(theme.colors)) mini.style.setProperty(`--t-${key}`, value);
  mini.style.setProperty("--t-accent-text", textOn(theme.colors.accent));
  mini.style.setProperty("--t-font", font);
  // Painted in gold on a dark page, the woodcut needs a little more opacity to show (Watermark.svelte).
  mini.style.setProperty("--t-woodcut-opacity", theme.dark ? "0.16" : "0.35");
  mini.style.setProperty("--t-woodcut", `url("${theme.watermark ?? "assets/skeletal-horse.svg"}")`);

  mini.append(el("div", "mini-header", "Curse of Strahd"));

  const body = el("div", "mini-body");
  const side = el("div", "mini-side");
  side.append(el("span"), el("span"), el("span"));
  const main = el("div", "mini-main");
  main.append(el("div", "mini-woodcut"), el("div", "mini-title", "The Tapestry"));
  for (const [name, live] of [["Combat", true], ["Rainy night", false]]) {
    const row = el("div", live ? "mini-row live" : "mini-row");
    row.append(el("b", "", name[0]), el("span", "", name));
    main.append(row);
  }
  body.append(side, main);

  const mixer = el("div", "mini-mixer");
  mixer.append(el("i"), el("span"), el("span"));
  mini.append(body, mixer);
  return mini;
}

async function themeCard(theme) {
  const card = el("article", "theme");
  card.append(miniature(theme, await themeFont(theme)));

  const info = el("div", "theme-info");
  info.append(el("h3", "", theme.name));
  if (theme.description) info.append(el("p", "muted", theme.description[0].toUpperCase() + theme.description.slice(1) + "."));

  const tags = el("div", "tags");
  tags.append(el("span", "", theme.dark ? "Dark" : "Light"));
  if (theme.fixedAccent) tags.append(el("span", "", "Keeps its own accent"));
  else if (theme.followsAccent) tags.append(el("span", "", "Follows your accent"));
  info.append(tags);

  const download = el("a", "button primary", `Download · ${formatSize(theme.size)}`);
  download.href = theme.download;
  download.download = theme.fileName;
  info.append(download);

  card.append(info);
  return card;
}

async function showThemes() {
  try {
    const response = await fetch("themes.json");
    if (!response.ok) throw new Error(response.statusText);
    const themes = await response.json();
    const cards = await Promise.all(themes.map(themeCard));
    list.replaceChildren(...cards);
  } catch {
    list.replaceChildren(el("p", "muted", "The themes couldn't be loaded. They're also in the extras/themes folder on GitHub."));
  }
}

showThemes();

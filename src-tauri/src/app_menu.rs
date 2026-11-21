//! Puts the AppImage in the desktop's app menu (decision 0065): a launcher (`.desktop` file)
//! and icons in the user's data folder, the way installed apps have them. An AppImage is just
//! a file, so without this it never shows up in the menu or the dock with its icon.
//!
//! Nothing here changes a file on its own: the UI asks first, every time (adding, updating or
//! removing). `status` only looks, so the UI knows whether to ask.
//!
//! Linux only, and only when running as an AppImage: the AppImage's runtime sets `APPIMAGE`
//! (the file's path) and `APPDIR` (where its contents are mounted).

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::Serialize;

use crate::app_id::APP_ID;

/// The window's class on X11 (the binary's name); on Wayland the window's app ID is `APP_ID`,
/// which is why the launcher is named after it. Docks match windows to launchers by these.
const WM_CLASS: &str = "comilito";
/// The name Tauri gives the icons inside the AppImage (`usr/share/icons/hicolor/<size>/apps/`).
const BUNDLED_ICON: &str = "comilito.png";

/// Where this AppImage is now, once `add` moved it to ~/Applications: `APPIMAGE` still names
/// the old path, which the menu entry mustn't point at. `Mutex` makes the shared static safe
/// to change from any thread (a `static` in Rust can't be mutated otherwise).
static MOVED_TO: Mutex<Option<PathBuf>> = Mutex::new(None);

/// What the menu holds for this AppImage. The UI asks about everything but `Current`,
/// `Elsewhere` and `Unavailable`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MenuState {
    /// Not running as an AppImage (a dev build, the Flatpak, Windows): nothing to do.
    Unavailable,
    /// No entry starts this AppImage.
    Missing,
    /// Our entry starts this AppImage and is as this version would write it.
    Current,
    /// Another tool's entry starts this AppImage (Gear Lever, AppImageLauncher): left alone.
    Elsewhere,
    /// Our entry starts a file that isn't there any more (the AppImage was moved or renamed).
    Moved,
    /// Our entry starts another copy of Comilito that still exists.
    OtherCopy,
    /// Our entry starts this AppImage, but this version writes it, or its icons, differently.
    Changed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMenuStatus {
    pub state: MenuState,
    /// This AppImage.
    pub appimage: Option<String>,
    /// What our entry starts, for `Moved` and `OtherCopy`.
    pub points_at: Option<String>,
    /// The other tool's launcher, for `Elsewhere`.
    pub other_entry: Option<String>,
    /// ~/Applications, where the AppImage can be moved first.
    pub applications_dir: String,
    /// Whether the AppImage is already in ~/Applications.
    pub in_applications_dir: bool,
    /// A different file with the AppImage's name is in ~/Applications already; moving there
    /// puts it in the trash first.
    pub replaces_in_applications_dir: bool,
}

/// Every path involved, so tests can point them at a scratch folder.
struct Places {
    /// The running AppImage, if it is one.
    appimage: Option<PathBuf>,
    /// The AppImage's icons (`$APPDIR/usr/share/icons/hicolor`).
    bundled_icons: Option<PathBuf>,
    /// `$XDG_DATA_HOME/applications`: launchers the menu lists.
    apps_dir: PathBuf,
    /// `$XDG_DATA_HOME/icons/hicolor`.
    icons_dir: PathBuf,
    /// ~/Applications.
    applications_dir: PathBuf,
    /// The home folder, shown as `~` in the paths the UI displays.
    home: PathBuf,
}

impl Places {
    fn from_env() -> Option<Places> {
        if !cfg!(target_os = "linux") || crate::flatpak::in_flatpak() {
            return None;
        }
        let home = PathBuf::from(std::env::var_os("HOME")?);
        // The XDG spec: an unset or relative XDG_DATA_HOME means ~/.local/share.
        let data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".local/share"));
        let appimage = MOVED_TO
            .lock()
            .ok()
            .and_then(|m| m.clone())
            .or_else(|| std::env::var_os("APPIMAGE").map(PathBuf::from));
        Some(Places {
            appimage,
            bundled_icons: std::env::var_os("APPDIR").map(|d| PathBuf::from(d).join("usr/share/icons/hicolor")),
            apps_dir: data_home.join("applications"),
            icons_dir: data_home.join("icons/hicolor"),
            applications_dir: home.join("Applications"),
            home,
        })
    }

    fn entry(&self) -> PathBuf {
        self.apps_dir.join(format!("{APP_ID}.desktop"))
    }

    /// A path for people to read: `~/Applications/…` rather than `/home/name/Applications/…`.
    fn show(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home) {
            Ok(rest) => Path::new("~").join(rest).display().to_string(),
            Err(_) => path.display().to_string(),
        }
    }
}

// ---- Commands -------------------------------------------------------------------------------

/// What the menu holds for this AppImage. Only looks; changes nothing.
#[tauri::command(async)]
pub fn app_menu_status() -> AppMenuStatus {
    match Places::from_env() {
        Some(p) => status(&p),
        None => unavailable(),
    }
}

/// Adds (or updates) the menu entry for this AppImage, after the user said yes. With
/// `move_to_applications`, first moves the AppImage to ~/Applications (any other file of that
/// name there goes to the trash), so the entry doesn't break when Downloads is cleaned.
#[tauri::command(async)]
pub fn add_to_app_menu(move_to_applications: bool) -> Result<AppMenuStatus, String> {
    let mut p = Places::from_env().ok_or("Comilito isn't running as an AppImage")?;
    if move_to_applications {
        let to = move_appimage(&p)?;
        if let Ok(mut m) = MOVED_TO.lock() {
            *m = Some(to.clone());
        }
        p.appimage = Some(to);
    }
    add(&p)?;
    Ok(status(&p))
}

/// Removes our menu entry and icons, after the user said yes. Another tool's entry is left alone.
#[tauri::command(async)]
pub fn remove_from_app_menu() -> Result<AppMenuStatus, String> {
    let p = Places::from_env().ok_or("Comilito isn't running as an AppImage")?;
    remove(&p)?;
    Ok(status(&p))
}

// ---- Looking --------------------------------------------------------------------------------

fn unavailable() -> AppMenuStatus {
    AppMenuStatus {
        state: MenuState::Unavailable,
        appimage: None,
        points_at: None,
        other_entry: None,
        applications_dir: String::new(),
        in_applications_dir: false,
        replaces_in_applications_dir: false,
    }
}

fn status(p: &Places) -> AppMenuStatus {
    let Some(appimage) = &p.appimage else {
        return unavailable();
    };
    let in_applications_dir = appimage.parent().is_some_and(|d| same_file(d, &p.applications_dir));
    let target = appimage.file_name().map(|n| p.applications_dir.join(n));
    let mut s = AppMenuStatus {
        state: MenuState::Missing,
        appimage: Some(p.show(appimage)),
        points_at: None,
        other_entry: None,
        applications_dir: p.show(&p.applications_dir),
        in_applications_dir,
        replaces_in_applications_dir: !in_applications_dir && target.is_some_and(|t| t.exists()),
    };

    // Our own entry first.
    if let Ok(text) = fs::read_to_string(p.entry()) {
        match exec_program(&text) {
            Some(prog) if same_file(&prog, appimage) => {
                s.state = if entry_is_current(p, appimage, &text) { MenuState::Current } else { MenuState::Changed };
            }
            Some(prog) => {
                s.state = if prog.exists() { MenuState::OtherCopy } else { MenuState::Moved };
                s.points_at = Some(p.show(&prog));
            }
            // Unreadable: rewriting it is what fixes it.
            None => s.state = MenuState::Changed,
        }
        return s;
    }

    // Then anyone else's: a launcher that starts this very file.
    if let Ok(dir) = fs::read_dir(&p.apps_dir) {
        for file in dir.flatten() {
            let path = file.path();
            if path.extension().is_none_or(|e| e != "desktop") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else { continue };
            if exec_program(&text).is_some_and(|prog| same_file(&prog, appimage)) {
                s.state = MenuState::Elsewhere;
                s.other_entry = Some(p.show(&path));
                return s;
            }
        }
    }
    s
}

/// Whether the entry and the icons are exactly what `add` would write now.
fn entry_is_current(p: &Places, appimage: &Path, text: &str) -> bool {
    if desktop_entry(appimage).ok().as_deref() != Some(text) {
        return false;
    }
    icons(p).iter().all(|(from, to)| match (fs::read(from), fs::read(to)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    })
}

/// Two paths name the same file (through symlinks or `..`), or are equal if they can't be resolved.
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

// ---- Changing -------------------------------------------------------------------------------

fn add(p: &Places) -> Result<(), String> {
    let appimage = p.appimage.as_ref().ok_or("Comilito isn't running as an AppImage")?;
    let text = desktop_entry(appimage)?;
    for (from, to) in icons(p) {
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("couldn't create {}: {e}", dir.display()))?;
        }
        fs::copy(&from, &to).map_err(|e| format!("couldn't copy the icon to {}: {e}", to.display()))?;
    }
    fs::create_dir_all(&p.apps_dir).map_err(|e| format!("couldn't create {}: {e}", p.apps_dir.display()))?;
    // Written whole and then renamed, so the menu never reads half a file.
    let entry = p.entry();
    let tmp = entry.with_extension("desktop.tmp");
    fs::write(&tmp, text).map_err(|e| format!("couldn't write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, &entry).map_err(|e| format!("couldn't write {}: {e}", entry.display()))?;
    Ok(())
}

fn remove(p: &Places) -> Result<(), String> {
    let entry = p.entry();
    if entry.exists() {
        fs::remove_file(&entry).map_err(|e| format!("couldn't remove {}: {e}", entry.display()))?;
    }
    // Ours alone: they're named after the app ID.
    if let Ok(sizes) = fs::read_dir(&p.icons_dir) {
        for size in sizes.flatten() {
            let icon = size.path().join("apps").join(format!("{APP_ID}.png"));
            if icon.exists() {
                fs::remove_file(&icon).map_err(|e| format!("couldn't remove {}: {e}", icon.display()))?;
            }
        }
    }
    Ok(())
}

/// Moves the AppImage into ~/Applications and returns where it is now.
fn move_appimage(p: &Places) -> Result<PathBuf, String> {
    let from = p.appimage.as_ref().ok_or("Comilito isn't running as an AppImage")?;
    let name = from.file_name().ok_or("the AppImage has no file name")?;
    let to = p.applications_dir.join(name);
    if same_file(from, &to) {
        return Ok(to);
    }
    fs::create_dir_all(&p.applications_dir).map_err(|e| format!("couldn't create {}: {e}", p.applications_dir.display()))?;
    if to.exists() {
        // Most likely an older Comilito, downloaded under the same name. The trash, not a
        // deletion, in case it's something else.
        crate::trash_bin::delete(&to)?;
    }
    // A running AppImage can be moved: its contents are mounted from the open file, which
    // stays open whatever its name. `rename` can't cross drives, so then copy and delete.
    if fs::rename(from, &to).is_err() {
        fs::copy(from, &to).map_err(|e| format!("couldn't copy the AppImage to {}: {e}", to.display()))?;
        fs::remove_file(from).map_err(|e| format!("copied the AppImage to {}, but couldn't remove {}: {e}", to.display(), from.display()))?;
    }
    Ok(to)
}

/// Pairs of (icon in the AppImage, where it goes), one per size the AppImage has.
fn icons(p: &Places) -> Vec<(PathBuf, PathBuf)> {
    let Some(bundled) = &p.bundled_icons else { return Vec::new() };
    let Ok(sizes) = fs::read_dir(bundled) else { return Vec::new() };
    let mut pairs = Vec::new();
    for size in sizes.flatten() {
        let name = size.file_name();
        let name = name.to_string_lossy();
        // "256x256@2" is the same picture as 512x512 for HiDPI screens; the menu doesn't need it.
        if name.contains('@') {
            continue;
        }
        let from = size.path().join("apps").join(BUNDLED_ICON);
        if from.is_file() {
            pairs.push((from, p.icons_dir.join(&*name).join("apps").join(format!("{APP_ID}.png"))));
        }
    }
    pairs.sort();
    pairs
}

// ---- The .desktop file (the freedesktop Desktop Entry spec) ---------------------------------

/// The launcher for `appimage`. Its text is compared byte for byte to tell whether it's
/// current, so a change here asks every user to update theirs; that's intended.
fn desktop_entry(appimage: &Path) -> Result<String, String> {
    let path = appimage.to_str().ok_or("the AppImage's path isn't valid UTF-8")?;
    if path.chars().any(|c| c.is_control()) {
        return Err("the AppImage's path has control characters".into());
    }
    Ok(format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Comilito\n\
         GenericName=Soundboard\n\
         Comment=Play music, ambience and sound effects at the gaming table\n\
         Exec={exec}\n\
         TryExec={try_exec}\n\
         Icon={APP_ID}\n\
         Terminal=false\n\
         Categories=AudioVideo;Audio;Mixer;\n\
         Keywords=soundboard;mixer;ambience;music;sound effects;RPG;tabletop;D&D;dungeons;game master;\n\
         StartupNotify=true\n\
         StartupWMClass={WM_CLASS}\n",
        exec = escape_value(&quote_exec_arg(path)),
        // TryExec hides the entry if the AppImage is deleted, instead of a launcher that fails.
        try_exec = escape_value(path),
    ))
}

/// Quotes one argument for `Exec`: in double quotes, with `"`, `` ` ``, `$` and `\` escaped,
/// and `%` doubled (a lone `%` starts a field code like `%U`).
fn quote_exec_arg(arg: &str) -> String {
    let mut out = String::from("\"");
    for c in arg.chars() {
        match c {
            '"' | '`' | '$' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            '%' => out.push_str("%%"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A string value's own escaping, applied after `Exec` quoting: `\` becomes `\\`.
fn escape_value(s: &str) -> String {
    s.replace('\\', "\\\\")
}

/// The program an entry's `Exec` starts (skipping `env VAR=value`), or `None` if there's no
/// `Exec` in its `[Desktop Entry]` group or it can't be read.
fn exec_program(text: &str) -> Option<PathBuf> {
    let mut in_main_group = false;
    for line in text.lines() {
        let line = line.trim_start();
        if line.starts_with('[') {
            in_main_group = line.trim_end() == "[Desktop Entry]";
            continue;
        }
        if !in_main_group {
            continue;
        }
        let Some(value) = line.strip_prefix("Exec").map(str::trim_start).and_then(|r| r.strip_prefix('=')) else {
            continue;
        };
        let args = split_exec(&unescape_value(value.trim()))?;
        let mut args = args.into_iter();
        let mut first = args.next()?;
        if first == "env" {
            first = args.find(|a| !a.contains('='))?;
        }
        return Some(PathBuf::from(first));
    }
    None
}

/// Undoes a string value's escaping: `\\`, `\s`, `\n`, `\t`, `\r`.
fn unescape_value(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// Splits an `Exec` value into arguments, undoing the quoting. Field codes (`%U`) are dropped
/// and `%%` becomes `%`. `None` if a quote isn't closed.
fn split_exec(s: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut chars = s.chars().peekable();
    loop {
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        let Some(&first) = chars.peek() else { break };
        let mut arg = String::new();
        if first == '"' {
            chars.next();
            loop {
                match chars.next()? {
                    '"' => break,
                    '\\' => arg.push(chars.next()?),
                    c => arg.push(c),
                }
            }
        } else {
            while let Some(c) = chars.next_if(|c| !c.is_whitespace()) {
                arg.push(c);
            }
        }
        if arg.starts_with('%') && arg.len() == 2 && first != '"' {
            continue; // a field code
        }
        args.push(arg.replace("%%", "%"));
    }
    Some(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::TempDir;

    /// A fake AppImage and its icons in a scratch folder.
    fn places(root: &Path, appimage: &str) -> Places {
        let appimage = root.join(appimage);
        fs::create_dir_all(appimage.parent().unwrap()).unwrap();
        fs::write(&appimage, b"appimage").unwrap();
        let bundled = root.join("appdir/usr/share/icons/hicolor");
        for size in ["32x32", "256x256", "256x256@2"] {
            fs::create_dir_all(bundled.join(size).join("apps")).unwrap();
            fs::write(bundled.join(size).join("apps").join(BUNDLED_ICON), size.as_bytes()).unwrap();
        }
        Places {
            appimage: Some(appimage),
            bundled_icons: Some(bundled),
            apps_dir: root.join("data/applications"),
            icons_dir: root.join("data/icons/hicolor"),
            applications_dir: root.join("home/Applications"),
            home: root.join("home"),
        }
    }

    #[test]
    fn quoting_round_trips() {
        for path in ["/home/a/Comilito_amd64.AppImage", "/home/a b/C \"x\" $HOME `y` 100% \\z.AppImage"] {
            let text = format!("[Desktop Entry]\nExec={} %U\n", escape_value(&quote_exec_arg(path)));
            assert_eq!(exec_program(&text), Some(PathBuf::from(path)));
        }
    }

    #[test]
    fn reads_other_tools_entries() {
        assert_eq!(exec_program("[Desktop Entry]\nExec=/opt/C.AppImage %F\n"), Some("/opt/C.AppImage".into()));
        assert_eq!(exec_program("[Desktop Entry]\nExec=env A=1 B=2 \"/x y/C\"\n"), Some("/x y/C".into()));
        // An action's Exec isn't the app's.
        assert_eq!(exec_program("[Desktop Action new]\nExec=/other\n[Desktop Entry]\nName=C\n"), None);
        assert_eq!(exec_program("[Desktop Entry]\nExec=\"/never closed\n"), None);
    }

    #[test]
    fn add_update_remove() {
        let root = TempDir::new();
        let p = places(&root.0, "home/Downloads/Comilito_amd64.AppImage");
        assert_eq!(status(&p).state, MenuState::Missing);

        add(&p).unwrap();
        assert_eq!(status(&p).state, MenuState::Current);
        assert!(p.icons_dir.join("32x32/apps").join(format!("{APP_ID}.png")).is_file());
        assert!(!p.icons_dir.join("256x256@2").exists());

        // A new version with a different icon.
        fs::write(p.bundled_icons.as_ref().unwrap().join("32x32/apps").join(BUNDLED_ICON), b"new").unwrap();
        assert_eq!(status(&p).state, MenuState::Changed);
        add(&p).unwrap();
        assert_eq!(status(&p).state, MenuState::Current);

        remove(&p).unwrap();
        assert_eq!(status(&p).state, MenuState::Missing);
        assert!(!p.icons_dir.join("32x32/apps").join(format!("{APP_ID}.png")).exists());
    }

    #[test]
    fn moved_and_other_copies() {
        let root = TempDir::new();
        let old = places(&root.0, "home/Downloads/Comilito_amd64.AppImage");
        add(&old).unwrap();

        // Another copy, while the first still exists.
        let new = places(&root.0, "home/Downloads/Comilito_amd64(1).AppImage");
        let s = status(&new);
        assert_eq!(s.state, MenuState::OtherCopy);
        assert_eq!(s.points_at.as_deref(), Some("~/Downloads/Comilito_amd64.AppImage"));

        // The first one deleted: the entry points at nothing.
        fs::remove_file(old.appimage.as_ref().unwrap()).unwrap();
        assert_eq!(status(&new).state, MenuState::Moved);
    }

    #[test]
    fn another_tools_entry_counts() {
        let root = TempDir::new();
        let p = places(&root.0, "home/Applications/Comilito.AppImage");
        fs::create_dir_all(&p.apps_dir).unwrap();
        let exec = p.appimage.as_ref().unwrap().display().to_string();
        fs::write(p.apps_dir.join("appimagekit_123-Comilito.desktop"), format!("[Desktop Entry]\nExec={exec}\n")).unwrap();
        let s = status(&p);
        assert_eq!(s.state, MenuState::Elsewhere);
        assert!(s.in_applications_dir);
    }

    #[test]
    fn moves_into_applications() {
        let root = TempDir::new();
        let p = places(&root.0, "home/Downloads/Comilito_amd64.AppImage");
        assert!(!status(&p).replaces_in_applications_dir);
        let to = move_appimage(&p).unwrap();
        assert_eq!(to, p.applications_dir.join("Comilito_amd64.AppImage"));
        assert!(to.is_file());
        assert!(!p.appimage.as_ref().unwrap().exists());
    }
}

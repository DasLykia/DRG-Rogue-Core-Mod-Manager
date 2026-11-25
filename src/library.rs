//! The mod library: what is installed, where the game is, and moving .pak files in and out of it.
//! A mod is "on" when its files sit in <Game>\RogueCore\Content\Paks as .pak/.utoc/.ucas,
//! and "off" when they carry an extra ".disabled" suffix (Unreal only mounts *.pak).
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use crate::ui::t;

pub const STEAM_APP_ID: u32 = 2605790;
pub const GAME_DIR_NAME: &str = "Deep Rock Galactic RogueCore";
const MOD_EXTS: [&str; 4] = ["pak", "utoc", "ucas", "sig"];
const OFF: &str = ".disabled";

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct InstalledMod {
    pub key: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub nexus_id: Option<u32>,
    pub file_id: Option<u32>,
    pub thumb: String,
    pub category: String,
    /// File names inside the Paks folder, without the ".disabled" suffix.
    pub files: Vec<String>,
    pub enabled: bool,
    pub installed_at: u64,
    pub order: u32,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    pub game_dir: String,
    pub api_key: String,
    pub show_adult: bool,
    /// Mods were switched off for a vanilla launch; "Mit Mods starten" switches them back.
    pub vanilla_active: bool,
    /// "de" or "en".
    pub lang: String,
    /// Id of a theme in ui::THEMES.
    pub theme: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Library {
    pub mods: Vec<InstalledMod>,
    pub settings: Settings,
    #[serde(skip)]
    pub data_dir: PathBuf,
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn default_data_dir() -> PathBuf {
    std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("CoreModManager")
}

impl Library {
    pub fn load(data_dir: PathBuf) -> Library {
        let _ = fs::create_dir_all(&data_dir);
        let mut lib: Library = fs::read_to_string(data_dir.join("library.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        lib.data_dir = data_dir;
        if lib.settings.game_dir.is_empty() || !Path::new(&lib.settings.game_dir).exists() {
            if let Some(g) = detect_game_dir() {
                lib.settings.game_dir = g.to_string_lossy().into();
            }
        }
        lib
    }

    pub fn save(&self) {
        if let Ok(t) = serde_json::to_string_pretty(self) {
            let tmp = self.data_dir.join("library.json.tmp");
            if fs::write(&tmp, t).is_ok() {
                let _ = fs::rename(tmp, self.data_dir.join("library.json"));
            }
        }
    }

    pub fn paks_dir(&self) -> Option<PathBuf> {
        if self.settings.game_dir.is_empty() {
            return None;
        }
        let p = Path::new(&self.settings.game_dir).join("RogueCore").join("Content").join("Paks");
        p.is_dir().then_some(p)
    }

    /// UE4SS's loader DLLs next to the game's exe (on or switched off), when UE4SS is installed.
    /// UE4SS starts every UE4SS mod (Lua, C++ and the BP mod loader) through one of these.
    fn ue4ss_loaders(&self) -> Vec<PathBuf> {
        let win64 = Path::new(&self.settings.game_dir).join("RogueCore").join("Binaries").join("Win64");
        if self.settings.game_dir.is_empty() || !(win64.join("UE4SS.dll").is_file() || win64.join("ue4ss").join("UE4SS.dll").is_file()) {
            return Vec::new();
        }
        ["dwmapi.dll", "xinput1_3.dll"]
            .iter()
            .map(|n| win64.join(n))
            .filter(|p| p.is_file() || PathBuf::from(format!("{}{OFF}", p.display())).is_file())
            .collect()
    }

    /// Some(true): UE4SS mods run; Some(false): switched off for a vanilla launch; None: no UE4SS.
    pub fn ue4ss_state(&self) -> Option<bool> {
        let loaders = self.ue4ss_loaders();
        (!loaders.is_empty()).then(|| loaders.iter().any(|p| p.is_file()))
    }

    /// Switches all UE4SS mods on or off by renaming UE4SS's loader DLL.
    fn set_ue4ss(&self, on: bool) -> Result<(), String> {
        for p in self.ue4ss_loaders() {
            let off = PathBuf::from(format!("{}{OFF}", p.display()));
            let (from, to) = if on { (off, p) } else { (p, off) };
            if from.is_file() && !to.exists() {
                fs::rename(&from, &to).map_err(|e| crate::tf!("UE4SS konnte nicht umgeschaltet werden ({}): {} – läuft das Spiel noch?", "Couldn't switch UE4SS ({}): {} – is the game still running?", to.display(), e))?;
            }
        }
        Ok(())
    }

    pub fn store_dir(&self) -> PathBuf {
        self.data_dir.join("mods")
    }

    pub fn downloads_dir(&self) -> PathBuf {
        self.data_dir.join("downloads")
    }

    pub fn active_count(&self) -> usize {
        self.mods.iter().filter(|m| m.enabled).count()
    }

    /// Picks up .pak files already in the Paks folder that the library doesn't know yet,
    /// syncs on/off state from the disk and forgets mods whose files are gone.
    pub fn scan(&mut self) {
        let Some(paks) = self.paks_dir() else { return };
        let mut groups: std::collections::BTreeMap<String, (Vec<String>, bool)> = Default::default();
        if let Ok(rd) = fs::read_dir(&paks) {
            for e in rd.flatten() {
                if !e.path().is_file() {
                    continue;
                }
                let name = e.file_name().to_string_lossy().to_string();
                let (base, off) = match name.strip_suffix(OFF) { Some(b) => (b.to_string(), true), None => (name.clone(), false) };
                let Some((stem, ext)) = base.rsplit_once('.') else { continue };
                if !MOD_EXTS.contains(&ext.to_ascii_lowercase().as_str()) || is_base_game(stem) {
                    continue;
                }
                if self.mods.iter().any(|m| m.files.iter().any(|f| f.eq_ignore_ascii_case(&base))) {
                    continue;
                }
                let on = ext.eq_ignore_ascii_case("pak") && !off;
                let g = groups.entry(stem.to_string()).or_default();
                g.0.push(base.clone());
                g.1 |= on;
            }
        }
        for (stem, (files, on)) in groups {
            let order = self.next_order();
            self.mods.push(InstalledMod {
                key: format!("local-{}", stem.to_lowercase()),
                name: pretty_name(&stem),
                author: "Lokal gefunden".into(),
                version: String::new(),
                files,
                enabled: on,
                installed_at: now(),
                order,
                category: "Lokal".into(),
                ..Default::default()
            });
        }
        let vanilla = self.settings.vanilla_active;
        self.mods.retain(|m| m.files.iter().any(|f| paks.join(f).exists() || paks.join(format!("{f}{OFF}")).exists()));
        if !vanilla {
            for m in &mut self.mods {
                m.enabled = m.files.iter().any(|f| f.to_ascii_lowercase().ends_with(".pak") && paks.join(f).exists());
            }
        }
        self.save();
    }

    fn next_order(&self) -> u32 {
        self.mods.iter().map(|m| m.order + 1).max().unwrap_or(0)
    }

    /// Puts each file of a mod in the on or off state on disk.
    fn place(&self, m: &InstalledMod, on: bool) -> Result<(), String> {
        let paks = self.paks_dir().ok_or(t("Spielordner nicht gefunden", "Game folder not found"))?;
        for f in &m.files {
            let (a, b) = (paks.join(f), paks.join(format!("{f}{OFF}")));
            let (from, to) = if on { (b, a) } else { (a, b) };
            if from.exists() {
                fs::rename(&from, &to).map_err(|e| format!("{}: {e}", to.display()))?;
            }
        }
        Ok(())
    }

    pub fn set_enabled(&mut self, key: &str, on: bool) -> Result<(), String> {
        let i = self.mods.iter().position(|m| m.key == key).ok_or(t("Mod nicht gefunden", "Mod not found"))?;
        if !self.settings.vanilla_active {
            self.place(&self.mods[i], on)?;
        }
        self.mods[i].enabled = on;
        self.save();
        Ok(())
    }

    pub fn remove(&mut self, key: &str) -> Result<(), String> {
        let i = self.mods.iter().position(|m| m.key == key).ok_or(t("Mod nicht gefunden", "Mod not found"))?;
        if let Some(paks) = self.paks_dir() {
            for f in &self.mods[i].files {
                for p in [paks.join(f), paks.join(format!("{f}{OFF}"))] {
                    if p.exists() {
                        fs::remove_file(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                    }
                }
            }
        }
        let _ = fs::remove_dir_all(self.store_dir().join(&self.mods[i].key));
        self.mods.remove(i);
        self.save();
        Ok(())
    }

    /// Switches every mod off on disk without forgetting which ones were on.
    pub fn suspend_all(&mut self) -> Result<(), String> {
        for m in &self.mods {
            self.place(m, false)?;
        }
        self.set_ue4ss(false)?;
        self.settings.vanilla_active = true;
        self.save();
        Ok(())
    }

    /// Makes the disk match the on/off switches again.
    pub fn apply_all(&mut self) -> Result<(), String> {
        for m in &self.mods {
            self.place(m, m.enabled)?;
        }
        self.set_ue4ss(true)?;
        self.settings.vanilla_active = false;
        self.save();
        Ok(())
    }

    /// Installs a downloaded or picked file (.zip or a loose .pak/.utoc/.ucas) as a mod.
    /// `meta` carries the Nexus details; an older install of the same Nexus mod is replaced.
    pub fn install(&mut self, source: &Path, mut meta: InstalledMod) -> Result<String, String> {
        let paks = self.paks_dir().ok_or(t("Spielordner nicht gefunden. Lege ihn in den Einstellungen fest.", "Game folder not found. Set it in Settings."))?;
        if meta.key.is_empty() {
            let stem = source.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            meta.key = format!("local-{}", stem.to_lowercase());
            if meta.name.is_empty() {
                meta.name = pretty_name(&stem);
            }
        }
        let store = self.store_dir().join(&meta.key);
        let _ = fs::remove_dir_all(&store);
        fs::create_dir_all(&store).map_err(|e| e.to_string())?;
        let ext = source.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let mut files: Vec<PathBuf> = Vec::new();
        if ext == "zip" {
            files = extract_mod_files(source, &store)?;
        } else if MOD_EXTS.contains(&ext.as_str()) {
            // A loose .pak brings its .utoc/.ucas/.sig siblings along.
            let stem = source.file_stem().unwrap_or_default().to_string_lossy().to_string();
            for x in MOD_EXTS {
                let p = source.with_file_name(format!("{stem}.{x}"));
                if p.is_file() {
                    let d = store.join(p.file_name().unwrap());
                    fs::copy(&p, &d).map_err(|e| e.to_string())?;
                    files.push(d);
                }
            }
        } else {
            return Err(crate::tf!(".{}-Archive werden nicht unterstützt. Bitte als .zip oder .pak hinzufügen.", ".{} archives are not supported. Please add a .zip or .pak.", ext));
        }
        if !files.iter().any(|f| f.extension().map_or(false, |e| e.eq_ignore_ascii_case("pak"))) {
            let _ = fs::remove_dir_all(&store);
            return Err(t("Im Archiv wurde keine .pak-Datei gefunden.", "No .pak file was found in the archive.").into());
        }
        // Replace an older version of the same mod (or a loose copy of the same files).
        let names: Vec<String> = files.iter().map(|f| f.file_name().unwrap().to_string_lossy().to_string()).collect();
        let old: Vec<String> = self
            .mods
            .iter()
            .filter(|m| m.key == meta.key || (meta.nexus_id.is_some() && m.nexus_id == meta.nexus_id) || m.files.iter().any(|f| names.iter().any(|n| n.eq_ignore_ascii_case(f))))
            .map(|m| m.key.clone())
            .collect();
        let order = self.mods.iter().find(|m| old.contains(&m.key)).map(|m| m.order).unwrap_or_else(|| self.next_order());
        for k in &old {
            let keep_store = *k == meta.key;
            if let Some(i) = self.mods.iter().position(|m| &m.key == k) {
                for f in self.mods[i].files.clone() {
                    let _ = fs::remove_file(paks.join(&f));
                    let _ = fs::remove_file(paks.join(format!("{f}{OFF}")));
                }
                if !keep_store {
                    let _ = fs::remove_dir_all(self.store_dir().join(k));
                }
                self.mods.remove(i);
            }
        }
        let on = !self.settings.vanilla_active;
        for (f, n) in files.iter().zip(&names) {
            let dest = if on { paks.join(n) } else { paks.join(format!("{n}{OFF}")) };
            fs::copy(f, &dest).map_err(|e| format!("{}: {e}", dest.display()))?;
        }
        meta.files = names;
        meta.enabled = true;
        meta.installed_at = now();
        meta.order = order;
        let key = meta.key.clone();
        self.mods.push(meta);
        self.save();
        Ok(key)
    }
}

/// What an export file carries besides the mod files themselves.
#[derive(Serialize, Deserialize)]
struct ModPack {
    version: u32,
    mods: Vec<InstalledMod>,
}

/// A plain file or folder name: no path separators, not "." or "..".
fn plain_name(s: &str) -> bool {
    !s.is_empty() && s != "." && s != ".." && !s.contains(['/', '\\', ':'])
}

impl Library {
    /// Writes every mod (its list entry and its files) into one .zip that `import` reads back.
    pub fn export(&self, dest: &Path) -> Result<usize, String> {
        let paks = self.paks_dir();
        let mut z = zip::ZipWriter::new(fs::File::create(dest).map_err(|e| e.to_string())?);
        let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let pack = ModPack { version: 1, mods: self.mods.clone() };
        z.start_file("modpack.json", opt).map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut z, serde_json::to_string_pretty(&pack).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;
        for m in &self.mods {
            for f in &m.files {
                let mut candidates = vec![self.store_dir().join(&m.key).join(f)];
                if let Some(p) = &paks {
                    candidates.insert(0, p.join(format!("{f}{OFF}")));
                    candidates.insert(0, p.join(f));
                }
                if let Some(src) = candidates.into_iter().find(|p| p.is_file()) {
                    z.start_file(format!("files/{}/{}", m.key, f), opt).map_err(|e| e.to_string())?;
                    let mut file = fs::File::open(&src).map_err(|e| e.to_string())?;
                    std::io::copy(&mut file, &mut z).map_err(|e| e.to_string())?;
                }
            }
        }
        z.finish().map_err(|e| e.to_string())?;
        Ok(self.mods.len())
    }

    /// Installs every mod from an `export` file, keeping each one's details and on/off state.
    pub fn import(&mut self, src: &Path) -> Result<usize, String> {
        let mut z = zip::ZipArchive::new(fs::File::open(src).map_err(|e| e.to_string())?).map_err(|e| crate::tf!("Archiv kann nicht gelesen werden: {}", "Can't read the archive: {}", e))?;
        let pack: ModPack = {
            let mut s = String::new();
            z.by_name("modpack.json")
                .map_err(|_| t("Das ist keine Mod-Liste dieser App (modpack.json fehlt).", "This isn't a mod list from this app (modpack.json is missing)."))?
                .read_to_string(&mut s)
                .map_err(|e| e.to_string())?;
            serde_json::from_str(&s).map_err(|e| e.to_string())?
        };
        let tmp = self.data_dir.join("import");
        let _ = fs::remove_dir_all(&tmp);
        let mut n = 0;
        for m in pack.mods {
            // The file comes from elsewhere: only plain names may become paths.
            if !plain_name(&m.key) || !m.files.iter().all(|f| plain_name(f)) {
                continue;
            }
            let dir = tmp.join(&m.key);
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            for f in &m.files {
                if let Ok(mut entry) = z.by_name(&format!("files/{}/{}", m.key, f)) {
                    let mut out = fs::File::create(dir.join(f)).map_err(|e| e.to_string())?;
                    std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
                }
            }
            let Some(pak) = m.files.iter().find(|f| f.to_ascii_lowercase().ends_with(".pak")).map(|f| dir.join(f)).filter(|p| p.is_file()) else { continue };
            let enabled = m.enabled;
            let key = self.install(&pak, InstalledMod { files: Vec::new(), ..m })?;
            if !enabled {
                self.set_enabled(&key, false)?;
            }
            n += 1;
        }
        let _ = fs::remove_dir_all(&tmp);
        Ok(n)
    }

    /// The installed game build and when Steam last updated it, from the Steam app manifest.
    pub fn local_build(&self) -> Option<(String, u64)> {
        let steamapps = Path::new(&self.settings.game_dir).parent()?.parent()?;
        let text = fs::read_to_string(steamapps.join(format!("appmanifest_{STEAM_APP_ID}.acf"))).ok()?;
        let field = |k: &str| text.lines().map(str::trim).find(|l| l.starts_with(&format!("\"{k}\""))).and_then(|l| l.split('"').nth(3)).map(String::from);
        Some((field("buildid")?, field("LastUpdated").and_then(|v| v.parse().ok()).unwrap_or(0)))
    }
}

fn is_base_game(stem: &str) -> bool {
    let s = stem.to_ascii_lowercase();
    s.starts_with("roguecore-") || s.starts_with("pakchunk") || s == "global"
}

/// "BioBoostersChoice_V2.3_P" -> "Bio Boosters Choice V2.3"
pub fn pretty_name(stem: &str) -> String {
    let s = stem.strip_suffix("_P").or_else(|| stem.strip_suffix("_p")).unwrap_or(stem).replace(['_', '-'], " ");
    let mut out = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_ascii_lowercase();
        out.push(c);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Pulls every .pak/.utoc/.ucas/.sig out of a zip (flattened) into `dest`.
fn extract_mod_files(zip_path: &Path, dest: &Path) -> Result<Vec<PathBuf>, String> {
    let f = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| crate::tf!("Archiv kann nicht gelesen werden: {}", "Can't read the archive: {}", e))?;
    let mut out = Vec::new();
    for i in 0..z.len() {
        let mut entry = z.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        // enclosed_name rejects paths that try to leave the folder.
        let Some(name) = entry.enclosed_name().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())) else { continue };
        let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
        if !MOD_EXTS.contains(&ext.as_str()) {
            continue;
        }
        let p = dest.join(&name);
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        fs::write(&p, buf).map_err(|e| e.to_string())?;
        out.push(p);
    }
    Ok(out)
}

/// Finds the game through Steam's library list.
pub fn detect_game_dir() -> Option<PathBuf> {
    let mut steam_roots = vec![PathBuf::from(r"C:\Program Files (x86)\Steam"), PathBuf::from(r"C:\Program Files\Steam")];
    let mut libs = Vec::new();
    for root in steam_roots.drain(..) {
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(t) = fs::read_to_string(&vdf) {
            for line in t.lines() {
                let l = line.trim();
                if l.starts_with("\"path\"") {
                    if let Some(v) = l.split('"').nth(3) {
                        libs.push(PathBuf::from(v.replace("\\\\", "\\")));
                    }
                }
            }
        }
        libs.push(root);
    }
    libs.into_iter().map(|l| l.join("steamapps").join("common").join(GAME_DIR_NAME)).find(|p| p.join("RogueCore").is_dir())
}

/// The person's Downloads folder: Windows' own setting (it can be moved), else %USERPROFILE%\Downloads.
pub fn downloads_folder() -> PathBuf {
    let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
    #[cfg(windows)]
    {
        let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders";
        if let Ok(out) = std::process::Command::new("reg").args(["query", key, "/v", "{374DE290-123F-4565-9164-39C4925E467B}"]).output() {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(v) = text.lines().find_map(|l| l.split_once("REG_EXPAND_SZ").or_else(|| l.split_once("REG_SZ")).map(|(_, v)| v.trim().to_string())) {
                let p = PathBuf::from(v.replace("%USERPROFILE%", &home.to_string_lossy()));
                if p.is_dir() {
                    return p;
                }
            }
        }
    }
    home.join("Downloads")
}

pub fn open_url(url: &str) {
    #[cfg(windows)]
    let _ = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn();
    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

pub fn open_folder(p: &Path) {
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(p).spawn();
    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open").arg(p).spawn();
}

pub fn launch_game() {
    open_url(&format!("steam://rungameid/{STEAM_APP_ID}"));
}

/// Registers this program as the handler for nxm:// links (Nexus "Mod Manager Download") for the current user.
pub fn register_nxm_handler() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let cmd = format!("\"{}\" \"%1\"", exe.display());
    let run = |args: &[&str]| -> Result<(), String> {
        let st = std::process::Command::new("reg").args(args).output().map_err(|e| e.to_string())?;
        if st.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&st.stderr).into()) }
    };
    run(&["add", r"HKCU\Software\Classes\nxm", "/ve", "/d", "URL:NXM Protocol", "/f"])?;
    run(&["add", r"HKCU\Software\Classes\nxm", "/v", "URL Protocol", "/d", "", "/f"])?;
    run(&["add", r"HKCU\Software\Classes\nxm\shell\open\command", "/ve", "/d", &cmd, "/f"])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn install_toggle_remove() {
        let root = std::env::temp_dir().join(format!("cmm-test-{}", now()));
        let paks = root.join("game").join("RogueCore").join("Content").join("Paks");
        fs::create_dir_all(&paks).unwrap();
        fs::write(paks.join("RogueCore-Windows.pak"), b"base").unwrap();
        fs::write(paks.join("Loose_P.pak"), b"x").unwrap();
        let mut lib = Library::load(root.join("data"));
        lib.settings.game_dir = root.join("game").to_string_lossy().into();
        lib.scan();
        assert_eq!(lib.mods.len(), 1);
        assert_eq!(lib.mods[0].name, "Loose");
        let src = root.join("New_P.pak");
        fs::write(&src, b"y").unwrap();
        let k = lib.install(&src, InstalledMod::default()).unwrap();
        assert!(paks.join("New_P.pak").exists());
        lib.set_enabled(&k, false).unwrap();
        assert!(paks.join("New_P.pak.disabled").exists() && !paks.join("New_P.pak").exists());
        lib.remove(&k).unwrap();
        assert!(!paks.join("New_P.pak.disabled").exists());
        assert!(paks.join("RogueCore-Windows.pak").exists());
        // Export, wipe, import: the mod comes back with its name and off state.
        let k = lib.install(&src, InstalledMod { name: "Nice Mod".into(), ..Default::default() }).unwrap();
        lib.set_enabled(&k, false).unwrap();
        let pack = root.join("pack.zip");
        lib.export(&pack).unwrap();
        lib.remove(&k).unwrap();
        assert_eq!(lib.import(&pack).unwrap(), 2);
        let m = lib.mods.iter().find(|m| m.key == k).unwrap();
        assert_eq!((m.name.as_str(), m.enabled), ("Nice Mod", false));
        assert!(paks.join("New_P.pak.disabled").exists());
        // "Ohne Mods starten" also switches UE4SS (and so every UE4SS mod) off, and back on.
        let win64 = root.join("game").join("RogueCore").join("Binaries").join("Win64");
        fs::create_dir_all(&win64).unwrap();
        fs::write(win64.join("UE4SS.dll"), b"u").unwrap();
        fs::write(win64.join("dwmapi.dll"), b"d").unwrap();
        assert_eq!(lib.ue4ss_state(), Some(true));
        lib.suspend_all().unwrap();
        assert!(win64.join("dwmapi.dll.disabled").exists() && !win64.join("dwmapi.dll").exists());
        assert_eq!(lib.ue4ss_state(), Some(false));
        lib.apply_all().unwrap();
        assert!(win64.join("dwmapi.dll").exists() && !win64.join("dwmapi.dll.disabled").exists());
        let _ = fs::remove_dir_all(root);
    }
}

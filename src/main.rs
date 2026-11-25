#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod icon;
mod library;
mod nexus;
mod ui;
mod updater;

use eframe::egui::{self, vec2, Align, Color32, CornerRadius, Layout, Margin, RichText, Sense, Stroke, Ui};
use library::{InstalledMod, Library};
use nexus::{NexusFile, NexusMod, NexusUser};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use ui::*;

const APP_NAME: &str = "Core Mod Manager";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const IPC_PORT: u16 = 47813;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Page {
    Wiki,
    Library,
    Store,
    Detail(u32),
    Downloads,
    AddMods,
    Settings,
}

#[derive(Clone, Copy, PartialEq)]
enum LibTab {
    All,
    Active,
    Inactive,
    Updates,
    Outdated,
}

#[derive(Clone, Copy, PartialEq)]
enum LibSort {
    Order,
    Name,
    Recent,
}

#[derive(Clone, Copy, PartialEq)]
enum StoreSort {
    Endorsements,
    Downloads,
    Updated,
    Newest,
    Name,
}

enum Msg {
    Store(Result<Vec<NexusMod>, String>),
    Thumb(String, Option<egui::ColorImage>),
    Detail(u32, Result<(String, Vec<NexusFile>), String>),
    User(Result<NexusUser, String>),
    DlProgress(u64, u64, u64),
    DlDone(u64, Result<PathBuf, String>),
    Ipc(String),
    Wiki(Result<Vec<(String, String)>, String>),
    Patch(Result<nexus::GamePatch, String>),
    Files(Result<HashMap<u32, Vec<NexusFile>>, String>),
    AppUpdate(Result<Option<velopack::UpdateInfo>, String>),
    AppUpdateFailed(String),
}

enum Thumb {
    Loading,
    Ready(egui::TextureHandle),
    Failed,
}

/// A manual Nexus download the app is waiting for in the Downloads folder.
struct Expect {
    dl: u64,
    mod_id: u32,
    uri: String,
    since: std::time::SystemTime,
}

enum DlState {
    /// Waiting for the person's manual download to land in the Downloads folder.
    Waiting,
    Running,
    Done,
    Failed(String),
}

struct Download {
    id: u64,
    name: String,
    done: u64,
    total: u64,
    state: DlState,
    meta: InstalledMod,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Screenshot,
    SelfTest,
}

#[derive(Default)]
struct Hud {
    sidebar: bool,
    topbar: bool,
    statusbar: bool,
    page_header: bool,
}

struct App {
    logo: egui::TextureHandle,
    lib: Library,
    page: Page,
    back_page: Page,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    ctx: egui::Context,
    store: Option<Vec<NexusMod>>,
    store_err: Option<String>,
    thumbs: HashMap<String, Thumb>,
    thumb_queue: Arc<Mutex<VecDeque<String>>>,
    expect: Arc<Mutex<Vec<Expect>>>,
    details: HashMap<u32, Result<(String, Vec<NexusFile>), String>>,
    user: Option<NexusUser>,
    user_msg: String,
    downloads: Vec<Download>,
    next_dl: u64,
    toasts: Vec<(String, f64, bool)>,
    status: String,
    lib_search: String,
    lib_tab: LibTab,
    lib_sort: LibSort,
    store_search: String,
    store_cat: String,
    store_sort: StoreSort,
    store_shown: usize,
    key_edit: String,
    confirm_delete: Option<String>,
    mode: Mode,
    out_file: Option<PathBuf>,
    frame: u64,
    hud: Hud,
    visited: HashSet<Page>,
    test_steps: Vec<serde_json::Value>,
    wiki_query: String,
    /// None: nothing searched; Some(None): searching.
    wiki_results: Option<Option<Result<Vec<(String, String)>, String>>>,
    /// Latest public game patch (Steam), once fetched.
    patch: Option<nexus::GamePatch>,
    /// Store filter: 0 all, 1 current, 2 possibly outdated.
    store_fresh: u8,
    /// Nexus files of each installed mod, for spotting updates.
    files: HashMap<u32, Vec<NexusFile>>,
    update_toast_shown: bool,
    /// A newer version of this app on GitHub.
    app_update: Option<velopack::UpdateInfo>,
    app_update_msg: String,
    app_updating: bool,
    shown_page: Page,
    page_since: f64,
}

fn main() -> eframe::Result<()> {
    // Velopack's install/update hooks; must run before anything else (it may exit or restart).
    velopack::VelopackApp::build().run();
    let args: Vec<String> = std::env::args().collect();
    let flag = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let (mode, out_file) = match (flag("--screenshot"), flag("--self-test")) {
        (Some(p), _) => (Mode::Screenshot, Some(p)),
        (_, Some(p)) => (Mode::SelfTest, Some(p)),
        _ => (Mode::Normal, None),
    };
    let nxm = args.iter().find(|a| a.starts_with("nxm://")).cloned();
    if mode == Mode::Normal {
        // Another copy is open: hand it the nxm link (or just bring it forward) and quit.
        if let Ok(mut s) = std::net::TcpStream::connect_timeout(&([127, 0, 0, 1], IPC_PORT).into(), std::time::Duration::from_millis(300)) {
            let _ = std::io::Write::write_all(&mut s, nxm.as_deref().unwrap_or("focus").as_bytes());
            return Ok(());
        }
    }
    let icon = egui::IconData { rgba: icon::icon_rgba(64), width: 64, height: 64 };
    let mut vp = egui::ViewportBuilder::default().with_title(APP_NAME).with_icon(Arc::new(icon)).with_inner_size([1280.0, 800.0]).with_min_inner_size([960.0, 620.0]);
    if mode != Mode::Normal {
        vp = vp.with_active(false);
        #[cfg(windows)]
        {
            vp = vp.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }
    let options = eframe::NativeOptions { viewport: vp, ..Default::default() };
    eframe::run_native(APP_NAME, options, Box::new(move |cc| Ok(Box::new(App::new(cc, mode, out_file, nxm)))))
}

impl App {
    fn new(cc: &eframe::CreationContext, mode: Mode, out_file: Option<PathBuf>, nxm: Option<String>) -> App {
        let (tx, rx) = channel();
        let data_dir = if mode == Mode::SelfTest {
            std::env::temp_dir().join(format!("cmm-selftest-{}", library::now()))
        } else {
            library::default_data_dir()
        };
        let mut lib = Library::load(data_dir);
        if mode == Mode::SelfTest {
            let game = lib.data_dir.join("game");
            let _ = std::fs::create_dir_all(game.join("RogueCore").join("Content").join("Paks"));
            lib.settings.game_dir = game.to_string_lossy().into();
            lib.settings.api_key.clear();
        }
        set_english(lib.settings.lang == "en");
        set_theme(&lib.settings.theme);
        setup_style(&cc.egui_ctx);
        lib.scan();
        let logo = cc.egui_ctx.load_texture("logo", egui::ColorImage::from_rgba_unmultiplied([128, 128], &icon::icon_rgba(128)), egui::TextureOptions::LINEAR);
        let mut app = App {
            logo,
            key_edit: lib.settings.api_key.clone(),
            lib,
            page: Page::Library,
            back_page: Page::Store,
            tx,
            rx,
            ctx: cc.egui_ctx.clone(),
            store: None,
            store_err: None,
            thumbs: HashMap::new(),
            thumb_queue: Arc::new(Mutex::new(VecDeque::new())),
            expect: Arc::new(Mutex::new(Vec::new())),
            details: HashMap::new(),
            user: None,
            user_msg: String::new(),
            downloads: Vec::new(),
            next_dl: 1,
            toasts: Vec::new(),
            status: String::new(),
            lib_search: String::new(),
            lib_tab: LibTab::All,
            lib_sort: LibSort::Order,
            store_search: String::new(),
            store_cat: String::new(),
            store_sort: StoreSort::Endorsements,
            store_shown: 48,
            confirm_delete: None,
            mode,
            out_file,
            frame: 0,
            hud: Hud::default(),
            visited: HashSet::new(),
            test_steps: Vec::new(),
            wiki_query: String::new(),
            wiki_results: None,
            patch: None,
            store_fresh: 0,
            files: HashMap::new(),
            update_toast_shown: false,
            app_update: None,
            app_update_msg: String::new(),
            app_updating: false,
            shown_page: Page::Library,
            page_since: 0.0,
        };
        if mode != Mode::SelfTest {
            app.start_thumb_workers();
            app.load_store();
            app.spawn(|| Msg::Patch(nexus::fetch_game_patch()));
            if !app.lib.settings.api_key.is_empty() {
                app.validate_key();
            }
        }
        if mode == Mode::Normal {
            app.start_ipc_listener();
            app.start_download_watcher();
            app.check_app_update();
        }
        if let Some(n) = nxm {
            app.handle_nxm(&n);
        }
        // `--page store|detail:<id>|downloads|add|settings` opens a page first (used to check screens).
        let args: Vec<String> = std::env::args().collect();
        if let Some(l) = args.iter().position(|a| a == "--lang").and_then(|i| args.get(i + 1)) {
            set_english(l == "en"); // this run only; the saved choice is untouched
        }
        if let Some(th) = args.iter().position(|a| a == "--theme").and_then(|i| args.get(i + 1)) {
            set_theme(th); // this run only
            apply_theme(&cc.egui_ctx);
        }
        if let Some(p) = args.iter().position(|a| a == "--page").and_then(|i| args.get(i + 1)) {
            match p.as_str() {
                "store" => app.page = Page::Store,
                "downloads" => app.page = Page::Downloads,
                "add" => app.page = Page::AddMods,
                "settings" => app.page = Page::Settings,
                "wiki" => app.page = Page::Wiki,
                d => {
                    if let Some(id) = d.strip_prefix("detail:").and_then(|x| x.parse().ok()) {
                        app.open_detail(id);
                    }
                }
            }
        }
        app
    }

    // ---------- background work ----------

    fn spawn(&self, f: impl FnOnce() -> Msg + Send + 'static) {
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(f());
            ctx.request_repaint();
        });
    }

    fn load_store(&mut self) {
        self.store_err = None;
        self.spawn(|| Msg::Store(nexus::fetch_all_mods()));
    }

    fn validate_key(&mut self) {
        let key = self.lib.settings.api_key.clone();
        self.user_msg = t("Wird geprüft…", "Checking…").into();
        self.spawn(move || Msg::User(nexus::validate_key(&key)));
    }

    fn start_ipc_listener(&self) {
        let Ok(listener) = std::net::TcpListener::bind(("127.0.0.1", IPC_PORT)) else { return };
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            for s in listener.incoming().flatten() {
                let mut t = String::new();
                let _ = std::io::Read::read_to_string(&mut std::io::Read::take(s, 4096), &mut t);
                let _ = tx.send(Msg::Ipc(t));
                ctx.request_repaint();
            }
        });
    }

    fn check_app_update(&mut self) {
        self.app_update_msg = t("Suche nach Updates…", "Checking for updates…").into();
        self.spawn(|| Msg::AppUpdate(updater::check()));
    }

    /// Downloads the new version in the background; Velopack then restarts the app into it.
    fn install_app_update(&mut self) {
        let Some(u) = self.app_update.clone() else { return };
        self.app_updating = true;
        self.spawn(move || Msg::AppUpdateFailed(updater::install(&u).err().unwrap_or_default()));
    }

    /// Watches the Downloads folder for the files the person downloads by hand from Nexus.
    fn start_download_watcher(&self) {
        let (expect, tx, ctx) = (self.expect.clone(), self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let dir = library::downloads_folder();
            let mut sizes: HashMap<PathBuf, u64> = HashMap::new();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(1500));
                let mut waiting = expect.lock().unwrap();
                if waiting.is_empty() {
                    sizes.clear();
                    continue;
                }
                // Give up after 30 minutes.
                waiting.retain(|e| {
                    let alive = e.since.elapsed().map_or(true, |d| d.as_secs() < 1800);
                    if !alive {
                        let _ = tx.send(Msg::DlDone(e.dl, Err(t("Keine Datei im Downloads-Ordner angekommen (30 Min.)", "No file arrived in the Downloads folder (30 min)").into())));
                    }
                    alive
                });
                let Ok(rd) = std::fs::read_dir(&dir) else { continue };
                for entry in rd.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();
                    let Ok(meta) = entry.metadata() else { continue };
                    let Ok(modified) = meta.modified() else { continue };
                    // Only finished files (browsers write .crdownload/.part first) whose size stopped changing.
                    let stable = sizes.insert(path.clone(), meta.len()) == Some(meta.len());
                    if !meta.is_file() || !stable || meta.len() == 0 {
                        continue;
                    }
                    if let Some(i) = waiting.iter().position(|e| modified + std::time::Duration::from_secs(5) >= e.since && nexus::is_nexus_download(&name, &e.uri, e.mod_id)) {
                        let e = waiting.remove(i);
                        let _ = tx.send(Msg::DlDone(e.dl, Ok(path)));
                        ctx.request_repaint();
                    }
                }
            }
        });
    }

    fn start_thumb_workers(&self) {
        let cache = self.lib.data_dir.join("thumbs");
        let _ = std::fs::create_dir_all(&cache);
        for _ in 0..6 {
            let (q, tx, ctx, cache) = (self.thumb_queue.clone(), self.tx.clone(), self.ctx.clone(), cache.clone());
            std::thread::spawn(move || loop {
                let next = q.lock().unwrap().pop_front();
                let Some(url) = next else {
                    std::thread::sleep(std::time::Duration::from_millis(60));
                    continue;
                };
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                url.hash(&mut h);
                let file = cache.join(format!("{:016x}.img", h.finish()));
                let bytes = std::fs::read(&file).ok().or_else(|| {
                    let b = nexus::fetch_bytes(&url)?;
                    let _ = std::fs::write(&file, &b);
                    Some(b)
                });
                let img = bytes.and_then(|b| image::load_from_memory(&b).ok()).map(|i| {
                    let i = i.thumbnail(640, 640).to_rgba8();
                    egui::ColorImage::from_rgba_unmultiplied([i.width() as usize, i.height() as usize], i.as_raw())
                });
                let _ = tx.send(Msg::Thumb(url, img));
                ctx.request_repaint();
            });
        }
    }

    fn thumb(&mut self, url: &str) -> Option<egui::TextureHandle> {
        if url.is_empty() {
            return None;
        }
        match self.thumbs.get(url) {
            Some(Thumb::Ready(t)) => Some(t.clone()),
            Some(_) => None,
            None => {
                self.thumbs.insert(url.to_string(), Thumb::Loading);
                self.thumb_queue.lock().unwrap().push_back(url.to_string());
                None
            }
        }
    }

    fn poll(&mut self) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Store(r) => match r {
                    Ok(v) => {
                        self.store = Some(v);
                        self.link_local_mods();
                        self.check_updates();
                    }
                    Err(e) => self.store_err = Some(e),
                },
                Msg::Thumb(url, img) => {
                    let t = match img {
                        Some(i) => Thumb::Ready(self.ctx.load_texture(&url, i, egui::TextureOptions::LINEAR)),
                        None => Thumb::Failed,
                    };
                    self.thumbs.insert(url, t);
                }
                Msg::Detail(id, r) => {
                    if let Ok((_, files)) = &r {
                        self.files.insert(id, files.clone());
                    }
                    self.details.insert(id, r);
                }
                Msg::User(r) => match r {
                    Ok(u) => {
                        self.user_msg = crate::tf!("Angemeldet als {}{}", "Signed in as {}{}", u.name, if u.is_premium { " (Premium)" } else { "" });
                        self.user = Some(u);
                    }
                    Err(e) => {
                        self.user = None;
                        self.user_msg = e;
                    }
                },
                Msg::DlProgress(id, d, t) => {
                    if let Some(dl) = self.downloads.iter_mut().find(|x| x.id == id) {
                        dl.done = d;
                        dl.total = t;
                    }
                }
                Msg::DlDone(id, r) => self.finish_download(id, r),
                Msg::Wiki(r) => self.wiki_results = Some(Some(r)),
                Msg::Patch(r) => self.patch = r.ok(),
                Msg::AppUpdate(r) => match r {
                    Ok(Some(u)) => {
                        self.app_update_msg = crate::tf!("Version {} ist verfügbar.", "Version {} is available.", u.TargetFullRelease.Version);
                        self.toast(crate::tf!("App-Update {} verfügbar – unten links installieren", "App update {} available – install it at the bottom left", u.TargetFullRelease.Version), false);
                        self.app_update = Some(u);
                    }
                    Ok(None) => self.app_update_msg = t("Du hast die neueste Version.", "You have the latest version.").into(),
                    Err(e) => self.app_update_msg = e,
                },
                Msg::AppUpdateFailed(e) => {
                    self.app_updating = false;
                    self.toast(crate::tf!("Update fehlgeschlagen: {}", "Update failed: {}", e), true);
                }
                Msg::Files(r) => {
                    if let Ok(map) = r {
                        self.files.extend(map);
                    }
                    let n = self.lib.mods.iter().filter(|m| self.update_available(m)).count();
                    if n > 0 && !self.update_toast_shown {
                        self.update_toast_shown = true;
                        self.toast(crate::tf!("{} Mod-Update(s) verfügbar – siehe Meine Mods → Updates", "{} mod update(s) available – see Mods Library → Updates", n), false);
                    }
                }
                Msg::Ipc(t) => {
                    self.ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    if t.starts_with("nxm://") {
                        self.handle_nxm(&t);
                    }
                }
            }
        }
    }

    fn set_language(&mut self, code: &str) {
        self.lib.settings.lang = code.into();
        set_english(code == "en");
        self.status.clear();
        self.lib.save();
    }

    fn toast(&mut self, text: impl Into<String>, err: bool) {
        let t = text.into();
        self.status = t.clone();
        self.toasts.push((t, self.ctx.input(|i| i.time), err));
    }

    // ---------- mod actions ----------

    fn open_detail(&mut self, id: u32) {
        if !matches!(self.page, Page::Detail(_)) {
            self.back_page = self.page;
        }
        self.page = Page::Detail(id);
        if !matches!(self.details.get(&id), Some(Ok(_))) {
            self.details.remove(&id);
            self.spawn(move || Msg::Detail(id, nexus::fetch_detail(id)));
        }
    }

    fn store_mod(&self, id: u32) -> Option<&NexusMod> {
        self.store.as_ref()?.iter().find(|m| m.id == id)
    }

    fn meta_for(&self, mod_id: u32, file: Option<&NexusFile>, file_id: u32) -> InstalledMod {
        let sm = self.store_mod(mod_id).cloned().unwrap_or_default();
        let file = file.or_else(|| self.files.get(&mod_id).and_then(|v| v.iter().find(|f| f.id == file_id)));
        InstalledMod {
            key: format!("nexus-{mod_id}"),
            name: if sm.name.is_empty() { format!("Mod #{mod_id}") } else { sm.name.clone() },
            author: sm.author.clone(),
            version: file.map(|f| f.version.clone()).filter(|v| !v.is_empty()).unwrap_or(sm.version.clone()),
            nexus_id: Some(mod_id),
            file_id: Some(file_id),
            thumb: sm.thumb.clone(),
            category: sm.category.clone(),
            ..Default::default()
        }
    }

    /// Premium: downloads directly. Everyone else: opens the file on Nexus and waits for it in the
    /// Downloads folder ("Manual download" needs no API key); with a key and the nxm handler,
    /// Nexus's "Mod Manager Download" button works too.
    fn request_download(&mut self, mod_id: u32, file: &NexusFile) {
        let premium = self.user.as_ref().map_or(false, |u| u.is_premium);
        if self.lib.settings.api_key.is_empty() || !premium {
            let id = self.next_dl;
            self.next_dl += 1;
            let meta = self.meta_for(mod_id, Some(file), file.id);
            self.downloads.push(Download { id, name: meta.name.clone(), done: 0, total: 0, state: DlState::Waiting, meta });
            self.expect.lock().unwrap().push(Expect { dl: id, mod_id, uri: file.uri.clone(), since: std::time::SystemTime::now() });
            library::open_url(&format!("https://www.nexusmods.com/{}/mods/{mod_id}?tab=files&file_id={}", nexus::GAME_DOMAIN, file.id));
            self.toast(
                t(
                    "Nexus geöffnet: klicke dort auf \"Manual download\". Sobald die Datei im Downloads-Ordner ist, installiert die App sie automatisch.",
                    "Nexus opened: click \"Manual download\" there. As soon as the file is in your Downloads folder, the app installs it automatically.",
                ),
                false,
            );
            return;
        }
        let meta = self.meta_for(mod_id, Some(file), file.id);
        self.start_download(meta, file.id, None);
    }

    fn handle_nxm(&mut self, url: &str) {
        let Some(l) = nexus::parse_nxm(url.trim()) else {
            self.toast(t("Dieser Nexus-Link gehört nicht zu Rogue Core.", "This Nexus link isn't for Rogue Core."), true);
            return;
        };
        if self.lib.settings.api_key.is_empty() {
            self.page = Page::Settings;
            self.toast(t("Für Downloads wird dein Nexus API-Schlüssel gebraucht.", "Downloads need your Nexus API key."), true);
            return;
        }
        let meta = self.meta_for(l.mod_id, None, l.file_id);
        self.start_download(meta, l.file_id, Some((l.key, l.expires)));
    }

    fn start_download(&mut self, meta: InstalledMod, file_id: u32, nxm: Option<(String, String)>) {
        let id = self.next_dl;
        self.next_dl += 1;
        let mod_id = meta.nexus_id.unwrap_or(0);
        self.downloads.push(Download { id, name: meta.name.clone(), done: 0, total: 0, state: DlState::Running, meta });
        let key = self.lib.settings.api_key.clone();
        let dir = self.lib.downloads_dir();
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let r = (|| {
                let url = nexus::download_url(&key, mod_id, file_id, nxm.as_ref().map(|(k, e)| (k.as_str(), e.as_str())))?;
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let name = url.split('?').next().and_then(|u| u.rsplit('/').next()).filter(|n| !n.is_empty()).unwrap_or("download.zip");
                let name = name.replace("%20", " ");
                let dest = dir.join(format!("{mod_id}-{file_id}-{name}"));
                let mut last = std::time::Instant::now();
                nexus::download_to(&url, &dest, |d, t| {
                    if last.elapsed().as_millis() > 100 {
                        last = std::time::Instant::now();
                        let _ = tx.send(Msg::DlProgress(id, d, t));
                        ctx.request_repaint();
                    }
                })?;
                Ok(dest)
            })();
            let _ = tx.send(Msg::DlDone(id, r));
            ctx.request_repaint();
        });
        self.toast(t("Download gestartet", "Download started"), false);
    }

    fn finish_download(&mut self, id: u64, r: Result<PathBuf, String>) {
        let Some(i) = self.downloads.iter().position(|d| d.id == id) else { return };
        let res = r.and_then(|p| self.lib.install(&p, self.downloads[i].meta.clone()));
        let name = self.downloads[i].name.clone();
        match res {
            Ok(_) => {
                self.downloads[i].state = DlState::Done;
                self.downloads[i].done = self.downloads[i].total.max(self.downloads[i].done);
                self.toast(crate::tf!("{} installiert", "{} installed", name), false);
                self.check_updates();
            }
            Err(e) => {
                self.downloads[i].state = DlState::Failed(e.clone());
                self.toast(format!("{name}: {e}"), true);
            }
        }
    }

    fn install_local(&mut self, paths: Vec<PathBuf>) {
        for p in paths {
            match self.lib.install(&p, InstalledMod { author: "Lokale Datei".into(), category: "Lokal".into(), ..Default::default() }) {
                Ok(_) => self.toast(crate::tf!("{} hinzugefügt", "{} added", p.file_name().unwrap_or_default().to_string_lossy()), false),
                Err(e) => self.toast(e, true),
            }
        }
        self.page = Page::Library;
    }

    /// Gives .pak files found in the game folder their Nexus page (picture, author)
    /// when the file name matches a Nexus mod name exactly, ignoring versions and punctuation.
    fn link_local_mods(&mut self) {
        let Some(store) = &self.store else { return };
        let mut changed = false;
        for m in self.lib.mods.iter_mut().filter(|m| m.nexus_id.is_none() && m.key.starts_with("local-")) {
            let key = name_key(&m.name);
            if let Some(s) = store.iter().find(|s| !key.is_empty() && name_key(&s.name) == key) {
                m.nexus_id = Some(s.id);
                m.name = s.name.clone();
                m.author = s.author.clone();
                m.thumb = s.thumb.clone();
                m.category = s.category.clone();
                changed = true;
            }
        }
        if changed {
            self.lib.save();
        }
    }

    /// When the game last changed: the public patch date from Steam, else when Steam updated this install.
    fn patch_time(&self) -> Option<u64> {
        let online = self.patch.as_ref().map(|p| p.released).filter(|&t| t > 0);
        online.or_else(|| self.lib.local_build().map(|(_, t)| t).filter(|&t| t > 0))
    }

    /// Some(true): the mod was updated after the last game patch; Some(false): it wasn't.
    /// Nexus says nothing about which game version a mod targets, so its last update date stands in.
    fn fresh(&self, m: &NexusMod) -> Option<bool> {
        let updated = parse_time(&m.updated);
        (updated > 0).then_some(())?;
        Some(updated >= self.patch_time()?)
    }

    fn installed_fresh(&self, m: &InstalledMod) -> Option<bool> {
        self.fresh(self.store_mod(m.nexus_id?)?)
    }

    fn fresh_note(&self, m: &NexusMod) -> Option<String> {
        let patch = self.patch_time()?;
        let name = self.patch.as_ref().map(|p| p.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| t("Spiel-Update", "game update").into());
        Some(crate::tf!(
            "Mod zuletzt aktualisiert am {} · letztes Spiel-Update {} am {}",
            "Mod last updated {} · last game update {} on {}",
            fmt_date(&m.updated),
            name,
            fmt_date(&unix_to_iso(patch))
        ))
    }

    /// "Spiel EA 03.04.00 · Build 26783375" plus a hint when Steam has a newer build.
    fn game_version_text(&self) -> Option<String> {
        let local = self.lib.local_build();
        let patch = self.patch.as_ref();
        let name = patch.map(|p| p.name.clone()).filter(|n| !n.is_empty());
        let mut s = match (&local, &name) {
            (Some((b, _)), Some(n)) if patch.map_or(true, |p| p.buildid.is_empty() || p.buildid == *b) => crate::tf!("Spiel {} · Build {}", "Game {} · build {}", n, b),
            (Some((b, _)), _) => crate::tf!("Spiel-Build {}", "Game build {}", b),
            (None, Some(n)) => crate::tf!("Aktuelle Spielversion {}", "Latest game version {}", n),
            (None, None) => return None,
        };
        if let (Some((b, _)), Some(p)) = (&local, patch) {
            if !p.buildid.is_empty() && p.buildid != *b {
                s.push_str(t(" · Update in Steam verfügbar", " · update available in Steam"));
            }
        }
        Some(s)
    }

    fn export_mods(&mut self) {
        let Some(dest) = rfd::FileDialog::new().add_filter("Mod-Paket", &["zip"]).set_file_name("rogue-core-mods.zip").save_file() else { return };
        match self.lib.export(&dest) {
            Ok(n) => self.toast(crate::tf!("{} Mods exportiert nach {}", "Exported {} mods to {}", n, dest.display()), false),
            Err(e) => self.toast(crate::tf!("Export fehlgeschlagen: {}", "Export failed: {}", e), true),
        }
    }

    fn import_mods(&mut self) {
        let Some(src) = rfd::FileDialog::new().add_filter("Mod-Paket", &["zip"]).pick_file() else { return };
        match self.lib.import(&src) {
            Ok(n) => {
                self.link_local_mods();
                self.check_updates();
                self.toast(crate::tf!("{} Mods importiert", "Imported {} mods", n), false);
            }
            Err(e) => self.toast(crate::tf!("Import fehlgeschlagen: {}", "Import failed: {}", e), true),
        }
    }

    /// Asks Nexus for the files of every installed Nexus mod.
    fn check_updates(&mut self) {
        let mut ids: Vec<u32> = self.lib.mods.iter().filter_map(|m| m.nexus_id).collect();
        ids.sort();
        ids.dedup();
        if !ids.is_empty() && self.mode != Mode::SelfTest {
            self.spawn(move || Msg::Files(nexus::fetch_files(&ids)));
        }
    }

    /// The Nexus file that would update this installed mod, if there is one.
    fn pending(&self, m: &InstalledMod) -> Option<&NexusFile> {
        nexus::pending_update(self.files.get(&m.nexus_id?)?, m.file_id, &m.files)
    }

    fn update_available(&self, m: &InstalledMod) -> bool {
        self.pending(m).is_some()
    }

    // ---------- layout ----------

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top").show_separator_line(false)
            .exact_height(44.0)
            .frame(egui::Frame::new().fill(pal().bg).inner_margin(Margin::symmetric(12, 6)).stroke(Stroke::new(1.0f32, pal().border)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let n = self.lib.active_count();
                        if primary_button(ui, &crate::tf!("▶ Mit Mods starten  ·  {} Mods", "▶ Launch modded  ·  {} mods", n)).clicked() {
                            match self.lib.apply_all() {
                                Ok(()) => {
                                    library::launch_game();
                                    self.toast(crate::tf!("Spiel startet mit {} Mods", "Starting the game with {} mods", n), false);
                                }
                                Err(e) => self.toast(e, true),
                            }
                        }
                        if outline_button(ui, t("⏵ Ohne Mods starten", "⏵ Launch without mods")).clicked() {
                            match self.lib.suspend_all() {
                                Ok(()) => {
                                    library::launch_game();
                                    self.toast(t("Spiel startet ohne Mods. \"Mit Mods starten\" schaltet sie wieder ein.", "Starting the game without mods. \"Launch modded\" turns them back on."), false);
                                }
                                Err(e) => self.toast(e, true),
                            }
                        }
                        self.hud.topbar = true;
                    });
                });
            });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status").show_separator_line(false)
            .exact_height(26.0)
            .frame(egui::Frame::new().fill(pal().bg).inner_margin(Margin::symmetric(12, 4)).stroke(Stroke::new(1.0f32, pal().border)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("Status:").small().color(pal().muted_fg));
                    ui.label(RichText::new(if self.status.is_empty() { t("Bereit", "Ready") } else { self.status.as_str() }).small().color(pal().fg));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("v{VERSION}")).small().color(pal().muted_fg));
                        ui.separator();
                        let (txt, col) = if self.lib.paks_dir().is_some() { (t("• Spiel gefunden", "• Game found"), OK) } else { (t("• Spiel nicht gefunden", "• Game not found"), BAD) };
                        ui.label(RichText::new(txt).small().color(col));
                        if let Some(v) = self.game_version_text() {
                            ui.separator();
                            ui.label(RichText::new(v).small().color(pal().muted_fg));
                        }
                        if self.lib.settings.vanilla_active {
                            ui.separator();
                            ui.label(RichText::new(t("Mods pausiert (Vanilla)", "Mods paused (vanilla)")).small().color(pal().secondary_fg));
                        }
                    });
                });
                self.hud.statusbar = true;
            });
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("side").show_separator_line(false)
            .exact_width(214.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(pal().bg).inner_margin(Margin::symmetric(10, 12)).stroke(Stroke::new(1.0f32, pal().sidebar_border)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new(egui::load::SizedTexture::new(self.logo.id(), vec2(36.0, 36.0))));
                    ui.vertical(|ui| {
                        ui.label(RichText::new(APP_NAME).family(semibold()).color(pal().fg));
                        ui.label(RichText::new(format!("v{VERSION} · Rogue Core")).small().color(pal().muted_fg));
                    });
                });
                ui.add_space(14.0);
                let running = self.downloads.iter().filter(|d| matches!(d.state, DlState::Running)).count();
                nav_group(ui, t("Allgemein", "General"));
                self.nav_item(ui, "⬇", "Downloads", Page::Downloads, (running > 0).then(|| running.to_string()));
                self.nav_item(ui, "⚙", t("Einstellungen", "Settings"), Page::Settings, None);
                ui.add_space(12.0);
                nav_group(ui, "Mods");
                let n = self.lib.mods.len();
                self.nav_item(ui, "📚", t("Meine Mods", "Mods Library"), Page::Library, Some(n.to_string()));
                self.nav_item(ui, "🔍", t("Mods entdecken", "Mods Store"), Page::Store, None);
                self.nav_item(ui, "➕", t("Mods hinzufügen", "Add Local Mod"), Page::AddMods, None);
                ui.add_space(12.0);
                nav_group(ui, t("Hilfe", "Help"));
                self.nav_item(ui, "📖", "Wiki", Page::Wiki, None);
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    if let Some(v) = self.app_update.as_ref().map(|u| u.TargetFullRelease.Version.clone()) {
                        let label = if self.app_updating { t("⏳ Update wird installiert…", "⏳ Installing update…").to_string() } else { crate::tf!("⬆ App-Update {}", "⬆ App update {}", v) };
                        if primary_button(ui, &label).clicked() && !self.app_updating {
                            self.install_app_update();
                        }
                        ui.add_space(6.0);
                    }
                    if tab_button(ui, t("🌐  Nexus Mods öffnen", "🌐  Open Nexus Mods"), false).clicked() {
                        library::open_url(&format!("https://www.nexusmods.com/games/{}/mods", nexus::GAME_DOMAIN));
                    }
                    if let Some(p) = self.lib.paks_dir() {
                        if tab_button(ui, t("📁  Paks-Ordner öffnen", "📁  Open Paks folder"), false).clicked() {
                            library::open_folder(&p);
                        }
                    }
                });
                self.hud.sidebar = true;
            });
    }

    fn nav_item(&mut self, ui: &mut Ui, icon: &str, label: &str, page: Page, badge: Option<String>) {
        let active = self.page == page || (page == Page::Store && matches!(self.page, Page::Detail(_)));
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
        let c = pal();
        let a = ui.ctx().animate_bool_with_time(resp.id.with("a"), active, 0.18);
        let h = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
        let p = ui.painter();
        p.rect_filled(rect, CornerRadius::same(8), c.muted.gamma_multiply(h * (1.0 - a)));
        p.rect_filled(rect, CornerRadius::same(8), c.primary.gamma_multiply(a));
        let col = lerp_color(lerp_color(c.muted_fg, c.fg, h), c.primary_fg, a);
        let font = egui::FontId::new(14.0, if active { semibold() } else { egui::FontFamily::Proportional });
        p.text(rect.left_center() + vec2(12.0, 0.0), egui::Align2::LEFT_CENTER, icon, egui::FontId::proportional(14.0), col);
        p.text(rect.left_center() + vec2(36.0, 0.0), egui::Align2::LEFT_CENTER, label, font, col);
        if let Some(b) = badge {
            p.text(rect.right_center() - vec2(12.0, 0.0), egui::Align2::RIGHT_CENTER, b, egui::FontId::proportional(12.0), lerp_color(c.muted_fg, c.primary_fg, a));
        }
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.page = page;
        }
        ui.add_space(2.0);
    }

    fn page_header(&mut self, ui: &mut Ui, title: &str, badge: Option<String>, subtitle: &str, actions: impl FnOnce(&mut Self, &mut Ui)) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).size(22.0).family(semibold()).color(pal().fg));
                    if let Some(b) = badge {
                        pill(ui, &b, pal().secondary, pal().fg);
                    }
                });
                ui.label(RichText::new(subtitle).color(pal().muted_fg));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| actions(self, ui));
        });
        ui.add_space(12.0);
        self.hud.page_header = true;
    }

    // ---------- pages ----------

    fn page_library(&mut self, ui: &mut Ui) {
        let files: usize = self.lib.mods.iter().map(|m| m.files.len()).sum();
        self.page_header(ui, t("Meine Mods", "Mods Library"), Some(crate::tf!("{} Mod-Dateien", "{} addon files", files)), t("Mods, die du heruntergeladen oder installiert hast", "Mods you have downloaded or installed"), |s, ui| {
            if primary_button(ui, t("➕ Mods hinzufügen", "➕ Add mods")).clicked() {
                s.page = Page::AddMods;
            }
            if outline_button(ui, t("🔄 Neu einlesen", "🔄 Rescan")).clicked() {
                s.lib.scan();
                s.toast(t("Paks-Ordner neu eingelesen", "Paks folder rescanned"), false);
            }
            if outline_button(ui, t("⬇ Importieren", "⬇ Import")).on_hover_text(t("Mods aus einer exportierten .zip installieren", "Install mods from an exported .zip")).clicked() {
                s.import_mods();
            }
            if outline_button(ui, t("⬆ Exportieren", "⬆ Export")).on_hover_text(t("Alle Mods samt Dateien in eine .zip speichern", "Save all mods with their files into a .zip")).clicked() {
                s.export_mods();
            }
        });
        if self.lib.paks_dir().is_none() {
            notice(ui, t("Der Spielordner von Deep Rock Galactic: Rogue Core wurde nicht gefunden. Lege ihn in den Einstellungen fest.", "The Deep Rock Galactic: Rogue Core game folder was not found. Set it in Settings."), pal().destructive);
            ui.add_space(8.0);
        }
        let updates = self.lib.mods.iter().filter(|m| self.update_available(m)).count();
        if updates > 0 && self.lib_tab != LibTab::Updates {
            let mut show = false;
            card(ui, 12, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(crate::tf!("⬆ {} Mod(s) haben ein Update, das du noch nicht installiert hast.", "⬆ {} mod(s) have an update you haven't installed yet.", updates)).family(semibold()).color(pal().fg));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| show = primary_button(ui, t("Updates anzeigen", "Show updates")).clicked());
                });
            });
            if show {
                self.lib_tab = LibTab::Updates;
            }
            ui.add_space(8.0);
        }
        let counts = [
            self.lib.mods.len(),
            self.lib.mods.iter().filter(|m| m.enabled).count(),
            self.lib.mods.iter().filter(|m| !m.enabled).count(),
            self.lib.mods.iter().filter(|m| self.update_available(m)).count(),
            self.lib.mods.iter().filter(|m| self.installed_fresh(m) == Some(false)).count(),
        ];
        ui.horizontal(|ui| {
            search_box(ui, &mut self.lib_search, t("Mods oder Autoren suchen", "Search mods or authors"), 280.0);
            ui.add_space(8.0);
            for (tab, label, n) in [(LibTab::All, t("Alle", "All"), counts[0]), (LibTab::Active, t("Aktiv", "Enabled"), counts[1]), (LibTab::Inactive, t("Deaktiviert", "Disabled"), counts[2]), (LibTab::Updates, "Updates", counts[3]), (LibTab::Outdated, t("Evtl. veraltet", "Maybe outdated"), counts[4])] {
                if tab_button(ui, &format!("{label}  {n}"), self.lib_tab == tab).clicked() {
                    self.lib_tab = tab;
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                egui::ComboBox::from_id_salt("libsort")
                    .selected_text(match self.lib_sort { LibSort::Order => t("Installationsreihenfolge", "Install Order"), LibSort::Name => "Name", LibSort::Recent => t("Zuletzt hinzugefügt", "Recently Added") })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.lib_sort, LibSort::Order, t("Installationsreihenfolge", "Install Order"));
                        ui.selectable_value(&mut self.lib_sort, LibSort::Name, "Name");
                        ui.selectable_value(&mut self.lib_sort, LibSort::Recent, t("Zuletzt hinzugefügt", "Recently Added"));
                    });
            });
        });
        ui.add_space(10.0);
        let q = self.lib_search.to_lowercase();
        let mut list: Vec<InstalledMod> = self
            .lib
            .mods
            .iter()
            .filter(|m| q.is_empty() || m.name.to_lowercase().contains(&q) || m.author.to_lowercase().contains(&q))
            .filter(|m| match self.lib_tab {
                LibTab::All => true,
                LibTab::Active => m.enabled,
                LibTab::Inactive => !m.enabled,
                LibTab::Updates => self.update_available(m),
                LibTab::Outdated => self.installed_fresh(m) == Some(false),
            })
            .cloned()
            .collect();
        match self.lib_sort {
            LibSort::Order => list.sort_by_key(|m| m.order),
            LibSort::Name => list.sort_by_key(|m| m.name.to_lowercase()),
            LibSort::Recent => list.sort_by(|a, b| b.installed_at.cmp(&a.installed_at)),
        }
        if list.is_empty() {
            empty_state(ui, t("Noch keine Mods hier", "No mods here yet"), t("Stöbere unter \"Mods entdecken\" oder füge eine .pak/.zip unter \"Mods hinzufügen\" hinzu.", "Browse the \"Mods Store\" or add a .pak/.zip under \"Add Local Mod\"."));
            return;
        }
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            grid(ui, list.len(), 196.0, 238.0, |ui, i| self.library_card(ui, &list[i]));
        });
    }

    fn library_card(&mut self, ui: &mut Ui, m: &InstalledMod) {
        let rect = ui.max_rect();
        let resp = ui.interact(rect, ui.id().with(&m.key), Sense::click());
        card_frame(ui, rect, &resp);
        let img = rect.with_max_y(rect.top() + 110.0);
        let tex = if ui.is_rect_visible(rect) { self.thumb(&m.thumb) } else { None };
        draw_cover(ui, img, tex.as_ref(), &m.name);
        if self.update_available(m) {
            overlay_badge(ui, img, t("⬆ Update verfügbar", "⬆ Update available"), Color32::from_rgb(30, 60, 110));
        } else if m.nexus_id.is_none() {
            overlay_badge(ui, img, t("Lokal", "Local"), pal().secondary);
        }
        if self.installed_fresh(m) == Some(false) {
            overlay_badge_at(ui, img, t("⚠ Evtl. veraltet", "⚠ Maybe outdated"), WARN, true);
        }
        let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(vec2(10.0, 0.0)).with_min_y(img.bottom() + 8.0)));
        inner.add(egui::Label::new(RichText::new(&m.name).family(semibold()).color(pal().fg)).truncate());
        let by = if m.author.is_empty() { t("Unbekannt", "Unknown").to_string() } else { if matches!(m.author.as_str(), "Lokal gefunden" | "Lokale Datei") { stored_label(&m.author) } else { crate::tf!("Von {}", "By {}", m.author) } };
        inner.add(egui::Label::new(RichText::new(by).small().color(pal().muted_fg)).truncate());
        if !m.version.is_empty() {
            inner.label(RichText::new(format!("Version {}", m.version)).small().color(pal().muted_fg));
        }
        if let Some(u) = self.pending(m) {
            let v = if u.version.is_empty() { u.name.clone() } else { format!("{} {}", u.name, u.version) };
            inner.add(egui::Label::new(RichText::new(crate::tf!("⬆ Neu: {}", "⬆ New: {}", v)).small().color(pal().primary)).truncate());
        }
        let bottom = egui::Rect::from_min_max(egui::pos2(rect.left() + 10.0, rect.bottom() - 34.0), egui::pos2(rect.right() - 10.0, rect.bottom() - 6.0));
        let mut row = ui.new_child(egui::UiBuilder::new().max_rect(bottom).layout(Layout::left_to_right(Align::Center)));
        if icon_button(&mut row, "🗑", t("Entfernen", "Remove")).clicked() {
            self.confirm_delete = Some(m.key.clone());
        }
        if let Some(id) = m.nexus_id {
            if icon_button(&mut row, "🌐", t("Auf Nexus ansehen", "View on Nexus")).clicked() {
                library::open_url(&format!("https://www.nexusmods.com/{}/mods/{id}", nexus::GAME_DOMAIN));
            }
        }
        row.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let mut on = m.enabled;
            if toggle(ui, &mut on).on_hover_text(if m.enabled { "Aktiv – klicken zum Ausschalten" } else { "Aus – klicken zum Einschalten" }).changed() {
                match self.lib.set_enabled(&m.key, on) {
                    Ok(()) => self.status = format!("{} {}", m.name, if on { t("eingeschaltet", "enabled") } else { t("ausgeschaltet", "disabled") }),
                    Err(e) => self.toast(e, true),
                }
            }
        });
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            if let Some(id) = m.nexus_id {
                self.open_detail(id);
            }
        }
    }

    fn page_store(&mut self, ui: &mut Ui) {
        let total = self.store.as_ref().map(|s| s.len());
        self.page_header(ui, t("Mods entdecken", "Mods Store"), total.map(|n| crate::tf!("{} Mods", "{} mods", n)), t("Alle Mods für Deep Rock Galactic: Rogue Core von Nexus Mods", "All Deep Rock Galactic: Rogue Core mods from Nexus Mods"), |s, ui| {
            if outline_button(ui, t("🌐 Auf Nexus öffnen", "🌐 Open on Nexus")).clicked() {
                library::open_url(&format!("https://www.nexusmods.com/games/{}/mods", nexus::GAME_DOMAIN));
            }
            if outline_button(ui, t("🔄 Aktualisieren", "🔄 Refresh")).clicked() {
                s.store = None;
                s.load_store();
            }
        });
        let Some(all) = self.store.clone() else {
            match self.store_err.clone() {
                Some(e) => {
                    notice(ui, &crate::tf!("Nexus Mods konnte nicht geladen werden: {}", "Couldn't load Nexus Mods: {}", e), pal().destructive);
                    if outline_button(ui, t("Erneut versuchen", "Try again")).clicked() {
                        self.load_store();
                    }
                }
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new(t("Mods werden von Nexus geladen…", "Loading mods from Nexus…")).color(pal().muted_fg));
                    });
                }
            }
            return;
        };
        let mut cats: Vec<String> = all.iter().map(|m| m.category.clone()).filter(|c| !c.is_empty()).collect();
        cats.sort();
        cats.dedup();
        let before = (self.store_search.clone(), self.store_cat.clone(), self.store_sort as u8);
        ui.horizontal(|ui| {
            search_box(ui, &mut self.store_search, t("Mods oder Autoren suchen", "Search mods or authors"), 300.0);
            egui::ComboBox::from_id_salt("cat")
                .selected_text(if self.store_cat.is_empty() { t("Alle Kategorien", "All Categories").to_string() } else { self.store_cat.clone() })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.store_cat, String::new(), t("Alle Kategorien", "All Categories"));
                    for c in &cats {
                        ui.selectable_value(&mut self.store_cat, c.clone(), c);
                    }
                });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                egui::ComboBox::from_id_salt("storesort").selected_text(store_sort_label(self.store_sort)).show_ui(ui, |ui| {
                    for s in [StoreSort::Endorsements, StoreSort::Downloads, StoreSort::Updated, StoreSort::Newest, StoreSort::Name] {
                        ui.selectable_value(&mut self.store_sort, s, store_sort_label(s));
                    }
                });
                ui.label(RichText::new(t("Sortieren nach", "Sort by")).color(pal().muted_fg));
                let fresh_labels = [t("Alle Stände", "Any status"), t("✔ Aktuell", "✔ Up to date"), t("⚠ Evtl. veraltet", "⚠ Maybe outdated")];
                egui::ComboBox::from_id_salt("fresh").selected_text(fresh_labels[self.store_fresh as usize]).show_ui(ui, |ui| {
                    for (i, l) in fresh_labels.iter().enumerate() {
                        ui.selectable_value(&mut self.store_fresh, i as u8, *l);
                    }
                });
            });
        });
        if before != (self.store_search.clone(), self.store_cat.clone(), self.store_sort as u8) {
            self.store_shown = 48;
        }
        ui.add_space(10.0);
        let q = self.store_search.to_lowercase();
        let mut list: Vec<NexusMod> = all
            .into_iter()
            .filter(|m| self.lib.settings.show_adult || !m.adult)
            .filter(|m| self.store_cat.is_empty() || m.category == self.store_cat)
            .filter(|m| match self.store_fresh { 1 => self.fresh(m) == Some(true), 2 => self.fresh(m) == Some(false), _ => true })
            .filter(|m| q.is_empty() || m.name.to_lowercase().contains(&q) || m.author.to_lowercase().contains(&q) || m.summary.to_lowercase().contains(&q))
            .collect();
        match self.store_sort {
            StoreSort::Endorsements => list.sort_by(|a, b| b.endorsements.cmp(&a.endorsements)),
            StoreSort::Downloads => list.sort_by(|a, b| b.downloads.cmp(&a.downloads)),
            StoreSort::Updated => list.sort_by(|a, b| b.updated.cmp(&a.updated)),
            StoreSort::Newest => list.sort_by(|a, b| b.created.cmp(&a.created)),
            StoreSort::Name => list.sort_by_key(|m| m.name.to_lowercase()),
        }
        if list.is_empty() {
            empty_state(ui, t("Keine Mods gefunden", "No mods found"), t("Versuche einen anderen Suchbegriff oder eine andere Kategorie.", "Try another search or category."));
            return;
        }
        let shown = list.len().min(self.store_shown);
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            grid(ui, shown, 230.0, 272.0, |ui, i| self.store_card(ui, &list[i]));
            if shown < list.len() {
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    if outline_button(ui, &crate::tf!("Mehr laden ({} weitere)", "Load more ({} more)", list.len() - shown)).clicked() {
                        self.store_shown += 48;
                    }
                });
            }
        });
    }

    fn store_card(&mut self, ui: &mut Ui, m: &NexusMod) {
        let rect = ui.max_rect();
        let resp = ui.interact(rect, ui.id().with(("store", m.id)), Sense::click());
        card_frame(ui, rect, &resp);
        let img = rect.with_max_y(rect.top() + 128.0);
        let tex = if ui.is_rect_visible(rect) { self.thumb(&m.thumb) } else { None };
        draw_cover(ui, img, tex.as_ref(), &m.name);
        if self.lib.mods.iter().any(|x| x.nexus_id == Some(m.id)) {
            overlay_badge(ui, img, t("✔ Installiert", "✔ Installed"), pal().secondary);
        } else if is_new(&m.created) {
            overlay_badge(ui, img, t("Neu", "New"), NEW);
        }
        match self.fresh(m) {
            Some(true) => overlay_badge_at(ui, img, t("✔ Aktuell", "✔ Up to date"), FRESH, true),
            Some(false) => overlay_badge_at(ui, img, t("⚠ Evtl. veraltet", "⚠ Maybe outdated"), WARN, true),
            None => {}
        }
        let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(vec2(10.0, 0.0)).with_min_y(img.bottom() + 8.0).with_max_y(rect.bottom() - 30.0)));
        inner.add(egui::Label::new(RichText::new(&m.name).family(semibold()).color(pal().fg)).truncate());
        inner.add(egui::Label::new(RichText::new(crate::tf!("Von {}", "By {}", m.author)).small().color(pal().muted_fg)).truncate());
        inner.add_space(2.0);
        inner.add(egui::Label::new(RichText::new(shorten(&m.summary, 95)).small().color(pal().muted_fg)).wrap());
        let p = ui.painter();
        let y = rect.bottom() - 16.0;
        p.text(egui::pos2(rect.left() + 10.0, y), egui::Align2::LEFT_CENTER, format!("⬇ {}   👍 {}", short_num(m.downloads), short_num(m.endorsements)), egui::FontId::proportional(12.0), pal().muted_fg);
        p.text(egui::pos2(rect.right() - 10.0, y), egui::Align2::RIGHT_CENTER, &m.category, egui::FontId::proportional(11.0), pal().secondary_fg);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.open_detail(m.id);
        }
    }

    fn page_detail(&mut self, ui: &mut Ui, id: u32) {
        if ui.add(egui::Button::new(RichText::new(t("⬅ Zurück", "⬅ Back")).color(pal().muted_fg)).frame(false)).clicked() {
            self.page = self.back_page;
        }
        ui.add_space(6.0);
        let m = self.store_mod(id).cloned();
        let installed = self.lib.mods.iter().find(|x| x.nexus_id == Some(id)).cloned();
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let w = (ui.available_width() * 0.42).min(520.0);
                let (r, _) = ui.allocate_exact_size(vec2(w, w * 9.0 / 16.0), Sense::hover());
                let tex = m.as_ref().and_then(|m| self.thumb(&m.picture));
                draw_cover_rounded(ui, r, tex.as_ref(), m.as_ref().map_or("", |m| m.name.as_str()), CornerRadius::same(RADIUS));
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    let name = m.as_ref().map(|m| m.name.clone()).or(installed.as_ref().map(|i| i.name.clone())).unwrap_or(format!("Mod #{id}"));
                    ui.label(RichText::new(name).size(24.0).family(semibold()).color(pal().fg));
                    if let Some(m) = &m {
                        ui.label(RichText::new(crate::tf!("Von {}", "By {}", m.author)).color(pal().muted_fg));
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            pill(ui, &m.category, pal().secondary, pal().secondary_fg);
                            if !m.version.is_empty() {
                                pill(ui, &format!("Version {}", m.version), pal().muted, pal().fg);
                            }
                            pill(ui, &crate::tf!("Aktualisiert {}", "Updated {}", fmt_date(&m.updated)), pal().muted, pal().fg);
                            match self.fresh(m) {
                                Some(true) => pill(ui, t("✔ Aktuell", "✔ Up to date"), FRESH, on(FRESH)),
                                Some(false) => pill(ui, t("⚠ Evtl. veraltet", "⚠ Maybe outdated"), WARN, on(WARN)),
                                None => {}
                            }
                        });
                        if let Some(line) = self.fresh_note(m) {
                            ui.label(RichText::new(line).small().color(pal().muted_fg));
                        }
                        ui.add_space(6.0);
                        ui.label(RichText::new(crate::tf!("⬇ {} Downloads    👍 {} Empfehlungen", "⬇ {} downloads    👍 {} endorsements", short_num(m.downloads), short_num(m.endorsements))).color(pal().muted_fg));
                        ui.add_space(6.0);
                        ui.label(RichText::new(&m.summary).color(pal().fg));
                    }
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if outline_button(ui, t("🌐 Auf Nexus ansehen", "🌐 View on Nexus")).clicked() {
                            library::open_url(&format!("https://www.nexusmods.com/{}/mods/{id}", nexus::GAME_DOMAIN));
                        }
                        if let Some(i) = &installed {
                            ui.add_space(8.0);
                            let mut on = i.enabled;
                            if toggle(ui, &mut on).changed() {
                                if let Err(e) = self.lib.set_enabled(&i.key, on) {
                                    self.toast(e, true);
                                }
                            }
                            ui.label(RichText::new(crate::tf!("Installiert{}", "Installed{}", if i.version.is_empty() { String::new() } else { format!(" (Version {})", i.version) })).color(pal().secondary_fg));
                            if let Some(u) = self.pending(i) {
                                pill(ui, &crate::tf!("⬆ Update: {} {}", "⬆ Update: {} {}", u.name, u.version), pal().primary, pal().primary_fg);
                            }
                        }
                    });
                });
            });
            ui.add_space(18.0);
            match self.details.get(&id).cloned() {
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new(t("Dateien werden geladen…", "Loading files…")).color(pal().muted_fg));
                    });
                }
                Some(Err(e)) => {
                    notice(ui, &format!("Details konnten nicht geladen werden: {e}"), pal().destructive);
                    if outline_button(ui, t("Erneut versuchen", "Try again")).clicked() {
                        self.details.remove(&id);
                        self.open_detail(id);
                    }
                }
                Some(Ok((desc, files))) => {
                    section_title(ui, t("Dateien", "Files"));
                    let premium = self.user.as_ref().map_or(false, |u| u.is_premium);
                    for f in files.iter().filter(|f| f.category != "ARCHIVED") {
                        card(ui, 12, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&f.name).family(semibold()).color(pal().fg));
                                        pill(ui, &file_cat(&f.category), pal().secondary, pal().secondary_fg);
                                        let is_update = installed.as_ref().and_then(|i| self.pending(i)).map_or(false, |u| u.id == f.id);
                                        if is_update {
                                            pill(ui, t("⬆ Update für dich", "⬆ Update for you"), pal().primary, pal().primary_fg);
                                        } else if installed.as_ref().map_or(false, |i| i.file_id == Some(f.id)) {
                                            pill(ui, t("✔ Installiert", "✔ Installed"), FRESH, on(FRESH));
                                        }
                                    });
                                    ui.label(RichText::new(format!("Version {} · {}", if f.version.is_empty() { "–" } else { &f.version }, fmt_size(f.size))).small().color(pal().muted_fg));
                                    if !f.description.is_empty() {
                                        ui.label(RichText::new(&f.description).small().color(pal().muted_fg));
                                    }
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let current = installed.as_ref().map_or(false, |i| i.file_id == Some(f.id));
                                    let label = if current { t("Neu installieren", "Reinstall") } else if premium { t("⬇ Herunterladen & installieren", "⬇ Download & install") } else { t("⬇ Herunterladen", "⬇ Download") };
                                    if primary_button(ui, label).clicked() {
                                        self.request_download(id, f);
                                    }
                                });
                            });
                        });
                        ui.add_space(6.0);
                    }
                    let archived = files.iter().filter(|f| f.category == "ARCHIVED").count();
                    if archived > 0 {
                        ui.label(RichText::new(crate::tf!("{} archivierte Datei(en) ausgeblendet", "{} archived file(s) hidden", archived)).small().color(pal().muted_fg));
                    }
                    if !premium {
                        ui.add_space(4.0);
                        ui.label(RichText::new(t("Ohne Nexus Premium öffnet \"Herunterladen\" die Datei auf Nexus (dort mit deinem kostenlosen Nexus-Konto angemeldet sein). Klicke \"Manual download\" – sobald die Datei in deinem Downloads-Ordner ist, installiert die App sie von selbst. Ein API-Schlüssel ist dafür nicht nötig.", "Without Nexus Premium, \"Download\" opens the file on Nexus (be signed in with your free Nexus account). Click \"Manual download\" – as soon as the file is in your Downloads folder, the app installs it by itself. No API key needed.")).small().color(pal().muted_fg));
                    }
                    ui.add_space(14.0);
                    section_title(ui, t("Beschreibung", "Description"));
                    card(ui, 14, |ui| {
                        ui.label(RichText::new(if desc.is_empty() { t("Keine Beschreibung.", "No description.") } else { desc.as_str() }).color(pal().fg));
                    });
                }
            }
        });
    }

    fn page_downloads(&mut self, ui: &mut Ui) {
        let dir = self.lib.downloads_dir();
        self.page_header(ui, "Downloads", None, t("Laufende und fertige Downloads von Nexus Mods", "Running and finished downloads from Nexus Mods"), |s, ui| {
            if outline_button(ui, t("📁 Download-Ordner", "📁 Download folder")).clicked() {
                let _ = std::fs::create_dir_all(&dir);
                library::open_folder(&dir);
            }
            if outline_button(ui, t("Fertige entfernen", "Clear finished")).clicked() {
                s.downloads.retain(|d| matches!(d.state, DlState::Running | DlState::Waiting));
            }
        });
        if self.downloads.is_empty() {
            empty_state(ui, t("Noch keine Downloads", "No downloads yet"), t("Wähle unter \"Mods entdecken\" einen Mod und lade eine Datei herunter.", "Pick a mod in the \"Mods Store\" and download a file."));
            return;
        }
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for d in self.downloads.iter().rev() {
                card(ui, 12, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&d.name).family(semibold()).color(pal().fg));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let (t, c) = match &d.state {
                                DlState::Waiting => (t("Warte auf deinen Download von Nexus…", "Waiting for your download from Nexus…").to_string(), pal().secondary_fg),
                                DlState::Running => (t("Lädt…", "Downloading…").to_string(), pal().primary),
                                DlState::Done => (t("✔ Installiert", "✔ Installed").to_string(), OK),
                                DlState::Failed(e) => (format!("✖ {e}"), BAD),
                            };
                            ui.label(RichText::new(t).color(c));
                        });
                    });
                    let frac = if d.total > 0 { d.done as f32 / d.total as f32 } else if matches!(d.state, DlState::Done) { 1.0 } else { 0.0 };
                    ui.add(egui::ProgressBar::new(frac).desired_height(6.0).fill(pal().primary));
                    ui.label(RichText::new(format!("{} / {}", fmt_size(d.done), if d.total > 0 { fmt_size(d.total) } else { "?".into() })).small().color(pal().muted_fg));
                });
                ui.add_space(6.0);
            }
        });
    }

    fn page_add(&mut self, ui: &mut Ui) {
        self.page_header(ui, t("Mods hinzufügen", "Add Mods"), None, t("Lade Mod-Dateien hoch. Ziehen & Ablegen wird unterstützt.", "Upload mod files. Drag & drop is supported."), |_, _| {});
        let mut picked = None;
        card(ui, 16, |ui| {
            ui.label(RichText::new(t("Dateien hochladen", "Upload Files")).size(16.0).family(semibold()).color(pal().fg));
            ui.label(RichText::new(t("Lege eine .pak-Datei oder ein .zip-Archiv ab. Die .pak (mit .utoc/.ucas) landet im Paks-Ordner des Spiels.", "Drop a .pak or a .zip archive. The .pak (with .utoc/.ucas) goes into the game's Paks folder.")).color(pal().muted_fg));
            ui.add_space(12.0);
            let hovering = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 200.0), Sense::click());
            let p = ui.painter();
            p.rect_filled(r, CornerRadius::same(RADIUS), if hovering || resp.hovered() { pal().secondary } else { pal().bg });
            dashed_rect(p, r.shrink(1.0), if hovering { pal().primary } else { pal().muted_fg });
            p.text(r.center() - vec2(0.0, 16.0), egui::Align2::CENTER_CENTER, "⬆", egui::FontId::proportional(30.0), pal().primary);
            p.text(r.center() + vec2(0.0, 20.0), egui::Align2::CENTER_CENTER, t("Dateien hier ablegen oder klicken zum Auswählen", "Drop files here, or click to select"), egui::FontId::proportional(15.0), pal().fg);
            p.text(r.center() + vec2(0.0, 42.0), egui::Align2::CENTER_CENTER, ".pak · .zip", egui::FontId::proportional(12.0), pal().muted_fg);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                picked = rfd::FileDialog::new().add_filter("Mods", &["zip", "pak"]).pick_files();
            }
            ui.add_space(8.0);
            ui.label(RichText::new(t(".rar und .7z werden noch nicht unterstützt – bitte vorher entpacken.", ".rar and .7z aren't supported yet – please unpack them first.")).small().color(pal().muted_fg));
        });
        if let Some(files) = picked {
            self.install_local(files);
        }
    }

    fn page_settings(&mut self, ui: &mut Ui) {
        self.page_header(ui, t("Einstellungen", "Settings"), None, t("Sprache, Design, Spielordner und Nexus-Konto", "Language, theme, game folder and Nexus account"), |_, _| {});
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            settings_card(ui, t("Sprache", "Language"), |ui| {
                ui.horizontal(|ui| {
                    for (code, label) in [("de", "Deutsch"), ("en", "English")] {
                        let on = (code == "en") == is_english();
                        if tab_button(ui, label, on).clicked() && !on {
                            self.set_language(code);
                        }
                    }
                });
            });
            settings_card(ui, t("Design", "Theme"), |ui| {
                ui.label(RichText::new(t("Farben der ganzen App.", "Colours of the whole app.")).color(pal().muted_fg));
                ui.add_space(6.0);
                let current = pal().id;
                ui.horizontal_wrapped(|ui| {
                    for th in THEMES {
                        if theme_tile(ui, &th, th.id == current).clicked() {
                            self.lib.settings.theme = th.id.into();
                            set_theme(th.id);
                            apply_theme(ui.ctx());
                            self.lib.save();
                        }
                    }
                });
            });
            settings_card(ui, t("Spiel", "Game"), |ui| {
                ui.label(RichText::new(t("Ordner von Deep Rock Galactic: Rogue Core", "Deep Rock Galactic: Rogue Core folder")).color(pal().muted_fg));
                ui.horizontal(|ui| {
                    let mut dir = self.lib.settings.game_dir.clone();
                    ui.add(egui::TextEdit::singleline(&mut dir).desired_width(460.0).interactive(false));
                    if outline_button(ui, t("Ändern…", "Change…")).clicked() {
                        if let Some(d) = rfd::FileDialog::new().pick_folder() {
                            self.lib.settings.game_dir = d.to_string_lossy().into();
                            self.lib.scan();
                        }
                    }
                    if outline_button(ui, t("Automatisch suchen", "Find automatically")).clicked() {
                        match library::detect_game_dir() {
                            Some(g) => {
                                self.lib.settings.game_dir = g.to_string_lossy().into();
                                self.lib.scan();
                                self.toast(t("Spiel gefunden", "Game found"), false);
                            }
                            None => self.toast(t("Spiel in den Steam-Bibliotheken nicht gefunden", "Game not found in the Steam libraries"), true),
                        }
                    }
                });
                match self.lib.paks_dir() {
                    Some(p) => {
                        if let Some(v) = self.game_version_text() {
                            ui.label(RichText::new(v).color(pal().fg));
                        }
                        ui.label(RichText::new(crate::tf!("✔ Paks-Ordner: {}", "✔ Paks folder: {}", p.display())).small().color(OK))
                    }
                    None => ui.label(RichText::new(t("✖ Kein Paks-Ordner (RogueCore\\Content\\Paks) gefunden", "✖ No Paks folder (RogueCore\\Content\\Paks) found")).small().color(BAD)),
                };
            });
            settings_card(ui, "Nexus Mods", |ui| {
                ui.label(RichText::new(t("Persönlicher API-Schlüssel – nötig zum Herunterladen. Mods ansehen geht auch ohne.", "Personal API key – needed for downloads. Browsing works without it.")).color(pal().muted_fg));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.key_edit).password(true).desired_width(460.0).hint_text(t("API-Schlüssel einfügen", "Paste API key")));
                    if primary_button(ui, t("Speichern & prüfen", "Save & check")).clicked() {
                        self.lib.settings.api_key = self.key_edit.trim().to_string();
                        self.lib.save();
                        if self.lib.settings.api_key.is_empty() {
                            self.user = None;
                            self.user_msg = t("Abgemeldet", "Signed out").into();
                        } else {
                            self.validate_key();
                        }
                    }
                    if outline_button(ui, t("Schlüssel holen", "Get key")).clicked() {
                        library::open_url("https://www.nexusmods.com/users/myaccount?tab=api%20access");
                    }
                });
                if !self.user_msg.is_empty() {
                    ui.label(RichText::new(&self.user_msg).small().color(pal().secondary_fg));
                }
                ui.add_space(8.0);
                ui.label(RichText::new(t("Ohne Premium lädt Nexus nur über den Knopf \"Mod Manager Download\" auf der Mod-Seite. Damit dieser Knopf diese App öffnet, einmal eintragen:", "Without Premium, Nexus only downloads through the \"Mod Manager Download\" button on the mod page. To make that button open this app, register it once:")).small().color(pal().muted_fg));
                if outline_button(ui, t("Als Nexus-Download-Programm (nxm://) eintragen", "Register as Nexus download program (nxm://)")).clicked() {
                    match library::register_nxm_handler() {
                        Ok(()) => self.toast(t("nxm-Links öffnen jetzt diese App", "nxm links now open this app"), false),
                        Err(e) => self.toast(e, true),
                    }
                }
            });
            settings_card(ui, t("Anzeige", "Display"), |ui| {
                if ui.checkbox(&mut self.lib.settings.show_adult, RichText::new(t("Mods für Erwachsene anzeigen", "Show adult mods")).color(pal().fg)).changed() {
                    self.lib.save();
                }
            });
            settings_card(ui, t("App-Updates", "App updates"), |ui| {
                ui.label(RichText::new(crate::tf!("Installierte Version: {}", "Installed version: {}", VERSION)).color(pal().fg));
                if !self.app_update_msg.is_empty() {
                    ui.label(RichText::new(&self.app_update_msg).small().color(pal().muted_fg));
                }
                ui.horizontal(|ui| {
                    if outline_button(ui, t("🔄 Nach Updates suchen", "🔄 Check for updates")).clicked() {
                        self.check_app_update();
                    }
                    if self.app_update.is_some() && !self.app_updating && primary_button(ui, t("⬆ Jetzt aktualisieren", "⬆ Update now")).clicked() {
                        self.install_app_update();
                    }
                    if outline_button(ui, t("GitHub öffnen ↗", "Open GitHub ↗")).clicked() {
                        library::open_url(updater::REPO_URL);
                    }
                });
            });
            settings_card(ui, t("Daten", "Data"), |ui| {
                ui.label(RichText::new(crate::tf!("Bibliothek und Downloads liegen in {}", "Library and downloads are stored in {}", self.lib.data_dir.display())).small().color(pal().muted_fg));
                if outline_button(ui, t("📁 Datenordner öffnen", "📁 Open data folder")).clicked() {
                    library::open_folder(&self.lib.data_dir);
                }
            });
        });
    }

    fn page_wiki(&mut self, ui: &mut Ui) {
        self.page_header(ui, "Wiki", None, t("Das Deep Rock Galactic Wiki zu Rogue Core", "The Deep Rock Galactic wiki for Rogue Core"), |_, ui| {
            if primary_button(ui, t("🌐 Wiki öffnen", "🌐 Open wiki")).clicked() {
                library::open_url(nexus::WIKI_MAIN);
            }
        });
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            card(ui, 18, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new(egui::load::SizedTexture::new(self.logo.id(), vec2(56.0, 56.0))));
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Rogue Core Wiki").size(18.0).family(semibold()).color(pal().fg));
                        ui.label(RichText::new(t("Waffen, Klassen, Gegner, Bio-Booster und mehr – auf deeprockgalactic.wiki.gg.", "Weapons, classes, enemies, bio boosters and more – on deeprockgalactic.wiki.gg.")).color(pal().muted_fg));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if outline_button(ui, t("Hauptseite öffnen ↗", "Open main page ↗")).clicked() {
                            library::open_url(nexus::WIKI_MAIN);
                        }
                    });
                });
            });
            ui.add_space(16.0);
            section_title(ui, t("Wiki durchsuchen", "Search the wiki"));
            let mut go = false;
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut self.wiki_query).hint_text(format!("🔍 {}", t("z. B. Gunner, Bio Booster, Waffe…", "e.g. gunner, bio booster, weapon…"))).desired_width(380.0).margin(Margin::symmetric(8, 6)));
                go = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                go |= primary_button(ui, t("Suchen", "Search")).clicked();
            });
            if go && !self.wiki_query.trim().is_empty() {
                let q = self.wiki_query.trim().to_string();
                self.wiki_results = Some(None);
                self.spawn(move || Msg::Wiki(nexus::wiki_search(&q)));
            }
            ui.add_space(8.0);
            match self.wiki_results.clone() {
                None => {}
                Some(None) => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new(t("Suche im Wiki…", "Searching the wiki…")).color(pal().muted_fg));
                    });
                }
                Some(Some(Err(e))) => notice(ui, &crate::tf!("Wiki nicht erreichbar: {}", "Couldn't reach the wiki: {}", e), pal().destructive),
                Some(Some(Ok(hits))) if hits.is_empty() => {
                    ui.label(RichText::new(t("Keine Treffer.", "No results.")).color(pal().muted_fg));
                }
                Some(Some(Ok(hits))) => {
                    for (title, snippet) in hits {
                        self.wiki_row(ui, &title, &snippet);
                    }
                }
            }
            ui.add_space(16.0);
            section_title(ui, t("Schnellzugriff", "Quick links"));
            let links: [(&str, &str, &str, &str); 6] = [
                ("Rogue Core:Equipment", "Ausrüstung", "Equipment", "⚒"),
                ("Rogue Core:Core Spawn", "Core Spawn", "Core Spawn", "👾"),
                ("Rogue Core:Bio Booster Decks", "Bio-Booster-Decks", "Bio Booster Decks", "💉"),
                ("Rogue Core:Alterations", "Veränderungen", "Alterations", "🔧"),
                ("Rogue Core:Ascension", "Aufstieg", "Ascension", "⬆"),
                ("Rogue Core:Accessories", "Zubehör", "Accessories", "🎒"),
            ];
            grid(ui, links.len(), 230.0, 64.0, |ui, i| {
                let (page, de, en, icon) = links[i];
                let rect = ui.max_rect();
                let resp = ui.interact(rect, ui.id().with(page), Sense::click());
                card_frame(ui, rect, &resp);
                let p = ui.painter();
                p.text(rect.left_center() + vec2(18.0, 0.0), egui::Align2::LEFT_CENTER, icon, egui::FontId::proportional(20.0), pal().primary);
                p.text(rect.left_center() + vec2(50.0, -8.0), egui::Align2::LEFT_CENTER, if is_english() { en } else { de }, egui::FontId::new(14.0, semibold()), pal().fg);
                p.text(rect.left_center() + vec2(50.0, 10.0), egui::Align2::LEFT_CENTER, t("Im Wiki öffnen ↗", "Open in wiki ↗"), egui::FontId::proportional(11.5), pal().muted_fg);
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    library::open_url(&nexus::wiki_url(page));
                }
            });
        });
    }

    fn wiki_row(&mut self, ui: &mut Ui, title: &str, snippet: &str) {
        let shown = title.strip_prefix("Rogue Core:").unwrap_or(title);
        let resp = card(ui, 12, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(shown).family(semibold()).color(pal().fg));
                    ui.add(egui::Label::new(RichText::new(shorten(snippet, 180)).small().color(pal().muted_fg)).wrap());
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| outline_button(ui, t("Öffnen ↗", "Open ↗")).clicked())
                    .inner
            })
            .inner
        });
        if resp {
            library::open_url(&nexus::wiki_url(title));
        }
        ui.add_space(6.0);
    }

    fn delete_dialog(&mut self, ctx: &egui::Context) {
        let Some(key) = self.confirm_delete.clone() else { return };
        let name = self.lib.mods.iter().find(|m| m.key == key).map(|m| m.name.clone()).unwrap_or_default();
        egui::Window::new(t("Mod entfernen?", "Remove mod?"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(egui::Frame::new().fill(pal().card).stroke(Stroke::new(1.0f32, pal().border)).corner_radius(CornerRadius::same(RADIUS)).inner_margin(Margin::same(16)))
            .show(ctx, |ui| {
                ui.label(RichText::new(crate::tf!("\"{}\" wird aus dem Spielordner gelöscht.", "\"{}\" will be deleted from the game folder.", name)).color(pal().fg));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if danger_button(ui, t("Entfernen", "Remove")).clicked() {
                        match self.lib.remove(&key) {
                            Ok(()) => self.toast(crate::tf!("{} entfernt", "{} removed", name), false),
                            Err(e) => self.toast(e, true),
                        }
                        self.confirm_delete = None;
                    }
                    if outline_button(ui, t("Abbrechen", "Cancel")).clicked() {
                        self.confirm_delete = None;
                    }
                });
            });
    }

    fn draw_toasts(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        self.toasts.retain(|t| now - t.1 < 6.0);
        if self.toasts.is_empty() {
            return;
        }
        egui::Area::new("toasts".into()).anchor(egui::Align2::RIGHT_BOTTOM, vec2(-16.0, -36.0)).show(ctx, |ui| {
            for (t, born, err) in &self.toasts {
                let age = (now - born) as f32;
                // Fade in over 0.15 s, out over the last 0.5 s.
                let a = (age / 0.15).min(1.0).min((6.0 - age) / 0.5).clamp(0.0, 1.0);
                ui.scope(|ui| {
                    ui.set_opacity(a);
                    egui::Frame::new()
                        .fill(pal().toast)
                        .stroke(Stroke::new(1.0f32, if *err { pal().destructive } else { pal().border }))
                        .shadow(egui::epaint::Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(90) })
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.set_max_width(360.0);
                            ui.label(RichText::new(t).color(pal().fg));
                        });
                });
                ui.add_space(8.0);
            }
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }

    // ---------- checks ----------

    fn run_self_test(&mut self, ctx: &egui::Context) {
        let pages = [Page::Library, Page::Store, Page::Settings, Page::Downloads, Page::AddMods];
        let f = self.frame as usize;
        if (5..5 + pages.len() * 5).contains(&f) && f % 5 == 0 {
            self.page = pages[(f - 5) / 5];
        }
        if f == 31 {
            let seen = pages.iter().filter(|p| self.visited.contains(p)).count();
            self.test_steps.push(serde_json::json!({"step": "move", "passed": seen == pages.len(), "detail": format!("visited {seen} of {} pages through the sidebar", pages.len())}));
            self.page = Page::Library;
        }
        if f == 32 {
            let r = self.self_test_install();
            self.test_steps.push(match r {
                Ok(d) => serde_json::json!({"step": "act", "passed": true, "detail": d}),
                Err(e) => serde_json::json!({"step": "act", "passed": false, "detail": e}),
            });
        }
        if f == 40 {
            let h = &self.hud;
            let ok = h.sidebar && h.topbar && h.statusbar && h.page_header && self.lib.mods.len() == 1;
            self.test_steps.push(serde_json::json!({"step": "hud", "passed": ok, "detail": format!("sidebar={} launch bar={} status bar={} page header={}, library shows {} mod(s)", h.sidebar, h.topbar, h.statusbar, h.page_header, self.lib.mods.len())}));
            let out = serde_json::json!({"version": 1, "steps": self.test_steps});
            if let Some(p) = &self.out_file {
                let _ = std::fs::write(p, serde_json::to_string_pretty(&out).unwrap());
            }
            let _ = std::fs::remove_dir_all(&self.lib.data_dir);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Installs a generated .zip into a throw-away game folder and flips it off and on.
    fn self_test_install(&mut self) -> Result<String, String> {
        let zip_path = self.lib.data_dir.join("TestMod.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&zip_path).map_err(|e| e.to_string())?);
            let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            z.start_file("TestMod/TestMod_P.pak", o).map_err(|e| e.to_string())?;
            std::io::Write::write_all(&mut z, b"pak").map_err(|e| e.to_string())?;
            z.start_file("TestMod/readme.txt", o).map_err(|e| e.to_string())?;
            std::io::Write::write_all(&mut z, b"hi").map_err(|e| e.to_string())?;
            z.finish().map_err(|e| e.to_string())?;
        }
        let paks = self.lib.paks_dir().ok_or("no Paks folder")?;
        let key = self.lib.install(&zip_path, InstalledMod { name: "Test Mod".into(), ..Default::default() })?;
        let installed = paks.join("TestMod_P.pak").exists() && !paks.join("readme.txt").exists();
        self.lib.set_enabled(&key, false)?;
        let off = paks.join("TestMod_P.pak.disabled").exists() && !paks.join("TestMod_P.pak").exists();
        self.lib.set_enabled(&key, true)?;
        let on = paks.join("TestMod_P.pak").exists();
        if installed && off && on {
            Ok("installed a .zip into Paks, switched it off (.disabled) and on again".into())
        } else {
            Err(format!("installed={installed} off={off} on={on}"))
        }
    }
}

impl eframe::App for App {
    // eframe's default is near-black, which shows as dark seams between panels in the light theme.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        pal().bg.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame += 1;
        self.poll();
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() {
            self.install_local(dropped);
        }
        self.top_bar(ctx);
        self.status_bar(ctx);
        self.sidebar(ctx);
        // Each page fades and slides in briefly when it opens.
        let now = ctx.input(|i| i.time);
        if self.page != self.shown_page {
            self.shown_page = self.page;
            self.page_since = now;
        }
        let k = ((now - self.page_since) / 0.22).clamp(0.0, 1.0) as f32;
        let ease = 1.0 - (1.0 - k).powi(3);
        if k < 1.0 {
            ctx.request_repaint();
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(pal().bg).inner_margin(Margin::symmetric(24, 20))).show(ctx, |ui| {
            ui.set_opacity(ease);
            ui.add_space((1.0 - ease) * 10.0);
            self.visited.insert(self.page);
            match self.page {
                Page::Library => self.page_library(ui),
                Page::Store => self.page_store(ui),
                Page::Detail(id) => self.page_detail(ui, id),
                Page::Downloads => self.page_downloads(ui),
                Page::AddMods => self.page_add(ui),
                Page::Settings => self.page_settings(ui),
                Page::Wiki => self.page_wiki(ui),
            }
        });
        self.delete_dialog(ctx);
        self.draw_toasts(ctx);
        if self.downloads.iter().any(|d| matches!(d.state, DlState::Running)) || self.thumbs.values().any(|t| matches!(t, Thumb::Loading)) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        match self.mode {
            Mode::Normal => {}
            Mode::SelfTest => {
                ctx.request_repaint();
                self.run_self_test(ctx);
            }
            Mode::Screenshot => {
                ctx.request_repaint();
                if self.frame == 120 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                }
                let shot = ctx.input(|i| {
                    i.raw.events.iter().find_map(|e| match e {
                        egui::Event::Screenshot { image, .. } => Some(image.clone()),
                        _ => None,
                    })
                });
                if let (Some(img), Some(p)) = (shot, &self.out_file) {
                    let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b(), 255]).collect();
                    if let Some(dir) = p.parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    let _ = image::save_buffer(p, &bytes, img.size[0] as u32, img.size[1] as u32, image::ColorType::Rgba8);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if self.frame > 600 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }
}

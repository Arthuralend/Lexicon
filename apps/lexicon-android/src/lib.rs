//! Lexicon for Android.
//!
//! Runs the same [`wordcraft_ui_egui::WordApp`] as the desktop app through eframe on a
//! `NativeActivity` (`android/app/src/main/java/.../LexiconActivity.java`). Differences from the
//! desktop app:
//!
//! - **Typing** comes from the on-screen keyboard through the activity's own input connection
//!   (`keyboard`): winit's NativeActivity backend has no IME support, so composing text, committed
//!   text, deletions and editing keys arrive over JNI and are fed to egui as IME and key events. The
//!   keyboard shows while the document (or a text field) has the caret, and hides otherwise.
//! - **Files** open and save through the built-in file browser, starting in shared storage
//!   (`/storage/emulated/0`) when the user allowed Lexicon to manage files, or else in the app's own
//!   folder (`Android/data/io.github.arthuralend.lexicon/files`).
//! - **Preferences** (`ui.json`) are written whenever they change: Android may stop the app
//!   without running exit code.
//!
//! Build: `cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build --release -p lexicon-android`,
//! then `gradle assembleDebug` in `android/` (see `.github/workflows/build.yml`).
#![cfg(target_os = "android")]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod keyboard;

use std::path::{Path, PathBuf};

use winit::platform::android::activity::AndroidApp;
use wordcraft_engine::Session;
use wordcraft_ui_egui::{FileDialogAnswer, FileDialogRequest, Services, UiState, WordApp};

/// Behind the status and navigation bars (their icons are light).
const SYSTEM_BARS: egui::Color32 = egui::Color32::from_rgb(0x1E, 0x2A, 0x5A);

/// Shared storage, browsed when the user lets Lexicon manage files.
const SHARED_STORAGE: &str = "/storage/emulated/0";

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("lexicon"));
    let settings = setup_folders(&app);
    let keyboard = keyboard::Keyboard::new(&app);
    let options = eframe::NativeOptions { android_app: Some(app), ..Default::default() };
    let result = eframe::run_native(
        "Lexicon",
        options,
        Box::new(move |cc| {
            keyboard::set_context(cc.egui_ctx.clone());
            let region = wordcraft_ui_egui::i18n::system_region();
            let mut session = Session::new(wordcraft_engine::blank_document(&region));
            session.system_region = region;
            let mut app = WordApp::new(session, services());
            let stored = settings.as_deref().and_then(|dir| std::fs::read(dir.join("ui.json")).ok());
            if let Some(ui) = stored.as_deref().and_then(UiState::from_json) {
                app.apply_prefs(ui);
            }
            if let Some(dir) = &settings {
                let words = dir.join(Session::USER_DICT_FILE);
                wordcraft_proof::load_user_dictionary(&words);
                app.session.user_dict_path = Some(words);
                let ctx = cc.egui_ctx.clone();
                if let Err(e) = app.session.enable_recovery(dir.join("recovery"), std::sync::Arc::new(move || ctx.request_repaint())) {
                    log::warn!("local recovery unavailable: {e}");
                }
            }
            if app.session.prefs.region.is_some() {
                let doc = app.session.blank_document();
                app.session.set_document(doc);
            }
            app.start_screen();
            let last_prefs = stored.map(|b| String::from_utf8_lossy(&b).into_owned());
            Ok(Box::new(Shell { app, keyboard, settings, last_prefs }))
        }),
    );
    if let Err(e) = result {
        log::error!("Lexicon failed to start: {e}");
    }
}

/// Point the settings folder (`$XDG_CONFIG_HOME/wordcraft`) at the app's private storage and the
/// home folder the file browser starts in at shared storage (or the app's own folder). Runs first
/// thing, before eframe starts its threads. Returns the settings folder.
fn setup_folders(app: &AndroidApp) -> Option<PathBuf> {
    let internal = app.internal_data_path();
    let external = app.external_data_path();
    let home =
        if std::fs::read_dir(SHARED_STORAGE).is_ok() { Some(PathBuf::from(SHARED_STORAGE)) } else { external.clone().or_else(|| internal.clone()) };
    // SAFETY: called at the very start of `android_main`, before this process's Rust code starts
    // any other thread that could read the environment.
    unsafe {
        if let Some(dir) = &internal {
            std::env::set_var("XDG_CONFIG_HOME", dir);
        }
        if let Some(dir) = &home {
            std::env::set_var("HOME", dir);
        }
    }
    if let Some(dir) = external.as_deref().map(|d| d.join("Documents")) {
        let _ = std::fs::create_dir_all(dir);
    }
    let settings = wordcraft_control_key::settings_dir();
    if let Some(dir) = &settings
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        log::warn!("settings: cannot create {}: {e}", dir.display());
    }
    log::info!("home {home:?}, settings {settings:?}");
    settings
}

/// No system file dialog: every Open and Save goes to the built-in file browser.
fn services() -> Services {
    Services {
        file_dialog: Some(Box::new(|_req: FileDialogRequest| {
            let (tx, rx) = std::sync::mpsc::channel::<FileDialogAnswer>();
            let _ = tx.send(Err("Android has no system file dialog".into()));
            rx
        })),
        ..Default::default()
    }
}

struct Shell {
    app: WordApp,
    keyboard: keyboard::Keyboard,
    settings: Option<PathBuf>,
    /// The preferences last written, so `ui.json` is written only when they change.
    last_prefs: Option<String>,
}

impl Shell {
    fn save_prefs(&mut self) {
        let Some(dir) = &self.settings else { return };
        let json = match serde_json::to_string_pretty(&self.app.prefs()) {
            Ok(j) => j,
            Err(e) => {
                log::warn!("preferences: cannot serialize: {e}");
                return;
            }
        };
        if self.last_prefs.as_deref() == Some(json.as_str()) {
            return;
        }
        if let Err(e) = write_atomically(&dir.join("ui.json"), json.as_bytes()) {
            log::warn!("preferences: cannot save: {e}");
        }
        self.last_prefs = Some(json);
    }
}

/// Write to a temporary file and rename it over `path`, so an interrupted write keeps the old file.
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    let written = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

impl eframe::App for Shell {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.logic(ctx);
        self.save_prefs();
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.keyboard.fix_hardware_text(&mut raw.events);
        raw.events.extend(keyboard::take_events());
        self.app.raw_input_hook(raw);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The app draws under the status bar, the navigation bar and the keyboard: keep its
        // interface in the part they leave free, and paint behind the bars.
        let full = ui.max_rect();
        let inner = keyboard::safe_area(full, ui.ctx().pixels_per_point());
        if inner != full {
            ui.painter().rect_filled(full, 0.0, SYSTEM_BARS);
        }
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| self.app.ui(ui));
        // The caret is in the document or a text field: the on-screen keyboard is wanted.
        let wants = ui.ctx().output(|o| o.ime.is_some());
        let tapped = ui.ctx().input(|i| i.pointer.any_click());
        self.keyboard.update(wants, tapped);
    }

    fn on_exit(&mut self) {
        self.save_prefs();
    }
}

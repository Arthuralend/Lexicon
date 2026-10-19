//! The on-screen keyboard.
//!
//! `LexiconActivity.java` owns an invisible view with an input connection. What the keyboard does
//! there arrives through the `Java_…_native*` functions below (on Android's UI thread), is queued as
//! egui events, and is handed to egui by the shell's `raw_input_hook` (on the app's thread):
//!
//! | Keyboard (`InputConnection`) | egui event |
//! |---|---|
//! | `setComposingText` | `Ime(Preedit)` |
//! | `commitText`, `finishComposingText` | `Ime(Commit)` |
//! | `deleteSurroundingText` | `Ime(DeleteSurrounding)` |
//! | `sendKeyEvent` (Backspace, Enter, arrows…) | `Key` pressed and released |
//!
//! [`Keyboard::update`] shows the keyboard (through `LexiconActivity.showKeyboard`) while egui
//! wants text input and hides it when it stops.

use std::sync::{Mutex, OnceLock};

use jni::JNIEnv;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::jint;
use winit::platform::android::activity::AndroidApp;

/// Most events held between frames (a stuck UI shouldn't grow the queue without bound).
const MAX_QUEUED: usize = 4096;

static EVENTS: Mutex<Vec<egui::Event>> = Mutex::new(Vec::new());
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

/// The egui context to wake when the keyboard sends something.
pub fn set_context(ctx: egui::Context) {
    let _ = CONTEXT.set(ctx);
}

/// The keyboard's events since the last frame.
pub fn take_events() -> Vec<egui::Event> {
    std::mem::take(&mut *EVENTS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn push(events: impl IntoIterator<Item = egui::Event>) {
    {
        let mut q = EVENTS.lock().unwrap_or_else(|e| e.into_inner());
        for e in events {
            if q.len() < MAX_QUEUED {
                q.push(e);
            }
        }
    }
    if let Some(ctx) = CONTEXT.get() {
        ctx.request_repaint();
    }
}

fn text(env: &mut JNIEnv, s: &JString) -> Option<String> {
    env.get_string(s).ok().map(String::from)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_arthuralend_lexicon_LexiconActivity_nativePreedit(mut env: JNIEnv, _class: JClass, s: JString) {
    if let Some(t) = text(&mut env, &s) {
        push([egui::Event::Ime(egui::ImeEvent::Preedit { text: t, active_range_chars: None })]);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_arthuralend_lexicon_LexiconActivity_nativeCommit(mut env: JNIEnv, _class: JClass, s: JString) {
    if let Some(t) = text(&mut env, &s) {
        // A new line from the keyboard is the Enter key, which starts a new paragraph.
        if t == "\n" {
            push(key(egui::Key::Enter, false));
        } else {
            push([egui::Event::Ime(egui::ImeEvent::Commit(t))]);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_arthuralend_lexicon_LexiconActivity_nativeDelete(_env: JNIEnv, _class: JClass, before: jint, after: jint) {
    let before_chars = usize::try_from(before).unwrap_or(0);
    let after_chars = usize::try_from(after).unwrap_or(0);
    if before_chars > 0 || after_chars > 0 {
        push([egui::Event::Ime(egui::ImeEvent::DeleteSurrounding { before_chars, after_chars })]);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_arthuralend_lexicon_LexiconActivity_nativeKey(_env: JNIEnv, _class: JClass, code: jint, shift: jni::sys::jboolean) {
    if let Some(k) = android_key(code) {
        push(key(k, shift != 0));
    }
}

/// A key pressed and released.
fn key(key: egui::Key, shift: bool) -> [egui::Event; 2] {
    let modifiers = egui::Modifiers { shift, ..Default::default() };
    let event = |pressed| egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers };
    [event(true), event(false)]
}

/// The editing keys an on-screen keyboard sends (Android `KeyEvent.KEYCODE_*`).
fn android_key(code: jint) -> Option<egui::Key> {
    Some(match code {
        19 => egui::Key::ArrowUp,
        20 => egui::Key::ArrowDown,
        21 => egui::Key::ArrowLeft,
        22 => egui::Key::ArrowRight,
        61 => egui::Key::Tab,
        66 | 160 => egui::Key::Enter,
        67 => egui::Key::Backspace,
        92 => egui::Key::PageUp,
        93 => egui::Key::PageDown,
        111 => egui::Key::Escape,
        112 => egui::Key::Delete,
        122 => egui::Key::Home,
        123 => egui::Key::End,
        _ => return None,
    })
}

/// Shows and hides the keyboard through the activity.
pub struct Keyboard {
    app: AndroidApp,
    shown: bool,
}

impl Keyboard {
    pub fn new(app: &AndroidApp) -> Self {
        Keyboard { app: app.clone(), shown: false }
    }

    /// Called every frame: `wants` is whether egui wants text input, `tapped` whether the user
    /// tapped this frame. A tap where text is wanted shows the keyboard again after the user
    /// dismissed it with Back.
    pub fn update(&mut self, wants: bool, tapped: bool) {
        if wants && (!self.shown || tapped) {
            self.show(true);
        } else if !wants && self.shown {
            self.show(false);
        }
    }

    fn show(&mut self, show: bool) {
        match self.call_activity(show) {
            Ok(()) => self.shown = show,
            Err(e) => {
                log::warn!("keyboard: {e}");
                // Don't retry every frame.
                self.shown = show;
            }
        }
    }

    fn call_activity(&self, show: bool) -> Result<(), String> {
        // SAFETY: android-activity hands out the process's JavaVM and a global reference to the
        // activity, both valid while the app runs; `JObject` doesn't release the reference.
        let vm = unsafe { jni::JavaVM::from_raw(self.app.vm_as_ptr().cast()) }.map_err(|e| e.to_string())?;
        let mut env = vm.attach_current_thread_permanently().map_err(|e| e.to_string())?;
        let activity = unsafe { JObject::from_raw(self.app.activity_as_ptr().cast()) };
        let result = env.call_method(&activity, "showKeyboard", "(Z)V", &[JValue::Bool(u8::from(show))]);
        if result.is_err() {
            // A Java exception must be cleared before the next JNI call.
            let _ = env.exception_describe();
            let _ = env.exception_clear();
        }
        result.map(|_| ()).map_err(|e| e.to_string())
    }
}

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

use std::sync::atomic::{AtomicI32, Ordering};
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
pub extern "system" fn Java_io_github_arthuralend_lexicon_LexiconActivity_nativeKey(
    _env: JNIEnv,
    _class: JClass,
    code: jint,
    shift: jni::sys::jboolean,
) {
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
        with_activity(&self.app, |env, activity| env.call_method(activity, "showKeyboard", "(Z)V", &[JValue::Bool(u8::from(show))]).map(|_| ()))
    }

    /// Text typed on a hardware keyboard, looked up again in the keyboard's current layout.
    ///
    /// android-activity keeps each keyboard's character map from its first key press, so after
    /// the user switches the layout (Russian to English, Ctrl+Space…) winit still sends the
    /// letters of the first one. For each key press winit turned into text, the activity looks the
    /// key up in the layout that is active now (`LexiconActivity.keyText`).
    pub fn fix_hardware_text(&self, events: &mut [egui::Event]) {
        let mut pressed: Option<(egui::Key, egui::Modifiers)> = None;
        for e in events.iter_mut() {
            match e {
                egui::Event::Key { physical_key, key, pressed: true, modifiers, .. } => pressed = Some((physical_key.unwrap_or(*key), *modifiers)),
                egui::Event::Text(text) => {
                    if let Some((k, m)) = pressed.take()
                        && let Some(code) = android_keycode(k)
                        && let Some(now) = self.key_text(code, m.shift)
                        && now != *text
                    {
                        // Caps Lock (which egui doesn't report) still capitalises.
                        let caps = !m.shift && text.chars().any(char::is_uppercase) && !text.chars().any(char::is_lowercase);
                        *text = if caps { now.to_uppercase() } else { now };
                    }
                }
                _ => pressed = None,
            }
        }
    }

    fn key_text(&self, code: jint, shift: bool) -> Option<String> {
        let mut out = None;
        let result = with_activity(&self.app, |env, activity| {
            let v = env.call_method(activity, "keyText", "(IZ)Ljava/lang/String;", &[JValue::Int(code), JValue::Bool(u8::from(shift))])?.l()?;
            if !v.is_null() {
                let s = JString::from(v);
                out = env.get_string(&s).ok().map(String::from);
            }
            Ok(())
        });
        if let Err(e) = result {
            log::warn!("keyboard: {e}");
        }
        out.filter(|s| !s.is_empty() && s.chars().all(|c| !c.is_control()))
    }
}

/// Show the pointer shape egui asks for (`LexiconActivity.setCursor`): Lexicon's own arrow by
/// default, Android's I-beam over text, hand over links, and so on.
pub fn set_cursor(app: &AndroidApp, icon: egui::CursorIcon) {
    use egui::CursorIcon as C;
    let kind: jint = match icon {
        C::Default => 0,
        C::Text | C::VerticalText => 1,
        C::PointingHand => 2,
        C::ResizeHorizontal | C::ResizeEast | C::ResizeWest | C::ResizeColumn => 3,
        C::ResizeVertical | C::ResizeNorth | C::ResizeSouth | C::ResizeRow => 4,
        C::ResizeNeSw | C::ResizeNorthEast | C::ResizeSouthWest => 5,
        C::ResizeNwSe | C::ResizeNorthWest | C::ResizeSouthEast => 6,
        C::Grab => 7,
        C::Grabbing => 8,
        C::Move | C::AllScroll => 9,
        C::Crosshair | C::Cell => 10,
        C::Wait | C::Progress => 11,
        C::NotAllowed | C::NoDrop => 12,
        C::ContextMenu => 13,
        C::Copy => 14,
        C::Alias => 15,
        C::Help => 16,
        C::ZoomIn => 17,
        C::ZoomOut => 18,
        C::None => 19,
    };
    if let Err(e) = with_activity(app, |env, activity| env.call_method(activity, "setCursor", "(I)V", &[JValue::Int(kind)]).map(|_| ())) {
        log::warn!("cursor: {e}");
    }
}

/// Put text on the system clipboard (Copy, Cut): egui's own clipboard isn't there on Android.
pub fn set_clipboard(app: &AndroidApp, text: &str) {
    let result = with_activity(app, |env, activity| {
        let s = env.new_string(text)?;
        env.call_method(activity, "setClipboardText", "(Ljava/lang/String;)V", &[JValue::Object(&s)]).map(|_| ())
    });
    if let Err(e) = result {
        log::warn!("clipboard: {e}");
    }
}

/// The text on the system clipboard (Paste), if any.
pub fn clipboard(app: &AndroidApp) -> Option<String> {
    let mut out = None;
    let result = with_activity(app, |env, activity| {
        let v = env.call_method(activity, "getClipboardText", "()Ljava/lang/String;", &[])?.l()?;
        if !v.is_null() {
            out = env.get_string(&JString::from(v)).ok().map(String::from);
        }
        Ok(())
    });
    if let Err(e) = result {
        log::warn!("clipboard: {e}");
    }
    out.filter(|s| !s.is_empty())
}

/// Ctrl+V (or Shift+Insert) from a hardware keyboard: egui-winit reads its own clipboard, which
/// Android lacks, so add the paste with the system clipboard's text.
pub fn add_paste(app: &AndroidApp, events: &mut Vec<egui::Event>) {
    let wants_paste = events.iter().any(|e| {
        matches!(e, egui::Event::Key { key: egui::Key::V, pressed: true, modifiers, .. } if modifiers.command || modifiers.ctrl)
            || matches!(e, egui::Event::Key { key: egui::Key::Insert, pressed: true, modifiers, .. } if modifiers.shift)
    });
    if wants_paste
        && !events.iter().any(|e| matches!(e, egui::Event::Paste(_)))
        && let Some(text) = clipboard(app)
    {
        events.push(egui::Event::Paste(text));
    }
}

/// Run `f` with a JNI environment and the activity.
pub(crate) fn with_activity(app: &AndroidApp, f: impl FnOnce(&mut JNIEnv, &JObject) -> jni::errors::Result<()>) -> Result<(), String> {
    // SAFETY: android-activity hands out the process's JavaVM and a global reference to the
    // activity, both valid while the app runs; `JObject` doesn't release the reference.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }.map_err(|e| e.to_string())?;
    let mut env = vm.attach_current_thread_permanently().map_err(|e| e.to_string())?;
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
    let result = f(&mut env, &activity);
    if result.is_err() {
        // A Java exception must be cleared before the next JNI call.
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    result.map_err(|e| e.to_string())
}

/// The Android `KeyEvent.KEYCODE_*` of a key that types a character.
fn android_keycode(key: egui::Key) -> Option<jint> {
    use egui::Key as K;
    let letters = [
        K::A,
        K::B,
        K::C,
        K::D,
        K::E,
        K::F,
        K::G,
        K::H,
        K::I,
        K::J,
        K::K,
        K::L,
        K::M,
        K::N,
        K::O,
        K::P,
        K::Q,
        K::R,
        K::S,
        K::T,
        K::U,
        K::V,
        K::W,
        K::X,
        K::Y,
        K::Z,
    ];
    let digits = [K::Num0, K::Num1, K::Num2, K::Num3, K::Num4, K::Num5, K::Num6, K::Num7, K::Num8, K::Num9];
    if let Some(i) = letters.iter().position(|k| *k == key) {
        return jint::try_from(i).ok().map(|i| 29 + i);
    }
    if let Some(i) = digits.iter().position(|k| *k == key) {
        return jint::try_from(i).ok().map(|i| 7 + i);
    }
    Some(match key {
        K::Backtick => 68,
        K::Minus => 69,
        K::Equals => 70,
        K::OpenBracket => 71,
        K::CloseBracket => 72,
        K::Backslash => 73,
        K::Semicolon => 74,
        K::Quote => 75,
        K::Slash => 76,
        K::Comma => 55,
        K::Period => 56,
        _ => return None,
    })
}

/// The system bars', the display cutout's and the on-screen keyboard's insets in physical pixels
/// (left, top, right, bottom), as `LexiconActivity` last reported them.
static INSETS: [AtomicI32; 4] = [AtomicI32::new(0), AtomicI32::new(0), AtomicI32::new(0), AtomicI32::new(0)];

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_arthuralend_lexicon_LexiconActivity_nativeInsets(
    _env: JNIEnv,
    _class: JClass,
    left: jint,
    top: jint,
    right: jint,
    bottom: jint,
) {
    for (slot, v) in INSETS.iter().zip([left, top, right, bottom]) {
        slot.store(v.max(0), Ordering::Relaxed);
    }
    if let Some(ctx) = CONTEXT.get() {
        ctx.request_repaint();
    }
}

/// The part of `full` (the window, in points) that the system bars and the keyboard leave free.
pub fn safe_area(full: egui::Rect, pixels_per_point: f32) -> egui::Rect {
    let [l, t, r, b] = INSETS.each_ref().map(|v| v.load(Ordering::Relaxed) as f32 / pixels_per_point.max(0.1));
    let inner = egui::Rect::from_min_max(egui::pos2(full.min.x + l, full.min.y + t), egui::pos2(full.max.x - r, full.max.y - b));
    // Insets that leave next to nothing are wrong (a report from a moment of rotation): ignore them.
    if inner.width() < 120.0 || inner.height() < 120.0 { full } else { inner }
}

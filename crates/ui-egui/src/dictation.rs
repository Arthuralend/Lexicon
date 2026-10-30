//! Lexicon: Home › Dictate with the operating system's speech tools.
//!
//! The host starts recognition (`Services::dictate`) and hands back what was heard
//! (`Services::heard`). While a phrase is spoken its words show on the status line; once it is
//! final (or after a pause) the words are typed at the caret and listening goes on until the
//! Dictate button is pressed again. Where the system types by itself (Windows voice typing,
//! Android's speech screen) no words come back here.

use serde_json::json;

use crate::WordApp;

/// How the host started dictation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Started {
    /// The host listens and reports [`Heard`] events.
    Listening,
    /// The system's own voice typing types into the window.
    System,
    /// The user is asked to allow speech recognition; [`Heard::Authorized`] follows.
    AskingPermission,
}

/// What the host's recogniser reported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Heard {
    Partial(String),
    Final(String),
    Error(String),
    Authorized(bool),
}

/// A pause this long (ms) after the last words ends the phrase.
const PAUSE_MS: f64 = 1800.0;

#[derive(Default)]
pub struct DictationState {
    /// Listening (the Dictate button is on).
    pub active: bool,
    /// The phrase heard so far, and when it last changed.
    partial: String,
    partial_at: f64,
    /// The phrase is being finished (waiting for its final words).
    finishing: bool,
}

/// Dictate pressed: start listening, or stop.
pub(crate) fn toggle(app: &mut WordApp) {
    if app.dictation.active {
        stop(app);
        app.status(tl!("Dictation stopped."));
        return;
    }
    start(app);
}

fn start(app: &mut WordApp) {
    let lang = crate::i18n::Lang::from_pref(&app.ui.language).code().to_string();
    let started = app.services.dictate.as_ref().map(|f| f(&lang));
    match started {
        Some(Ok(Started::Listening)) => {
            app.dictation.active = true;
            app.dictation.partial.clear();
            app.dictation.finishing = false;
            app.status(tl!("Listening… Speak now; your words appear at the caret."));
        }
        Some(Ok(Started::System)) => app.status(tl!("The system's voice typing is open: speak, and the words are typed at the caret.")),
        Some(Ok(Started::AskingPermission)) => app.status(tl!("Allow speech recognition and the microphone when macOS asks.")),
        Some(Err(e)) if !e.is_empty() => app.status(e),
        _ if cfg!(windows) => app.status(tl!("Press the Windows logo key + H to dictate with Windows voice typing.")),
        _ => app.status(tl!("Voice typing isn't available on this system.")),
    }
}

fn stop(app: &mut WordApp) {
    app.dictation.active = false;
    if let Some(f) = app.services.dictate_stop.as_ref() {
        f();
    }
}

/// Each frame: type what was heard, end phrases after a pause, keep listening.
pub(crate) fn poll(app: &mut WordApp, now: f64) {
    let heard = app.services.heard.as_ref().map(|f| f()).unwrap_or_default();
    for h in heard {
        match h {
            Heard::Partial(text) => {
                if text != app.dictation.partial {
                    app.dictation.partial = text.clone();
                    app.dictation.partial_at = now;
                }
                if app.dictation.active {
                    app.status(format!("● {text}"));
                }
            }
            Heard::Final(text) => {
                let text = text.trim();
                if !text.is_empty() {
                    let _ = app.run("text.insert", json!({"text": format!("{text} ")}));
                }
                app.dictation.partial.clear();
                app.dictation.finishing = false;
                // Listening goes on with the next phrase.
                if app.dictation.active {
                    app.dictation.active = false;
                    start(app);
                }
            }
            Heard::Error(e) => {
                // A phrase ended without words is not an error worth showing.
                let had_words = !app.dictation.partial.is_empty();
                app.dictation.partial.clear();
                app.dictation.finishing = false;
                if app.dictation.active && !had_words {
                    app.dictation.active = false;
                    start(app);
                } else if app.dictation.active {
                    app.dictation.active = false;
                    app.status(e);
                }
            }
            Heard::Authorized(true) => start(app),
            Heard::Authorized(false) => {
                app.dictation.active = false;
                app.status(tl!("Speech recognition isn't allowed: turn it on in System Settings › Privacy & Security."));
            }
        }
    }
    if app.dictation.active && !app.dictation.finishing && !app.dictation.partial.is_empty() && now - app.dictation.partial_at > PAUSE_MS {
        app.dictation.finishing = true;
        if let Some(f) = app.services.dictate_stop.as_ref() {
            f();
        }
    }
    if app.dictation.active
        && let Some(ctx) = &app.ctx
    {
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use wordcraft_engine::Session;

    #[test]
    fn heard_words_are_typed_and_listening_goes_on() {
        let queue: Rc<RefCell<Vec<Heard>>> = Rc::default();
        let starts = Rc::new(RefCell::new(0));
        let stops = Rc::new(RefCell::new(0));
        let (q, s1, s2) = (queue.clone(), starts.clone(), stops.clone());
        let services = crate::Services {
            dictate: Some(Box::new(move |_| {
                *s1.borrow_mut() += 1;
                Ok(Started::Listening)
            })),
            dictate_stop: Some(Box::new(move || *s2.borrow_mut() += 1)),
            heard: Some(Box::new(move || std::mem::take(&mut *q.borrow_mut()))),
            ..Default::default()
        };
        let mut a = WordApp::new(Session::new(wordcraft_doc::Document::new()), services);
        a.run("tools.dictate", json!({})).unwrap();
        assert!(a.dictation.active);
        queue.borrow_mut().push(Heard::Partial("привет".into()));
        poll(&mut a, 1000.0);
        assert!(a.session.doc.plain_text(wordcraft_doc::StoryRef::Body).trim().is_empty(), "partial words aren't typed");
        poll(&mut a, 1000.0 + PAUSE_MS + 1.0);
        assert_eq!(*stops.borrow(), 1, "a pause ends the phrase");
        queue.borrow_mut().push(Heard::Final("привет мир".into()));
        poll(&mut a, 5000.0);
        assert!(a.session.doc.plain_text(wordcraft_doc::StoryRef::Body).contains("привет мир"));
        assert_eq!(*starts.borrow(), 2, "listening goes on");
        a.run("tools.dictate", json!({})).unwrap();
        assert!(!a.dictation.active, "pressing Dictate again stops");
    }
}

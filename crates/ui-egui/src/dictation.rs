//! Asynchronous platform dictation. Results are previewed before insertion and never applied to
//! a document/selection that changed while the microphone was listening.
use crate::WordApp;
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, TryRecvError},
};

pub type Hook = Box<dyn Fn(String, Arc<AtomicBool>) -> Receiver<Result<String, String>>>;

pub struct Capture {
    receiver: Receiver<Result<String, String>>,
    cancel: Arc<AtomicBool>,
    document: u64,
    revision: u64,
    selection: wordcraft_engine::Selection,
    result: Option<Result<String, String>>,
}
impl Capture {
    fn can_insert(&self, app: &WordApp) -> bool {
        self.document == app.session.document_id() && self.revision == app.session.rev() && self.selection == app.session.sel
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

pub fn start(app: &mut WordApp) -> Result<(), String> {
    if app.dictation.is_some() {
        app.dictation = None;
        return Ok(());
    }
    let hook = app.services.dictation.as_ref().ok_or_else(|| tl!("Dictation is unavailable on this platform.").to_owned())?;
    let cancel = Arc::new(AtomicBool::new(false));
    let lang = crate::i18n::Lang::from_pref(&app.ui.language).editing_lang().unwrap_or("en-US").to_string();
    app.dictation = Some(Capture {
        receiver: hook(lang, cancel.clone()),
        cancel,
        document: app.session.document_id(),
        revision: app.session.rev(),
        selection: app.session.sel.clone(),
        result: None,
    });
    Ok(())
}

pub fn show(app: &mut WordApp, ctx: &egui::Context) {
    let Some(mut capture) = app.dictation.take() else { return };
    if capture.result.is_none() {
        match capture.receiver.try_recv() {
            Ok(result) => capture.result = Some(result),
            Err(TryRecvError::Disconnected) => capture.result = Some(Err(tl!("Speech recognition stopped.").into())),
            Err(TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
        }
    }
    let unchanged = capture.can_insert(app);
    let mut open = true;
    let mut insert = None;
    let mut cancel = false;
    egui::Window::new(tl!("Dictate")).id(egui::Id::new("lexicon_dictation")).open(&mut open).resizable(false).show(ctx, |ui| {
        ui.set_min_width(340.0);
        match &mut capture.result {
            None => {
                ui.spinner();
                ui.label(tl!("Listening… Speak a sentence, then pause."));
            }
            Some(Err(error)) => {
                ui.label(error.as_str());
            }
            Some(Ok(text)) => {
                ui.add(egui::TextEdit::multiline(text).desired_rows(5).desired_width(340.0));
                if !unchanged {
                    ui.label(tl!("The document changed. Copy this text or start dictation again."));
                }
                if ui.add_enabled(unchanged && !text.trim().is_empty(), egui::Button::new(tl!("Insert"))).clicked() {
                    insert = Some(text.clone());
                }
                if ui.button(tl!("Copy")).clicked() {
                    ui.ctx().copy_text(text.clone());
                }
            }
        }
        cancel = ui.button(tl!("Cancel")).clicked();
    });
    if let Some(text) = insert {
        if let Err(e) = app.run("text.insert", json!({"text": text})) {
            app.status(e);
            app.dictation = Some(capture);
            return;
        }
        app.canvas.want_focus = true;
    } else if open && !cancel {
        app.dictation = Some(capture);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_result_cannot_overwrite_a_replacement_or_edited_document() {
        let mut app = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        let (_tx, rx) = std::sync::mpsc::channel();
        let capture = Capture {
            receiver: rx,
            cancel: Arc::new(AtomicBool::new(false)),
            document: app.session.document_id(),
            revision: app.session.rev(),
            selection: app.session.sel.clone(),
            result: Some(Ok("Dictated text".into())),
        };
        assert!(capture.can_insert(&app));
        app.run("text.insert", json!({"text": "Typed meanwhile"})).unwrap();
        assert!(!capture.can_insert(&app));
        app.session.set_document(wordcraft_doc::Document::new());
        assert!(!capture.can_insert(&app));
        assert!(app.session.doc.plain_text(wordcraft_doc::StoryRef::Body).is_empty());
    }

    #[test]
    fn cancelled_capture_notifies_the_platform() {
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let s = wordcraft_engine::Session::new(wordcraft_doc::Document::new());
        let capture =
            Capture { receiver: rx, cancel: cancel.clone(), document: s.document_id(), revision: s.rev(), selection: s.sel.clone(), result: None };
        drop(capture);
        assert!(cancel.load(Ordering::Relaxed));
        assert!(tx.send(Ok(String::from("late result"))).is_err());
    }
}

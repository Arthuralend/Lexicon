//! macOS: the Speech framework, fed by the microphone through AVAudioEngine.

use std::cell::RefCell;
use std::ptr::NonNull;

use block2::RcBlock;
use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2_avf_audio::{AVAudioEngine, AVAudioPCMBuffer, AVAudioTime};
use objc2_foundation::{NSError, NSLocale, NSString};
use objc2_speech::{
    SFSpeechAudioBufferRecognitionRequest, SFSpeechRecognitionResult, SFSpeechRecognitionTask, SFSpeechRecognizer,
    SFSpeechRecognizerAuthorizationStatus,
};

use crate::{Event, Mode, Sink};

/// One dictation: kept until the next starts, so the last phrase can still arrive after `stop`.
struct Dictation {
    engine: Retained<AVAudioEngine>,
    request: Retained<SFSpeechAudioBufferRecognitionRequest>,
    task: Retained<SFSpeechRecognitionTask>,
    _recognizer: Retained<SFSpeechRecognizer>,
    listening: bool,
}

thread_local! {
    static CURRENT: RefCell<Option<Dictation>> = const { RefCell::new(None) };
}

pub fn start(lang: &str, sink: Sink) -> Result<Mode, String> {
    stop();
    CURRENT.with(|c| {
        if let Some(old) = c.borrow_mut().take() {
            // SAFETY: cancelling a task we own; it stops delivering results.
            unsafe { old.task.cancel() };
        }
    });
    // SAFETY: a class method without arguments.
    let status = unsafe { SFSpeechRecognizer::authorizationStatus() };
    if status == SFSpeechRecognizerAuthorizationStatus::NotDetermined {
        let s = sink.clone();
        let handler = RcBlock::new(move |status: SFSpeechRecognizerAuthorizationStatus| {
            s(Event::Authorized(status == SFSpeechRecognizerAuthorizationStatus::Authorized));
        });
        // SAFETY: the handler block is retained by the framework until it is called.
        unsafe { SFSpeechRecognizer::requestAuthorization(&handler) };
        return Ok(Mode::AskingPermission);
    }
    if status != SFSpeechRecognizerAuthorizationStatus::Authorized {
        return Err("speech recognition isn't allowed: System Settings › Privacy & Security › Speech Recognition".into());
    }
    begin(lang, sink).map(|()| Mode::Listening)
}

fn begin(lang: &str, sink: Sink) -> Result<(), String> {
    let id = NSString::from_str(crate::locale_for(lang));
    let locale = NSLocale::initWithLocaleIdentifier(NSLocale::alloc(), &id);
    // SAFETY: initialising a freshly allocated recogniser with a valid locale.
    let recognizer = unsafe { SFSpeechRecognizer::initWithLocale(SFSpeechRecognizer::alloc(), &locale) }
        .ok_or_else(|| format!("macOS can't recognise speech in {}", crate::locale_for(lang)))?;
    // SAFETY: plain getters on a valid recogniser.
    if !unsafe { recognizer.isAvailable() } {
        return Err("speech recognition is unavailable right now (no network?)".into());
    }
    // SAFETY: creating and configuring a request we own.
    let request = unsafe { SFSpeechAudioBufferRecognitionRequest::new() };
    unsafe { request.setShouldReportPartialResults(true) };

    // SAFETY: creating an engine and reading its input node and format.
    let engine = unsafe { AVAudioEngine::new() };
    let input = unsafe { engine.inputNode() };
    let format = unsafe { input.outputFormatForBus(0) };
    let feed = request.clone();
    let tap = RcBlock::new(move |buffer: NonNull<AVAudioPCMBuffer>, _when: NonNull<AVAudioTime>| {
        // SAFETY: AVAudioEngine hands the tap a valid buffer for the duration of the call.
        unsafe { feed.appendAudioPCMBuffer(buffer.as_ref()) };
    });
    // SAFETY: the tap block is copied by AVAudioEngine; the format is the node's own.
    unsafe { input.installTapOnBus_bufferSize_format_block(0, 1024, Some(&format), RcBlock::as_ptr(&tap)) };
    // SAFETY: starting the engine we configured.
    unsafe { engine.prepare() };
    if let Err(e) = unsafe { engine.startAndReturnError() } {
        unsafe { input.removeTapOnBus(0) };
        return Err(e.localizedDescription().to_string());
    }

    let handler = RcBlock::new(move |result: *mut SFSpeechRecognitionResult, error: *mut NSError| {
        // SAFETY: the framework passes either a valid result, a valid error, or null.
        if let Some(r) = unsafe { result.as_ref() } {
            let text = unsafe { r.bestTranscription().formattedString() }.to_string();
            if unsafe { r.isFinal() } {
                sink(Event::Final(text));
            } else {
                sink(Event::Partial(text));
            }
        } else if let Some(e) = unsafe { error.as_ref() } {
            sink(Event::Error(e.localizedDescription().to_string()));
        }
    });
    // SAFETY: the request and handler outlive the call; the framework retains both.
    let task = unsafe { recognizer.recognitionTaskWithRequest_resultHandler(&request, &handler) };
    CURRENT.with(|c| *c.borrow_mut() = Some(Dictation { engine, request, task, _recognizer: recognizer, listening: true }));
    Ok(())
}

pub fn stop() {
    CURRENT.with(|c| {
        if let Some(d) = c.borrow_mut().as_mut().filter(|d| d.listening) {
            d.listening = false;
            // SAFETY: stopping the engine and request we started.
            unsafe {
                d.engine.stop();
                d.engine.inputNode().removeTapOnBus(0);
                d.request.endAudio();
            }
        }
    });
}

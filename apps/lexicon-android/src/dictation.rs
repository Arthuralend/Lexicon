//! Android's speech activity owns microphone permissions and returns a sentence over JNI.
use jni::{
    JNIEnv,
    objects::{JClass, JString, JValue},
};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use winit::platform::android::activity::AndroidApp;

type Reply = mpsc::Sender<Result<String, String>>;
static PENDING: Mutex<Option<(u64, Reply)>> = Mutex::new(None);
static NEXT: AtomicU64 = AtomicU64::new(1);

pub fn hook(app: AndroidApp) -> wordcraft_ui_egui::dictation::Hook {
    Box::new(move |language, cancel| {
        let (out, receiver) = mpsc::channel();
        let (tx, rx) = mpsc::channel();
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some((id, tx));
        let android = app.clone();
        let failure = out.clone();
        if let Err(e) = std::thread::Builder::new().name("dictation".into()).spawn(move || {
            let launched = crate::keyboard::with_activity(&android, |env, activity| {
                let lang = env.new_string(language)?;
                env.call_method(activity, "startDictation", "(Ljava/lang/String;J)V", &[JValue::Object(&lang), JValue::Long(id as i64)]).map(|_| ())
            });
            let result = if let Err(e) = launched {
                Err(e)
            } else {
                let start = std::time::Instant::now();
                loop {
                    if cancel.load(Ordering::Relaxed) || start.elapsed().as_secs() > 65 {
                        let _ = crate::keyboard::with_activity(&android, |env, activity| {
                            env.call_method(activity, "cancelDictation", "(J)V", &[JValue::Long(id as i64)]).map(|_| ())
                        });
                        break Err("Speech recognition cancelled or timed out.".into());
                    }
                    match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                        Ok(result) => break result,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(_) => break Err("Speech recognition stopped.".into()),
                    }
                }
            };
            let mut pending = PENDING.lock().unwrap_or_else(|e| e.into_inner());
            if pending.as_ref().is_some_and(|(key, _)| *key == id) {
                *pending = None;
            }
            let _ = out.send(result);
        }) {
            let _ = failure.send(Err(e.to_string()));
        }
        receiver
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_arthuralend_lexicon_LexiconActivity_nativeDictation(
    mut env: JNIEnv,
    _class: JClass,
    id: jni::sys::jlong,
    value: JString,
    error: jni::sys::jboolean,
) {
    let text = env.get_string(&value).map(String::from).unwrap_or_default();
    let pending = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((key, tx)) = &*pending
        && *key == id as u64
    {
        let _ = tx.send(if error != 0 { Err(text) } else { Ok(text) });
    }
}

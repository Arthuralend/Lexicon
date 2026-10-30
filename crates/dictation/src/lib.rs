//! Lexicon: dictation with the operating system's own speech tools.
//!
//! - macOS: the Speech framework recognises the microphone's audio (Apple's on-device or server
//!   recognition, in the language asked for); the words come back as [`Event`]s for the app to
//!   type. The app's Info.plist must carry `NSMicrophoneUsageDescription` and
//!   `NSSpeechRecognitionUsageDescription`, or macOS ends the app on first use.
//! - Windows: the system's voice typing (Win+H) opens and types into the window itself.
//! - Elsewhere: not available ([`start`] says so).

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::sync::Arc;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod windows;

/// What the recogniser reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The words heard so far in this phrase (they may still change).
    Partial(String),
    /// The phrase's final words.
    Final(String),
    /// Recognition stopped with this message.
    Error(String),
    /// The user answered the speech-recognition permission prompt: start again when granted.
    Authorized(bool),
}

/// Where events go; called from any thread.
pub type Sink = Arc<dyn Fn(Event) + Send + Sync>;

/// How dictation runs once started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Lexicon listens and receives [`Event`]s.
    Listening,
    /// The system's own voice typing types into the window; no events come.
    System,
    /// Waiting for the user to allow speech recognition ([`Event::Authorized`] follows).
    AskingPermission,
}

/// Start dictating in `lang` (an interface language code like `ru`, `hy`, `en`). Call on the main
/// thread.
pub fn start(lang: &str, sink: Sink) -> Result<Mode, String> {
    #[cfg(target_os = "macos")]
    {
        mac::start(lang, sink)
    }
    #[cfg(windows)]
    {
        let _ = (lang, sink);
        windows::voice_typing().map(|()| Mode::System)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = (lang, sink);
        Err("dictation isn't available on this system".into())
    }
}

/// Stop listening; the last phrase still arrives as [`Event::Final`]. Call on the main thread.
pub fn stop() {
    #[cfg(target_os = "macos")]
    mac::stop();
}

/// A BCP 47 locale for an interface language code.
pub fn locale_for(lang: &str) -> &'static str {
    match lang {
        "ru" => "ru-RU",
        "hy" => "hy-AM",
        "ka" => "ka-GE",
        "az" => "az-AZ",
        "uk" => "uk-UA",
        "de" => "de-DE",
        "fr" => "fr-FR",
        "es" => "es-ES",
        "it" => "it-IT",
        "pl" => "pl-PL",
        "tr" => "tr-TR",
        "pt-br" => "pt-BR",
        "cs" => "cs-CZ",
        "hu" => "hu-HU",
        "sv" => "sv-SE",
        "hr" => "hr-HR",
        "bg" => "bg-BG",
        "ja" => "ja-JP",
        "ko" => "ko-KR",
        "zh-hans" => "zh-CN",
        "zh-hant" => "zh-TW",
        "vi" => "vi-VN",
        _ => "en-US",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn locales() {
        assert_eq!(super::locale_for("ru"), "ru-RU");
        assert_eq!(super::locale_for("??"), "en-US");
    }
}

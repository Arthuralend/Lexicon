//! Native recognition runs outside the UI thread. Cancellation terminates the owned worker.
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn hook() -> Option<wordcraft_ui_egui::dictation::Hook> {
    Some(Box::new(|language, cancel| {
        let (tx, rx) = std::sync::mpsc::channel();
        let error_tx = tx.clone();
        if let Err(e) = std::thread::Builder::new().name("lexicon-dictation".into()).spawn(move || {
            let _ = tx.send(recognize(&language, cancel));
        }) {
            let _ = error_tx.send(Err(e.to_string()));
        }
        rx
    }))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn hook() -> Option<wordcraft_ui_egui::dictation::Hook> {
    None
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn recognize(language: &str, cancel: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<String, String> {
    use std::io::Read;
    use std::process::Stdio;
    use std::sync::atomic::Ordering;
    let mut cmd = command(language)?;
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("Could not start speech recognition: {e}"))?;
    let start = std::time::Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) || start.elapsed().as_secs() > 70 {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Speech recognition cancelled or timed out.".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut text = String::new();
                if let Some(stdout) = child.stdout.take() {
                    let _ = stdout.take(65536).read_to_string(&mut text);
                }
                if !status.success() {
                    let mut error = String::new();
                    if let Some(stderr) = child.stderr.take() {
                        let _ = stderr.take(8192).read_to_string(&mut error);
                    }
                    return Err(format!("Speech recognition: {}", error.trim()));
                }
                let text = text.trim().trim_start_matches('\u{feff}').to_owned();
                return if text.is_empty() {
                    Err("No speech detected. Check the microphone and installed speech language.".into())
                } else {
                    Ok(text)
                };
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn command(language: &str) -> Result<std::process::Command, String> {
    use std::os::windows::process::CommandExt;
    // A fixed script; language is data in the environment, never interpolated into code.
    let system = std::env::var_os("SystemRoot").ok_or("SystemRoot is unavailable")?;
    let mut cmd = std::process::Command::new(std::path::Path::new(&system).join("System32/WindowsPowerShell/v1.0/powershell.exe"));
    cmd.creation_flags(0x08000000).env("LEXICON_SPEECH_LANGUAGE", language).args(["-NoProfile", "-NonInteractive", "-Command", r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
Add-Type -AssemblyName System.Speech
$language = $env:LEXICON_SPEECH_LANGUAGE
$available = [System.Speech.Recognition.SpeechRecognitionEngine]::InstalledRecognizers()
$recognizer = $available | Where-Object { $_.Culture.Name -eq $language } | Select-Object -First 1
if (!$recognizer) { $recognizer = $available | Where-Object { $_.Culture.TwoLetterISOLanguageName -eq $language.Split('-')[0] } | Select-Object -First 1 }
if (!$recognizer) { throw "Install the speech language $language in Windows Settings > Time & language > Speech, then try again." }
$s = New-Object System.Speech.Recognition.SpeechRecognitionEngine($recognizer)
try {
  $s.LoadGrammar((New-Object System.Speech.Recognition.DictationGrammar))
  $s.SetInputToDefaultAudioDevice()
  $s.InitialSilenceTimeout = [TimeSpan]::FromSeconds(15)
  $s.EndSilenceTimeout = [TimeSpan]::FromSeconds(1.5)
  $result = $s.Recognize([TimeSpan]::FromSeconds(45))
  if ($result) { [Console]::Write($result.Text) }
} finally { $s.Dispose() }
"#]);
    Ok(cmd)
}

#[cfg(target_os = "macos")]
fn command(language: &str) -> Result<std::process::Command, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let helper = exe.parent().ok_or("App folder unavailable")?.join("LexiconSpeech.app/Contents/MacOS/LexiconSpeech");
    if !helper.is_file() {
        return Err("Speech helper is missing. Install the packaged Lexicon application.".into());
    }
    let mut cmd = std::process::Command::new(helper);
    cmd.arg(language);
    Ok(cmd)
}

//! Документы: which window has which file, so a file opens only once, as in Word. Each window (a
//! process of its own) keeps `windows/<pid>.path` in the settings folder up to date with its
//! document's path, touched every couple of seconds; a window that wants a file another one
//! holds asks that one to come forward with `windows/<pid>.focus` instead of opening it again.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// A record not touched for this long belongs to a window that has gone.
const STALE: Duration = Duration::from_secs(8);

fn dir() -> Option<PathBuf> {
    wordcraft_control_key::settings_dir().map(|d| d.join("windows"))
}

fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Note this window's document (`None`: none saved yet).
pub fn publish(path: Option<&Path>) {
    let Some(dir) = dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let text = path.map(|p| canonical(p).to_string_lossy().into_owned()).unwrap_or_default();
    let _ = std::fs::write(dir.join(format!("{}.path", std::process::id())), text);
}

/// This window is closing.
pub fn forget() {
    let Some(dir) = dir() else { return };
    let me = std::process::id();
    let _ = std::fs::remove_file(dir.join(format!("{me}.path")));
    let _ = std::fs::remove_file(dir.join(format!("{me}.focus")));
}

/// Whether another window asked this one to come forward (and forget the request).
pub fn take_focus_request() -> bool {
    let Some(dir) = dir() else { return false };
    std::fs::remove_file(dir.join(format!("{}.focus", std::process::id()))).is_ok()
}

/// If another window has `path` open, ask it to come forward and say so.
pub fn ask_owner(path: &str) -> bool {
    let Some(dir) = dir() else { return false };
    let want = canonical(Path::new(path));
    let me = std::process::id().to_string();
    let Ok(entries) = std::fs::read_dir(&dir) else { return false };
    for e in entries.flatten().take(256) {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(pid) = name.strip_suffix(".path") else { continue };
        if pid == me {
            continue;
        }
        let fresh =
            e.metadata().and_then(|m| m.modified()).ok().and_then(|t| SystemTime::now().duration_since(t).ok()).is_some_and(|age| age < STALE);
        if !fresh {
            let _ = std::fs::remove_file(e.path());
            continue;
        }
        let held = std::fs::read_to_string(e.path()).unwrap_or_default();
        if !held.is_empty() && Path::new(&held) == want {
            let _ = std::fs::write(dir.join(format!("{pid}.focus")), b"");
            return true;
        }
    }
    false
}

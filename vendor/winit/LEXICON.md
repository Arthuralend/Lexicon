# winit 0.30.13, patched for Lexicon

The crates.io release of winit 0.30.13 (Apache-2.0, see `LICENSE`), with one change in
`src/platform_impl/android/mod.rs`: on Android, events from a mouse or touchpad are reported as
mouse events (`CursorMoved` including hover, `MouseInput` for the left, right and middle buttons,
`MouseWheel`, `CursorEntered`/`CursorLeft`) instead of touches, and are left unhandled so Android
still resolves the pointer icon. Upstream treats every motion event as a touch, so a right click
or the wheel did nothing in Lexicon on Android. Other platforms are unchanged.

Root `Cargo.toml` uses it through `[patch.crates-io]`.

## macOS: start Dictation

`platform::macos::start_dictation()` sends `startDictation:` along the responder chain (what
Edit › Start Dictation does), so Lexicon's Dictate button opens the system's Dictation; the words
arrive through the view's `NSTextInputClient` as `Ime::Commit`.

## macOS: Quit asks first

`applicationShouldTerminate:` (Command-Q from the default menu, Dock › Quit, logout) sends every
window `CloseRequested` and cancels, so the app's unsaved-changes prompt runs; the app calls
`platform::macos::allow_terminate()` before its own `terminate:`. Before this, Command-Q skipped
the prompt and tore the window down inside `-[NSApplication run]`.

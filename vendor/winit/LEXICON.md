# winit 0.30.13, patched for Lexicon

The crates.io release of winit 0.30.13 (Apache-2.0, see `LICENSE`), with one change in
`src/platform_impl/android/mod.rs`: on Android, events from a mouse or touchpad are reported as
mouse events (`CursorMoved` including hover, `MouseInput` for the left, right and middle buttons,
`MouseWheel`, `CursorEntered`/`CursorLeft`) instead of touches, and are left unhandled so Android
still resolves the pointer icon. Upstream treats every motion event as a touch, so a right click
or the wheel did nothing in Lexicon on Android. Other platforms are unchanged.

Root `Cargo.toml` uses it through `[patch.crates-io]`.

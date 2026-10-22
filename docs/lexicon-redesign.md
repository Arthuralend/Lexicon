# Lexicon appearance and editing update

Updated: 2026-10-22.

- Graphite/mint dark theme; warm off-white light theme. File > Options > General > Interface theme selects Light, Dark or System. Preferences survive restart; page appearance remains independent.
- Original document/orbit L icon for Windows, macOS and Android; rounded controls and theme-aware File navigation.
- No Discord controls or Zotero ribbon tab. About identifies LuSoft and CEO Arthur Stepanian; upstream notices remain under Licenses and in the distributed license files.
- File > Save As goes directly to the existing platform picker (or Android's built-in file browser).
- Header double-click requires an earlier canvas click in the same document, preventing a launch/start-screen click from entering header editing.
- Dictation previews recognized text before Insert; cancelling stops the native worker and results cannot overwrite a changed document. Windows uses installed System.Speech dictation languages, macOS uses Apple's Speech framework in a packaged helper, Android uses its installed speech recognition activity. macOS/Android recognition may use their provider's online service. No microphone is opened until Dictate is clicked.

## Verification

Build and test results are recorded in the pull request and GitHub Actions. Real-device microphone permission and recognition checks are still required on each OS; compilation alone does not verify microphone hardware. The macOS native bridge uses Swift because Apple's speech authorization and audio callbacks require a platform adapter, like Android's existing Java adapter; document and UI logic remain Rust.

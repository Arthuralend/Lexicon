// Native macOS speech bridge; built into a signed helper app, never compiled on the user's Mac.
import AppKit
import AVFoundation
import Speech

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let engine = AVAudioEngine()
var task: SFSpeechRecognitionTask?
var request: SFSpeechAudioBufferRecognitionRequest?
var finished = false
var silence: Timer?
var lastText = ""
var tapped = false
func finish(_ text: String?, _ error: String?) {
    guard !finished else { return }
    finished = true
    engine.stop()
    if tapped { engine.inputNode.removeTap(onBus: 0) }
    request?.endAudio()
    task?.cancel()
    if let error = error {
        FileHandle.standardError.write(Data(error.utf8))
        exit(1)
    }
    FileHandle.standardOutput.write(Data((text ?? "").utf8))
    exit(0)
}
func begin() {
    let locale = CommandLine.arguments.dropFirst().first ?? Locale.current.identifier
    guard let recognizer = SFSpeechRecognizer(locale: Locale(identifier: locale)), recognizer.isAvailable else {
        finish(nil, "Speech recognition is unavailable for this language. Check your internet connection and Speech settings.")
        return
    }
    let input = engine.inputNode
    let format = input.outputFormat(forBus: 0)
    guard format.sampleRate > 0 && format.channelCount > 0 else { finish(nil, "No microphone is available."); return }
    let req = SFSpeechAudioBufferRecognitionRequest()
    request = req
    req.shouldReportPartialResults = true
    input.installTap(onBus: 0, bufferSize: 1024, format: format) { buffer, _ in req.append(buffer) }
    tapped = true
    task = recognizer.recognitionTask(with: req) { result, error in
        DispatchQueue.main.async {
            if let result = result {
                lastText = result.bestTranscription.formattedString
                silence?.invalidate()
                if result.isFinal { finish(lastText, nil); return }
                silence = Timer.scheduledTimer(withTimeInterval: 2, repeats: false) { _ in finish(lastText, nil) }
            } else if let error = error { finish(nil, error.localizedDescription) }
        }
    }
    do { engine.prepare(); try engine.start() } catch { finish(nil, error.localizedDescription) }
}
SFSpeechRecognizer.requestAuthorization { status in
    guard status == .authorized else {
        DispatchQueue.main.async { finish(nil, "Allow Speech Recognition for Lexicon in System Settings > Privacy & Security.") }
        return
    }
    AVCaptureDevice.requestAccess(for: .audio) { allowed in
        DispatchQueue.main.async {
            if allowed { begin() } else { finish(nil, "Allow microphone access for Lexicon in System Settings > Privacy & Security.") }
        }
    }
}
Timer.scheduledTimer(withTimeInterval: 55, repeats: false) { _ in finish(lastText, lastText.isEmpty ? "No speech detected." : nil) }
app.run()

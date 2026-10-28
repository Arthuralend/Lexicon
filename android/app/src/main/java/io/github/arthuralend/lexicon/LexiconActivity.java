package io.github.arthuralend.lexicon;

import android.Manifest;
import android.app.AlertDialog;
import android.app.NativeActivity;
import android.content.ActivityNotFoundException;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Insets;
import android.graphics.Paint;
import android.graphics.Path;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.provider.Settings;
import android.speech.RecognitionListener;
import android.speech.RecognizerIntent;
import android.speech.SpeechRecognizer;
import android.widget.Toast;
import java.util.ArrayList;
import android.text.InputType;
import android.view.InputDevice;
import android.view.KeyCharacterMap;
import android.view.KeyEvent;
import android.view.PointerIcon;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowInsets;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;
import java.util.Locale;

/**
 * Lexicon's activity: the Rust app (liblexicon.so, apps/lexicon-android) draws the window through
 * NativeActivity. This class adds what NativeActivity lacks:
 *
 * <ul>
 *   <li>Typing with the on-screen keyboard. An invisible view owns the keyboard's input
 *       connection and hands composing text, committed text, deletions and editing keys to Rust
 *       (apps/lexicon-android/src/keyboard.rs), which shows and hides the keyboard through
 *       {@link #showKeyboard}.
 *   <li>Asking once for access to shared storage, so documents open and save in Documents and
 *       Download, not only in the app's own folder.
 * </ul>
 */
public class LexiconActivity extends NativeActivity {
    static {
        // NativeActivity loads the library itself; loading it here too binds the native methods.
        System.loadLibrary("lexicon");
    }

    static native void nativePreedit(String text);

    static native void nativeCommit(String text);

    static native void nativeDelete(int before, int after);

    static native void nativeKey(int keyCode, boolean shift);

    static native void nativeInsets(int left, int top, int right, int bottom);

    private KeyboardView input;
    /** The user went to grant file access: restart once it is granted. */
    private boolean awaitingFileAccess;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        input = new KeyboardView(this);
        addContentView(input, new ViewGroup.LayoutParams(1, 1));
        // The native surface covers the whole window, under the status bar, the navigation bar
        // and the keyboard: tell Rust how much of it they take.
        View decor = getWindow().getDecorView();
        decor.setOnApplyWindowInsetsListener((v, insets) -> {
            reportInsets(insets);
            return v.onApplyWindowInsets(insets);
        });
        decor.requestApplyInsets();
        setCursor(0);
        askForFileAccess();
    }

    private static void reportInsets(WindowInsets insets) {
        if (Build.VERSION.SDK_INT >= 30) {
            Insets i = insets.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout() | WindowInsets.Type.ime());
            nativeInsets(i.left, i.top, i.right, i.bottom);
        } else {
            nativeInsets(insets.getSystemWindowInsetLeft(), insets.getSystemWindowInsetTop(), insets.getSystemWindowInsetRight(), insets.getSystemWindowInsetBottom());
        }
    }

    /**
     * Called from Rust: the text a hardware keyboard's key types in the layout active now, or
     * null when there is no hardware keyboard or the key types nothing. (The native side keeps the
     * layout from the first key press, so a layout switch wouldn't reach the app otherwise.)
     */
    public String keyText(int keyCode, boolean shift) {
        int meta = shift ? KeyEvent.META_SHIFT_ON : 0;
        for (int id : InputDevice.getDeviceIds()) {
            InputDevice device = InputDevice.getDevice(id);
            if (device == null || device.isVirtual() || device.getKeyboardType() != InputDevice.KEYBOARD_TYPE_ALPHABETIC) {
                continue;
            }
            int c = device.getKeyCharacterMap().get(keyCode, meta);
            if (c != 0 && (c & KeyCharacterMap.COMBINING_ACCENT) == 0) {
                return new String(Character.toChars(c));
            }
        }
        return null;
    }

    // ---- Mouse pointer ---------------------------------------------------------------------------

    /** The arrow drawn for Lexicon, made once. */
    private PointerIcon arrow;
    private int cursorKind = -1;

    /**
     * Lexicon's arrow: a slim navy arrow with a white rim and a soft shadow, sized for the
     * screen's density. Drawn here, so it needs no image file.
     */
    private PointerIcon makeArrow() {
        float d = getResources().getDisplayMetrics().density;
        int size = Math.round(28 * d);
        Bitmap bmp = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888);
        Canvas c = new Canvas(bmp);
        float u = d; // one dp
        Path arrow = new Path();
        arrow.moveTo(3 * u, 2 * u);
        arrow.lineTo(3 * u, 20 * u);
        arrow.lineTo(7.6f * u, 15.8f * u);
        arrow.lineTo(10.6f * u, 22.6f * u);
        arrow.lineTo(13.6f * u, 21.3f * u);
        arrow.lineTo(10.7f * u, 14.6f * u);
        arrow.lineTo(16.8f * u, 14.6f * u);
        arrow.close();
        Paint shadow = new Paint(Paint.ANTI_ALIAS_FLAG);
        shadow.setColor(Color.argb(70, 0, 0, 0));
        shadow.setStyle(Paint.Style.FILL);
        c.save();
        c.translate(0.8f * u, 1.2f * u);
        c.drawPath(arrow, shadow);
        c.restore();
        Paint rim = new Paint(Paint.ANTI_ALIAS_FLAG);
        rim.setColor(Color.WHITE);
        rim.setStyle(Paint.Style.STROKE);
        rim.setStrokeWidth(2.4f * u);
        rim.setStrokeJoin(Paint.Join.ROUND);
        c.drawPath(arrow, rim);
        Paint fill = new Paint(Paint.ANTI_ALIAS_FLAG);
        fill.setColor(Color.rgb(0x1E, 0x2A, 0x5A));
        fill.setStyle(Paint.Style.FILL);
        c.drawPath(arrow, fill);
        return PointerIcon.create(bmp, 3 * u, 2 * u);
    }

    /**
     * Called from Rust when the pointer shape egui wants changes: 0 Lexicon's arrow, 1 text, 2 hand,
     * 3 left-right, 4 up-down, 5 diagonal ↗↙, 6 diagonal ↖↘, 7 grab, 8 grabbing, 9 move,
     * 10 crosshair, 11 wait, 12 not allowed, 13 context menu, 14 copy, 15 alias, 16 help,
     * 17 zoom in, 18 zoom out, 19 hidden.
     */
    public void setCursor(final int kind) {
        runOnUiThread(() -> {
            if (kind == cursorKind) {
                return;
            }
            cursorKind = kind;
            PointerIcon icon;
            if (kind == 0) {
                if (arrow == null) {
                    arrow = makeArrow();
                }
                icon = arrow;
            } else {
                int[] types = {
                    PointerIcon.TYPE_ARROW, PointerIcon.TYPE_TEXT, PointerIcon.TYPE_HAND,
                    PointerIcon.TYPE_HORIZONTAL_DOUBLE_ARROW, PointerIcon.TYPE_VERTICAL_DOUBLE_ARROW,
                    PointerIcon.TYPE_TOP_RIGHT_DIAGONAL_DOUBLE_ARROW, PointerIcon.TYPE_TOP_LEFT_DIAGONAL_DOUBLE_ARROW,
                    PointerIcon.TYPE_GRAB, PointerIcon.TYPE_GRABBING, PointerIcon.TYPE_ALL_SCROLL,
                    PointerIcon.TYPE_CROSSHAIR, PointerIcon.TYPE_WAIT, PointerIcon.TYPE_NO_DROP,
                    PointerIcon.TYPE_CONTEXT_MENU, PointerIcon.TYPE_COPY, PointerIcon.TYPE_ALIAS,
                    PointerIcon.TYPE_HELP, PointerIcon.TYPE_ZOOM_IN, PointerIcon.TYPE_ZOOM_OUT,
                    PointerIcon.TYPE_NULL,
                };
                int type = kind >= 0 && kind < types.length ? types[kind] : PointerIcon.TYPE_ARROW;
                icon = PointerIcon.getSystemIcon(this, type);
            }
            getWindow().getDecorView().setPointerIcon(icon);
            if (input != null) {
                input.setPointerIcon(icon);
            }
        });
    }

    // ---- Clipboard -------------------------------------------------------------------------------

    /** Called from Rust: put text on the system clipboard (Copy, Cut). */
    public void setClipboardText(final String text) {
        runOnUiThread(() -> {
            ClipboardManager cm = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
            if (cm != null) {
                cm.setPrimaryClip(ClipData.newPlainText("Lexicon", text));
            }
        });
    }

    /** Called from Rust: the text on the system clipboard, or null (Paste). */
    public String getClipboardText() {
        ClipboardManager cm = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        if (cm == null || !cm.hasPrimaryClip()) {
            return null;
        }
        ClipData clip = cm.getPrimaryClip();
        if (clip == null || clip.getItemCount() == 0) {
            return null;
        }
        CharSequence text = clip.getItemAt(0).coerceToText(this);
        return text == null ? null : text.toString();
    }

    /** Called from Rust on its own thread. */
    public void showKeyboard(final boolean show) {
        runOnUiThread(() -> {
            InputMethodManager imm = (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
            if (imm == null) {
                return;
            }
            if (show) {
                input.requestFocus();
                // The caret may have moved: the keyboard starts a fresh composition.
                imm.restartInput(input);
                imm.showSoftInput(input, 0);
            } else {
                imm.hideSoftInputFromWindow(input.getWindowToken(), 0);
            }
        });
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (awaitingFileAccess && hasFileAccess()) {
            awaitingFileAccess = false;
            restart();
        }
    }

    // Home › Dictate -------------------------------------------------------------------------

    private static final int DICTATE_PERMISSION = 2;
    private static final int DICTATE_ACTIVITY = 3;
    private SpeechRecognizer recognizer;
    private String dictateLang = "";

    /** Called from Rust: listen for speech in `lang` ("ru", "hy", "en"…) and type the words. */
    public void startDictation(final String lang) {
        runOnUiThread(() -> {
            dictateLang = lang == null ? "" : lang;
            if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
                requestPermissions(new String[] {Manifest.permission.RECORD_AUDIO}, DICTATE_PERMISSION);
                return;
            }
            listen(dictateLang);
        });
    }

    private Intent speechIntent(String lang) {
        Intent i = new Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH);
        i.putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM);
        String tag = speechTag(lang);
        if (!tag.isEmpty()) {
            i.putExtra(RecognizerIntent.EXTRA_LANGUAGE, tag);
            i.putExtra(RecognizerIntent.EXTRA_LANGUAGE_PREFERENCE, tag);
        }
        i.putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 1);
        return i;
    }

    private static String speechTag(String lang) {
        switch (lang) {
            case "ru": return "ru-RU";
            case "hy": return "hy-AM";
            case "ka": return "ka-GE";
            case "az": return "az-AZ";
            case "en": return "en-US";
            case "uk": return "uk-UA";
            default: return lang.replace('_', '-');
        }
    }

    private void listen(String lang) {
        if (!SpeechRecognizer.isRecognitionAvailable(this)) {
            // No recognition service: the recogniser's own screen, when there is one.
            try {
                startActivityForResult(speechIntent(lang), DICTATE_ACTIVITY);
            } catch (ActivityNotFoundException e) {
                toast("none");
            }
            return;
        }
        if (recognizer == null) {
            recognizer = SpeechRecognizer.createSpeechRecognizer(this);
            recognizer.setRecognitionListener(new RecognitionListener() {
                @Override public void onReadyForSpeech(Bundle params) { toast("listen"); }
                @Override public void onBeginningOfSpeech() {}
                @Override public void onRmsChanged(float rmsdB) {}
                @Override public void onBufferReceived(byte[] buffer) {}
                @Override public void onEndOfSpeech() {}
                @Override public void onError(int error) {
                    toast(error == SpeechRecognizer.ERROR_NO_MATCH || error == SpeechRecognizer.ERROR_SPEECH_TIMEOUT ? "nomatch" : "error");
                }
                @Override public void onResults(Bundle results) { commitSpeech(results.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)); }
                @Override public void onPartialResults(Bundle partialResults) {}
                @Override public void onEvent(int eventType, Bundle params) {}
            });
        }
        try {
            recognizer.startListening(speechIntent(lang));
        } catch (RuntimeException e) {
            toast("error");
        }
    }

    private void commitSpeech(ArrayList<String> words) {
        if (words == null || words.isEmpty() || words.get(0) == null || words.get(0).isEmpty()) {
            toast("nomatch");
            return;
        }
        nativeCommit(words.get(0) + " ");
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode == DICTATE_ACTIVITY && resultCode == RESULT_OK && data != null) {
            commitSpeech(data.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS));
        }
    }

    /** A short message in the dictation language (Russian, Armenian or English). */
    private void toast(String what) {
        boolean ru = dictateLang.equals("ru") || dictateLang.equals("uk");
        boolean hy = dictateLang.equals("hy");
        String text;
        switch (what) {
            case "listen": text = ru ? "Говорите…" : hy ? "Խոսեք…" : "Speak now…"; break;
            case "nomatch": text = ru ? "Речь не распознана" : hy ? "Խոսքը չճանաչվեց" : "No speech recognised"; break;
            case "mic": text = ru ? "Нужен доступ к микрофону" : hy ? "Անհրաժեշտ է խոսափողի թույլտվություն" : "Microphone access is needed"; break;
            case "none": text = ru ? "Распознавание речи недоступно на этом устройстве" : hy ? "Խոսքի ճանաչումը հասանելի չէ" : "Speech recognition isn't available on this device"; break;
            default: text = ru ? "Ошибка распознавания речи" : hy ? "Խոսքի ճանաչման սխալ" : "Speech recognition failed"; break;
        }
        Toast.makeText(this, text, Toast.LENGTH_SHORT).show();
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] results) {
        super.onRequestPermissionsResult(requestCode, permissions, results);
        if (requestCode == DICTATE_PERMISSION) {
            if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED) {
                listen(dictateLang);
            } else {
                toast("mic");
            }
            return;
        }
        if (awaitingFileAccess && hasFileAccess()) {
            awaitingFileAccess = false;
            restart();
        }
    }

    private boolean hasFileAccess() {
        if (Build.VERSION.SDK_INT >= 30) {
            return Environment.isExternalStorageManager();
        }
        return checkSelfPermission(Manifest.permission.WRITE_EXTERNAL_STORAGE) == PackageManager.PERMISSION_GRANTED;
    }

    private void askForFileAccess() {
        if (hasFileAccess()) {
            return;
        }
        SharedPreferences prefs = getSharedPreferences("lexicon", MODE_PRIVATE);
        if (prefs.getBoolean("askedForFileAccess", false)) {
            return;
        }
        prefs.edit().putBoolean("askedForFileAccess", true).apply();
        boolean ru = "ru".equals(Locale.getDefault().getLanguage());
        new AlertDialog.Builder(this)
                .setTitle(ru ? "Доступ к файлам" : "File access")
                .setMessage(ru
                        ? "Разрешите Lexicon доступ к файлам, чтобы открывать и сохранять документы в папках «Документы» и «Загрузки». Без него документы хранятся в папке приложения."
                        : "Allow Lexicon to access files to open and save documents in Documents and Download. Without it, documents are kept in the app's own folder.")
                .setPositiveButton(ru ? "Разрешить" : "Allow", (d, w) -> requestFileAccess())
                .setNegativeButton(ru ? "Не сейчас" : "Not now", null)
                .show();
    }

    private void requestFileAccess() {
        awaitingFileAccess = true;
        if (Build.VERSION.SDK_INT >= 30) {
            try {
                startActivity(new Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION, Uri.parse("package:" + getPackageName())));
            } catch (ActivityNotFoundException e) {
                startActivity(new Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION));
            }
        } else {
            requestPermissions(new String[] {Manifest.permission.READ_EXTERNAL_STORAGE, Manifest.permission.WRITE_EXTERNAL_STORAGE}, 1);
        }
    }

    /** The Rust side picks its folders at start: start again so it sees shared storage. */
    private void restart() {
        Intent intent = getPackageManager().getLaunchIntentForPackage(getPackageName());
        if (intent != null) {
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            startActivity(intent);
        }
        finish();
        android.os.Process.killProcess(android.os.Process.myPid());
    }

    /** The invisible view the keyboard types into. */
    static final class KeyboardView extends View {
        KeyboardView(Context context) {
            super(context);
            setFocusable(true);
            setFocusableInTouchMode(true);
        }

        @Override
        public boolean onCheckIsTextEditor() {
            return true;
        }

        @Override
        public InputConnection onCreateInputConnection(EditorInfo out) {
            out.inputType = InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_MULTI_LINE | InputType.TYPE_TEXT_FLAG_AUTO_CORRECT;
            out.imeOptions = EditorInfo.IME_FLAG_NO_FULLSCREEN | EditorInfo.IME_FLAG_NO_EXTRACT_UI | EditorInfo.IME_ACTION_NONE;
            out.initialSelStart = 0;
            out.initialSelEnd = 0;
            return new Connection(this);
        }
    }

    /** Forwards what the keyboard does to Rust; the document itself is the editor. */
    static final class Connection extends BaseInputConnection {
        private String composing = "";

        Connection(View view) {
            super(view, false);
        }

        @Override
        public boolean setComposingText(CharSequence text, int newCursorPosition) {
            composing = text.toString();
            nativePreedit(composing);
            return true;
        }

        @Override
        public boolean setComposingRegion(int start, int end) {
            // Text already in the document can't be taken back into a composition.
            return true;
        }

        @Override
        public boolean finishComposingText() {
            if (!composing.isEmpty()) {
                nativeCommit(composing);
                composing = "";
            }
            return true;
        }

        @Override
        public boolean commitText(CharSequence text, int newCursorPosition) {
            composing = "";
            nativeCommit(text.toString());
            return true;
        }

        @Override
        public boolean deleteSurroundingText(int beforeLength, int afterLength) {
            nativeDelete(beforeLength, afterLength);
            return true;
        }

        @Override
        public boolean deleteSurroundingTextInCodePoints(int beforeLength, int afterLength) {
            nativeDelete(beforeLength, afterLength);
            return true;
        }

        @Override
        public boolean sendKeyEvent(KeyEvent event) {
            if (event.getAction() != KeyEvent.ACTION_DOWN) {
                return true;
            }
            int code = event.getKeyCode();
            int ch = event.getUnicodeChar();
            if (isEditingKey(code)) {
                if (!composing.isEmpty()) {
                    finishComposingText();
                }
                nativeKey(code, event.isShiftPressed());
            } else if (ch != 0 && !Character.isISOControl(ch)) {
                nativeCommit(new String(Character.toChars(ch)));
            }
            return true;
        }

        @Override
        public boolean performEditorAction(int actionCode) {
            nativeKey(KeyEvent.KEYCODE_ENTER, false);
            return true;
        }

        @Override
        public CharSequence getTextBeforeCursor(int length, int flags) {
            return "";
        }

        @Override
        public CharSequence getTextAfterCursor(int length, int flags) {
            return "";
        }

        private static boolean isEditingKey(int code) {
            switch (code) {
                case KeyEvent.KEYCODE_DEL:
                case KeyEvent.KEYCODE_FORWARD_DEL:
                case KeyEvent.KEYCODE_ENTER:
                case KeyEvent.KEYCODE_NUMPAD_ENTER:
                case KeyEvent.KEYCODE_TAB:
                case KeyEvent.KEYCODE_DPAD_UP:
                case KeyEvent.KEYCODE_DPAD_DOWN:
                case KeyEvent.KEYCODE_DPAD_LEFT:
                case KeyEvent.KEYCODE_DPAD_RIGHT:
                case KeyEvent.KEYCODE_MOVE_HOME:
                case KeyEvent.KEYCODE_MOVE_END:
                case KeyEvent.KEYCODE_PAGE_UP:
                case KeyEvent.KEYCODE_PAGE_DOWN:
                case KeyEvent.KEYCODE_ESCAPE:
                    return true;
                default:
                    return false;
            }
        }
    }
}

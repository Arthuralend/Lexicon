package io.github.arthuralend.lexicon;

import android.Manifest;
import android.app.AlertDialog;
import android.app.NativeActivity;
import android.content.ActivityNotFoundException;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.graphics.Insets;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.provider.Settings;
import android.text.InputType;
import android.view.InputDevice;
import android.view.KeyCharacterMap;
import android.view.KeyEvent;
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

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] results) {
        super.onRequestPermissionsResult(requestCode, permissions, results);
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

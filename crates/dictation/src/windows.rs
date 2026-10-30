//! Windows: open the system's voice typing (Win+H), which types into the focused window.

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_H, VK_LWIN};

fn key(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 } },
    }
}

/// Press Win+H.
pub fn voice_typing() -> Result<(), String> {
    let inputs = [key(VK_LWIN, false), key(VK_H, false), key(VK_H, true), key(VK_LWIN, true)];
    let size = i32::try_from(std::mem::size_of::<INPUT>()).map_err(|e| e.to_string())?;
    // SAFETY: `inputs` is a valid array of keyboard INPUTs for the duration of the call, and
    // `size` is the size of one INPUT, as SendInput requires.
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), size) };
    if sent as usize == inputs.len() { Ok(()) } else { Err("Windows didn't accept the voice typing shortcut".into()) }
}

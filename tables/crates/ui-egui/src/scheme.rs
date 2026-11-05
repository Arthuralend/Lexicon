//! Таблицы looks like Документы: the colour scheme chosen in Документы (its settings' `theme`)
//! colours the ribbon, panels, headers and accents here too. The sheet itself stays white, as in
//! Excel. Документы' settings are read at start and every few seconds, so a change there follows.

use std::sync::RwLock;

use egui::Color32;

/// One of Документы' colour schemes (the colours Таблицы takes from it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scheme {
    pub dark: bool,
    pub title: u32,
    pub ribbon: u32,
    pub tabs: u32,
    pub canvas: u32,
    pub panel: u32,
    pub border: u32,
    pub border_strong: u32,
    pub text: u32,
    pub text_dim: u32,
    pub text_disabled: u32,
    pub hover: u32,
    pub pressed: u32,
    pub checked: u32,
    pub accent: u32,
    pub accent_text: u32,
    pub on_accent: u32,
    pub input: u32,
    pub input_border: u32,
    pub status: u32,
    pub menu: u32,
}

/// Документы' schemes by their settings code (generated from its theme palettes).
const SCHEMES: &[(&str, Scheme)] = &[
    ("navy", Scheme { dark: true, title: 0x14213D, ribbon: 0x1B2A4A, tabs: 0x14213D, canvas: 0x0F1A30, panel: 0x192745, border: 0x2A3C62, border_strong: 0x40578A, text: 0xEAF0FA, text_dim: 0xB3C0D8, text_disabled: 0x6A6A6A, hover: 0x243659, pressed: 0x2E4470, checked: 0x293F6C, accent: 0x7AA8FF, accent_text: 0xA5C4FF, on_accent: 0x0B1530, input: 0x13203A, input_border: 0x6A80AE, status: 0x14213D, menu: 0x1D2D50 }),
    ("night", Scheme { dark: true, title: 0x0B1020, ribbon: 0x111A2E, tabs: 0x0B1020, canvas: 0x070B16, panel: 0x0F1729, border: 0x1E2A44, border_strong: 0x34466C, text: 0xE6EAF2, text_dim: 0xA9B4C8, text_disabled: 0x6A6A6A, hover: 0x1B2640, pressed: 0x24314F, checked: 0x22345E, accent: 0x6F9BFF, accent_text: 0x9DBBFF, on_accent: 0x0B1020, input: 0x0C1424, input_border: 0x5D6F95, status: 0x0B1020, menu: 0x131D33 }),
    ("light", Scheme { dark: false, title: 0xF0F0F0, ribbon: 0xF9F9F9, tabs: 0xF0F0F0, canvas: 0xE6E6E6, panel: 0xFAFAFA, border: 0xE1E1E1, border_strong: 0xC8C8C8, text: 0x242424, text_dim: 0x616161, text_disabled: 0xA8A8A8, hover: 0xE6E6E6, pressed: 0xD6D6D6, checked: 0xDDE3F8, accent: 0x3B5BDB, accent_text: 0x2B47B5, on_accent: 0xFFFFFF, input: 0xFFFFFF, input_border: 0x8A8A8A, status: 0xF0F0F0, menu: 0xFFFFFF }),
    ("dark", Scheme { dark: true, title: 0x1F1F1F, ribbon: 0x292929, tabs: 0x1F1F1F, canvas: 0x141414, panel: 0x262626, border: 0x3A3A3A, border_strong: 0x555555, text: 0xEEEEEE, text_dim: 0xC8C8C8, text_disabled: 0x6A6A6A, hover: 0x3D3D3D, pressed: 0x4A4A4A, checked: 0x2E3A66, accent: 0x7A93F0, accent_text: 0x9CB0F5, on_accent: 0x11162E, input: 0x1E1E1E, input_border: 0x7A7A7A, status: 0x1F1F1F, menu: 0x2B2B2B }),
    ("black", Scheme { dark: true, title: 0x000000, ribbon: 0x0A0A0A, tabs: 0x000000, canvas: 0x050505, panel: 0x0D0D0D, border: 0x222222, border_strong: 0x3A3A3A, text: 0xEEEEEE, text_dim: 0xC8C8C8, text_disabled: 0x6A6A6A, hover: 0x1C1C1C, pressed: 0x262626, checked: 0x2A2F3A, accent: 0x7A93F0, accent_text: 0x9CB0F5, on_accent: 0x11162E, input: 0x0A0A0A, input_border: 0x7A7A7A, status: 0x000000, menu: 0x111111 }),
    ("white", Scheme { dark: false, title: 0xFFFFFF, ribbon: 0xFFFFFF, tabs: 0xFFFFFF, canvas: 0xEDEDED, panel: 0xFFFFFF, border: 0xE5E5E5, border_strong: 0xC8C8C8, text: 0x242424, text_dim: 0x616161, text_disabled: 0xA8A8A8, hover: 0xEEEEEE, pressed: 0xE0E0E0, checked: 0xDDE3F8, accent: 0x3B5BDB, accent_text: 0x2B47B5, on_accent: 0xFFFFFF, input: 0xFFFFFF, input_border: 0x8A8A8A, status: 0xF7F7F7, menu: 0xFFFFFF }),
    ("sky", Scheme { dark: false, title: 0xDCE8F8, ribbon: 0xEEF4FC, tabs: 0xDCE8F8, canvas: 0xD5E2F3, panel: 0xF3F7FD, border: 0xC9D8EE, border_strong: 0xA9BEDD, text: 0x14243C, text_dim: 0x435673, text_disabled: 0xA8A8A8, hover: 0xD6E4F7, pressed: 0xC3D5F0, checked: 0xC6D8F5, accent: 0x2F64C8, accent_text: 0x1E4FA8, on_accent: 0xFFFFFF, input: 0xFFFFFF, input_border: 0x5E7598, status: 0xDCE8F8, menu: 0xFFFFFF }),
    ("burgundy", Scheme { dark: false, title: 0xEFE3D3, ribbon: 0xF8F0E5, tabs: 0xEFE3D3, canvas: 0xE6D8C6, panel: 0xFBF5EC, border: 0xE0D0BC, border_strong: 0xCBB59A, text: 0x2E1A1E, text_dim: 0x654848, text_disabled: 0xA8A8A8, hover: 0xEDDCC8, pressed: 0xE2CCB2, checked: 0xEFD9DC, accent: 0x8C1D3A, accent_text: 0x7A1832, on_accent: 0xFFFFFF, input: 0xFFFDF9, input_border: 0x85695A, status: 0xEFE3D3, menu: 0xFFFBF5 }),
    ("forest", Scheme { dark: true, title: 0x0F1E17, ribbon: 0x15291F, tabs: 0x0F1E17, canvas: 0x0B1611, panel: 0x132519, border: 0x22402F, border_strong: 0x356049, text: 0xE6F2EA, text_dim: 0xA9C4B3, text_disabled: 0x6A6A6A, hover: 0x1D3828, pressed: 0x264733, checked: 0x24503A, accent: 0x4CC38A, accent_text: 0x7FDCAE, on_accent: 0x08140E, input: 0x0E1C14, input_border: 0x55836A, status: 0x0F1E17, menu: 0x17301F }),
    ("lavender", Scheme { dark: false, title: 0xECE6F7, ribbon: 0xF6F2FC, tabs: 0xECE6F7, canvas: 0xE2DAF2, panel: 0xFAF7FE, border: 0xDDD3EF, border_strong: 0xBFAFDF, text: 0x231A33, text_dim: 0x56486E, text_disabled: 0xA8A8A8, hover: 0xE6DCF5, pressed: 0xD8CAEE, checked: 0xE3D5F7, accent: 0x6A3FB5, accent_text: 0x5A32A0, on_accent: 0xFFFFFF, input: 0xFFFFFF, input_border: 0x7D6A9E, status: 0xECE6F7, menu: 0xFFFFFF }),
    ("mint", Scheme { dark: false, title: 0xE2F3EC, ribbon: 0xF1FAF6, tabs: 0xE2F3EC, canvas: 0xD6EDE3, panel: 0xF7FCFA, border: 0xCDE6DA, border_strong: 0xA9D2BF, text: 0x15291F, text_dim: 0x3F5C4E, text_disabled: 0xA8A8A8, hover: 0xD9EFE5, pressed: 0xC6E5D7, checked: 0xCDEBDD, accent: 0x1E7A55, accent_text: 0x17654A, on_accent: 0xFFFFFF, input: 0xFFFFFF, input_border: 0x638A78, status: 0xE2F3EC, menu: 0xFFFFFF }),
    ("dawn", Scheme { dark: false, title: 0xFBE9DF, ribbon: 0xFEF5EF, tabs: 0xFBE9DF, canvas: 0xF5DDD0, panel: 0xFFF9F5, border: 0xF0D5C5, border_strong: 0xE0B39A, text: 0x33201A, text_dim: 0x6A4B40, text_disabled: 0xA8A8A8, hover: 0xF8E1D4, pressed: 0xF1CFBD, checked: 0xF9DCD0, accent: 0xC2482E, accent_text: 0xA63C25, on_accent: 0xFFFFFF, input: 0xFFFFFF, input_border: 0x9C7666, status: 0xFBE9DF, menu: 0xFFFFFF }),
    ("ocean", Scheme { dark: true, title: 0x0B1E24, ribbon: 0x10282F, tabs: 0x0B1E24, canvas: 0x07161B, panel: 0x0E242B, border: 0x1C3F48, border_strong: 0x2C5C68, text: 0xE3F2F4, text_dim: 0xA6C6CC, text_disabled: 0x6A6A6A, hover: 0x173840, pressed: 0x1F4751, checked: 0x1F4A55, accent: 0x45C6C9, accent_text: 0x7ADDE0, on_accent: 0x0B1E24, input: 0x07161B, input_border: 0x4E8590, status: 0x0B1E24, menu: 0x10282F }),
    ("plum", Scheme { dark: true, title: 0x1C1024, ribbon: 0x251530, tabs: 0x1C1024, canvas: 0x140A1A, panel: 0x211329, border: 0x3A2447, border_strong: 0x56366A, text: 0xF1E6F6, text_dim: 0xC5AED2, text_disabled: 0x6A6A6A, hover: 0x322040, pressed: 0x3D2850, checked: 0x3C2852, accent: 0xC08BEA, accent_text: 0xD7B3F4, on_accent: 0x1C1024, input: 0x140A1A, input_border: 0x80659A, status: 0x1C1024, menu: 0x251530 }),
    ("coffee", Scheme { dark: true, title: 0x1E1612, ribbon: 0x28201A, tabs: 0x1E1612, canvas: 0x15100D, panel: 0x241C17, border: 0x3D2F26, border_strong: 0x5A4638, text: 0xF3EAE2, text_dim: 0xC9B8A8, text_disabled: 0x6A6A6A, hover: 0x352A22, pressed: 0x42342A, checked: 0x47372C, accent: 0xE0A060, accent_text: 0xEDBE88, on_accent: 0x1E1612, input: 0x15100D, input_border: 0x85705E, status: 0x1E1612, menu: 0x28201A }),
    ("rose", Scheme { dark: false, title: 0xF9E4EA, ribbon: 0xFDF2F5, tabs: 0xF9E4EA, canvas: 0xF3D9E1, panel: 0xFFF8FA, border: 0xEED3DC, border_strong: 0xDBAFBE, text: 0x2E1A22, text_dim: 0x6A4A56, text_disabled: 0xA8A8A8, hover: 0xF6DDE5, pressed: 0xEFCAD6, checked: 0xF5D3DF, accent: 0xB03A66, accent_text: 0x9A2F57, on_accent: 0xFFFFFF, input: 0xFFFFFF, input_border: 0x917080, status: 0xF9E4EA, menu: 0xFFFFFF }),
    ("apricot", Scheme { dark: false, title: 0xFCE9DC, ribbon: 0xFEF4EC, tabs: 0xFCE9DC, canvas: 0xF6DECD, panel: 0xFFF9F4, border: 0xF1D6C2, border_strong: 0xE0B596, text: 0x33231A, text_dim: 0x6B4E3E, text_disabled: 0xA8A8A8, hover: 0xF9E0CE, pressed: 0xF2CFB6, checked: 0xF8D9C3, accent: 0xB5521E, accent_text: 0x9C4416, on_accent: 0xFFFFFF, input: 0xFFFFFF, input_border: 0x93715E, status: 0xFCE9DC, menu: 0xFFFFFF }),
    ("powder", Scheme { dark: false, title: 0xE1ECF8, ribbon: 0xF1F6FD, tabs: 0xE1ECF8, canvas: 0xD6E4F4, panel: 0xF8FBFF, border: 0xCFDDF0, border_strong: 0xA9C0E0, text: 0x17263A, text_dim: 0x46586F, text_disabled: 0xA8A8A8, hover: 0xD9E6F6, pressed: 0xC6D8F0, checked: 0xCFE0F6, accent: 0x2F62B0, accent_text: 0x24549C, on_accent: 0xFFFFFF, input: 0xFFFFFF, input_border: 0x6B819E, status: 0xE1ECF8, menu: 0xFFFFFF }),
    ("lilac", Scheme { dark: false, title: 0xECE5F7, ribbon: 0xF6F2FC, tabs: 0xECE5F7, canvas: 0xE2D9F2, panel: 0xFBF8FF, border: 0xDCD0EF, border_strong: 0xBBA9DE, text: 0x241A35, text_dim: 0x56496D, text_disabled: 0xA8A8A8, hover: 0xE5DCF5, pressed: 0xD6C8EE, checked: 0xE0D3F6, accent: 0x7247B8, accent_text: 0x5F3AA0, on_accent: 0xFFFFFF, input: 0xFFFFFF, input_border: 0x7E6E9C, status: 0xECE5F7, menu: 0xFFFFFF }),
    ("sage", Scheme { dark: false, title: 0xE3EFE4, ribbon: 0xF2F8F2, tabs: 0xE3EFE4, canvas: 0xD7E8D9, panel: 0xF8FCF8, border: 0xCEE2D1, border_strong: 0xA8C9AD, text: 0x17291A, text_dim: 0x46604B, text_disabled: 0xA8A8A8, hover: 0xD9EBDC, pressed: 0xC6E0CA, checked: 0xCFE7D3, accent: 0x2F7A45, accent_text: 0x246638, on_accent: 0xFFFFFF, input: 0xFFFFFF, input_border: 0x6A8B70, status: 0xE3EFE4, menu: 0xFFFFFF }),
    ("vanilla", Scheme { dark: false, title: 0xF8F1D9, ribbon: 0xFCF9EC, tabs: 0xF8F1D9, canvas: 0xF1E8C8, panel: 0xFFFDF4, border: 0xEDE2BC, border_strong: 0xD6C38D, text: 0x2E2914, text_dim: 0x655B36, text_disabled: 0xA8A8A8, hover: 0xF4ECCF, pressed: 0xEDE0B5, checked: 0xF3E7BE, accent: 0x8A6A10, accent_text: 0x75590B, on_accent: 0xFFFFFF, input: 0xFFFFFF, input_border: 0x8F8259, status: 0xF8F1D9, menu: 0xFFFFFF }),
];

/// Документы' default.
const DEFAULT: &str = "navy";

static CURRENT: RwLock<Option<Scheme>> = RwLock::new(None);

pub fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// The scheme in force (None: Документы follows the system's light or dark).
pub fn current() -> Option<Scheme> {
    *CURRENT.read().unwrap_or_else(|e| e.into_inner())
}

/// Документы' settings file.
fn settings_file() -> Option<std::path::PathBuf> {
    let dir = if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("Library/Application Support/Lexicon"))
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(|a| std::path::PathBuf::from(a).join("Lexicon"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
            .map(|c| c.join("wordcraft"))
    };
    dir.map(|d| d.join("ui.json"))
}

/// The scheme for a settings code; an unknown or missing one is Документы' default.
pub fn for_code(code: Option<&str>) -> Option<Scheme> {
    let code = code.unwrap_or(DEFAULT);
    if code == "system" {
        return None;
    }
    SCHEMES.iter().find(|(c, _)| *c == code).or_else(|| SCHEMES.iter().find(|(c, _)| *c == DEFAULT)).map(|(_, s)| *s)
}

/// Read Документы' scheme; true when it changed.
pub fn refresh() -> bool {
    let code = settings_file()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v.get("theme").and_then(|t| t.as_str()).map(str::to_string));
    let next = for_code(code.as_deref());
    let mut cur = CURRENT.write().unwrap_or_else(|e| e.into_inner());
    let changed = *cur != next;
    *cur = next;
    changed
}

#[cfg(test)]
mod tests {
    #[test]
    fn codes_resolve() {
        assert!(super::for_code(None).is_some_and(|s| s.dark), "the default is dark blue");
        assert!(super::for_code(Some("rose")).is_some_and(|s| !s.dark));
        assert!(super::for_code(Some("system")).is_none());
        assert_eq!(super::for_code(Some("??")), super::for_code(None));
    }
}

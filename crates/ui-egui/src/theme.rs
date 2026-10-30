//! Design tokens (light and dark), UI fonts and the egui style.

use std::sync::Arc;

use egui::{Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Rect, Stroke, Visuals};

/// Lexicon's app colour (and its darker ink for text on light backgrounds).
pub const APP_COLOR: Color32 = Color32::from_rgb(0x3B, 0x5B, 0xDB);
pub const APP_INK: Color32 = Color32::from_rgb(0x2B, 0x47, 0xB5);

/// The interface theme setting (File › Options › General, View › Interface Theme): a fixed light or
/// dark palette, or `System`, which follows the OS light/dark appearance live (#115). Dark page
/// (View › Switch Modes) is separate: it changes how pages are drawn, never the interface (#312).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(from = "String", into = "String")]
pub enum Appearance {
    Light,
    Dark,
    System,
    /// Dark navy, almost black.
    Midnight,
    /// Dark, on black.
    Black,
    /// Light, on white.
    White,
    /// Dark blue: Lexicon's default.
    #[default]
    Navy,
    /// Light blue.
    Sky,
    /// Burgundy on beige.
    Burgundy,
    /// Dark green.
    Forest,
    /// Light lilac.
    Lavender,
    /// Light mint green.
    Mint,
    /// Light peach and coral.
    Dawn,
    /// Dark teal.
    Ocean,
    /// Dark plum.
    Plum,
    /// Dark coffee brown.
    Coffee,
}

impl Appearance {
    /// In menu order.
    pub const ALL: [Appearance; 16] = [
        Appearance::Navy,
        Appearance::Midnight,
        Appearance::System,
        Appearance::Light,
        Appearance::Dark,
        Appearance::Black,
        Appearance::White,
        Appearance::Sky,
        Appearance::Burgundy,
        Appearance::Forest,
        Appearance::Lavender,
        Appearance::Mint,
        Appearance::Dawn,
        Appearance::Ocean,
        Appearance::Plum,
        Appearance::Coffee,
    ];

    /// The value used in `ui.json` and by the `ui.theme` command.
    pub fn code(self) -> &'static str {
        match self {
            Appearance::Light => "light",
            Appearance::Dark => "dark",
            Appearance::System => "system",
            // Not "midnight": that was saved as the default by one earlier build, and now reads as
            // the new default, Navy.
            Appearance::Midnight => "night",
            Appearance::Black => "black",
            Appearance::White => "white",
            Appearance::Navy => "navy",
            Appearance::Sky => "sky",
            Appearance::Burgundy => "burgundy",
            Appearance::Forest => "forest",
            Appearance::Lavender => "lavender",
            Appearance::Mint => "mint",
            Appearance::Dawn => "dawn",
            Appearance::Ocean => "ocean",
            Appearance::Plum => "plum",
            Appearance::Coffee => "coffee",
        }
    }

    pub fn from_code(code: &str) -> Option<Appearance> {
        Appearance::ALL.into_iter().find(|a| a.code().eq_ignore_ascii_case(code.trim()))
    }

    /// The English label (translated with `tl!` where shown).
    pub fn label(self) -> &'static str {
        match self {
            Appearance::Light => "Light",
            Appearance::Dark => "Dark",
            Appearance::System => "Use system setting",
            Appearance::Midnight => "Midnight",
            Appearance::Black => "Black",
            Appearance::White => "White",
            Appearance::Navy => "Dark Blue",
            Appearance::Sky => "Light Blue",
            Appearance::Burgundy => "Burgundy and Beige",
            Appearance::Forest => "Forest",
            Appearance::Lavender => "Lavender",
            Appearance::Mint => "Mint",
            Appearance::Dawn => "Dawn",
            Appearance::Ocean => "Ocean",
            Appearance::Plum => "Plum",
            Appearance::Coffee => "Coffee",
        }
    }

    /// The palette of a fixed theme; `None` for `System`, which follows the OS.
    pub fn palette(self) -> Option<Tokens> {
        Some(match self {
            Appearance::System => return None,
            Appearance::Light => Tokens::light(),
            Appearance::Dark => Tokens::dark(),
            Appearance::Midnight => Tokens::midnight(),
            Appearance::Black => Tokens::black(),
            Appearance::White => Tokens::white(),
            Appearance::Navy => Tokens::navy(),
            Appearance::Sky => Tokens::sky(),
            Appearance::Burgundy => Tokens::burgundy(),
            Appearance::Forest => Tokens::forest(),
            Appearance::Lavender => Tokens::lavender(),
            Appearance::Mint => Tokens::mint(),
            Appearance::Dawn => Tokens::dawn(),
            Appearance::Ocean => Tokens::ocean(),
            Appearance::Plum => Tokens::plum(),
            Appearance::Coffee => Tokens::coffee(),
        })
    }

    /// Whether the interface is dark, given the OS appearance egui reports (`Context::system_theme`).
    /// `System` falls back to light when the OS reports none.
    pub fn is_dark(self, system: Option<egui::Theme>) -> bool {
        match self.palette() {
            Some(t) => t.dark,
            None => system == Some(egui::Theme::Dark),
        }
    }

    /// The egui theme preference: `System` stays automatic rather than resolving to a fixed theme.
    pub fn preference(self) -> egui::ThemePreference {
        match self.palette() {
            Some(t) if t.dark => egui::ThemePreference::Dark,
            Some(_) => egui::ThemePreference::Light,
            None => egui::ThemePreference::System,
        }
    }
}

/// An unknown saved value reads as the default rather than discarding the whole `ui.json`.
impl From<String> for Appearance {
    fn from(s: String) -> Self {
        Appearance::from_code(&s).unwrap_or_default()
    }
}

impl From<Appearance> for String {
    fn from(a: Appearance) -> Self {
        a.code().to_string()
    }
}

/// Interface size (File › Options › General, `ui.interfaceSize`): how large the ribbon, menus, panes
/// and dialogs are drawn, through egui's zoom factor. Document zoom (View › Zoom) is a separate
/// setting and never changes with it.
///
/// `Auto` (the default) keeps the display scale the system reports. A chosen size is absolute, as in
/// PhotoCraft and PdfCraft (#822): "100%" is one screen pixel per point whatever the display reports,
/// so a system that guesses 200 % (a bare window manager on X11, where winit falls back to the
/// monitor's DPI) doesn't make "100%" draw at 200 %. The display's scale is divided back out in
/// [`InterfaceSize::zoom_factor`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(from = "serde_json::Value", into = "serde_json::Value")]
pub struct InterfaceSize(Option<u16>);

impl InterfaceSize {
    pub const MIN: u16 = 50;
    pub const MAX: u16 = 300;
    /// Follow the display scale the system reports.
    pub const AUTO: InterfaceSize = InterfaceSize(None);
    /// The sizes File › Options offers after Auto, in percent (PhotoCraft's and PdfCraft's).
    pub const PRESETS: [u16; 12] = [75, 80, 85, 90, 95, 100, 125, 150, 175, 200, 250, 300];

    /// A chosen size from any number: rounded and clamped to 50–300 %; `None` for NaN or infinity.
    pub fn from_percent(percent: f64) -> Option<InterfaceSize> {
        percent.is_finite().then(|| InterfaceSize(Some(percent.round().clamp(f64::from(Self::MIN), f64::from(Self::MAX)) as u16)))
    }

    /// The chosen size in percent, or `None` for Auto.
    pub fn percent(self) -> Option<u16> {
        self.0
    }

    pub fn is_auto(self) -> bool {
        self.0.is_none()
    }

    /// The egui zoom factor on a display whose own scale (native pixels per point) is `native`:
    /// 1.0 for Auto, or the chosen size with the display's scale divided back out.
    pub fn zoom_factor(self, native: f32) -> f32 {
        match self.0 {
            None => 1.0,
            Some(pct) => f32::from(pct) / 100.0 / usable_scale(native),
        }
    }

    /// The size the interface is drawn at on that display, in percent (screen pixels per point × 100).
    pub fn drawn_percent(self, native: f32) -> u16 {
        let drawn = usable_scale(native) * self.zoom_factor(native) * 100.0;
        drawn.round().clamp(0.0, f32::from(u16::MAX)) as u16
    }
}

/// The display's scale, or 1.0 when egui doesn't know it (or reports nonsense).
pub fn usable_scale(native: f32) -> f32 {
    if native.is_finite() && native > 0.0 { native } else { 1.0 }
}

/// `"auto"`, or a percentage; anything else reads as Auto rather than discarding the whole `ui.json`.
impl From<serde_json::Value> for InterfaceSize {
    fn from(v: serde_json::Value) -> Self {
        v.as_f64().and_then(InterfaceSize::from_percent).unwrap_or_default()
    }
}

impl From<InterfaceSize> for serde_json::Value {
    fn from(s: InterfaceSize) -> Self {
        s.0.map_or_else(|| serde_json::Value::from("auto"), serde_json::Value::from)
    }
}

/// A `ui.json` from before #822 saved `interfaceSize` as a percentage on top of the display scale.
/// Its default, 100, meant "the display's scale", which is Auto now (`None`); another value was
/// picked on purpose and is returned, to become an absolute size once the display's scale is
/// known ([`migrate_legacy_interface_size`]). Junk reads as Auto.
pub fn legacy_interface_percent(v: &serde_json::Value) -> Option<f64> {
    v.as_f64().filter(|p| p.is_finite() && (p - 100.0).abs() >= 0.5)
}

/// The absolute size a pre-#822 `interfaceSize` of `old` percent drew at on a display whose own
/// scale is `native`: `old × native`, clamped to 50–300 %. A 2× display with 125 % drew at 250 %,
/// and keeps drawing there.
pub fn migrate_legacy_interface_size(old: f64, native: f32) -> InterfaceSize {
    InterfaceSize::from_percent(old * f64::from(usable_scale(native))).unwrap_or_default()
}

/// The Animation effects setting (File › Options › Accessibility, `ui.animations`): on, off, or
/// `System`, which follows the system's own setting as the host reports it
/// ([`Services::animations_enabled`](crate::Services::animations_enabled): Windows' Animation
/// effects, macOS' Reduce motion, the Linux/BSD desktop's setting, a browser's
/// `prefers-reduced-motion`).
/// Off: wheel notches scroll and zoom at once ([`crate::scroll`]) and nothing fades or glides
/// ([`apply_animations`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(from = "String", into = "String")]
pub enum Animations {
    #[default]
    System,
    On,
    Off,
}

impl Animations {
    /// In menu order.
    pub const ALL: [Animations; 3] = [Animations::System, Animations::On, Animations::Off];

    /// The value used in `ui.json` and by the `ui.animations` command.
    pub fn code(self) -> &'static str {
        match self {
            Animations::System => "system",
            Animations::On => "on",
            Animations::Off => "off",
        }
    }

    pub fn from_code(code: &str) -> Option<Animations> {
        Animations::ALL.into_iter().find(|a| a.code().eq_ignore_ascii_case(code.trim()))
    }

    /// The English label (translated with `tl!` where shown).
    pub fn label(self) -> &'static str {
        match self {
            Animations::System => "Use system setting",
            Animations::On => "On",
            Animations::Off => "Off",
        }
    }

    /// Whether animation effects are on, given what the system reports (`None`: it doesn't say).
    /// `System` falls back to on when the system reports nothing.
    pub fn enabled(self, system: Option<bool>) -> bool {
        match self {
            Animations::System => system.unwrap_or(true),
            Animations::On => true,
            Animations::Off => false,
        }
    }
}

/// An unknown saved value reads as the default rather than discarding the whole `ui.json`.
impl From<String> for Animations {
    fn from(s: String) -> Self {
        Animations::from_code(&s).unwrap_or_default()
    }
}

impl From<Animations> for String {
    fn from(a: Animations) -> Self {
        a.code().to_string()
    }
}

/// `0xRRGGBB` as a colour.
const fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Where [`apply`] keeps the fixed theme's palette for [`Tokens::get`].
fn palette_id() -> egui::Id {
    egui::Id::new("lexicon-palette")
}

/// Every colour the UI uses; widgets never hard-code colours.
#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub dark: bool,
    pub title_bar: Color32,
    pub ribbon: Color32,
    pub tab_strip: Color32,
    pub canvas: Color32,
    pub panel: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_disabled: Color32,
    pub icon: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub checked: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    /// Text/icon colour drawn on top of `accent` (e.g. keytip badge letters).
    pub on_accent: Color32,
    pub input: Color32,
    pub input_border: Color32,
    pub group_label: Color32,
    pub ruler: Color32,
    pub ruler_margin: Color32,
    pub ruler_tick: Color32,
    pub status_bar: Color32,
    pub selection: Color32,
    /// Find › Reading Highlight.
    pub find_highlight: Color32,
    pub page_shadow: Color32,
    /// Selection-frame handles (pictures, shapes, text boxes) and their shadow.
    pub handle: Color32,
    pub handle_shadow: Color32,
    /// Alignment guides while moving a picture or shape: drawn on the page, so legible on white
    /// paper in both themes.
    pub guide: Color32,
    /// Crop mode: the crop handles, and the dim over the cropped-away part of a picture.
    pub crop_handle: Color32,
    pub crop_dim: Color32,
    /// Comment and revision balloons in the markup area beside the page (#2045): the card, the
    /// area behind it and its edge. The card holds `text`, so it is dark in the dark theme.
    pub balloon: Color32,
    pub markup_area: Color32,
    pub markup_edge: Color32,
    /// "Resolved" on a balloon: `green` is only 4.2:1 on the light card.
    pub resolved_text: Color32,
    /// One stripe and leader colour per comment author, in turn.
    pub author_colors: [Color32; 5],
    /// The same colours for the leader line across the page: the paper stays white in the dark
    /// theme, so these are the light-theme colours in both.
    pub leader_colors: [Color32; 5],
    pub menu: Color32,
    pub blue: Color32,
    pub red: Color32,
    pub green: Color32,
    pub orange: Color32,
    /// A window's close box while the pointer is over it (the red of a Windows caption button),
    /// while pressed, and the ✕ drawn on it then (#1885).
    pub close_hover: Color32,
    pub close_pressed: Color32,
    pub on_close: Color32,
}

impl Tokens {
    pub fn light() -> Self {
        Tokens {
            dark: false,
            title_bar: Color32::from_rgb(0xF0, 0xF0, 0xF0),
            ribbon: Color32::from_rgb(0xF9, 0xF9, 0xF9),
            tab_strip: Color32::from_rgb(0xF0, 0xF0, 0xF0),
            canvas: Color32::from_rgb(0xE6, 0xE6, 0xE6),
            panel: Color32::from_rgb(0xFA, 0xFA, 0xFA),
            border: Color32::from_rgb(0xE1, 0xE1, 0xE1),
            border_strong: Color32::from_rgb(0xC8, 0xC8, 0xC8),
            text: Color32::from_rgb(0x24, 0x24, 0x24),
            text_dim: Color32::from_rgb(0x61, 0x61, 0x61),
            text_disabled: Color32::from_rgb(0xA8, 0xA8, 0xA8),
            icon: Color32::from_rgb(0x42, 0x42, 0x42),
            hover: Color32::from_rgb(0xE6, 0xE6, 0xE6),
            pressed: Color32::from_rgb(0xD6, 0xD6, 0xD6),
            checked: Color32::from_rgb(0xDD, 0xE3, 0xF8),
            accent: APP_COLOR,
            accent_text: APP_INK,
            on_accent: Color32::WHITE,
            input: Color32::WHITE,
            input_border: Color32::from_rgb(0x8A, 0x8A, 0x8A),
            group_label: Color32::from_rgb(0x70, 0x70, 0x70),
            ruler: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            ruler_margin: Color32::from_rgb(0xD4, 0xD4, 0xD4),
            ruler_tick: Color32::from_rgb(0x70, 0x70, 0x70),
            status_bar: Color32::from_rgb(0xF0, 0xF0, 0xF0),
            selection: Color32::from_rgba_unmultiplied(0x3B, 0x5B, 0xDB, 0x48),
            find_highlight: Color32::from_rgba_unmultiplied(0xFF, 0xE0, 0x3D, 0x90),
            page_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 0x22),
            handle: Color32::WHITE,
            handle_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 0x28),
            guide: Color32::from_rgb(0xC0, 0x28, 0x8C),
            crop_handle: Color32::BLACK,
            crop_dim: Color32::from_rgba_unmultiplied(0x30, 0x30, 0x30, 0xE0),
            balloon: Color32::WHITE,
            markup_area: Color32::from_rgb(0xF3, 0xF3, 0xF3),
            markup_edge: Color32::from_rgb(0xE0, 0xE0, 0xE0),
            resolved_text: Color32::from_rgb(0x1B, 0x6B, 0x42),
            author_colors: [
                Color32::from_rgb(0x2F, 0x6F, 0xD0),
                Color32::from_rgb(0xB0, 0x3A, 0x2E),
                Color32::from_rgb(0x2E, 0x7D, 0x32),
                Color32::from_rgb(0x8E, 0x44, 0xAD),
                Color32::from_rgb(0xE0, 0x7B, 0x1F),
            ],
            leader_colors: [
                Color32::from_rgb(0x2F, 0x6F, 0xD0),
                Color32::from_rgb(0xB0, 0x3A, 0x2E),
                Color32::from_rgb(0x2E, 0x7D, 0x32),
                Color32::from_rgb(0x8E, 0x44, 0xAD),
                Color32::from_rgb(0xE0, 0x7B, 0x1F),
            ],
            menu: Color32::WHITE,
            blue: Color32::from_rgb(0x2F, 0x6F, 0xD0),
            red: Color32::from_rgb(0xD1, 0x3B, 0x3B),
            green: Color32::from_rgb(0x2E, 0x8B, 0x57),
            orange: Color32::from_rgb(0xE0, 0x7B, 0x1F),
            close_hover: Color32::from_rgb(0xC4, 0x2B, 0x1C),
            close_pressed: Color32::from_rgb(0xA6, 0x23, 0x16),
            on_close: Color32::WHITE,
        }
    }
    pub fn dark() -> Self {
        Tokens {
            dark: true,
            title_bar: Color32::from_rgb(0x1F, 0x1F, 0x1F),
            ribbon: Color32::from_rgb(0x29, 0x29, 0x29),
            tab_strip: Color32::from_rgb(0x1F, 0x1F, 0x1F),
            canvas: Color32::from_rgb(0x14, 0x14, 0x14),
            panel: Color32::from_rgb(0x26, 0x26, 0x26),
            border: Color32::from_rgb(0x3A, 0x3A, 0x3A),
            border_strong: Color32::from_rgb(0x55, 0x55, 0x55),
            text: Color32::from_rgb(0xEE, 0xEE, 0xEE),
            text_dim: Color32::from_rgb(0xC8, 0xC8, 0xC8),
            text_disabled: Color32::from_rgb(0x6A, 0x6A, 0x6A),
            icon: Color32::from_rgb(0xDD, 0xDD, 0xDD),
            hover: Color32::from_rgb(0x3D, 0x3D, 0x3D),
            pressed: Color32::from_rgb(0x4A, 0x4A, 0x4A),
            checked: Color32::from_rgb(0x2E, 0x3A, 0x66),
            accent: Color32::from_rgb(0x7A, 0x93, 0xF0),
            accent_text: Color32::from_rgb(0x9C, 0xB0, 0xF5),
            // Dark text on the light dark-theme accent: white on #7a93f0 is only 2.9:1 (#655).
            on_accent: Color32::from_rgb(0x11, 0x16, 0x2E),
            input: Color32::from_rgb(0x1E, 0x1E, 0x1E),
            input_border: Color32::from_rgb(0x7A, 0x7A, 0x7A),
            group_label: Color32::from_rgb(0xA0, 0xA0, 0xA0),
            // The text area is lighter than the margins, as in the light theme (#1148).
            ruler: Color32::from_rgb(0x45, 0x45, 0x45),
            ruler_margin: Color32::from_rgb(0x2E, 0x2E, 0x2E),
            ruler_tick: Color32::from_rgb(0xB4, 0xB4, 0xB4),
            status_bar: Color32::from_rgb(0x1F, 0x1F, 0x1F),
            selection: Color32::from_rgba_unmultiplied(0x7A, 0x93, 0xF0, 0x55),
            find_highlight: Color32::from_rgba_unmultiplied(0xC8, 0xA8, 0x00, 0x70),
            page_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 0x60),
            handle: Color32::from_rgb(0xF2, 0xF2, 0xF2),
            handle_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 0x5A),
            guide: Color32::from_rgb(0xE0, 0x4C, 0xB0),
            crop_handle: Color32::BLACK,
            crop_dim: Color32::from_rgba_unmultiplied(0x30, 0x30, 0x30, 0xE0),
            balloon: Color32::from_rgb(0x2B, 0x2B, 0x2B),
            markup_area: Color32::from_rgb(0x1B, 0x1B, 0x1B),
            markup_edge: Color32::from_rgb(0x3A, 0x3A, 0x3A),
            resolved_text: Color32::from_rgb(0x5B, 0xC0, 0x8A),
            author_colors: [
                Color32::from_rgb(0x7F, 0xA8, 0xF2),
                Color32::from_rgb(0xE5, 0x78, 0x6B),
                Color32::from_rgb(0x6F, 0xBF, 0x73),
                Color32::from_rgb(0xC0, 0x8F, 0xE0),
                Color32::from_rgb(0xF0, 0xA0, 0x50),
            ],
            leader_colors: [
                Color32::from_rgb(0x2F, 0x6F, 0xD0),
                Color32::from_rgb(0xB0, 0x3A, 0x2E),
                Color32::from_rgb(0x2E, 0x7D, 0x32),
                Color32::from_rgb(0x8E, 0x44, 0xAD),
                Color32::from_rgb(0xE0, 0x7B, 0x1F),
            ],
            menu: Color32::from_rgb(0x2B, 0x2B, 0x2B),
            blue: Color32::from_rgb(0x6C, 0x9B, 0xEA),
            red: Color32::from_rgb(0xEE, 0x6A, 0x6A),
            green: Color32::from_rgb(0x5B, 0xC0, 0x8A),
            orange: Color32::from_rgb(0xF0, 0xA0, 0x50),
            // A little lighter than the light theme's red, so it reads against the dark window.
            close_hover: Color32::from_rgb(0xD6, 0x3C, 0x2C),
            close_pressed: Color32::from_rgb(0xB3, 0x30, 0x23),
            on_close: Color32::WHITE,
        }
    }
    /// The palette [`apply`] installed (a fixed theme), else the light or dark one egui shows.
    pub fn get(ctx: &egui::Context) -> Tokens {
        let dark = ctx.global_style().visuals.dark_mode;
        match ctx.data(|d| d.get_temp::<Tokens>(palette_id())) {
            Some(t) if t.dark == dark => t,
            _ if dark => Tokens::dark(),
            _ => Tokens::light(),
        }
    }

    /// Every fixed palette (for tests and previews).
    pub fn all_palettes() -> Vec<Tokens> {
        Appearance::ALL.into_iter().filter_map(Appearance::palette).collect()
    }

    /// Dark navy, almost black.
    pub fn midnight() -> Self {
        Tokens {
            title_bar: rgb(0x0B1020),
            ribbon: rgb(0x111A2E),
            tab_strip: rgb(0x0B1020),
            canvas: rgb(0x070B16),
            panel: rgb(0x0F1729),
            border: rgb(0x1E2A44),
            border_strong: rgb(0x34466C),
            text: rgb(0xE6EAF2),
            text_dim: rgb(0xA9B4C8),
            icon: rgb(0xC8D1E0),
            hover: rgb(0x1B2640),
            pressed: rgb(0x24314F),
            checked: rgb(0x22345E),
            accent: rgb(0x6F9BFF),
            accent_text: rgb(0x9DBBFF),
            on_accent: rgb(0x0B1020),
            input: rgb(0x0C1424),
            input_border: rgb(0x5D6F95),
            group_label: rgb(0x96A2B8),
            ruler: rgb(0x1E2840),
            ruler_margin: rgb(0x111827),
            ruler_tick: rgb(0xB8C2D6),
            status_bar: rgb(0x0B1020),
            balloon: rgb(0x16203A),
            markup_area: rgb(0x0D1424),
            markup_edge: rgb(0x2A3654),
            menu: rgb(0x131D33),
            selection: Color32::from_rgba_unmultiplied(0x6F, 0x9B, 0xFF, 0x55),
            resolved_text: rgb(0x8EE6B5),
            ..Tokens::dark()
        }
    }

    /// Dark, on black.
    pub fn black() -> Self {
        Tokens {
            title_bar: rgb(0x000000),
            ribbon: rgb(0x0A0A0A),
            tab_strip: rgb(0x000000),
            canvas: rgb(0x050505),
            panel: rgb(0x0D0D0D),
            border: rgb(0x222222),
            border_strong: rgb(0x3A3A3A),
            hover: rgb(0x1C1C1C),
            pressed: rgb(0x262626),
            checked: rgb(0x2A2F3A),
            input: rgb(0x0A0A0A),
            ruler: rgb(0x1A1A1A),
            ruler_margin: rgb(0x0A0A0A),
            status_bar: rgb(0x000000),
            balloon: rgb(0x151515),
            markup_area: rgb(0x050505),
            markup_edge: rgb(0x2A2A2A),
            menu: rgb(0x111111),
            ..Tokens::dark()
        }
    }

    /// Light, on white (as Word 2024 for Mac looks).
    pub fn white() -> Self {
        Tokens {
            title_bar: rgb(0xFFFFFF),
            ribbon: rgb(0xFFFFFF),
            tab_strip: rgb(0xFFFFFF),
            canvas: rgb(0xEDEDED),
            panel: rgb(0xFFFFFF),
            border: rgb(0xE5E5E5),
            hover: rgb(0xEEEEEE),
            pressed: rgb(0xE0E0E0),
            status_bar: rgb(0xF7F7F7),
            markup_area: rgb(0xF5F5F5),
            ..Tokens::light()
        }
    }

    /// Dark blue.
    pub fn navy() -> Self {
        Tokens {
            title_bar: rgb(0x14213D),
            ribbon: rgb(0x1B2A4A),
            tab_strip: rgb(0x14213D),
            canvas: rgb(0x0F1A30),
            panel: rgb(0x192745),
            border: rgb(0x2A3C62),
            border_strong: rgb(0x40578A),
            text: rgb(0xEAF0FA),
            text_dim: rgb(0xB3C0D8),
            icon: rgb(0xD0DAEC),
            hover: rgb(0x243659),
            pressed: rgb(0x2E4470),
            checked: rgb(0x293F6C),
            accent: rgb(0x7AA8FF),
            accent_text: rgb(0xA5C4FF),
            on_accent: rgb(0x0B1530),
            input: rgb(0x13203A),
            input_border: rgb(0x6A80AE),
            group_label: rgb(0xA2B1CC),
            ruler: rgb(0x26385F),
            ruler_margin: rgb(0x18264A),
            ruler_tick: rgb(0xC6D2E8),
            status_bar: rgb(0x14213D),
            balloon: rgb(0x1E2F52),
            markup_area: rgb(0x132038),
            markup_edge: rgb(0x2F4470),
            menu: rgb(0x1D2D50),
            selection: Color32::from_rgba_unmultiplied(0x7A, 0xA8, 0xFF, 0x55),
            resolved_text: rgb(0x8EE6B5),
            ..Tokens::dark()
        }
    }

    /// Light blue.
    pub fn sky() -> Self {
        Tokens {
            title_bar: rgb(0xDCE8F8),
            ribbon: rgb(0xEEF4FC),
            tab_strip: rgb(0xDCE8F8),
            canvas: rgb(0xD5E2F3),
            panel: rgb(0xF3F7FD),
            border: rgb(0xC9D8EE),
            border_strong: rgb(0xA9BEDD),
            text: rgb(0x14243C),
            text_dim: rgb(0x435673),
            icon: rgb(0x2A3E5E),
            hover: rgb(0xD6E4F7),
            pressed: rgb(0xC3D5F0),
            checked: rgb(0xC6D8F5),
            accent: rgb(0x2F64C8),
            accent_text: rgb(0x1E4FA8),
            input_border: rgb(0x5E7598),
            group_label: rgb(0x4E6283),
            ruler_margin: rgb(0xC9D8EE),
            ruler_tick: rgb(0x435673),
            status_bar: rgb(0xDCE8F8),
            markup_area: rgb(0xE8F0FA),
            markup_edge: rgb(0xC9D8EE),
            selection: Color32::from_rgba_unmultiplied(0x2F, 0x64, 0xC8, 0x44),
            ..Tokens::light()
        }
    }

    /// Burgundy on beige.
    pub fn burgundy() -> Self {
        Tokens {
            title_bar: rgb(0xEFE3D3),
            ribbon: rgb(0xF8F0E5),
            tab_strip: rgb(0xEFE3D3),
            canvas: rgb(0xE6D8C6),
            panel: rgb(0xFBF5EC),
            border: rgb(0xE0D0BC),
            border_strong: rgb(0xCBB59A),
            text: rgb(0x2E1A1E),
            text_dim: rgb(0x654848),
            icon: rgb(0x5A2A35),
            hover: rgb(0xEDDCC8),
            pressed: rgb(0xE2CCB2),
            checked: rgb(0xEFD9DC),
            accent: rgb(0x8C1D3A),
            accent_text: rgb(0x7A1832),
            input: rgb(0xFFFDF9),
            input_border: rgb(0x85695A),
            group_label: rgb(0x735852),
            ruler: rgb(0xFFFBF5),
            ruler_margin: rgb(0xE3D3BF),
            ruler_tick: rgb(0x654848),
            status_bar: rgb(0xEFE3D3),
            balloon: rgb(0xFFFBF5),
            markup_area: rgb(0xF2E8DA),
            markup_edge: rgb(0xDCCAB4),
            menu: rgb(0xFFFBF5),
            selection: Color32::from_rgba_unmultiplied(0x8C, 0x1D, 0x3A, 0x40),
            ..Tokens::light()
        }
    }

    /// Light lilac.
    pub fn lavender() -> Self {
        Tokens {
            title_bar: rgb(0xECE6F7),
            ribbon: rgb(0xF6F2FC),
            tab_strip: rgb(0xECE6F7),
            canvas: rgb(0xE2DAF2),
            panel: rgb(0xFAF7FE),
            border: rgb(0xDDD3EF),
            border_strong: rgb(0xBFAFDF),
            text: rgb(0x231A33),
            text_dim: rgb(0x56486E),
            icon: rgb(0x3D2F57),
            hover: rgb(0xE6DCF5),
            pressed: rgb(0xD8CAEE),
            checked: rgb(0xE3D5F7),
            accent: rgb(0x6A3FB5),
            accent_text: rgb(0x5A32A0),
            input_border: rgb(0x7D6A9E),
            group_label: rgb(0x56486E),
            ruler_margin: rgb(0xDDD3EF),
            ruler_tick: rgb(0x56486E),
            status_bar: rgb(0xECE6F7),
            markup_area: rgb(0xFAF7FE),
            markup_edge: rgb(0xDDD3EF),
            ..Tokens::light()
        }
    }

    /// Light mint green.
    pub fn mint() -> Self {
        Tokens {
            title_bar: rgb(0xE2F3EC),
            ribbon: rgb(0xF1FAF6),
            tab_strip: rgb(0xE2F3EC),
            canvas: rgb(0xD6EDE3),
            panel: rgb(0xF7FCFA),
            border: rgb(0xCDE6DA),
            border_strong: rgb(0xA9D2BF),
            text: rgb(0x15291F),
            text_dim: rgb(0x3F5C4E),
            icon: rgb(0x24443A),
            hover: rgb(0xD9EFE5),
            pressed: rgb(0xC6E5D7),
            checked: rgb(0xCDEBDD),
            accent: rgb(0x1E7A55),
            accent_text: rgb(0x17654A),
            input_border: rgb(0x638A78),
            group_label: rgb(0x3F5C4E),
            ruler_margin: rgb(0xCDE6DA),
            ruler_tick: rgb(0x3F5C4E),
            status_bar: rgb(0xE2F3EC),
            markup_area: rgb(0xF7FCFA),
            markup_edge: rgb(0xCDE6DA),
            ..Tokens::light()
        }
    }

    /// Light peach and coral.
    pub fn dawn() -> Self {
        Tokens {
            title_bar: rgb(0xFBE9DF),
            ribbon: rgb(0xFEF5EF),
            tab_strip: rgb(0xFBE9DF),
            canvas: rgb(0xF5DDD0),
            panel: rgb(0xFFF9F5),
            border: rgb(0xF0D5C5),
            border_strong: rgb(0xE0B39A),
            text: rgb(0x33201A),
            text_dim: rgb(0x6A4B40),
            icon: rgb(0x5A3428),
            hover: rgb(0xF8E1D4),
            pressed: rgb(0xF1CFBD),
            checked: rgb(0xF9DCD0),
            accent: rgb(0xC2482E),
            accent_text: rgb(0xA63C25),
            input_border: rgb(0x9C7666),
            group_label: rgb(0x6A4B40),
            ruler_margin: rgb(0xF0D5C5),
            ruler_tick: rgb(0x6A4B40),
            status_bar: rgb(0xFBE9DF),
            markup_area: rgb(0xFFF9F5),
            markup_edge: rgb(0xF0D5C5),
            ..Tokens::light()
        }
    }

    /// Dark teal.
    pub fn ocean() -> Self {
        Tokens {
            title_bar: rgb(0x0B1E24),
            ribbon: rgb(0x10282F),
            tab_strip: rgb(0x0B1E24),
            canvas: rgb(0x07161B),
            panel: rgb(0x0E242B),
            border: rgb(0x1C3F48),
            border_strong: rgb(0x2C5C68),
            text: rgb(0xE3F2F4),
            text_dim: rgb(0xA6C6CC),
            icon: rgb(0xC6DFE3),
            hover: rgb(0x173840),
            pressed: rgb(0x1F4751),
            checked: rgb(0x1F4A55),
            accent: rgb(0x45C6C9),
            accent_text: rgb(0x7ADDE0),
            on_accent: rgb(0x0B1E24),
            input: rgb(0x07161B),
            input_border: rgb(0x4E8590),
            group_label: rgb(0xA6C6CC),
            ruler: rgb(0x1A3840),
            ruler_margin: rgb(0x07161B),
            ruler_tick: rgb(0xA6C6CC),
            status_bar: rgb(0x0B1E24),
            balloon: rgb(0x0E242B),
            markup_area: rgb(0x07161B),
            markup_edge: rgb(0x1C3F48),
            menu: rgb(0x10282F),
            resolved_text: rgb(0x8EE6B5),
            ..Tokens::dark()
        }
    }

    /// Dark plum.
    pub fn plum() -> Self {
        Tokens {
            title_bar: rgb(0x1C1024),
            ribbon: rgb(0x251530),
            tab_strip: rgb(0x1C1024),
            canvas: rgb(0x140A1A),
            panel: rgb(0x211329),
            border: rgb(0x3A2447),
            border_strong: rgb(0x56366A),
            text: rgb(0xF1E6F6),
            text_dim: rgb(0xC5AED2),
            icon: rgb(0xDCCAE6),
            hover: rgb(0x322040),
            pressed: rgb(0x3D2850),
            checked: rgb(0x3C2852),
            accent: rgb(0xC08BEA),
            accent_text: rgb(0xD7B3F4),
            on_accent: rgb(0x1C1024),
            input: rgb(0x140A1A),
            input_border: rgb(0x80659A),
            group_label: rgb(0xC5AED2),
            ruler: rgb(0x322044),
            ruler_margin: rgb(0x140A1A),
            ruler_tick: rgb(0xC5AED2),
            status_bar: rgb(0x1C1024),
            balloon: rgb(0x211329),
            markup_area: rgb(0x140A1A),
            markup_edge: rgb(0x3A2447),
            menu: rgb(0x251530),
            resolved_text: rgb(0x8EE6B5),
            ..Tokens::dark()
        }
    }

    /// Dark coffee brown.
    pub fn coffee() -> Self {
        Tokens {
            title_bar: rgb(0x1E1612),
            ribbon: rgb(0x28201A),
            tab_strip: rgb(0x1E1612),
            canvas: rgb(0x15100D),
            panel: rgb(0x241C17),
            border: rgb(0x3D2F26),
            border_strong: rgb(0x5A4638),
            text: rgb(0xF3EAE2),
            text_dim: rgb(0xC9B8A8),
            icon: rgb(0xE0D2C4),
            hover: rgb(0x352A22),
            pressed: rgb(0x42342A),
            checked: rgb(0x47372C),
            accent: rgb(0xE0A060),
            accent_text: rgb(0xEDBE88),
            on_accent: rgb(0x1E1612),
            input: rgb(0x15100D),
            input_border: rgb(0x85705E),
            group_label: rgb(0xC9B8A8),
            ruler: rgb(0x35291F),
            ruler_margin: rgb(0x15100D),
            ruler_tick: rgb(0xC9B8A8),
            status_bar: rgb(0x1E1612),
            balloon: rgb(0x241C17),
            markup_area: rgb(0x15100D),
            markup_edge: rgb(0x3D2F26),
            menu: rgb(0x28201A),
            resolved_text: rgb(0x8EE6B5),
            ..Tokens::dark()
        }
    }

    /// Dark green.
    pub fn forest() -> Self {
        Tokens {
            title_bar: rgb(0x0F1E17),
            ribbon: rgb(0x15291F),
            tab_strip: rgb(0x0F1E17),
            canvas: rgb(0x0B1611),
            panel: rgb(0x132519),
            border: rgb(0x22402F),
            border_strong: rgb(0x356049),
            text: rgb(0xE6F2EA),
            text_dim: rgb(0xA9C4B3),
            icon: rgb(0xC9DECF),
            hover: rgb(0x1D3828),
            pressed: rgb(0x264733),
            checked: rgb(0x24503A),
            accent: rgb(0x4CC38A),
            accent_text: rgb(0x7FDCAE),
            on_accent: rgb(0x08140E),
            input: rgb(0x0E1C14),
            input_border: rgb(0x55836A),
            group_label: rgb(0x98B5A3),
            ruler: rgb(0x1C3727),
            ruler_margin: rgb(0x122419),
            ruler_tick: rgb(0xBCD3C4),
            status_bar: rgb(0x0F1E17),
            balloon: rgb(0x183022),
            markup_area: rgb(0x0E1C14),
            markup_edge: rgb(0x2A4C38),
            menu: rgb(0x17301F),
            selection: Color32::from_rgba_unmultiplied(0x4C, 0xC3, 0x8A, 0x50),
            resolved_text: rgb(0x8EE6B5),
            ..Tokens::dark()
        }
    }
}

/// Install the UI fonts (Inter, JetBrains Mono) and the craft-fonts CJK interface faces if built
/// in, Japanese first.
pub fn install_fonts(ctx: &egui::Context) {
    install_fonts_for(ctx, false);
}

/// [`install_fonts`] with the Chinese or Japanese CJK fallback order. The new fonts apply from the
/// next frame.
pub fn install_fonts_for(ctx: &egui::Context, prefer_hans: bool) {
    install_fonts_with(ctx, prefer_hans, false);
}

/// [`install_fonts_for`] with the CJK fallback order for a Korean interface.
pub fn install_fonts_language(ctx: &egui::Context, prefer_hans: bool, prefer_korean: bool) {
    install_fonts_language_with(ctx, prefer_hans, prefer_korean, false);
}

/// [`install_fonts_for`], also adding an installed CJK font after the embedded faces when
/// `system_cjk` is set and the embedded faces leave CJK characters of the interface text without
/// a glyph (#241, #486). Reading that font is a large file read, so callers ask for it only when
/// CJK text is on screen: a CJK interface language, or the language names in Options.
pub fn install_fonts_with(ctx: &egui::Context, prefer_hans: bool, system_cjk: bool) {
    install_fonts_extra(ctx, prefer_hans, system_cjk, &[]);
}

/// [`install_fonts_with`] with the CJK font order selected for the interface language.
pub fn install_fonts_language_with(ctx: &egui::Context, prefer_hans: bool, prefer_korean: bool, system_cjk: bool) {
    install_fonts_language_extra(ctx, prefer_hans, prefer_korean, system_cjk, &[]);
}

/// [`install_fonts_with`], also adding document-font `faces` as last-resort fallbacks (#565): the
/// faces that cover text the interface fonts cannot draw (an input method's composition, or a
/// file name in the window caption).
pub fn install_fonts_extra(ctx: &egui::Context, prefer_hans: bool, system_cjk: bool, faces: &[Arc<wordcraft_fonts::FontFace>]) {
    install_fonts_language_extra(ctx, prefer_hans, false, system_cjk, faces);
}

/// [`install_fonts_extra`] with the CJK font order selected for the interface language.
pub fn install_fonts_language_extra(
    ctx: &egui::Context,
    prefer_hans: bool,
    prefer_korean: bool,
    system_cjk: bool,
    faces: &[Arc<wordcraft_fonts::FontFace>],
) {
    let mut fonts = font_definitions_language(prefer_hans, prefer_korean, system_cjk);
    add_fallback_faces(&mut fonts, faces);
    ctx.set_fonts(fonts);
}

/// Add `faces` after every other font of the interface families.
pub fn add_fallback_faces(fonts: &mut FontDefinitions, faces: &[Arc<wordcraft_fonts::FontFace>]) {
    for f in faces {
        let name = format!("fallback {} {} {}", f.family, f.style, f.id());
        let mut data = FontData::from_owned(f.data().to_vec());
        data.index = f.index();
        fonts.font_data.insert(name.clone(), Arc::new(data));
        for fam in [FontFamily::Proportional, FontFamily::Name("medium".into()), FontFamily::Name("semibold".into())] {
            if let Some(v) = fonts.families.get_mut(&fam) {
                v.push(name.clone());
            }
        }
    }
}

/// A character the window caption can already draw without a document fallback: the Latin range
/// the interface faces cover, or CJK, which uses the CJK UI font ([`install_fonts_with`]) rather
/// than a document face. System CJK collections are larger than [`faces_for_preedit`] will load.
pub fn caption_char_covered(c: char) -> bool {
    (c as u32) < 0x250 || wordcraft_fonts::is_cjk(c)
}

/// Document faces needed to draw `text` (the input method's composition, or a file name in the
/// window caption), one for each character the interface fonts cannot draw, skipping faces in
/// `have` (#565). Characters of the embedded interface fonts are not looked up; very large faces
/// (system CJK collections) are left to the CJK fallback.
pub fn faces_for_preedit(
    text: &str,
    have: &[Arc<wordcraft_fonts::FontFace>],
    covered: impl Fn(char) -> bool,
    lookup: impl Fn(char) -> Option<Arc<wordcraft_fonts::FontFace>>,
) -> Vec<Arc<wordcraft_fonts::FontFace>> {
    let mut out: Vec<Arc<wordcraft_fonts::FontFace>> = Vec::new();
    for c in text.chars().filter(|c| !c.is_ascii() && !c.is_whitespace() && !covered(*c)) {
        if have.iter().chain(&out).any(|f| f.covers(c)) {
            continue;
        }
        if let Some(f) = lookup(c).filter(|f| f.data().len() <= 16 << 20 && !have.iter().chain(&out).any(|h| h.id() == f.id())) {
            out.push(f);
        }
    }
    out
}

/// The interface fonts; see [`install_fonts_with`].
pub fn font_definitions(prefer_hans: bool, system_cjk: bool) -> FontDefinitions {
    font_definitions_language(prefer_hans, false, system_cjk)
}

/// The interface fonts, ordered for the preferred CJK writing system.
pub fn font_definitions_language(prefer_hans: bool, prefer_korean: bool, system_cjk: bool) -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let add = |fonts: &mut FontDefinitions, name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
    };
    add(&mut fonts, "Inter", include_bytes!("../../../assets/fonts/Inter-Regular.ttf"));
    add(&mut fonts, "InterMedium", include_bytes!("../../../assets/fonts/Inter-Medium.ttf"));
    add(&mut fonts, "InterSemiBold", include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf"));
    add(&mut fonts, "SourceSans", include_bytes!("../../../assets/fonts/SourceSans3-Regular.ttf"));
    add(&mut fonts, "Mono", include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf"));
    if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
        prop.insert(0, "Inter".into());
        prop.push("SourceSans".into());
    }
    if let Some(m) = fonts.families.get_mut(&FontFamily::Monospace) {
        m.insert(0, "Mono".into());
    }
    fonts.families.insert(FontFamily::Name("medium".into()), vec!["InterMedium".into(), "Inter".into(), "SourceSans".into()]);
    fonts.families.insert(FontFamily::Name("semibold".into()), vec!["InterSemiBold".into(), "Inter".into(), "SourceSans".into()]);
    let add_cjk = |fonts: &mut FontDefinitions, name: String, data: FontData| {
        fonts.font_data.insert(name.clone(), Arc::new(data));
        for fam in [FontFamily::Proportional, FontFamily::Name("medium".into()), FontFamily::Name("semibold".into())] {
            if let Some(v) = fonts.families.get_mut(&fam) {
                v.push(name.clone());
            }
        }
    };
    let cjk = wordcraft_fonts::ui_cjk_fonts_for(prefer_hans, prefer_korean);
    for f in &cjk {
        // The same static bytes the document fonts use: one copy in the binary.
        add_cjk(&mut fonts, format!("{} {}", f.family, f.style), FontData::from_static(f.bytes));
    }
    // The embedded faces leave interface characters without a glyph (a build without craft-fonts,
    // without its Chinese face, #241, or with faces lacking catalog characters, #486): an
    // installed CJK font after them, so the menus don't show boxes or `?`.
    #[cfg(not(target_arch = "wasm32"))]
    if system_cjk && let Some(f) = system_cjk_font(prefer_hans, prefer_korean) {
        let mut data = FontData::from_static(&f.bytes);
        data.index = f.index;
        add_cjk(&mut fonts, format!("system {}", f.family), data);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = system_cjk;
    // Symbols and emoji fall back to egui's defaults (kept in the families).
    fonts
}

/// The installed CJK interface font for a Chinese (`hans`) or other interface, chosen once:
/// `None` when the embedded faces already cover the interface text
/// ([`cjk_font_text`](crate::i18n::Lang::cjk_font_text)). The font is picked for the catalogs'
/// missing characters, or for the language names when only those are missing.
#[cfg(not(target_arch = "wasm32"))]
fn system_cjk_font(hans: bool, korean: bool) -> Option<&'static wordcraft_fonts::SystemUiFont> {
    static ZH: std::sync::OnceLock<Option<wordcraft_fonts::SystemUiFont>> = std::sync::OnceLock::new();
    static OTHER: std::sync::OnceLock<Option<wordcraft_fonts::SystemUiFont>> = std::sync::OnceLock::new();
    static KO: std::sync::OnceLock<Option<wordcraft_fonts::SystemUiFont>> = std::sync::OnceLock::new();
    (if hans {
        &ZH
    } else if korean {
        &KO
    } else {
        &OTHER
    })
    .get_or_init(|| {
        let embedded = wordcraft_fonts::ui_cjk_fonts_for(hans, korean);
        let (catalogs, names) = crate::i18n::Lang::cjk_font_text_for(hans, korean);
        let mut needed = wordcraft_fonts::ui_uncovered_cjk(&catalogs, &embedded);
        if needed.is_empty() {
            needed = wordcraft_fonts::ui_uncovered_cjk(&[&names], &embedded);
            if needed.is_empty() {
                return None;
            }
        }
        log::debug!("interface: {} CJK characters lack an embedded glyph", needed.len());
        wordcraft_fonts::system_cjk_ui_font_for(hans, korean, &needed)
    })
    .as_ref()
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}
pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}

/// A UI type rung: size, weight pairing, and size-specific letter-spacing (large text is
/// tightened, small text opened). Use these instead of raw `regular(size)`/`semibold(size)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TypeRung {
    /// Tab strip, title bar, centred window title.
    Chrome,
    /// Ribbon buttons, menus, dialogs.
    Control,
    /// Group labels, status bar, tooltips.
    Caption,
}

impl TypeRung {
    /// Font size in points.
    pub fn size(self) -> f32 {
        match self {
            TypeRung::Chrome => 12.5,
            TypeRung::Control => 11.5,
            TypeRung::Caption => 11.0,
        }
    }
    /// Extra letter-spacing in points (em-relative, so it scales with the size).
    pub fn tracking(self) -> f32 {
        match self {
            TypeRung::Chrome => -0.012 * self.size(),
            TypeRung::Control => 0.0,
            TypeRung::Caption => 0.018 * self.size(),
        }
    }
    /// Line height in points; tighter leading as the text grows.
    pub fn line_height(self) -> f32 {
        match self {
            TypeRung::Chrome => 1.25 * self.size(),
            TypeRung::Control => 1.42 * self.size(),
            TypeRung::Caption => 1.5 * self.size(),
        }
    }

    pub fn regular(self) -> FontId {
        regular(self.size())
    }
    pub fn medium(self) -> FontId {
        medium(self.size())
    }
    pub fn semibold(self) -> FontId {
        semibold(self.size())
    }
}

/// Width in points of one line of `tracking`-spaced text. `layout_no_wrap` ignores
/// letter-spacing, so non-zero tracking goes through an explicit `LayoutJob`.
pub fn text_width(painter: &egui::Painter, text: &str, font: &FontId, tracking: f32, color: Color32) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    if tracking.abs() <= 1e-6 {
        return painter.layout_no_wrap(text.to_string(), font.clone(), color).size().x;
    }
    painter.layout_job(tracked_job(text, font, tracking, color)).size().x
}

/// Paint one line of tracked text with the given anchor and return its rect.
pub fn paint_text(painter: &egui::Painter, pos: egui::Pos2, anchor: Align2, text: &str, font: &FontId, tracking: f32, color: Color32) -> Rect {
    if tracking.abs() <= 1e-6 {
        return painter.text(pos, anchor, text.to_string(), font.clone(), color);
    }
    let galley = painter.layout_job(tracked_job(text, font, tracking, color));
    let rect = anchor.anchor_size(pos, galley.size());
    painter.galley(rect.min, galley, color);
    rect
}

/// A single-section, no-wrap `LayoutJob` carrying `tracking` (egui 0.36 has no `default_format`).
fn tracked_job(text: &str, font: &FontId, tracking: f32, color: Color32) -> egui::text::LayoutJob {
    let format = egui::text::TextFormat { font_id: font.clone(), extra_letter_spacing: tracking, line_height: None, color, ..Default::default() };
    egui::text::LayoutJob::single_section(text.to_string(), format)
}

/// Apply the interface theme setting to egui. Both palettes go in, our light one as egui's light
/// style and our dark one as its dark style, and egui picks between them each frame: `System`
/// leaves egui's theme preference on `System`, so it follows the OS appearance it reports (light
/// when the OS reports none, like [`Appearance::is_dark`]) and the native window keeps its own
/// appearance; a manual Light or Dark pins egui (and the window decorations) to that theme.
/// Setting a concrete theme for `System` would pin the native window too, and macOS then stops
/// reporting appearance changes (#311).
pub fn apply(ctx: &egui::Context, appearance: Appearance) {
    // A fixed theme's palette is egui's style for its side (light or dark); the other side keeps
    // the plain palette.
    let palette = appearance.palette();
    let light = palette.filter(|t| !t.dark).unwrap_or_else(Tokens::light);
    let dark = palette.filter(|t| t.dark).unwrap_or_else(Tokens::dark);
    ctx.set_visuals_of(egui::Theme::Light, visuals(&light));
    ctx.set_visuals_of(egui::Theme::Dark, visuals(&dark));
    ctx.data_mut(|d| match palette {
        Some(t) => {
            d.insert_temp(palette_id(), t);
        }
        None => {
            d.remove::<Tokens>(palette_id());
        }
    });
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(8.0, 3.0);
        s.spacing.interact_size = egui::vec2(24.0, 22.0);
        s.spacing.menu_margin = egui::Margin::same(6);
        s.text_styles.insert(egui::TextStyle::Body, regular(12.5));
        s.text_styles.insert(egui::TextStyle::Button, regular(12.5));
        s.text_styles.insert(egui::TextStyle::Small, regular(10.5));
        s.text_styles.insert(egui::TextStyle::Heading, semibold(17.0));
        s.interaction.tooltip_delay = 0.45;
    });
    ctx.options_mut(|o| {
        o.fallback_theme = egui::Theme::Light;
        o.theme_preference = appearance.preference();
    });
}

/// Turn egui's own animations on or off: hover and press fades, menus, tooltips and windows fading
/// in, sections sliding open, and the glide when a scroll area brings the caret or a list item
/// into view. Off, each of them happens at once.
pub fn apply_animations(ctx: &egui::Context, on: bool) {
    let defaults = egui::Style::default();
    ctx.all_styles_mut(|s| {
        s.animation_time = if on { defaults.animation_time } else { 0.0 };
        s.scroll_animation = if on { defaults.scroll_animation } else { egui::style::ScrollAnimation::none() };
    });
}

/// egui's visuals for one of our palettes.
fn visuals(t: &Tokens) -> Visuals {
    let mut v = if t.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.ribbon;
    v.window_fill = t.menu;
    v.extreme_bg_color = t.input;
    v.faint_bg_color = t.panel;
    v.window_stroke = Stroke::new(1.0, t.border_strong);
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(6);
    v.selection.bg_fill = t.selection;
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v.hyperlink_color = t.accent_text;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.inactive.bg_fill = t.input;
    v.widgets.inactive.weak_bg_fill = t.input;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, t.input_border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.inactive.corner_radius = CornerRadius::same(4);
    v.widgets.hovered.bg_fill = t.hover;
    v.widgets.hovered.weak_bg_fill = t.hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, t.border_strong);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.hovered.corner_radius = CornerRadius::same(4);
    v.widgets.active.bg_fill = t.pressed;
    v.widgets.active.weak_bg_fill = t.pressed;
    v.widgets.active.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.active.corner_radius = CornerRadius::same(4);
    v.widgets.open.bg_fill = t.pressed;
    v.widgets.open.weak_bg_fill = t.pressed;
    v.popup_shadow = egui::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(if t.dark { 120 } else { 40 }) };
    v.window_shadow = egui::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(if t.dark { 140 } else { 50 }) };
    v
}

pub fn c32(c: wordcraft_doc::Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    /// WCAG contrast ratio between two opaque colours.
    pub(super) fn contrast(a: egui::Color32, b: egui::Color32) -> f32 {
        let lum = |c: egui::Color32| {
            let ch = |v: u8| {
                let v = f32::from(v) / 255.0;
                if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
            };
            0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
        };
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Lexicon: the engine's `view.interfaceTheme` accepts exactly the themes the interface has,
    /// and the default is Dark Blue.
    #[test]
    fn every_theme_is_known_to_the_engine() {
        let codes: Vec<&str> = super::Appearance::ALL.iter().map(|a| a.code()).collect();
        assert_eq!(codes, wordcraft_engine::cmd::view::INTERFACE_THEMES);
        assert_eq!(super::Appearance::default(), super::Appearance::Navy);
        assert_eq!(super::Appearance::from(String::from("midnight")), super::Appearance::Navy);
        for a in super::Appearance::ALL {
            assert_eq!(super::Appearance::from_code(a.code()), Some(a));
        }
    }

    /// #1885: the ✕ on a window's close box reads on its hover and pressed reds in both themes.
    #[test]
    fn close_box_glyph_reads_on_its_reds() {
        for t in super::Tokens::all_palettes() {
            for (name, bg) in [("close_hover", t.close_hover), ("close_pressed", t.close_pressed)] {
                let r = contrast(t.on_close, bg);
                assert!(r >= 4.5, "dark={}: on_close on {name} is {r:.2}:1", t.dark);
            }
        }
    }

    /// #655: text and knobs on filled accent controls (Share, avatar, AutoSave, keytips) reach 4.5:1
    /// on the accent and on its hover shade, in both themes.
    #[test]
    fn on_accent_reads_on_the_accent() {
        for t in super::Tokens::all_palettes() {
            for (name, bg) in [("accent", t.accent), ("hover", t.accent_text)] {
                let r = contrast(t.on_accent, bg);
                assert!(r >= 4.5, "dark={}: on_accent on {name} is {r:.2}:1", t.dark);
            }
        }
    }

    /// #657: text field and combo outlines reach 3:1 against the field and everything it sits on.
    #[test]
    fn input_borders_reach_three_to_one() {
        for t in super::Tokens::all_palettes() {
            for (name, bg) in [("input", t.input), ("ribbon", t.ribbon), ("panel", t.panel), ("menu", t.menu), ("title bar", t.title_bar)] {
                let r = contrast(t.input_border, bg);
                assert!(r >= 3.0, "dark={}: input_border on {name} is {r:.2}:1", t.dark);
            }
        }
    }

    /// #1148: on both rulers the text area is lighter than the margins in both themes, as in the
    /// light theme, and the numbers drawn on it still reach 4.5:1.
    #[test]
    fn ruler_text_area_is_lighter_than_its_margins() {
        let lum = |c: egui::Color32| contrast(c, egui::Color32::BLACK);
        for t in super::Tokens::all_palettes() {
            assert!(lum(t.ruler) > lum(t.ruler_margin), "dark={}: ruler text area is darker than its margins", t.dark);
            let r = contrast(t.ruler_tick, t.ruler);
            assert!(r >= 4.5, "dark={}: ruler numbers on the text area are {r:.2}:1", t.dark);
        }
    }

    /// #2045: comment and revision balloons were drawn on a hard-coded white card, so the light
    /// text of the dark theme was near-invisible. Every text colour a balloon uses reaches 4.5:1
    /// on the card, and the author stripes and borders 3:1 on the card and the markup area.
    #[test]
    fn comment_balloons_are_readable() {
        for t in super::Tokens::all_palettes() {
            for (name, card) in [("card", t.balloon), ("selected card", t.checked)] {
                for (what, fg) in [("text", t.text), ("dim text", t.text_dim), ("resolved label", t.resolved_text)] {
                    let r = contrast(fg, card);
                    assert!(r >= 4.5, "dark={}: {what} on the {name} is {r:.2}:1", t.dark);
                }
            }
            // The light stripes are unchanged; only the dark theme's were unreadable.
            for (i, c) in t.author_colors.iter().enumerate().filter(|_| t.dark) {
                for (name, bg) in [("card", t.balloon), ("selected card", t.checked), ("markup area", t.markup_area)] {
                    let r = contrast(*c, bg);
                    assert!(r >= 3.0, "dark={}: author colour {i} on the {name} is {r:.2}:1", t.dark);
                }
            }
            // The leader's on-page part is drawn at 70% on white paper in both themes, no fainter than before (#2045).
            for (i, c) in t.leader_colors.iter().enumerate() {
                let on_paper = |a: u8| u8::try_from((u16::from(a) * 7 + 255 * 3) / 10).unwrap_or(255);
                let blended = egui::Color32::from_rgb(on_paper(c.r()), on_paper(c.g()), on_paper(c.b()));
                let r = contrast(blended, egui::Color32::WHITE);
                assert!(r >= 2.1, "dark={}: leader colour {i} on the page is {r:.2}:1", t.dark);
            }
            let r = contrast(t.markup_edge, t.markup_area);
            assert!(r >= 1.1, "dark={}: the markup area edge is invisible ({r:.2}:1)", t.dark);
        }
    }

    /// #565: the composition of an input method (Bangla through ibus) drew as boxes because only the
    /// interface fonts were used. Faces found for the uncovered characters are added as fallbacks,
    /// once, and not for characters the interface fonts draw.
    #[test]
    fn preedit_gets_document_fallback_faces() {
        let face = wordcraft_fonts::FontDb::global().face("Source Sans 3", "Regular");
        let lookup = |_: char| Some(face.clone());
        let bangla = "\u{0986}\u{09AE}\u{09BF}";
        let got = super::faces_for_preedit(bangla, &[], |_| false, lookup);
        assert_eq!(got.len(), 1, "one face, not one per character");
        assert!(super::faces_for_preedit(bangla, &got, |_| false, lookup).is_empty(), "already added");
        assert!(super::faces_for_preedit(bangla, &[], |_| true, lookup).is_empty(), "the interface fonts draw it");
        assert!(super::faces_for_preedit("abc", &[], |_| false, lookup).is_empty(), "ASCII never needs one");
        let mut defs = super::font_definitions(false, false);
        let before = defs.font_data.len();
        super::add_fallback_faces(&mut defs, &got);
        assert_eq!(defs.font_data.len(), before + 1);
        assert!(defs.families[&egui::FontFamily::Proportional].last().is_some_and(|n| n.starts_with("fallback ")));
    }

    /// A file name in a script Inter lacks (Hebrew, Arabic, …) is drawn in the window caption,
    /// which uses the semibold face. egui paints a missing glyph as `?`, so the caption borrows
    /// one document fallback face. CJK stays with the CJK UI font: those files are too large to
    /// add here.
    #[test]
    fn caption_borrows_one_fallback_face_for_hebrew() {
        let face = wordcraft_fonts::FontDb::global().face("Source Sans 3", "Regular");
        let lookup = |c: char| {
            assert!(!super::caption_char_covered(c), "{c} is already drawn by the interface fonts");
            Some(face.clone())
        };
        let hebrew = "מסמך";
        let got = super::faces_for_preedit(hebrew, &[], super::caption_char_covered, lookup);
        assert_eq!(got.len(), 1, "one face for the whole file name");
        assert!(super::faces_for_preedit(hebrew, &got, super::caption_char_covered, lookup).is_empty(), "already added");
        assert!(super::faces_for_preedit("文档", &[], super::caption_char_covered, lookup).is_empty(), "CJK uses the CJK UI font");
        assert!(super::faces_for_preedit("report", &[], super::caption_char_covered, lookup).is_empty(), "Latin needs nothing");
        let mut defs = super::font_definitions(false, false);
        super::add_fallback_faces(&mut defs, &got);
        let semi = &defs.families[&egui::FontFamily::Name("semibold".into())];
        assert!(semi.iter().any(|n| n.starts_with("fallback ")), "the caption is painted with semibold");
    }

    /// #656: dialog primary buttons, the Discord pill, the table eraser and the Navigation search
    /// highlight take their colours from the theme, not fixed light-theme values.
    #[test]
    fn interface_colours_come_from_the_theme() {
        let banned = ["fill(crate::theme::APP_COLOR)", "0x58, 0x65, 0xF2", "0x47, 0x52, 0xC4", "0xFF, 0xF1, 0x76", "0xD1, 0x3B, 0x3B"];
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if !name.ends_with(".rs") || name == "theme.rs" || name == "icons.rs" {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            for b in banned {
                assert!(!src.contains(b), "{name} hard-codes {b}; use theme::Tokens");
            }
        }
    }

    /// Symbols written as text in the interface code must exist in the interface fonts (Inter and
    /// egui's emoji fallbacks); a missing glyph draws as an empty box, as the panes' "✕" close
    /// button once did. Letters are the catalogs' business (see the i18n font tests).
    #[test]
    fn interface_symbols_have_glyphs() {
        let ctx = egui::Context::default();
        ctx.set_fonts(super::font_definitions(false, false));
        ctx.run_ui(egui::RawInput::default(), |_| {}).drop_without_applying_deltas();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut symbols = BTreeSet::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let code = std::fs::read_to_string(&path).unwrap();
            // Characters inside string literals, line by line (good enough for this code base).
            for line in code.lines().filter(|l| !l.trim_start().starts_with("//")) {
                for (i, part) in line.split('"').enumerate() {
                    if i % 2 == 1 {
                        symbols.extend(
                            part.chars()
                                .filter(|c| !c.is_ascii() && !c.is_alphabetic() && !c.is_whitespace())
                                .map(|c| (c, path.file_name().unwrap().to_string_lossy().to_string())),
                        );
                    }
                }
            }
        }
        // Equation gallery templates are UnicodeMath input (`lim┬(n→∞)`, `≜`) that the math
        // renderer draws with document fonts, not interface text.
        // The ➢ bullet is a value for `para.bullets`; its menu tile draws the shape instead.
        let exempt = |c: char, file: &str| match file {
            "equation_tab.rs" => matches!(c, '≜' | '┬'),
            "ribbon.rs" => c == '➢',
            _ => false,
        };
        symbols.retain(|(c, file)| !exempt(*c, file));
        let font = super::regular(12.5);
        let missing: Vec<String> = ctx.fonts_mut(|f| {
            symbols.iter().filter(|(c, _)| !f.has_glyph(&font, *c)).map(|(c, file)| format!("{c} U+{:04X} in {file}", *c as u32)).collect()
        });
        assert!(missing.is_empty(), "symbols without a glyph: {missing:#?}");
    }
}

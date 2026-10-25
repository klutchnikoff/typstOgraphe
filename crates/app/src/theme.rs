//! Jetons visuels de l’application : une seule source pour les couleurs et les
//! échelles de l’interface, avec le contrôle de leurs contrastes (D40, L.1).
//!
//! Les valeurs de `ACCENT`, `PREVIEW_SELECTION`, `ERROR_INLINE` et de la sélection
//! reprennent celles qui étaient dispersées dans l’application. `Tokens` décrit la
//! palette clair/sombre validée ; les surfaces la reprendront pendant leur migration.
use eframe::egui::{Color32, Stroke};

pub const ACCENT: Color32 = Color32::from_rgb(40, 92, 119);
pub const PREVIEW_SELECTION: Color32 = Color32::from_rgba_premultiplied(16, 33, 59, 65);
/// Rouge des erreurs affichées sous un champ ou dans l’aperçu.
pub const ERROR_INLINE: Color32 = Color32::from_rgb(183, 69, 57);

pub fn selection_fill(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(40, 80, 108)
    } else {
        Color32::from_rgb(174, 211, 235)
    }
}
pub fn selection_stroke(dark: bool) -> Stroke {
    Stroke::new(1.0_f32, if dark { Color32::LIGHT_BLUE } else { ACCENT })
}

/// Voile derrière les dialogues.
pub fn scrim(dark: bool) -> Color32 {
    Color32::from_black_alpha(if dark { 140 } else { 97 })
}
/// Mélange linéaire de deux couleurs opaques (`amount` de 0 à 1 vers `to`).
pub fn mix(from: Color32, to: Color32, amount: f32) -> Color32 {
    let channel =
        |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * amount).round() as u8;
    Color32::from_rgb(
        channel(from.r(), to.r()),
        channel(from.g(), to.g()),
        channel(from.b(), to.b()),
    )
}

/// Échelles communes (points egui).
pub mod scale {
    pub const SPACE: [f32; 5] = [4.0, 8.0, 12.0, 16.0, 24.0];
    pub const RADIUS_CONTROL: u8 = 6;
    pub const RADIUS_CARD: u8 = 8;
    pub const RADIUS_DIALOG: u8 = 12;
    pub const TEXT: [f32; 5] = [11.0, 12.5, 13.5, 15.0, 20.0];
}

const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// Palette d’un thème. `border` est décorative ; `border_strong` délimite les
/// contrôles (champs, boutons) et vise un contraste d’au moins 3:1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tokens {
    pub bg: Color32,
    pub panel: Color32,
    pub raised: Color32,
    pub field: Color32,
    pub hover: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub accent_soft: Color32,
    pub danger: Color32,
    pub danger_soft: Color32,
    pub ok: Color32,
    pub ok_soft: Color32,
}
impl Tokens {
    pub const LIGHT: Self = Self {
        bg: hex(0xf4f4f1),
        panel: hex(0xffffff),
        raised: hex(0xffffff),
        field: hex(0xffffff),
        hover: hex(0xecece8),
        text: hex(0x1f2328),
        muted: hex(0x5f656c),
        border: hex(0xd9d9d2),
        border_strong: hex(0x80858c),
        accent: ACCENT,
        on_accent: hex(0xffffff),
        accent_soft: hex(0xe8f0f4),
        danger: hex(0xa53a32),
        danger_soft: hex(0xfbe7e4),
        ok: hex(0x2f6b4a),
        ok_soft: hex(0xe3f0e8),
    };
    pub const DARK: Self = Self {
        bg: hex(0x1b1c1e),
        panel: hex(0x232427),
        raised: hex(0x2a2b2f),
        field: hex(0x2b2d31),
        hover: hex(0x303236),
        text: hex(0xe6e7e9),
        muted: hex(0x9aa0a8),
        border: hex(0x35373c),
        border_strong: hex(0x7d838b),
        accent: hex(0x8cc3e0),
        on_accent: hex(0x10222c),
        accent_soft: hex(0x26343d),
        danger: hex(0xe58b82),
        danger_soft: hex(0x4a2f2d),
        ok: hex(0x7fc59d),
        ok_soft: hex(0x24382d),
    };
    pub fn new(dark: bool) -> Self {
        if dark { Self::DARK } else { Self::LIGHT }
    }
}

fn luminance(color: Color32) -> f32 {
    let linear = |channel: u8| {
        let value = f32::from(channel) / 255.0;
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
}
/// Rapport de contraste WCAG 2.x entre deux couleurs opaques.
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn contrast_follows_wcag_reference_values() {
        assert!((contrast_ratio(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
        assert!((contrast_ratio(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 0.001);
    }
    #[test]
    fn both_palettes_keep_text_at_aa_and_controls_at_three_to_one() {
        for (name, t) in [("clair", Tokens::LIGHT), ("sombre", Tokens::DARK)] {
            let text = [
                (t.text, t.bg),
                (t.text, t.panel),
                (t.text, t.field),
                (t.text, t.hover),
                (t.muted, t.hover),
                (t.muted, t.bg),
                (t.muted, t.panel),
                (t.muted, t.field),
                (t.accent, t.panel),
                (t.accent, t.accent_soft),
                (t.danger, t.panel),
                (t.danger, t.danger_soft),
                (t.ok, t.panel),
                (t.ok, t.ok_soft),
                (t.on_accent, t.accent),
            ];
            for (fg, bg) in text {
                assert!(
                    contrast_ratio(fg, bg) >= 4.5,
                    "{name}: texte {fg:?} sur {bg:?}"
                );
            }
            for ground in [t.bg, t.panel, t.field] {
                assert!(
                    contrast_ratio(t.border_strong, ground) >= 3.0,
                    "{name}: bordure forte sur {ground:?}"
                );
            }
        }
    }
    #[test]
    fn shared_constants_keep_their_original_values() {
        assert_eq!(Tokens::LIGHT.accent, Color32::from_rgb(40, 92, 119));
        assert_eq!(ERROR_INLINE, Color32::from_rgb(183, 69, 57));
        assert_eq!(selection_fill(false), Color32::from_rgb(174, 211, 235));
        assert_eq!(selection_fill(true), Color32::from_rgb(40, 80, 108));
        assert_eq!(selection_stroke(false).color, ACCENT);
        assert_eq!(selection_stroke(true).color, Color32::LIGHT_BLUE);
    }
}

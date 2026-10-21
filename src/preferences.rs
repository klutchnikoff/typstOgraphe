//! Préférences locales de l’application, indépendantes du courrier et du modèle.
use eframe::egui;
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "typstographe.preferences";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorFont {
    #[default]
    Libertinus,
    ComputerModern,
    Monospace,
}
impl EditorFont {
    pub const ALL: [Self; 3] = [Self::Libertinus, Self::ComputerModern, Self::Monospace];
    pub fn label(self) -> &'static str {
        match self {
            Self::Libertinus => "Libertinus Serif",
            Self::ComputerModern => "New Computer Modern",
            Self::Monospace => "DejaVu Sans Mono",
        }
    }
    pub fn font_id(self, size: f32, variant: usize) -> egui::FontId {
        egui::FontId::new(
            size,
            egui::FontFamily::Name(format!("editor-{self:?}-{variant}").into()),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EditorStyle {
    pub font: EditorFont,
    pub size: f32,
}
impl Default for EditorStyle {
    fn default() -> Self {
        Self {
            font: EditorFont::default(),
            size: 18.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}
impl Appearance {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];
    pub fn label(self) -> String {
        match self {
            Self::System => crate::tr!("theme-system"),
            Self::Light => crate::tr!("theme-light"),
            Self::Dark => crate::tr!("theme-dark"),
        }
    }
    pub fn theme(self) -> egui::ThemePreference {
        match self {
            Self::System => egui::ThemePreference::System,
            Self::Light => egui::ThemePreference::Light,
            Self::Dark => egui::ThemePreference::Dark,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preferences {
    pub editor: EditorStyle,
    pub appearance: Appearance,
    pub editor_fraction: f32,
    #[serde(default)]
    pub spelling: crate::spelling::Preferences,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            editor: EditorStyle::default(),
            appearance: Appearance::System,
            editor_fraction: 0.5,
            spelling: Default::default(),
        }
    }
}
impl Preferences {
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        let mut preferences: Self = storage
            .and_then(|s| s.get_string(STORAGE_KEY))
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        preferences.editor.size = preferences.editor.size.clamp(10.0, 36.0);
        preferences.editor_fraction = preferences.editor_fraction.clamp(0.0, 1.0);
        preferences
    }
    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        storage.set_string(
            STORAGE_KEY,
            serde_json::to_string(self).expect("préférences sérialisables"),
        );
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use eframe::Storage;
    #[derive(Default)]
    struct MemoryStorage(std::collections::HashMap<String, String>);
    impl eframe::Storage for MemoryStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.into(), value);
        }
        fn flush(&mut self) {}
    }
    #[test]
    fn appearance_and_column_preferences_survive_a_new_session() {
        let mut storage = MemoryStorage::default();
        let preferences = Preferences {
            editor: EditorStyle {
                font: EditorFont::Monospace,
                size: 24.0,
            },
            appearance: Appearance::Dark,
            editor_fraction: 0.31,
            spelling: crate::spelling::Preferences {
                enabled: false,
                personal: std::collections::BTreeMap::from([(
                    "fr".into(),
                    std::collections::BTreeSet::from(["typstographe".into()]),
                )]),
            },
        };
        preferences.save(&mut storage);
        assert_eq!(Preferences::load(Some(&storage)), preferences);
        storage.set_string(STORAGE_KEY, "fichier invalide".into());
        assert_eq!(Preferences::load(Some(&storage)), Preferences::default());
    }
}

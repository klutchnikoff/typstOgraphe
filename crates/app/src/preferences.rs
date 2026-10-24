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
    #[serde(default)]
    pub profile: crate::profile::Profile,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            editor: EditorStyle::default(),
            appearance: Appearance::System,
            editor_fraction: 0.5,
            spelling: Default::default(),
            profile: Default::default(),
        }
    }
}

/// Sauvegarde après modification, une fois la saisie stabilisée ; jamais périodique.
pub struct SaveTracker {
    observed: Preferences,
    saved: Preferences,
    changed_at: Option<std::time::Instant>,
}
impl SaveTracker {
    pub const DELAY: std::time::Duration = std::time::Duration::from_millis(500);
    pub fn new(preferences: &Preferences) -> Self {
        Self {
            observed: preferences.clone(),
            saved: preferences.clone(),
            changed_at: None,
        }
    }
    pub fn observe(
        &mut self,
        preferences: &Preferences,
        now: std::time::Instant,
    ) -> Option<std::time::Duration> {
        if &self.observed != preferences {
            self.observed = preferences.clone();
            self.changed_at = Some(now);
        }
        if &self.saved == preferences {
            self.changed_at = None;
            return None;
        }
        Some(
            Self::DELAY
                .saturating_sub(now.saturating_duration_since(self.changed_at.unwrap_or(now))),
        )
    }
    pub fn save(&mut self, preferences: &Preferences, storage: &mut dyn eframe::Storage) {
        preferences.save(storage);
        storage.flush();
        self.saved = preferences.clone();
        self.observed = preferences.clone();
        self.changed_at = None;
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
    struct MemoryStorage(std::collections::HashMap<String, String>, usize, usize);
    impl eframe::Storage for MemoryStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.into(), value);
            self.1 += 1;
        }
        fn flush(&mut self) {
            self.2 += 1;
        }
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
            profile: Default::default(),
        };
        preferences.save(&mut storage);
        assert_eq!(Preferences::load(Some(&storage)), preferences);
        storage.set_string(STORAGE_KEY, "fichier invalide".into());
        assert_eq!(Preferences::load(Some(&storage)), Preferences::default());
    }

    #[test]
    fn settings_save_only_after_changes_debounce_typing_and_persist_on_close() {
        use std::time::{Duration, Instant};
        let mut prefs = Preferences::default();
        let mut tracker = SaveTracker::new(&prefs);
        let mut storage = MemoryStorage::default();
        let start = Instant::now();
        assert!(
            tracker
                .observe(&prefs, start + Duration::from_secs(500))
                .is_none()
        );
        assert_eq!((storage.1, storage.2), (0, 0));
        prefs.profile.name = "Zoé".into();
        assert_eq!(tracker.observe(&prefs, start), Some(SaveTracker::DELAY));
        prefs.profile.name = "Zoé Martin".into();
        assert_eq!(
            tracker.observe(&prefs, start + Duration::from_millis(300)),
            Some(SaveTracker::DELAY)
        );
        assert_eq!(
            tracker.observe(&prefs, start + Duration::from_millis(799)),
            Some(Duration::from_millis(1))
        );
        assert_eq!(
            tracker.observe(&prefs, start + Duration::from_millis(800)),
            Some(Duration::ZERO)
        );
        tracker.save(&prefs, &mut storage);
        assert_eq!(Preferences::load(Some(&storage)), prefs);
        assert_eq!((storage.1, storage.2), (1, 1));
        assert!(
            tracker
                .observe(&prefs, start + Duration::from_secs(900))
                .is_none()
        );
        prefs.profile.email = "zoe@example.org".into();
        assert!(tracker.observe(&prefs, start).is_some());
        tracker.save(&prefs, &mut storage); // Fermeture avant la fin du délai.
        assert_eq!(
            Preferences::load(Some(&storage)).profile.email,
            "zoe@example.org"
        );
        prefs.profile.email.clear();
        assert!(tracker.observe(&prefs, start).is_some());
        prefs.profile.email = "zoe@example.org".into();
        assert!(tracker.observe(&prefs, start).is_none());
        assert_eq!((storage.1, storage.2), (2, 2));
    }
}

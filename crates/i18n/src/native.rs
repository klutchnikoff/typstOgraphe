//! Labels for native actions whose toolkit defaults are English, not OS translations.
use i18n_embed::fluent::FluentLanguageLoader;

pub struct NativeMenuLabels {
    pub about: String,
    pub hide: String,
    pub hide_others: String,
    pub cut: String,
    pub copy: String,
    pub paste: String,
    pub select_all: String,
}
pub fn native_menu_labels() -> NativeMenuLabels {
    labels_with(crate::loader())
}
fn labels_with(loader: &FluentLanguageLoader) -> NativeMenuLabels {
    NativeMenuLabels {
        about: loader.get("native-menu-about"),
        hide: loader.get("native-menu-hide"),
        hide_others: loader.get("native-menu-hide-others"),
        cut: loader.get("native-menu-cut"),
        copy: loader.get("native-menu-copy"),
        paste: loader.get("native-menu-paste"),
        select_all: loader.get("native-menu-select-all"),
    }
}
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    #[test]
    fn toolkit_actions_use_the_selected_fluent_language_and_english_fallback() {
        for (language, expected) in [
            (
                "fr",
                [
                    "À propos de typstOgraphe",
                    "Masquer typstOgraphe",
                    "Masquer les autres",
                    "Couper",
                    "Copier",
                    "Coller",
                    "Tout sélectionner",
                ],
            ),
            (
                "en",
                [
                    "About typstOgraphe",
                    "Hide typstOgraphe",
                    "Hide Others",
                    "Cut",
                    "Copy",
                    "Paste",
                    "Select All",
                ],
            ),
            (
                "ja",
                [
                    "About typstOgraphe",
                    "Hide typstOgraphe",
                    "Hide Others",
                    "Cut",
                    "Copy",
                    "Paste",
                    "Select All",
                ],
            ),
        ] {
            let labels = labels_with(&crate::loader_for(&[language.parse().unwrap()]));
            assert_eq!(
                [
                    labels.about,
                    labels.hide,
                    labels.hide_others,
                    labels.cut,
                    labels.copy,
                    labels.paste,
                    labels.select_all
                ],
                expected
            );
        }
    }
}

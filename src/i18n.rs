//! Localisation de l'interface uniquement. Les catalogues sont embarqués dans
//! le binaire ; aucun choix de langue ne touche aux données des courriers.
use i18n_embed::{fluent::FluentLanguageLoader, unic_langid::LanguageIdentifier};
use rust_embed::RustEmbed;
use std::sync::OnceLock;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Catalogs;

fn build(requested: &[LanguageIdentifier]) -> FluentLanguageLoader {
    let loader = i18n_embed::fluent::fluent_language_loader!();
    i18n_embed::select(&loader, &Catalogs, requested).expect("catalogues embarqués valides");
    loader
}

pub fn loader() -> &'static FluentLanguageLoader {
    static LOADER: OnceLock<FluentLanguageLoader> = OnceLock::new();
    LOADER.get_or_init(|| {
        // Les tests de l'interface utilisent une langue explicite pour rester
        // reproductibles sur tout système. La négociation est testée séparément.
        #[cfg(test)]
        let requested = vec!["fr".parse().unwrap()];
        #[cfg(not(test))]
        let requested = i18n_embed::DesktopLanguageRequester::requested_languages();
        build(&requested)
    })
}

/// Les clés du registre sont dynamiques et vérifiées par les tests de catalogue.
pub fn text(key: &str) -> String {
    loader().get(key)
}

/// Les messages écrits dans le code et leurs arguments sont vérifiés à la
/// compilation par i18n-embed-fl, contre le catalogue de repli français.
#[macro_export]
macro_rules! tr {
    ($id:literal $(, $name:ident = $value:expr)* $(,)?) => {
        i18n_embed_fl::fl!($crate::i18n::loader(), $id $(, $name = ($value))*)
    };
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use fluent_syntax::{ast, parser};
    use i18n_embed::LanguageLoader;
    use std::collections::{BTreeMap, BTreeSet, HashMap};

    fn requested(languages: &[&str]) -> FluentLanguageLoader {
        build(
            &languages
                .iter()
                .map(|s| s.parse().unwrap())
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn negotiates_system_preferences_and_falls_back_to_french() {
        for (languages, expected, save) in [
            (vec!["en-US"], "en", "Save"),
            (vec!["fr-CA"], "fr", "Enregistrer"),
            (vec!["de-DE", "en-GB", "fr"], "en", "Save"),
            (vec!["de-DE"], "fr", "Enregistrer"),
            (vec![], "fr", "Enregistrer"),
        ] {
            let loader = requested(&languages);
            assert_eq!(loader.current_language().to_string(), expected);
            assert_eq!(loader.get("cmd-save"), save);
        }
    }

    #[test]
    fn missing_english_message_uses_french_without_changing_selected_language() {
        use i18n_embed::I18nAssets;
        use std::borrow::Cow;

        // Simule un catalogue anglais encore incomplet, avec le vrai repli embarqué.
        struct PartialCatalogs;
        impl I18nAssets for PartialCatalogs {
            fn get_files(&self, path: &str) -> Vec<Cow<'_, [u8]>> {
                if path == "en/typstographe.ftl" {
                    vec![Cow::Borrowed(b"cmd-save = Save")]
                } else {
                    Catalogs.get_files(path)
                }
            }
            fn filenames_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
                Catalogs.filenames_iter()
            }
        }
        let loader = i18n_embed::fluent::fluent_language_loader!();
        i18n_embed::select(&loader, &PartialCatalogs, &["en".parse().unwrap()]).unwrap();
        assert_eq!(loader.get("cmd-save"), "Save");
        assert_eq!(loader.get("field-recipient"), "Destinataire");
        assert_eq!(loader.current_language().to_string(), "en");
    }

    #[test]
    fn commands_have_complete_translations_in_both_catalogs() {
        for language in ["fr", "en"] {
            let loader = requested(&[language]);
            let translated: BTreeSet<_> = loader
                .with_message_iter(&language.parse().unwrap(), |messages| {
                    messages.map(|message| message.id.name.to_owned()).collect()
                });
            // Examiner chaque catalogue sans masquer ses lacunes par le repli.
            for command in crate::document_types::DocumentType::ALL
                .into_iter()
                .flat_map(|kind| crate::commands::Registry::new(kind).definitions())
            {
                for key in [Some(command.label_key), command.short_label_key]
                    .into_iter()
                    .flatten()
                {
                    assert!(translated.contains(key), "{language}: {key}");
                }
            }
        }
    }

    fn variables<'a>(pattern: &ast::Pattern<&'a str>, found: &mut BTreeSet<&'a str>) {
        for element in &pattern.elements {
            if let ast::PatternElement::Placeable { expression } = element {
                expression_variables(expression, found);
            }
        }
    }

    fn expression_variables<'a>(
        expression: &ast::Expression<&'a str>,
        found: &mut BTreeSet<&'a str>,
    ) {
        match expression {
            ast::Expression::Inline(inline) => inline_variables(inline, found),
            ast::Expression::Select { selector, variants } => {
                inline_variables(selector, found);
                for variant in variants {
                    variables(&variant.value, found);
                }
            }
        }
    }

    fn inline_variables<'a>(
        inline: &ast::InlineExpression<&'a str>,
        found: &mut BTreeSet<&'a str>,
    ) {
        match inline {
            ast::InlineExpression::VariableReference { id } => {
                found.insert(id.name);
            }
            ast::InlineExpression::Placeable { expression } => {
                expression_variables(expression, found)
            }
            ast::InlineExpression::FunctionReference { arguments, .. }
            | ast::InlineExpression::TermReference {
                arguments: Some(arguments),
                ..
            } => {
                for argument in &arguments.positional {
                    inline_variables(argument, found);
                }
                for argument in &arguments.named {
                    inline_variables(&argument.value, found);
                }
            }
            _ => {}
        }
    }

    fn catalog_contract(source: &str) -> BTreeMap<String, BTreeSet<&str>> {
        let resource = parser::parse(source).expect("syntaxe Fluent valide");
        let mut contract = BTreeMap::new();
        for entry in resource.body {
            let (id, value, attributes) = match entry {
                ast::Entry::Message(message) => (
                    message.id.name.to_owned(),
                    message.value,
                    message.attributes,
                ),
                ast::Entry::Term(term) => (
                    format!("-{}", term.id.name),
                    Some(term.value),
                    term.attributes,
                ),
                _ => continue,
            };
            if let Some(value) = value {
                let mut found = BTreeSet::new();
                variables(&value, &mut found);
                assert!(
                    contract.insert(id.clone(), found).is_none(),
                    "clé dupliquée : {id}"
                );
            }
            for attribute in attributes {
                let key = format!("{id}.{}", attribute.id.name);
                let mut found = BTreeSet::new();
                variables(&attribute.value, &mut found);
                assert!(
                    contract.insert(key.clone(), found).is_none(),
                    "attribut dupliqué : {key}"
                );
            }
        }
        contract
    }

    #[test]
    fn catalogs_have_the_same_keys_and_parameters() {
        let french = include_str!("../i18n/fr/typstographe.ftl");
        let english = include_str!("../i18n/en/typstographe.ftl");
        assert_eq!(catalog_contract(french), catalog_contract(english));
    }

    #[test]
    fn fluent_handles_plural_rules_and_unicode_parameters() {
        for (language, singular, plural) in [("fr", "mot", "mots"), ("en", "word", "words")] {
            let loader = requested(&[language]);
            for count in [0, 1, 2] {
                let is_singular = count == 1 || (language == "fr" && count == 0);
                for key in ["document-stats", "selection-stats"] {
                    let text =
                        loader.get_args(key, HashMap::from([("words", count), ("signs", count)]));
                    let text: String = text
                        .chars()
                        .filter(|c| !matches!(c, '\u{2068}' | '\u{2069}'))
                        .collect();
                    assert!(
                        text.split_whitespace()
                            .any(|word| word == if is_singular { singular } else { plural }),
                        "{text}"
                    );
                }
            }
            let path = "Élodie/lettre à Noël.typsto";
            assert!(
                loader
                    .get_args("letter-saved", HashMap::from([("path", path)]))
                    .contains(path)
            );
        }
    }
}

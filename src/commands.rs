//! Registre unique des commandes et résolution des raccourcis.
//! Les identifiants sont stables : les préférences pourront remplacer les
//! raccourcis par défaut sans dépendre des libellés français ou du code de l'UI.
use crate::{
    document::BlockKind,
    document_types::{DocumentType, models::Model},
};
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[path = "commands/document_types.rs"]
mod document_types;
#[path = "commands/presentation.rs"]
mod presentation;
use document_types::{model_presentation, type_commands};
pub use presentation::Presentation;
pub(crate) use presentation::Route;
pub(crate) use presentation::{define, short};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileCommand {
    New,
    Open,
    Save,
    SaveAs,
    Export,
    Close,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuGroup {
    File,
    Edit,
    View,
    Insert,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiCommand {
    File(FileCommand),
    Palette,
    Preferences,
    Information,
    Model(Model),
    Bold,
    Italic,
    Clear,
    Block(BlockKind),
    Link,
    RemoveLink,
    InsertImage,
    InsertPostscript,
    InsertAttachment,
    ImportModel,
    RecoverDocuments,
    Find,
    FindReplace,
    FindNext,
    FindPrevious,
    ReplaceCurrent,
    ReplaceAll,
    ToggleSpelling,
    NextSpelling,
    PreviousSpelling,
    SpellingSuggestions,
    IgnoreSpelling,
    LearnSpelling,
    ResetSpelling,
    Undo,
    Redo,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Global,
    Editor,
}

const PRIMARY_SHIFT: Modifiers = Modifiers {
    command: true,
    shift: true,
    ..Modifiers::NONE
};
const fn primary(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::COMMAND, key)
}
const fn primary_shift(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(PRIMARY_SHIFT, key)
}
/// Chaque commande, y compris sans raccourci, est définie une seule fois ici.
pub const COMMON_COMMANDS: &[Presentation] = &[
    define(
        "spelling.suggestions",
        "cmd-spelling-suggestions",
        UiCommand::SpellingSuggestions,
        Scope::Global,
        &[],
    ),
    define(
        "spelling.toggle",
        "cmd-spelling-toggle",
        UiCommand::ToggleSpelling,
        Scope::Global,
        &[],
    ),
    define(
        "spelling.next",
        "cmd-spelling-next",
        UiCommand::NextSpelling,
        Scope::Global,
        &[],
    ),
    define(
        "spelling.previous",
        "cmd-spelling-previous",
        UiCommand::PreviousSpelling,
        Scope::Global,
        &[],
    ),
    define(
        "spelling.ignore",
        "cmd-spelling-ignore",
        UiCommand::IgnoreSpelling,
        Scope::Global,
        &[],
    ),
    define(
        "spelling.learn",
        "cmd-spelling-learn",
        UiCommand::LearnSpelling,
        Scope::Global,
        &[],
    ),
    define(
        "spelling.reset",
        "cmd-spelling-reset",
        UiCommand::ResetSpelling,
        Scope::Global,
        &[],
    ),
    define(
        "edit.replace_current",
        "cmd-replace-current",
        UiCommand::ReplaceCurrent,
        Scope::Global,
        &[],
    ),
    define(
        "edit.replace_all",
        "cmd-replace-all",
        UiCommand::ReplaceAll,
        Scope::Global,
        &[],
    ),
    define(
        "edit.find",
        "cmd-find",
        UiCommand::Find,
        Scope::Global,
        &[primary(Key::F)],
    ),
    Presentation {
        macos: Some(&[KeyboardShortcut::new(
            Modifiers {
                alt: true,
                ..Modifiers::COMMAND
            },
            Key::F,
        )]),
        ..define(
            "edit.find_replace",
            "cmd-find-replace",
            UiCommand::FindReplace,
            Scope::Global,
            &[primary(Key::H)],
        )
    },
    Presentation {
        macos: Some(&[primary(Key::G)]),
        ..define(
            "edit.find_next",
            "cmd-find-next",
            UiCommand::FindNext,
            Scope::Global,
            &[KeyboardShortcut::new(Modifiers::NONE, Key::F3)],
        )
    },
    Presentation {
        macos: Some(&[primary_shift(Key::G)]),
        ..define(
            "edit.find_previous",
            "cmd-find-previous",
            UiCommand::FindPrevious,
            Scope::Global,
            &[KeyboardShortcut::new(Modifiers::SHIFT, Key::F3)],
        )
    },
    define(
        "document.recover",
        "cmd-recover-documents",
        UiCommand::RecoverDocuments,
        Scope::Global,
        &[],
    ),
    define(
        "document.import_model",
        "cmd-import-model",
        UiCommand::ImportModel,
        Scope::Global,
        &[],
    ),
    define(
        "insert.image",
        "cmd-insert-image",
        UiCommand::InsertImage,
        Scope::Global,
        &[],
    ),
    define(
        "application.preferences",
        "cmd-preferences",
        UiCommand::Preferences,
        Scope::Global,
        &[primary(Key::Comma)],
    ),
    define(
        "document.new",
        "cmd-new",
        UiCommand::File(FileCommand::New),
        Scope::Global,
        &[primary(Key::N)],
    ),
    define(
        "document.open",
        "cmd-open",
        UiCommand::File(FileCommand::Open),
        Scope::Global,
        &[primary(Key::O)],
    ),
    define(
        "document.save",
        "cmd-save",
        UiCommand::File(FileCommand::Save),
        Scope::Global,
        &[primary(Key::S)],
    ),
    define(
        "document.save_as",
        "cmd-save-as",
        UiCommand::File(FileCommand::SaveAs),
        Scope::Global,
        &[primary_shift(Key::S)],
    ),
    short(
        define(
            "document.export",
            "cmd-export",
            UiCommand::File(FileCommand::Export),
            Scope::Global,
            &[primary(Key::E)],
        ),
        "cmd-export-short",
    ),
    short(
        define(
            "document.information",
            "cmd-information",
            UiCommand::Information,
            Scope::Global,
            &[],
        ),
        "cmd-information-short",
    ),
    define(
        "document.close",
        "cmd-close",
        UiCommand::File(FileCommand::Close),
        Scope::Global,
        &[primary(Key::W)],
    ),
    define(
        "application.quit",
        "cmd-quit",
        UiCommand::File(FileCommand::Quit),
        Scope::Global,
        &[primary(Key::Q)],
    ),
    define(
        "commands.palette",
        "cmd-palette",
        UiCommand::Palette,
        Scope::Global,
        &[primary_shift(Key::P)],
    ),
    short(
        define(
            "text.bold",
            "cmd-bold",
            UiCommand::Bold,
            Scope::Editor,
            &[primary(Key::B)],
        ),
        "cmd-bold-short",
    ),
    short(
        define(
            "text.italic",
            "cmd-italic",
            UiCommand::Italic,
            Scope::Editor,
            &[primary(Key::I)],
        ),
        "cmd-italic-short",
    ),
    short(
        define(
            "text.link",
            "cmd-link",
            UiCommand::Link,
            Scope::Editor,
            &[primary(Key::K)],
        ),
        "cmd-link-short",
    ),
    define(
        "text.remove_link",
        "cmd-remove-link",
        UiCommand::RemoveLink,
        Scope::Editor,
        &[],
    ),
    define(
        "text.clear",
        "cmd-clear",
        UiCommand::Clear,
        Scope::Editor,
        &[],
    ),
    define(
        "block.paragraph",
        "cmd-paragraph",
        UiCommand::Block(BlockKind::Paragraph),
        Scope::Editor,
        &[],
    ),
    define(
        "block.heading_1",
        "cmd-heading-1",
        UiCommand::Block(BlockKind::Heading(1)),
        Scope::Editor,
        &[],
    ),
    define(
        "block.heading_2",
        "cmd-heading-2",
        UiCommand::Block(BlockKind::Heading(2)),
        Scope::Editor,
        &[],
    ),
    define(
        "block.heading_3",
        "cmd-heading-3",
        UiCommand::Block(BlockKind::Heading(3)),
        Scope::Editor,
        &[],
    ),
    define(
        "block.quote",
        "cmd-quote",
        UiCommand::Block(BlockKind::Quote),
        Scope::Editor,
        &[],
    ),
    short(
        define(
            "block.bullet",
            "cmd-bullet",
            UiCommand::Block(BlockKind::Bullet),
            Scope::Editor,
            &[],
        ),
        "cmd-bullet-short",
    ),
    short(
        define(
            "block.numbered",
            "cmd-numbered",
            UiCommand::Block(BlockKind::Numbered),
            Scope::Editor,
            &[],
        ),
        "cmd-numbered-short",
    ),
    short(
        define(
            "block.description",
            "cmd-description",
            UiCommand::Block(BlockKind::Description),
            Scope::Editor,
            &[],
        ),
        "cmd-description-short",
    ),
    define(
        "edit.undo",
        "cmd-undo",
        UiCommand::Undo,
        Scope::Editor,
        &[primary(Key::Z)],
    ),
    Presentation {
        macos: Some(&[primary_shift(Key::Z), primary(Key::Y)]),
        ..define(
            "edit.redo",
            "cmd-redo",
            UiCommand::Redo,
            Scope::Editor,
            &[primary(Key::Y), primary_shift(Key::Z)],
        )
    },
];

/// Vue du registre pour un document : socle commun + commandes de son type.
#[derive(Clone, Copy)]
pub struct Registry {
    kind: DocumentType,
}
impl Registry {
    pub fn new(kind: DocumentType) -> Self {
        let registry = Self { kind };
        let mut ids = std::collections::BTreeSet::new();
        for presentation in registry.definitions() {
            let _: crate::engine::CommandId = presentation
                .id
                .to_owned()
                .try_into()
                .expect("valid UI command ID");
            assert!(ids.insert(presentation.id), "duplicate UI command ID");
            if let Some(command) = presentation.command.document_command() {
                assert_eq!(
                    presentation.id,
                    command.id(),
                    "canonical command ID mismatch"
                );
            }
        }
        registry
    }
    pub fn document_type(self) -> DocumentType {
        self.kind
    }
    pub fn definitions(self) -> impl Iterator<Item = &'static Presentation> + Clone {
        COMMON_COMMANDS
            .iter()
            .chain(type_commands(self.kind).iter())
            .chain(
                self.kind
                    .models()
                    .map(|model| model_presentation(model.model)),
            )
    }
    pub fn contains(self, command: UiCommand) -> bool {
        self.definitions()
            .any(|definition| definition.command == command)
    }
}

/// Couche facultative destinée aux futures préférences. Une liste vide retire
/// un raccourci ; une entrée absente conserve les valeurs par défaut.
pub type ShortcutOverrides = BTreeMap<String, Vec<Binding>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub key: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
}
impl Binding {
    fn shortcut(&self) -> Result<KeyboardShortcut, String> {
        let key = Key::from_name(&self.key)
            .ok_or_else(|| crate::tr!("shortcut-unknown-key", key = self.key.as_str()))?;
        if !(self.primary || self.ctrl || self.alt || (Key::F1..=Key::F35).contains(&key)) {
            return Err(crate::tr!("shortcut-modifier-required"));
        }
        Ok(KeyboardShortcut::new(
            Modifiers {
                command: self.primary,
                ctrl: self.ctrl,
                shift: self.shift,
                alt: self.alt,
                ..Modifiers::NONE
            },
            key,
        ))
    }
}

struct ActiveCommand {
    definition: &'static Presentation,
    shortcuts: Vec<KeyboardShortcut>,
}
pub struct Keymap {
    registry: Registry,
    overrides: ShortcutOverrides,
    commands: Vec<ActiveCommand>,
    is_mac: bool,
}
impl Keymap {
    /// Construction transactionnelle : des préférences invalides ne remplacent
    /// jamais la configuration active. Aucun fichier de préférences n'est lu ici.
    pub fn new(
        registry: Registry,
        is_mac: bool,
        overrides: &ShortcutOverrides,
    ) -> Result<Self, String> {
        for id in overrides.keys() {
            if !DocumentType::ALL.into_iter().any(|kind| {
                Registry::new(kind)
                    .definitions()
                    .any(|definition| definition.id == id)
            }) {
                return Err(crate::tr!("shortcut-unknown-command", id = id.as_str()));
            }
        }
        let mut commands: Vec<ActiveCommand> = Vec::new();
        for definition in registry.definitions() {
            let shortcuts = if let Some(bindings) = overrides.get(definition.id) {
                bindings
                    .iter()
                    .map(Binding::shortcut)
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                (if is_mac {
                    definition.macos.unwrap_or(definition.defaults)
                } else {
                    definition.defaults
                })
                .to_vec()
            };
            for (index, shortcut) in shortcuts.iter().enumerate() {
                if shortcuts[..index]
                    .iter()
                    .any(|other| equivalent(shortcut, other, is_mac))
                {
                    return Err(crate::tr!(
                        "shortcut-duplicate",
                        command = definition.command.label()
                    ));
                }
                for other in &commands {
                    // Les commandes globales et celles de l'éditeur peuvent être
                    // actives ensemble : elles ne peuvent partager une touche.
                    if other
                        .shortcuts
                        .iter()
                        .any(|other| equivalent(shortcut, other, is_mac))
                    {
                        return Err(crate::tr!(
                            "shortcut-conflict",
                            first = other.definition.command.label(),
                            second = definition.command.label()
                        ));
                    }
                }
            }
            commands.push(ActiveCommand {
                definition,
                shortcuts,
            });
        }
        Ok(Self {
            registry,
            overrides: overrides.clone(),
            commands,
            is_mac,
        })
    }

    pub fn registry(&self) -> Registry {
        self.registry
    }
    /// Réactiver un document conserve les personnalisations, en appliquant
    /// seulement celles des commandes disponibles dans son type.
    pub fn for_document(&self, kind: DocumentType) -> Result<Self, String> {
        let registry = Registry::new(kind);
        let active = self
            .overrides
            .iter()
            .filter(|(id, _)| registry.definitions().any(|d| d.id == id.as_str()))
            .map(|(id, bindings)| (id.clone(), bindings.clone()))
            .collect();
        let mut keymap = Self::new(registry, self.is_mac, &active)?;
        keymap.overrides = self.overrides.clone();
        Ok(keymap)
    }
    pub fn shortcuts(&self, command: UiCommand) -> &[KeyboardShortcut] {
        self.commands
            .iter()
            .find(|entry| entry.definition.command == command)
            .map_or(&[], |entry| entry.shortcuts.as_slice())
    }
    pub fn shortcut_text(&self, ctx: &egui::Context, command: UiCommand) -> String {
        self.shortcuts(command)
            .first()
            .map(|shortcut| ctx.format_shortcut(shortcut))
            .unwrap_or_default()
    }
    pub fn hint(&self, ctx: &egui::Context, command: UiCommand) -> String {
        let shortcut = self.shortcut_text(ctx, command);
        if shortcut.is_empty() {
            command.label()
        } else {
            crate::tr!(
                "command-shortcut",
                label = command.label(),
                shortcut = shortcut
            )
        }
    }
    pub fn consume(
        &self,
        input: &mut egui::InputState,
        editor_focused: bool,
        native_shortcuts: &[KeyboardShortcut],
    ) -> Vec<UiCommand> {
        let mut commands = Vec::new();
        input.events.retain(|event| {
            if let egui::Event::Key {
                key,
                modifiers,
                pressed: true,
                ..
            } = event
            {
                let pressed = KeyboardShortcut::new(*modifiers, *key);
                // AppKit a déjà envoyé l’action correspondante par le menu natif.
                // Retirer l’éventuelle copie clavier sans rejouer la commande.
                if native_shortcuts
                    .iter()
                    .any(|shortcut| equivalent(&pressed, shortcut, self.is_mac))
                {
                    return false;
                }
                for entry in &self.commands {
                    if entry.definition.scope == Scope::Editor && !editor_focused {
                        continue;
                    }
                    if entry.shortcuts.iter().any(|shortcut| {
                        equivalent(
                            &KeyboardShortcut::new(*modifiers, *key),
                            shortcut,
                            self.is_mac,
                        )
                    }) {
                        commands.push(entry.definition.command);
                        return false;
                    }
                }
            }
            true
        });
        commands
    }
}

fn equivalent(a: &KeyboardShortcut, b: &KeyboardShortcut, is_mac: bool) -> bool {
    let normalize = |modifiers: Modifiers| {
        (
            modifiers.ctrl || (modifiers.command && !is_mac),
            modifiers.mac_cmd || (modifiers.command && is_mac),
            modifiers.alt,
            modifiers.shift,
        )
    };
    a.logical_key == b.logical_key && normalize(a.modifiers) == normalize(b.modifiers)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn dispatch(
        keymap: &Keymap,
        key: Key,
        modifiers: Modifiers,
        editor: bool,
    ) -> (Vec<UiCommand>, bool) {
        let ctx = egui::Context::default();
        let mut commands = Vec::new();
        let mut retained = false;
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    modifiers,
                    pressed: true,
                    repeat: false,
                    physical_key: None,
                }],
                ..Default::default()
            },
            |ctx| {
                commands = ctx.input_mut(|input| keymap.consume(input, editor, &[]));
                retained = ctx.input(|input| {
                    input
                        .events
                        .iter()
                        .any(|event| matches!(event, egui::Event::Key { pressed: true, .. }))
                });
            },
        );
        (commands, retained)
    }
    fn overrides(json: &str) -> ShortcutOverrides {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn native_shortcuts_are_not_dispatched_twice_and_other_bindings_still_work() {
        let map = Keymap::new(
            Registry::new(DocumentType::Courrier),
            true,
            &ShortcutOverrides::default(),
        )
        .unwrap();
        let reserved = [primary(Key::E), primary_shift(Key::Z)];
        let ctx = egui::Context::default();
        let mut commands = Vec::new();
        let _ = ctx.run(
            egui::RawInput {
                events: [Key::E, Key::Y]
                    .into_iter()
                    .map(|key| egui::Event::Key {
                        key,
                        modifiers: Modifiers::COMMAND,
                        pressed: true,
                        repeat: false,
                        physical_key: None,
                    })
                    .collect(),
                ..Default::default()
            },
            |ctx| {
                commands = ctx.input_mut(|input| map.consume(input, true, &reserved));
                assert!(ctx.input(|input| input.events.is_empty()));
            },
        );
        // Cmd+E appartient au menu ; Cmd+Y reste l’alias egui de Rétablir.
        assert_eq!(commands, vec![UiCommand::Redo]);
    }

    #[test]
    fn registry_ids_are_unique_and_defaults_have_no_conflicts_on_either_platform() {
        let mut ids = std::collections::HashSet::new();
        let mut actions = Vec::new();
        for definition in Registry::new(DocumentType::Courrier).definitions() {
            assert!(ids.insert(definition.id));
            assert!(!actions.contains(&definition.command));
            actions.push(definition.command);
        }
        assert!(
            COMMON_COMMANDS
                .iter()
                .all(|d| !matches!(d.command, UiCommand::Model(_)))
        );
        assert!(
            crate::document_types::models::ALL
                .iter()
                .all(|d| matches!(model_presentation(d.model).command, UiCommand::Model(_)))
        );
        for kind in DocumentType::ALL {
            let models: Vec<_> = Registry::new(kind)
                .definitions()
                .filter(|p| p.in_model_menu())
                .collect();
            assert_eq!(models.len(), kind.models().count());
            for model in kind.models() {
                let presentation = model_presentation(model.model);
                assert_eq!(presentation.id, model.model.id());
                assert!(
                    models
                        .iter()
                        .any(|p| p.command == UiCommand::Model(model.model))
                );
                assert!(
                    crate::engine::catalogue()
                        .iter()
                        .any(|entry| entry.id.as_str() == presentation.id)
                );
            }
        }
        assert!(
            Keymap::new(
                Registry::new(DocumentType::Courrier),
                true,
                &ShortcutOverrides::default()
            )
            .is_ok()
        );
        assert!(
            Keymap::new(
                Registry::new(DocumentType::Courrier),
                false,
                &ShortcutOverrides::default()
            )
            .is_ok()
        );
    }

    #[test]
    fn primary_modifier_resolves_to_command_on_mac_and_control_elsewhere() {
        let mac = Keymap::new(
            Registry::new(DocumentType::Courrier),
            true,
            &ShortcutOverrides::default(),
        )
        .unwrap();
        let pc = Keymap::new(
            Registry::new(DocumentType::Courrier),
            false,
            &ShortcutOverrides::default(),
        )
        .unwrap();
        let control = Modifiers {
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        };
        let command = Modifiers {
            mac_cmd: true,
            command: true,
            ..Modifiers::NONE
        };
        assert_eq!(
            dispatch(&pc, Key::E, control, false),
            (vec![UiCommand::File(FileCommand::Export)], false)
        );
        assert_eq!(
            dispatch(&mac, Key::E, command, false),
            (vec![UiCommand::File(FileCommand::Export)], false)
        );
        assert_eq!(
            dispatch(&mac, Key::E, Modifiers::CTRL, false),
            (vec![], true)
        );
        assert_eq!(
            dispatch(
                &mac,
                Key::E,
                Modifiers {
                    ctrl: true,
                    ..command
                },
                false
            ),
            (vec![], true)
        );
    }

    #[test]
    fn courrier_shortcuts_share_the_common_conflict_checks_and_dispatch() {
        let registry = Registry::new(DocumentType::Courrier);
        let action = UiCommand::Model(Model::Personal);
        assert!(registry.contains(action));
        let config = overrides(r#"{"courrier.model.personal":[{"key":"F7"}]}"#);
        let keymap = Keymap::new(registry, false, &config).unwrap();
        let keymap = keymap.for_document(DocumentType::Courrier).unwrap();
        assert_eq!(
            dispatch(&keymap, Key::F7, Modifiers::NONE, false),
            (vec![action], false)
        );
        let minutes = keymap.for_document(DocumentType::CompteRendu).unwrap();
        assert_eq!(
            dispatch(&minutes, Key::F7, Modifiers::NONE, false),
            (vec![], true)
        );
        let keymap = minutes.for_document(DocumentType::Courrier).unwrap();
        assert_eq!(
            dispatch(&keymap, Key::F7, Modifiers::NONE, false),
            (vec![action], false)
        );
        let config = overrides(r#"{"courrier.model.personal":[{"key":"S","primary":true}]}"#);
        assert!(Keymap::new(registry, false, &config).is_err());
    }

    #[test]
    fn context_and_extra_modifiers_do_not_trigger_unrelated_commands() {
        let map = Keymap::new(
            Registry::new(DocumentType::Courrier),
            false,
            &ShortcutOverrides::default(),
        )
        .unwrap();
        assert_eq!(
            dispatch(&map, Key::B, Modifiers::COMMAND, false),
            (vec![], true)
        );
        assert_eq!(
            dispatch(&map, Key::B, Modifiers::COMMAND, true),
            (vec![UiCommand::Bold], false)
        );
        assert_eq!(
            dispatch(&map, Key::S, PRIMARY_SHIFT, true),
            (vec![UiCommand::File(FileCommand::SaveAs)], false)
        );
        assert_eq!(dispatch(&map, Key::E, PRIMARY_SHIFT, true), (vec![], true));
        assert_eq!(
            dispatch(&map, Key::A, Modifiers::NONE, true),
            (vec![], true)
        );
    }

    #[test]
    fn overrides_replace_or_disable_defaults_and_accept_function_keys() {
        let config = overrides(
            r#"{"document.export":[{"key":"E","primary":true,"shift":true}],"text.bold":[],"text.italic":[{"key":"F6"}]}"#,
        );
        let config = serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        let map = Keymap::new(Registry::new(DocumentType::Courrier), false, &config).unwrap();
        assert_eq!(
            dispatch(&map, Key::E, Modifiers::COMMAND, true),
            (vec![], true)
        );
        assert_eq!(
            dispatch(&map, Key::E, PRIMARY_SHIFT, true),
            (vec![UiCommand::File(FileCommand::Export)], false)
        );
        assert_eq!(
            dispatch(&map, Key::B, Modifiers::COMMAND, true),
            (vec![], true)
        );
        assert_eq!(
            dispatch(&map, Key::F6, Modifiers::NONE, true),
            (vec![UiCommand::Italic], false)
        );
        assert_eq!(
            dispatch(&map, Key::I, Modifiers::COMMAND, true),
            (vec![], true)
        );
        assert_eq!(
            dispatch(&map, Key::F6, Modifiers::NONE, false),
            (vec![], true)
        );
    }

    #[test]
    fn conflicts_include_platform_aliases_but_allow_distinct_mac_modifiers() {
        let config = overrides(r#"{"text.bold":[{"key":"E","ctrl":true}]}"#);
        assert!(Keymap::new(Registry::new(DocumentType::Courrier), false, &config).is_err());
        let mac = Keymap::new(Registry::new(DocumentType::Courrier), true, &config).unwrap();
        assert_eq!(
            dispatch(&mac, Key::E, Modifiers::CTRL, true),
            (vec![UiCommand::Bold], false)
        );
        assert_eq!(
            dispatch(&mac, Key::E, Modifiers::COMMAND, true),
            (vec![UiCommand::File(FileCommand::Export)], false)
        );
        for config in [
            r#"{"document.export":[{"key":"S","primary":true}]}"#,
            r#"{"document.export":[{"key":"E","primary":true},{"key":"E","ctrl":true}]}"#,
            r#"{"unknown.command":[]}"#,
            r#"{"document.export":[{"key":"Unknown","primary":true}]}"#,
            r#"{"document.export":[{"key":"E"}]}"#,
        ] {
            assert!(
                Keymap::new(
                    Registry::new(DocumentType::Courrier),
                    false,
                    &overrides(config)
                )
                .is_err(),
                "{config}"
            );
        }
    }
}

//! Registre unique des commandes et résolution des raccourcis.
//! Les identifiants sont stables : les préférences pourront remplacer les
//! raccourcis par défaut sans dépendre des libellés français ou du code de l'UI.
use crate::{
    document::BlockKind,
    document_types::{DocumentType, courrier},
};
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    File(FileCommand),
    Palette,
    Preferences,
    Information,
    Courrier(courrier::Command),
    Bold,
    Italic,
    Clear,
    Block(BlockKind),
    Link,
    RemoveLink,
    Undo,
    Redo,
}
impl Command {
    pub fn model_source(self) -> Option<String> {
        match self {
            Self::Courrier(courrier::Command::Model(model)) => Some(model.source()),
            _ => None,
        }
    }
    pub fn definition(self) -> &'static Definition {
        if let Self::Courrier(command) = self {
            return command.definition();
        }
        COMMON_COMMANDS
            .iter()
            .find(|definition| definition.command == self)
            .expect("commande enregistrée")
    }
    pub fn label(self) -> String {
        crate::i18n::text(self.definition().label_key)
    }
    pub fn short_label(self) -> String {
        let definition = self.definition();
        crate::i18n::text(definition.short_label_key.unwrap_or(definition.label_key))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Global,
    Editor,
}

pub struct Definition {
    pub id: &'static str,
    pub label_key: &'static str, // Clé Fluent, distincte de l’identifiant de commande.
    pub command: Command,
    pub scope: Scope,
    pub short_label_key: Option<&'static str>,
    defaults: &'static [KeyboardShortcut],
    macos: Option<&'static [KeyboardShortcut]>,
}
impl Definition {
    pub fn in_palette(&self) -> bool {
        self.command != Command::Palette
    }
    /// Le groupe et le rang définissent aussi les séparateurs (par dizaines).
    pub fn menu_position(&self) -> Option<(MenuGroup, u8)> {
        Some(match self.command {
            Command::File(FileCommand::New) => (MenuGroup::File, 0),
            Command::File(FileCommand::Open) => (MenuGroup::File, 1),
            Command::File(FileCommand::Save) => (MenuGroup::File, 10),
            Command::File(FileCommand::SaveAs) => (MenuGroup::File, 11),
            Command::File(FileCommand::Export) => (MenuGroup::File, 20),
            Command::File(FileCommand::Close) => (MenuGroup::File, 30),
            Command::File(FileCommand::Quit) => (MenuGroup::File, 31),
            Command::Undo => (MenuGroup::Edit, 0),
            Command::Redo => (MenuGroup::Edit, 1),
            Command::Clear => (MenuGroup::Edit, 10),
            Command::Palette => (MenuGroup::Edit, 20),
            Command::Preferences => (MenuGroup::Edit, 30),
            Command::Information => (MenuGroup::View, 0),
            _ => return None,
        })
    }
    pub fn in_model_menu(&self) -> bool {
        matches!(self.command, Command::Courrier(courrier::Command::Model(_)))
    }
    pub fn in_block_style_menu(&self) -> bool {
        matches!(
            self.command,
            Command::Block(BlockKind::Paragraph | BlockKind::Heading(_) | BlockKind::Description)
        )
    }
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
pub(crate) const fn define(
    id: &'static str,
    label_key: &'static str,
    command: Command,
    scope: Scope,
    defaults: &'static [KeyboardShortcut],
) -> Definition {
    Definition {
        id,
        label_key,
        command,
        scope,
        defaults,
        macos: None,
        short_label_key: None,
    }
}
pub(crate) const fn short(mut definition: Definition, label: &'static str) -> Definition {
    definition.short_label_key = Some(label);
    definition
}

/// Chaque commande, y compris sans raccourci, est définie une seule fois ici.
pub const COMMON_COMMANDS: &[Definition] = &[
    define(
        "application.preferences",
        "cmd-preferences",
        Command::Preferences,
        Scope::Global,
        &[primary(Key::Comma)],
    ),
    define(
        "document.new",
        "cmd-new",
        Command::File(FileCommand::New),
        Scope::Global,
        &[primary(Key::N)],
    ),
    define(
        "document.open",
        "cmd-open",
        Command::File(FileCommand::Open),
        Scope::Global,
        &[primary(Key::O)],
    ),
    define(
        "document.save",
        "cmd-save",
        Command::File(FileCommand::Save),
        Scope::Global,
        &[primary(Key::S)],
    ),
    define(
        "document.save_as",
        "cmd-save-as",
        Command::File(FileCommand::SaveAs),
        Scope::Global,
        &[primary_shift(Key::S)],
    ),
    short(
        define(
            "document.export",
            "cmd-export",
            Command::File(FileCommand::Export),
            Scope::Global,
            &[primary(Key::E)],
        ),
        "cmd-export-short",
    ),
    short(
        define(
            "document.information",
            "cmd-information",
            Command::Information,
            Scope::Global,
            &[],
        ),
        "cmd-information-short",
    ),
    define(
        "document.close",
        "cmd-close",
        Command::File(FileCommand::Close),
        Scope::Global,
        &[primary(Key::W)],
    ),
    define(
        "application.quit",
        "cmd-quit",
        Command::File(FileCommand::Quit),
        Scope::Global,
        &[primary(Key::Q)],
    ),
    define(
        "commands.palette",
        "cmd-palette",
        Command::Palette,
        Scope::Global,
        &[primary_shift(Key::P)],
    ),
    short(
        define(
            "text.bold",
            "cmd-bold",
            Command::Bold,
            Scope::Editor,
            &[primary(Key::B)],
        ),
        "cmd-bold-short",
    ),
    short(
        define(
            "text.italic",
            "cmd-italic",
            Command::Italic,
            Scope::Editor,
            &[primary(Key::I)],
        ),
        "cmd-italic-short",
    ),
    short(
        define(
            "text.link",
            "cmd-link",
            Command::Link,
            Scope::Editor,
            &[primary(Key::K)],
        ),
        "cmd-link-short",
    ),
    define(
        "text.remove_link",
        "cmd-remove-link",
        Command::RemoveLink,
        Scope::Editor,
        &[],
    ),
    define(
        "text.clear",
        "cmd-clear",
        Command::Clear,
        Scope::Editor,
        &[],
    ),
    define(
        "block.paragraph",
        "cmd-paragraph",
        Command::Block(BlockKind::Paragraph),
        Scope::Editor,
        &[],
    ),
    define(
        "block.heading_1",
        "cmd-heading-1",
        Command::Block(BlockKind::Heading(1)),
        Scope::Editor,
        &[],
    ),
    define(
        "block.heading_2",
        "cmd-heading-2",
        Command::Block(BlockKind::Heading(2)),
        Scope::Editor,
        &[],
    ),
    define(
        "block.heading_3",
        "cmd-heading-3",
        Command::Block(BlockKind::Heading(3)),
        Scope::Editor,
        &[],
    ),
    short(
        define(
            "block.bullet",
            "cmd-bullet",
            Command::Block(BlockKind::Bullet),
            Scope::Editor,
            &[],
        ),
        "cmd-bullet-short",
    ),
    short(
        define(
            "block.numbered",
            "cmd-numbered",
            Command::Block(BlockKind::Numbered),
            Scope::Editor,
            &[],
        ),
        "cmd-numbered-short",
    ),
    short(
        define(
            "block.description",
            "cmd-description",
            Command::Block(BlockKind::Description),
            Scope::Editor,
            &[],
        ),
        "cmd-description-short",
    ),
    define(
        "edit.undo",
        "cmd-undo",
        Command::Undo,
        Scope::Editor,
        &[primary(Key::Z)],
    ),
    Definition {
        macos: Some(&[primary_shift(Key::Z), primary(Key::Y)]),
        ..define(
            "edit.redo",
            "cmd-redo",
            Command::Redo,
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
        Self { kind }
    }
    pub fn document_type(self) -> DocumentType {
        self.kind
    }
    pub fn definitions(self) -> impl Iterator<Item = &'static Definition> + Clone {
        COMMON_COMMANDS.iter().chain(self.kind.commands().iter())
    }
    pub fn contains(self, command: Command) -> bool {
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
    definition: &'static Definition,
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
            if !registry.definitions().any(|definition| definition.id == id) {
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
    pub fn shortcuts(&self, command: Command) -> &[KeyboardShortcut] {
        self.commands
            .iter()
            .find(|entry| entry.definition.command == command)
            .map_or(&[], |entry| entry.shortcuts.as_slice())
    }
    pub fn shortcut_text(&self, ctx: &egui::Context, command: Command) -> String {
        self.shortcuts(command)
            .first()
            .map(|shortcut| ctx.format_shortcut(shortcut))
            .unwrap_or_default()
    }
    pub fn hint(&self, ctx: &egui::Context, command: Command) -> String {
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
    ) -> Vec<Command> {
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
mod tests {
    use super::*;

    fn dispatch(
        keymap: &Keymap,
        key: Key,
        modifiers: Modifiers,
        editor: bool,
    ) -> (Vec<Command>, bool) {
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
        assert_eq!(commands, vec![Command::Redo]);
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
                .all(|d| !matches!(d.command, Command::Courrier(_)))
        );
        assert!(
            courrier::COMMANDS
                .iter()
                .all(|d| matches!(d.command, Command::Courrier(_)))
        );
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
            (vec![Command::File(FileCommand::Export)], false)
        );
        assert_eq!(
            dispatch(&mac, Key::E, command, false),
            (vec![Command::File(FileCommand::Export)], false)
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
        let action = Command::Courrier(courrier::Command::Model(courrier::Model::Service));
        assert!(registry.contains(action));
        let config = overrides(r#"{"courrier.model.service":[{"key":"F7"}]}"#);
        let keymap = Keymap::new(registry, false, &config).unwrap();
        let keymap = keymap.for_document(DocumentType::Courrier).unwrap();
        assert_eq!(
            dispatch(&keymap, Key::F7, Modifiers::NONE, false),
            (vec![action], false)
        );
        let config = overrides(r#"{"courrier.model.service":[{"key":"S","primary":true}]}"#);
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
            (vec![Command::Bold], false)
        );
        assert_eq!(
            dispatch(&map, Key::S, PRIMARY_SHIFT, true),
            (vec![Command::File(FileCommand::SaveAs)], false)
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
            (vec![Command::File(FileCommand::Export)], false)
        );
        assert_eq!(
            dispatch(&map, Key::B, Modifiers::COMMAND, true),
            (vec![], true)
        );
        assert_eq!(
            dispatch(&map, Key::F6, Modifiers::NONE, true),
            (vec![Command::Italic], false)
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
            (vec![Command::Bold], false)
        );
        assert_eq!(
            dispatch(&mac, Key::E, Modifiers::COMMAND, true),
            (vec![Command::File(FileCommand::Export)], false)
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

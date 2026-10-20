//! Registre unique des commandes et résolution des raccourcis.
//! Les identifiants sont stables : les préférences pourront remplacer les
//! raccourcis par défaut sans dépendre des libellés français ou du code de l'UI.
use crate::document::BlockKind;
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presentation {
    Institutional,
    Service,
}
impl Presentation {
    pub fn compact(self) -> bool {
        self == Self::Service
    }
    pub fn from_compact(compact: bool) -> Self {
        if compact {
            Self::Service
        } else {
            Self::Institutional
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    File(FileCommand),
    Palette,
    Information,
    Presentation(Presentation),
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
    pub fn definition(self) -> &'static Definition {
        DEFINITIONS
            .iter()
            .find(|definition| definition.command == self)
            .expect("commande enregistrée")
    }
    pub fn label(self) -> &'static str {
        self.definition().label
    }
    pub fn short_label(self) -> &'static str {
        let definition = self.definition();
        definition.short_label.unwrap_or(definition.label)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Global,
    Editor,
}

pub struct Definition {
    pub id: &'static str,
    pub label: &'static str,
    pub command: Command,
    pub scope: Scope,
    short_label: Option<&'static str>,
    defaults: &'static [KeyboardShortcut],
    macos: Option<&'static [KeyboardShortcut]>,
}
impl Definition {
    pub fn in_palette(&self) -> bool {
        self.command != Command::Palette
    }
    pub fn in_file_menu(&self) -> bool {
        matches!(self.command, Command::File(_) | Command::Information)
    }
    pub fn in_presentation_menu(&self) -> bool {
        matches!(self.command, Command::Presentation(_))
    }
    pub fn in_block_style_menu(&self) -> bool {
        matches!(
            self.command,
            Command::Block(BlockKind::Paragraph | BlockKind::Heading(_))
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
const fn define(
    id: &'static str,
    label: &'static str,
    command: Command,
    scope: Scope,
    defaults: &'static [KeyboardShortcut],
) -> Definition {
    Definition {
        id,
        label,
        command,
        scope,
        defaults,
        macos: None,
        short_label: None,
    }
}
const fn short(mut definition: Definition, label: &'static str) -> Definition {
    definition.short_label = Some(label);
    definition
}

/// Chaque commande, y compris sans raccourci, est définie une seule fois ici.
pub static DEFINITIONS: &[Definition] = &[
    define(
        "document.new",
        "Nouveau courrier",
        Command::File(FileCommand::New),
        Scope::Global,
        &[primary(Key::N)],
    ),
    define(
        "document.open",
        "Ouvrir un courrier…",
        Command::File(FileCommand::Open),
        Scope::Global,
        &[primary(Key::O)],
    ),
    define(
        "document.save",
        "Enregistrer",
        Command::File(FileCommand::Save),
        Scope::Global,
        &[primary(Key::S)],
    ),
    define(
        "document.save_as",
        "Enregistrer sous…",
        Command::File(FileCommand::SaveAs),
        Scope::Global,
        &[primary_shift(Key::S)],
    ),
    short(
        define(
            "document.export",
            "Exporter le courrier…",
            Command::File(FileCommand::Export),
            Scope::Global,
            &[primary(Key::E)],
        ),
        "Exporter",
    ),
    short(
        define(
            "document.information",
            "Informations du courrier",
            Command::Information,
            Scope::Global,
            &[],
        ),
        "Informations",
    ),
    define(
        "document.close",
        "Fermer",
        Command::File(FileCommand::Close),
        Scope::Global,
        &[],
    ),
    define(
        "commands.palette",
        "Commandes",
        Command::Palette,
        Scope::Global,
        &[primary_shift(Key::P)],
    ),
    short(
        define(
            "document.presentation.institutional",
            "Utiliser la présentation institutionnelle",
            Command::Presentation(Presentation::Institutional),
            Scope::Global,
            &[],
        ),
        "Courrier institutionnel",
    ),
    short(
        define(
            "document.presentation.service",
            "Utiliser la présentation de service",
            Command::Presentation(Presentation::Service),
            Scope::Global,
            &[],
        ),
        "Courrier de service",
    ),
    short(
        define(
            "text.bold",
            "Gras",
            Command::Bold,
            Scope::Editor,
            &[primary(Key::B)],
        ),
        "G",
    ),
    short(
        define(
            "text.italic",
            "Italique",
            Command::Italic,
            Scope::Editor,
            &[primary(Key::I)],
        ),
        "I",
    ),
    short(
        define(
            "text.link",
            "Insérer ou modifier un lien",
            Command::Link,
            Scope::Editor,
            &[primary(Key::K)],
        ),
        "Lien",
    ),
    define(
        "text.remove_link",
        "Retirer le lien",
        Command::RemoveLink,
        Scope::Editor,
        &[],
    ),
    short(
        define(
            "text.clear",
            "Retirer la mise en forme",
            Command::Clear,
            Scope::Editor,
            &[],
        ),
        "Effacer",
    ),
    define(
        "block.paragraph",
        "Paragraphe",
        Command::Block(BlockKind::Paragraph),
        Scope::Editor,
        &[],
    ),
    define(
        "block.heading_1",
        "Titre 1",
        Command::Block(BlockKind::Heading(1)),
        Scope::Editor,
        &[],
    ),
    define(
        "block.heading_2",
        "Titre 2",
        Command::Block(BlockKind::Heading(2)),
        Scope::Editor,
        &[],
    ),
    define(
        "block.heading_3",
        "Titre 3",
        Command::Block(BlockKind::Heading(3)),
        Scope::Editor,
        &[],
    ),
    short(
        define(
            "block.bullet",
            "Liste à puces",
            Command::Block(BlockKind::Bullet),
            Scope::Editor,
            &[],
        ),
        "• Liste",
    ),
    short(
        define(
            "block.numbered",
            "Liste numérotée",
            Command::Block(BlockKind::Numbered),
            Scope::Editor,
            &[],
        ),
        "1. Liste",
    ),
    define(
        "edit.undo",
        "Annuler",
        Command::Undo,
        Scope::Editor,
        &[primary(Key::Z)],
    ),
    Definition {
        macos: Some(&[primary_shift(Key::Z), primary(Key::Y)]),
        ..define(
            "edit.redo",
            "Rétablir",
            Command::Redo,
            Scope::Editor,
            &[primary(Key::Y), primary_shift(Key::Z)],
        )
    },
];

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
        let key =
            Key::from_name(&self.key).ok_or_else(|| format!("Touche inconnue : {}", self.key))?;
        if !(self.primary || self.ctrl || self.alt || (Key::F1..=Key::F35).contains(&key)) {
            return Err("Utilisez Ctrl, ⌘ ou Alt, ou une touche de fonction, pour préserver la saisie du texte.".into());
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
    commands: Vec<ActiveCommand>,
    is_mac: bool,
}
impl Keymap {
    /// Construction transactionnelle : des préférences invalides ne remplacent
    /// jamais la configuration active. Aucun fichier de préférences n'est lu ici.
    pub fn new(is_mac: bool, overrides: &ShortcutOverrides) -> Result<Self, String> {
        for id in overrides.keys() {
            if !DEFINITIONS.iter().any(|definition| definition.id == id) {
                return Err(format!("Commande inconnue : {id}"));
            }
        }
        let mut commands: Vec<ActiveCommand> = Vec::new();
        for definition in DEFINITIONS {
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
                    return Err(format!(
                        "Raccourci défini plusieurs fois pour {}",
                        definition.label
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
                        return Err(format!(
                            "Conflit de raccourcis : {} et {}",
                            other.definition.label, definition.label
                        ));
                    }
                }
            }
            commands.push(ActiveCommand {
                definition,
                shortcuts,
            });
        }
        Ok(Self { commands, is_mac })
    }

    pub fn shortcut_text(&self, ctx: &egui::Context, command: Command) -> String {
        self.commands
            .iter()
            .find(|entry| entry.definition.command == command)
            .and_then(|entry| entry.shortcuts.first())
            .map(|shortcut| ctx.format_shortcut(shortcut))
            .unwrap_or_default()
    }
    pub fn hint(&self, ctx: &egui::Context, command: Command) -> String {
        let shortcut = self.shortcut_text(ctx, command);
        if shortcut.is_empty() {
            command.label().into()
        } else {
            format!("{} · {shortcut}", command.label())
        }
    }
    pub fn consume(&self, input: &mut egui::InputState, editor_focused: bool) -> Vec<Command> {
        let mut commands = Vec::new();
        input.events.retain(|event| {
            if let egui::Event::Key {
                key,
                modifiers,
                pressed: true,
                ..
            } = event
            {
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
                commands = ctx.input_mut(|input| keymap.consume(input, editor));
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
    fn registry_ids_are_unique_and_defaults_have_no_conflicts_on_either_platform() {
        let mut ids = std::collections::HashSet::new();
        for (index, definition) in DEFINITIONS.iter().enumerate() {
            assert!(ids.insert(definition.id));
            assert!(
                !DEFINITIONS[..index]
                    .iter()
                    .any(|other| other.command == definition.command)
            );
        }
        assert!(Keymap::new(true, &ShortcutOverrides::default()).is_ok());
        assert!(Keymap::new(false, &ShortcutOverrides::default()).is_ok());
    }

    #[test]
    fn primary_modifier_resolves_to_command_on_mac_and_control_elsewhere() {
        let mac = Keymap::new(true, &ShortcutOverrides::default()).unwrap();
        let pc = Keymap::new(false, &ShortcutOverrides::default()).unwrap();
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
    fn context_and_extra_modifiers_do_not_trigger_unrelated_commands() {
        let map = Keymap::new(false, &ShortcutOverrides::default()).unwrap();
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
        let map = Keymap::new(false, &config).unwrap();
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
        assert!(Keymap::new(false, &config).is_err());
        let mac = Keymap::new(true, &config).unwrap();
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
            assert!(Keymap::new(false, &overrides(config)).is_err(), "{config}");
        }
    }
}

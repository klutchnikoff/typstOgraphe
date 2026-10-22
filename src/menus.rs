//! Deux présentations des mêmes menus, construites depuis le registre.
use crate::commands::{FileCommand, Keymap, MenuGroup, Registry, UiCommand as Command};
use eframe::egui;

pub const GROUPS: [MenuGroup; 4] = [
    MenuGroup::File,
    MenuGroup::Edit,
    MenuGroup::Insert,
    MenuGroup::View,
];

impl MenuGroup {
    pub fn label(self) -> String {
        match self {
            Self::File => crate::tr!("file-menu"),
            Self::Edit => crate::tr!("edit-menu"),
            Self::View => crate::tr!("view-menu"),
            Self::Insert => crate::tr!("insert-menu"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub enabled: bool,
    pub checked: bool,
}

pub fn items(registry: Registry, group: MenuGroup, macos: bool) -> Vec<(Command, bool)> {
    let mut commands: Vec<_> = registry
        .definitions()
        .filter_map(|definition| {
            let (menu, order) = definition.menu_position()?;
            (menu == group
                && !(macos
                    && matches!(
                        definition.command,
                        Command::File(FileCommand::Quit) | Command::Preferences
                    )))
            .then_some((order, definition.command))
        })
        .collect();
    commands.sort_by_key(|(order, _)| *order);
    let mut previous = None;
    commands
        .into_iter()
        .map(|(order, command)| {
            let section = order / 10;
            let separator = previous.is_some_and(|p| p != section);
            previous = Some(section);
            (command, separator)
        })
        .collect()
}

pub fn egui_bar(
    ui: &mut egui::Ui,
    keymap: &Keymap,
    statuses: &[(Command, Status)],
) -> Option<Command> {
    let mut chosen = None;
    egui::MenuBar::new().ui(ui, |ui| {
        for group in GROUPS {
            ui.menu_button(group.label(), |ui| {
                for (command, separator) in items(keymap.registry(), group, false) {
                    if separator {
                        ui.separator();
                    }
                    let status = statuses.iter().find(|(c, _)| *c == command).unwrap().1;
                    let button = egui::Button::new(command.label())
                        .selected(status.checked)
                        .shortcut_text(keymap.shortcut_text(ui.ctx(), command));
                    if ui.add_enabled(status.enabled, button).clicked() {
                        chosen = Some(command);
                        ui.close();
                    }
                }
            });
        }
    });
    chosen
}

#[cfg(target_os = "macos")]
pub mod native {
    use super::*;
    use crate::document_types::DocumentType;
    use egui::{Key, KeyboardShortcut};
    use muda::{
        CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
        accelerator::{Key as NativeKey, KeyAccelerator, Modifiers},
    };
    use std::sync::mpsc;

    pub fn accelerator(shortcut: KeyboardShortcut) -> Option<KeyAccelerator> {
        let key = match shortcut.logical_key {
            Key::Space => NativeKey::Character(" ".into()),
            Key::Minus => NativeKey::Character("-".into()),
            Key::Quote => NativeKey::Character("'".into()),
            Key::Copy | Key::Cut | Key::Paste | Key::BrowserBack => return None,
            Key::ArrowDown => NativeKey::Named(muda::accelerator::NamedKey::ArrowDown),
            Key::ArrowLeft => NativeKey::Named(muda::accelerator::NamedKey::ArrowLeft),
            Key::ArrowRight => NativeKey::Named(muda::accelerator::NamedKey::ArrowRight),
            Key::ArrowUp => NativeKey::Named(muda::accelerator::NamedKey::ArrowUp),
            key => {
                let symbol = key.symbol_or_name();
                if symbol.chars().count() == 1 {
                    NativeKey::Character(symbol.to_lowercase())
                } else {
                    NativeKey::Named(key.name().parse().ok()?)
                }
            }
        };
        let mut modifiers = Modifiers::empty();
        let m = shortcut.modifiers;
        if m.command || m.mac_cmd {
            modifiers |= Modifiers::META;
        }
        if m.ctrl {
            modifiers |= Modifiers::CONTROL;
        }
        if m.shift {
            modifiers |= Modifiers::SHIFT;
        }
        if m.alt {
            modifiers |= Modifiers::ALT;
        }
        Some(KeyAccelerator::new(modifiers, key))
    }

    enum Item {
        Action(MenuItem),
        Toggle(CheckMenuItem),
    }
    impl Item {
        fn as_item(&self) -> &dyn IsMenuItem {
            match self {
                Self::Action(item) => item,
                Self::Toggle(item) => item,
            }
        }
        fn set_shortcut(&self, shortcut: Option<KeyboardShortcut>) -> muda::Result<()> {
            let accelerator = shortcut.and_then(accelerator);
            match self {
                Self::Action(item) => item.set_key_accelerator(accelerator),
                Self::Toggle(item) => item.set_key_accelerator(accelerator),
            }
        }
        fn set_status(&self, status: Status) {
            match self {
                Self::Action(item) => item.set_enabled(status.enabled),
                Self::Toggle(item) => {
                    item.set_enabled(status.enabled);
                    item.set_checked(status.checked);
                }
            }
        }
    }
    struct Entry {
        command: Command,
        item: Item,
        shortcut: Option<KeyboardShortcut>,
        status: Status,
    }
    pub struct NativeMenus {
        menu: Menu,
        document_type: DocumentType,
        entries: Vec<Entry>,
        receiver: mpsc::Receiver<MenuEvent>,
    }
    impl NativeMenus {
        /// À créer après la fenêtre eframe, sur le thread principal AppKit.
        pub fn new(ctx: egui::Context, keymap: &Keymap) -> muda::Result<Self> {
            let menu = Menu::new();
            let mut entries = Vec::new();
            let application = Submenu::new("typstOgraphe", true);
            Self::append(&application, Command::Preferences, keymap, &mut entries)?;
            application.append(&PredefinedMenuItem::separator())?;
            application.append_items(&[
                &PredefinedMenuItem::services(Some(&crate::tr!("mac-services"))),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(Some(&crate::tr!("mac-hide"))),
                &PredefinedMenuItem::hide_others(Some(&crate::tr!("mac-hide-others"))),
                &PredefinedMenuItem::show_all(Some(&crate::tr!("mac-show-all"))),
                &PredefinedMenuItem::separator(),
            ])?;
            Self::append(
                &application,
                Command::File(FileCommand::Quit),
                keymap,
                &mut entries,
            )?;
            menu.append(&application)?;
            for group in GROUPS {
                let submenu = Submenu::new(group.label(), true);
                for (command, separator) in items(keymap.registry(), group, true) {
                    if separator {
                        submenu.append(&PredefinedMenuItem::separator())?;
                    }
                    Self::append(&submenu, command, keymap, &mut entries)?;
                }
                menu.append(&submenu)?;
            }
            let (sender, receiver) = mpsc::channel();
            MenuEvent::set_event_handler(Some(move |event| {
                let _ = sender.send(event);
                ctx.request_repaint();
            }));
            menu.init_for_nsapp();
            Ok(Self {
                menu,
                document_type: keymap.registry().document_type(),
                entries,
                receiver,
            })
        }
        fn append(
            submenu: &Submenu,
            command: Command,
            keymap: &Keymap,
            entries: &mut Vec<Entry>,
        ) -> muda::Result<()> {
            let definition = command.definition();
            let item = if matches!(command, Command::Information | Command::ToggleSpelling) {
                Item::Toggle(CheckMenuItem::with_id(
                    definition.id,
                    command.label(),
                    false,
                    false,
                    None,
                ))
            } else {
                Item::Action(MenuItem::with_id(
                    definition.id,
                    command.label(),
                    false,
                    None,
                ))
            };
            let shortcut = keymap
                .shortcuts(command)
                .first()
                .copied()
                .filter(|s| accelerator(*s).is_some());
            item.set_shortcut(shortcut)?;
            submenu.append(item.as_item())?;
            entries.push(Entry {
                command,
                item,
                shortcut,
                status: Status {
                    enabled: false,
                    checked: false,
                },
            });
            Ok(())
        }
        pub fn document_type(&self) -> DocumentType {
            self.document_type
        }
        pub fn shortcuts(&self) -> Vec<KeyboardShortcut> {
            self.entries
                .iter()
                .filter_map(|entry| entry.shortcut)
                .collect()
        }
        pub fn drain(&self) -> Vec<Command> {
            self.receiver
                .try_iter()
                .filter_map(|event| {
                    self.entries
                        .iter()
                        .find(|entry| entry.command.definition().id == event.id.as_ref())
                        .map(|entry| entry.command)
                })
                .collect()
        }
        pub fn update(
            &mut self,
            keymap: &Keymap,
            statuses: &[(Command, Status)],
        ) -> muda::Result<()> {
            for entry in &mut self.entries {
                let shortcut = keymap
                    .shortcuts(entry.command)
                    .first()
                    .copied()
                    .filter(|s| accelerator(*s).is_some());
                if shortcut != entry.shortcut {
                    entry.item.set_shortcut(shortcut)?;
                    entry.shortcut = shortcut;
                }
                let status = statuses
                    .iter()
                    .find(|(c, _)| *c == entry.command)
                    .unwrap()
                    .1;
                if status != entry.status {
                    entry.item.set_status(status);
                    entry.status = status;
                } else if let Item::Toggle(item) = &entry.item {
                    // Un clic natif coche immédiatement l’item. Réconcilier avec
                    // l’état applicatif même si l’action a été bloquée par un dialogue.
                    if item.is_checked() != status.checked {
                        item.set_checked(status.checked);
                    }
                }
            }
            Ok(())
        }
    }
    impl Drop for NativeMenus {
        fn drop(&mut self) {
            self.menu.remove_for_nsapp();
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::document_types::DocumentType;

    #[test]
    fn menus_use_registered_commands_and_keep_platform_placement_consistent() {
        assert_eq!(
            items(
                Registry::new(DocumentType::Courrier),
                MenuGroup::Edit,
                false
            ),
            vec![
                (Command::Undo, false),
                (Command::Redo, false),
                (Command::Clear, true),
                (Command::Find, true),
                (Command::FindReplace, false),
                (Command::FindNext, false),
                (Command::FindPrevious, false),
                (Command::Palette, true),
                (Command::Preferences, true),
                (Command::ToggleSpelling, true),
                (Command::NextSpelling, false),
                (Command::PreviousSpelling, false),
                (Command::SpellingSuggestions, false),
                (Command::IgnoreSpelling, false),
                (Command::LearnSpelling, false),
                (Command::ResetSpelling, false),
            ]
        );
        assert_eq!(
            items(
                Registry::new(DocumentType::Courrier),
                MenuGroup::View,
                false
            ),
            vec![(Command::Information, false), (Command::ImportModel, true)]
        );
        let file = items(
            Registry::new(DocumentType::Courrier),
            MenuGroup::File,
            false,
        );
        assert!(file.contains(&(Command::File(FileCommand::Export), true)));
        assert!(!file.iter().any(|(c, _)| *c == Command::Information));
        for macos in [false, true] {
            let mut seen = std::collections::HashSet::new();
            for group in GROUPS {
                for (command, _) in items(Registry::new(DocumentType::Courrier), group, macos) {
                    assert!(seen.insert(command.definition().id));
                }
            }
            assert_eq!(seen.contains("application.quit"), !macos);
            assert_eq!(seen.contains("application.preferences"), !macos);
            assert!(seen.contains("document.close"));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_accelerators_preserve_logical_keys_and_registry_overrides() {
        use egui::{Key, KeyboardShortcut, Modifiers};
        use muda::accelerator::{Key as NativeKey, KeyAccelerator, Modifiers as NativeModifiers};
        use native::accelerator;
        let config =
            serde_json::from_str(r#"{"document.export":[{"key":"F6","ctrl":true,"alt":true}]}"#)
                .unwrap();
        let keymap = Keymap::new(Registry::new(DocumentType::Courrier), true, &config).unwrap();
        assert_eq!(
            accelerator(keymap.shortcuts(Command::File(FileCommand::Export))[0]),
            Some(KeyAccelerator::new(
                NativeModifiers::CONTROL | NativeModifiers::ALT,
                NativeKey::Named(muda::accelerator::NamedKey::F6)
            ))
        );
        for (key, character) in [
            (Key::P, "p"),
            (Key::Minus, "-"),
            (Key::Space, " "),
            (Key::Comma, ","),
            (Key::Quote, "'"),
            (Key::Plus, "+"),
        ] {
            assert_eq!(
                accelerator(KeyboardShortcut::new(Modifiers::COMMAND, key)),
                Some(KeyAccelerator::new(
                    NativeModifiers::META,
                    NativeKey::Character(character.into())
                ))
            );
        }
        assert!(accelerator(KeyboardShortcut::new(Modifiers::COMMAND, Key::BrowserBack)).is_none());
    }
}

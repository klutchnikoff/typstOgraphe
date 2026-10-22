//! Présentation et préparation des commandes graphiques, distinctes du catalogue du moteur.
use super::*;

impl UiCommand {
    pub fn icon(self) -> Option<crate::icons::Icon> {
        use crate::icons::Icon;
        Some(match self {
            Self::Bold => Icon::Bold,
            Self::Italic => Icon::Italic,
            Self::Link => Icon::Link,
            Self::InsertImage => Icon::Image,
            Self::Block(kind) => Icon::for_block(kind),
            _ => return None,
        })
    }
    /// Paramètres métier des gestes déjà migrés. Les autres variantes ouvrent
    /// encore leur préparation graphique pendant la migration de phase 2.
    pub fn document_command(self) -> Option<crate::engine::Command> {
        use crate::engine::{Command as Core, Empty, StyleParams, TargetParams};
        Some(match self {
            Self::Bold => Core::Bold(StyleParams::default()),
            Self::Italic => Core::Italic(StyleParams::default()),
            Self::Clear => Core::Clear(TargetParams::default()),
            Self::RemoveLink => Core::RemoveLink(TargetParams::default()),
            Self::Block(kind) => match kind {
                BlockKind::Paragraph => Core::Paragraph(TargetParams::default()),
                BlockKind::Heading(1) => Core::Heading1(TargetParams::default()),
                BlockKind::Heading(2) => Core::Heading2(TargetParams::default()),
                BlockKind::Heading(3) => Core::Heading3(TargetParams::default()),
                BlockKind::Bullet => Core::Bullet(TargetParams::default()),
                BlockKind::Numbered => Core::Numbered(TargetParams::default()),
                BlockKind::Description => Core::Description(TargetParams::default()),
                BlockKind::Quote => Core::Quote(TargetParams::default()),
                _ => return None,
            },
            Self::InsertPostscript => Core::Postscript(Empty {}),
            Self::InsertAttachment => Core::Attachment(Empty {}),
            Self::Undo => Core::Undo(Empty {}),
            Self::Redo => Core::Redo(Empty {}),
            _ => return None,
        })
    }
    pub fn model_source(self) -> Option<String> {
        match self {
            Self::Model(model) => Some(model.source()),
            _ => None,
        }
    }
    pub fn available_without_document(self) -> bool {
        matches!(
            self,
            Self::File(FileCommand::New | FileCommand::Open | FileCommand::Quit)
                | Self::Palette
                | Self::Preferences
                | Self::RecoverDocuments
        )
    }
    pub fn definition(self) -> &'static Presentation {
        if let Self::Model(model) = self {
            return &model.definition().command;
        }
        COMMON_COMMANDS
            .iter()
            .chain(
                DocumentType::ALL
                    .iter()
                    .flat_map(|kind| kind.commands().iter()),
            )
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

pub struct Presentation {
    pub id: &'static str,
    pub label_key: &'static str, // Clé Fluent, distincte de l’identifiant de commande.
    pub command: UiCommand,
    pub scope: Scope,
    pub short_label_key: Option<&'static str>,
    pub(super) defaults: &'static [KeyboardShortcut],
    pub(super) macos: Option<&'static [KeyboardShortcut]>,
}
impl Presentation {
    pub fn icon(&self) -> Option<crate::icons::Icon> {
        self.command.icon()
    }
    pub fn in_palette(&self) -> bool {
        self.command != UiCommand::Palette
    }
    /// Le groupe et le rang définissent aussi les séparateurs (par dizaines).
    pub fn menu_position(&self) -> Option<(MenuGroup, u8)> {
        Some(match self.command {
            UiCommand::File(FileCommand::New) => (MenuGroup::File, 0),
            UiCommand::File(FileCommand::Open) => (MenuGroup::File, 1),
            UiCommand::RecoverDocuments => (MenuGroup::File, 2),
            UiCommand::File(FileCommand::Save) => (MenuGroup::File, 10),
            UiCommand::File(FileCommand::SaveAs) => (MenuGroup::File, 11),
            UiCommand::File(FileCommand::Export) => (MenuGroup::File, 20),
            UiCommand::File(FileCommand::Close) => (MenuGroup::File, 30),
            UiCommand::File(FileCommand::Quit) => (MenuGroup::File, 31),
            UiCommand::Undo => (MenuGroup::Edit, 0),
            UiCommand::Redo => (MenuGroup::Edit, 1),
            UiCommand::Clear => (MenuGroup::Edit, 10),
            UiCommand::Find => (MenuGroup::Edit, 20),
            UiCommand::FindReplace => (MenuGroup::Edit, 21),
            UiCommand::FindNext => (MenuGroup::Edit, 22),
            UiCommand::FindPrevious => (MenuGroup::Edit, 23),
            UiCommand::Palette => (MenuGroup::Edit, 30),
            UiCommand::Preferences => (MenuGroup::Edit, 40),
            UiCommand::ToggleSpelling => (MenuGroup::Edit, 50),
            UiCommand::NextSpelling => (MenuGroup::Edit, 51),
            UiCommand::PreviousSpelling => (MenuGroup::Edit, 52),
            UiCommand::SpellingSuggestions => (MenuGroup::Edit, 53),
            UiCommand::IgnoreSpelling => (MenuGroup::Edit, 54),
            UiCommand::LearnSpelling => (MenuGroup::Edit, 55),
            UiCommand::ResetSpelling => (MenuGroup::Edit, 56),
            UiCommand::Information => (MenuGroup::View, 0),
            UiCommand::InsertImage => (MenuGroup::Insert, 0),
            UiCommand::InsertPostscript => (MenuGroup::Insert, 10),
            UiCommand::InsertAttachment => (MenuGroup::Insert, 11),
            UiCommand::ImportModel => (MenuGroup::View, 10),
            _ => return None,
        })
    }
    pub fn in_model_menu(&self) -> bool {
        matches!(self.command, UiCommand::Model(_))
    }
    pub fn in_heading_menu(&self) -> bool {
        matches!(
            self.command,
            UiCommand::Block(BlockKind::Paragraph | BlockKind::Heading(_))
        )
    }
    pub fn in_list_menu(&self) -> bool {
        matches!(self.command, UiCommand::Block(kind) if kind.is_list())
    }
}

pub(crate) const fn define(
    id: &'static str,
    label_key: &'static str,
    command: UiCommand,
    scope: Scope,
    defaults: &'static [KeyboardShortcut],
) -> Presentation {
    Presentation {
        id,
        label_key,
        command,
        scope,
        defaults,
        macos: None,
        short_label_key: None,
    }
}
pub(crate) const fn short(mut definition: Presentation, label: &'static str) -> Presentation {
    definition.short_label_key = Some(label);
    definition
}

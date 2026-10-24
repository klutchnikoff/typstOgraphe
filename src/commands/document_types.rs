//! Présentation graphique des commandes de types et modèles, côté application.
//! Le cœur fournit les identités et contrats ; ni raccourci ni UiCommand n'y entre.
use super::*;

pub(super) fn type_commands(kind: DocumentType) -> &'static [Presentation] {
    match kind {
        DocumentType::Courrier => LETTER_COMMANDS,
        DocumentType::CompteRendu => &[],
    }
}

const LETTER_COMMANDS: &[Presentation] = &[
    define(
        "letter.insert_postscript",
        "cmd-insert-postscript",
        UiCommand::InsertPostscript,
        Scope::Editor,
        &[],
    ),
    define(
        "letter.insert_attachment",
        "cmd-insert-attachment",
        UiCommand::InsertAttachment,
        Scope::Editor,
        &[],
    ),
];

pub(super) fn model_presentation(model: Model) -> &'static Presentation {
    match model {
        Model::Personal => &PERSONAL_MODEL,
    }
}

const PERSONAL_MODEL: Presentation = short(
    define(
        Model::Personal.id(),
        "cmd-personal-letter",
        UiCommand::Model(Model::Personal),
        Scope::Global,
        &[],
    ),
    "model-personal-letter",
);

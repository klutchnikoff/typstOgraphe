//! Données de coordination d'une vue ; ni commandes documentaires, ni protocole MCP.
use crate::{
    engine::{Effect, Outcome, RenderReference, SelectionTarget, TargetGuard},
    navigation::{PagePoint, SelectionHighlight},
};
use serde::{Deserialize, Serialize};

/// Origine de l'intention, sans identifiant de widget ni notion de focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewOrigin {
    User,
    /// Sélection réussie après localisation inverse, pas le clic brut.
    Preview,
    Agent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevealDestination {
    Editor,
    Preview,
    Both,
}

/// Partie d'un résultat de commande utile à la vue. La planification de `Render`
/// et les valeurs métier restent à l'appelant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewCommandResult {
    pub guard: TargetGuard,
    /// Sélection de CETTE vue après publication, pas le curseur d'un autre acteur.
    pub selection: Option<SelectionTarget>,
    pub changed: bool,
    pub saved: bool,
    pub reveal_selection: bool,
    pub document_replaced: bool,
}

impl From<&Outcome> for ViewCommandResult {
    /// Conversion du parcours local actuel. Un adaptateur multi-acteur devra
    /// fournir la sélection reportée de sa vue, distincte de celle de l'agent.
    fn from(outcome: &Outcome) -> Self {
        Self {
            guard: outcome.guard,
            selection: outcome.selection,
            changed: outcome.changed,
            saved: outcome.effects.contains(&Effect::Saved),
            reveal_selection: outcome.effects.contains(&Effect::RevealSelection),
            document_replaced: outcome.effects.contains(&Effect::DocumentReplaced),
        }
    }
}

/// Faits ordonnés par l'adaptateur. Une préédition IME n'est pas une publication.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum ViewEvent {
    /// Seul événement autorisant une autre identité ; None ferme la vue.
    DocumentReplaced { selection: Option<SelectionTarget> },
    /// Publication native ou modification externe avec position de vue reportée.
    /// N'implique pas à elle seule une demande de révélation.
    DocumentPublished {
        selection: SelectionTarget,
        origin: ViewOrigin,
    },
    CommandCompleted {
        result: ViewCommandResult,
        origin: ViewOrigin,
    },
    /// À révision égale. Agent est refusé : son curseur n'est pas celui de la vue.
    SelectionChanged {
        selection: SelectionTarget,
        origin: ViewOrigin,
    },
    /// Compléments UI (recherche, P.-S./P.J.) ou intention volontaire de navigation.
    RevealRequested {
        guard: TargetGuard,
        destination: RevealDestination,
    },
    /// À envoyer après installation réussie dans le cache du moteur.
    RenderInstalled { render: RenderReference },
    /// Échec courant de compilation ou du service, sans texte d'erreur localisé.
    RenderFailed { guard: TargetGuard },
}

/// Actions à exécuter dans le cycle visuel courant. Une attente de placement
/// reste locale et doit revalider la cible/intention avant son exécution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ViewAction {
    RevealInPreview {
        render: RenderReference,
        target: SelectionTarget,
        point: PagePoint,
    },
    RevealInEditor {
        target: SelectionTarget,
    },
    /// Émis aussi avec des boîtes vides pour effacer une ancienne surbrillance.
    Highlights {
        render: Option<RenderReference>,
        target: Option<SelectionTarget>,
        boxes: Vec<SelectionHighlight>,
    },
}

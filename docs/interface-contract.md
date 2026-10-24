# Contrat général des interfaces

Contrat livré à la clôture du chantier K, le 24 octobre 2026. Il décrit la frontière
commune utilisée par egui, la CLI et MCP, ainsi que la coordination des vues.
Le registre Rust reste la référence des **49 commandes et 19 requêtes** ; ce guide
n'ajoute ni commande, ni transport, ni contexte d'édition au moteur.

## Responsabilités et propriété

| Couche | Responsabilité | État possédé |
| --- | --- | --- |
| `Engine` | Valider et publier les éditions, résoudre les cibles, gérer historique, fichiers et lectures. | Un document publié, sa sélection et son style de frappe actuels ; caches dérivés de recherche, orthographe, rendu et compteurs. |
| `ViewSync`, une instance par vue | Coordonner sélection, révélation et surbrillances, attendre un rendu utilisable et éviter le retour inverse immédiat. | Position logique de la vue, référence/statut du rendu et demandes de navigation. |
| Adaptateur | Traduire gestes/appels, fournir les valeurs explicites et exécuter les actions adaptées à son interface. | Saisie/IME, curseur natif, focus, attente du placement, défilement en pixels, textures, dialogues, préférences et notifications. |
| Services | Exécuter les tâches et réveiller l'hôte par rappel. | Worker et caches Typst, canaux, instantanés d'export, planification locale. |

`Engine` possède sa session sans exposer de référence mutable au document publié.
`Engine::empty()` sert à l'accueil sans document. `from_stored` construit une
identité indépendante depuis des valeurs validées ; ouvrir/remplacer un document
dans une session existante passe par les commandes et leurs contrôles.

Le profil, la date du jour et les choix de dialogue sont résolus par l'hôte puis
transmis explicitement. Les erreurs sont des codes et paramètres structurés ;
leur traduction appartient à `typstographe-i18n` et à l'adaptateur.
L'hôte ordonne les opérations sur son moteur propriétaire. Les workers reçoivent
des instantanés possédés et renvoient des résultats gardés ; ils ne modifient pas
directement une session partagée depuis leur rappel de réveil.

## Appels, lectures et disponibilité

| Besoin | Contrat commun | Rôle de l'adaptateur |
| --- | --- | --- |
| Exécuter | `run(Command)` ou `run_call(Call)` → `Result<Outcome, CommandError>`. | Construire les paramètres réels, traiter succès/erreur, adapter les effets une fois. |
| Lire | `query(Query)` ou `query_call(Call)` → valeur structurée ; méthodes Rust typées pour certaines lectures. | Présenter ou sérialiser le résultat sans ajouter une mutation documentaire. |
| Préparer une action | `available(&Command)` ; `available_intent(intent, target)` avant un dialogue dont les paramètres manquent. | Ajouter les conditions propres à l'interface : modalité, champ actif, service configuré. Revalider au moment de l'exécution. |
| Découvrir | `catalogue`, `commands.list`, `document_types.list`, documentation des types/capacités du moteur. | Réutiliser le registre et les fiches ; ne pas maintenir un second vocabulaire documentaire. |

`Call` contient un identifiant et ses paramètres. Il n'est pas à lui seul une
enveloppe de transport : CLI et MCP ajoutent leurs règles de session, chemins,
autorisations et représentation des références. Les actions d'interface seule
(préférences, palette, focus) restent hors du catalogue documentaire.

Une requête lit le document publié, jamais une préédition. Elle ne change pas
son contenu, sa sélection, son style de frappe, sa révision ou son historique.
Elle peut mettre à jour un cache dérivé : `render.compile`, par exemple, installe
un rendu sans créer d'édition ni écrire un export. Les erreurs de compilation
rapportées en diagnostics sont distinctes d'un refus de paramètres ou de garde.

## Cibles, révisions et transactions

`TargetGuard` associe **identité documentaire et révision**. Une révision égale
dans un autre document ne rend pas une référence valide. Les identifiants de bloc
et de zone sont opaques ; les offsets textuels sont des valeurs scalaires Unicode
locales à une zone, pas des octets Typst ni des positions graphiques.

`SelectionTarget` est une sélection logique orientée avec sa garde.
`selection_target` convertit les indices natifs en cible ; `resolve_target` fait
l'inverse en vérifiant la référence. `RenderReference` ajoute un identifiant de
rendu : une installation ultérieure peut remplacer une référence à révision égale.

`TextTarget::Current` et `All` sont résolus **au moment de l'appel**. Pour conserver
une sélection pendant un dialogue ou un travail différé, capturer une cible
explicite et sa garde. Les commandes qui portent une garde la vérifient ; le
catalogue actuel n'impose pas une garde extérieure à toutes les commandes utilisant
`Current`. La protection stricte des interventions rattachées sera ajoutée en phase 6.
Une référence périmée est refusée sans réancrage approximatif.

Une édition validée est une transaction. Déplacer une sélection, rechercher ou
sauvegarder ne crée pas d'étape d'annulation. La frappe garde ses groupes naturels.
`document.batch` valide sur un brouillon privé, publie une seule transaction et
peut effectuer une sauvegarde atomique finale explicite. Une étape de lot ne peut
ni sauvegarder ni exporter ; les lots n'incluent pas les requêtes. En cas d'échec,
aucune édition partielle du lot n'est publiée.

### Exemple : appel commun, lecture et cible périmée

Exemple autonome, sans fichier ni interface graphique. La disponibilité est une
information préalable ; `run_call` effectue toujours ses propres contrôles.

```rust
use serde_json::json;
use typstographe_core::{
    document_types::DocumentType,
    engine::{Availability, Command, Engine, Query, TextQueryParams, TextTarget},
    storage::StoredDocument,
    view_sync::{ViewEvent, ViewOrigin, ViewSync},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = Engine::from_stored(
        StoredDocument::blank(DocumentType::Courrier), None,
    )?;
    let mut view = ViewSync::default();
    let selection = engine.selection_target(engine.selection())
        .map_err(typstographe_core::engine::CommandError::from)?;
    assert!(view.apply(ViewEvent::DocumentReplaced { selection: Some(selection) }));

    let command = Command::from_call("text.insert", json!({"text": "Bonjour."}))?;
    assert!(matches!(engine.available(&command), Availability::Enabled));
    let outcome = engine.run_call(command.to_call())?;
    assert!(view.apply(ViewEvent::CommandCompleted {
        result: (&outcome).into(), origin: ViewOrigin::User,
    }));
    let text = engine.query_call(Query::Text(TextQueryParams {
        target: TextTarget::All {},
    }).to_call())?;
    assert_eq!(text["text"], "Bonjour.");

    let captured = outcome.selection.expect("sélection après insertion");
    engine.run_call(Command::from_call("text.insert", json!({"text": " Suite."}))?.to_call())?;
    let before = engine.stored_document();
    let error = engine.run_call(Command::from_call("text.insert", json!({
        "target": TextTarget::from(captured), "text": "À refuser",
    }))?.to_call()).unwrap_err();
    assert_eq!(error.reason(), "stale_revision");
    assert_eq!(engine.stored_document(), before);
    Ok(())
}
```

## Résultats et effets

`Outcome` contient `changed`, la garde finale, la sélection logique éventuelle,
les `effects` et une `value` propre à l'opération. `changed` décrit le changement
de révision, pas l'ensemble des conséquences visuelles : un style de frappe ou
une sélection peut changer sans nouvelle révision.
Le statut d'ouverture se lit avec `Engine::is_open()` ou `document.inspect`,
pas en déduisant celui-ci de la présence d'une sélection dans un résultat.

| Effet | Sens | Adaptation |
| --- | --- | --- |
| `Render` | Le document a changé et son rendu doit être actualisé. | Planifier le service selon l'hôte ; aucun délai ou thread n'est imposé par l'effet. |
| `RevealSelection` | Une opération demande de révéler sa sélection finale. | Alimenter la coordination ; ce n'est ni une prise de focus, ni un ordre de défilement en pixels. |
| `DocumentReplaced` | Nouveau document, ouverture, fermeture ou restauration réussis. | Vider les caches/cibles locaux concernés et réinitialiser la coordination une fois. |
| `Saved` | Une sauvegarde explicite a réussi, éventuellement en fin de lot. | Mettre à jour l'état local, nettoyer la récupération concernée ; ne pas sauvegarder une seconde fois. |

Les lots agrègent les effets pour leur état final, sans révélation des étapes
intermédiaires. Les erreurs ne produisent pas de faux résultat réussi.
`ViewCommandResult::from(&outcome)` projette les données utiles au parcours local ;
ce composant ne gère pas les messages, l'export ou le nettoyage de récupération.

## Saisie native et IME

1. `begin_native_edit(selection)` capture un `NativeEdit` opaque et gardé.
2. L'adaptateur travaille sur une projection possédée via `TextInput`, dont les
   opérations protègent les frontières et conservent la première erreur.
3. `commit_native_edit` valide et publie ensemble texte, sélection, révision et
   historique, ou refuse la publication. Il renvoie une sélection éventuellement
   ajustée, **pas un `Outcome`**.
4. Après succès, transmettre `DocumentPublished`, puis `SelectionChanged(User)`
   lorsqu'un déplacement doit demander le suivi. Un Commit IME demande aussi le
   suivi de la sélection publiée dans le parcours actuel.

Une préédition reste dans l'adaptateur, avec son curseur et son défilement. À la
validation, `NativeInput::Composition` crée une transaction discrète ; `Typing`
conserve les groupes de frappe. Après rejet, restaurer le widget depuis l'état
publié. Annulation, perte de focus et filtrage d'un Commit tardif restent natifs.
`NativeEdit` n'est ni sérialisable ni utilisable comme message CLI/MCP.

`set_adapter_selection` synchronise le cache sans annuler un style activé au
curseur. `update_adapter_cursor` reprend aussi le contexte de frappe et termine
le groupe courant. Ces méthodes n'ajoutent pas de révision documentaire.

## Coordination de la vue

Les événements sont des faits **déjà validés**, ordonnés par l'adaptateur.
`ViewSync` n'est pas un second validateur ni une frontière d'autorisation.
`apply(event) -> bool` renvoie `false` pour un fait refusé sans changer son état.
Seul un remplacement explicite peut changer d'identité.
Événements et actions sont sérialisables, comme données d'intégration ; cela
ne les transforme pas en protocole réseau versionné ni en opérations du catalogue.

| Événement | Intention |
| --- | --- |
| `DocumentReplaced` | Réinitialiser pour une sélection publiée ; `None` ferme la vue. |
| `DocumentPublished` | Actualiser la position après publication, sans révélation implicite. |
| `CommandCompleted` | Adapter le résultat final d'une commande ou d'un lot, selon son origine. |
| `SelectionChanged` | Signaler une intention à garde égale ; `Preview` supprime le retour inverse immédiat. |
| `RevealRequested` | Demander explicitement éditeur, aperçu ou les deux, par exemple pour recherche/P.-S./P.J. |
| `RenderInstalled` | Signaler un rendu après installation réussie dans le moteur. |
| `RenderFailed` | Rendre la synchronisation indisponible pour une erreur courante, sans texte localisé. |

`preview_actions` renvoie une mise à jour `Highlights`, même vide pour effacer
les anciennes boîtes, et au plus un `RevealInPreview`. `take_editor_action`
consomme au plus un `RevealInEditor` lorsque l'adaptateur est prêt à le placer.
`preview_target` localise un point sans sélectionner : appliquer `text.select`,
puis signaler seulement le succès avec l'origine `Preview`.

Un rendu périmé ou échoué diffère le suivi. L'installation d'un rendu courant
réarme le parcours local et utilise la sélection du moment. Plusieurs intentions
fusionnent en gardant la dernière sélection, même à révision égale. Sur un rendu
courant, une localisation sans point consomme la demande sans répétition implicite.
Une sauvegarde seule n'ajoute pas de suivi ; une commande réussie sans changement
et sans `Saved` peut en ajouter. Ces règles historiques sont conservées.

Les points et quadrilatères sont en **points Typst**, avec pages à partir de zéro.
L'adaptateur conserve échelle, clip, couleurs, visibilité du rectangle et centrage.
Les actions appartiennent au cycle visuel courant ; une attente de placement doit
revalider cible et intention, y compris si la révision n'a pas changé.
Révéler un passage ne demande pas automatiquement le focus : egui conserve par
exemple celui du champ de recherche. Son rectangle utilise le curseur/galley natifs
de la frame, qui peuvent précéder la publication de la frappe.

## Séquence complète : frappe → rendu → révélation

L'exemple exécute le calcul sur le worker commun et attend sa réponse uniquement
pour être autonome. Une interface graphique utilise un rappel de réveil, traite
le canal sans attente bloquante et installe les textures sur son propre thread.
Le callback ne reçoit ni `Engine` mutable ni type egui.

```rust
use std::{sync::Arc, time::Duration};
use typstographe_core::{
    document::edits::TextInput,
    document_types::DocumentType,
    engine::{Caret, Command, CommandError, Engine, NativeInput, SelectTextParams, Selection},
    rendering::{PreviewRequest, RenderWorker},
    storage::StoredDocument,
    view_sync::{ViewAction, ViewEvent, ViewOrigin, ViewSync},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = Engine::from_stored(
        StoredDocument::blank(DocumentType::Courrier), None,
    )?;
    let mut view = ViewSync::default();
    view.apply(ViewEvent::DocumentReplaced {
        selection: Some(engine.selection_target(engine.selection()).map_err(CommandError::from)?),
    });

    let edit = engine.begin_native_edit(engine.selection());
    let guard_before = edit.guard();
    let mut input = TextInput { view: edit.view().clone() };
    let count = input.insert_text("Noël", 0);
    assert_eq!(engine.target_guard(), guard_before); // Le brouillon n'est pas publié.
    let caret = Caret { index: count, ..Default::default() };
    let selection = engine.commit_native_edit(
        edit, input.view, Selection { primary: caret, secondary: caret }, NativeInput::Composition,
    ).map_err(CommandError::from)?;
    let target = engine.selection_target(selection).map_err(CommandError::from)?;
    assert!(view.apply(ViewEvent::DocumentPublished { selection: target, origin: ViewOrigin::User }));
    assert!(view.apply(ViewEvent::SelectionChanged { selection: target, origin: ViewOrigin::User }));

    let worker = RenderWorker::spawn(Arc::new(|| { /* réveiller l'hôte */ }));
    worker.sender.send(PreviewRequest {
        guard: engine.target_guard(), request: engine.preview_request()?,
    })?;
    let response = worker.receiver.recv_timeout(Duration::from_secs(30))?;
    let rendered = response.result?;
    let info = engine.install_render(response.guard, rendered.sync)?; // Refuse une garde périmée.
    assert!(view.apply(ViewEvent::RenderInstalled { render: info.reference }));
    let point = view.preview_actions(&engine).into_iter().find_map(|action| match action {
        ViewAction::RevealInPreview { point, .. } => Some(point),
        _ => None,
    }).expect("passage éditable dans l'aperçu");
    assert!(!view.preview_pending());

    // Simuler une navigation inverse après un double-clic sur ce texte.
    let target = view.preview_target(&engine, point).expect("cible du corps");
    let outcome = engine.run(Command::SelectText(SelectTextParams {
        target: target.into(), part: None,
    }))?;
    assert!(view.apply(ViewEvent::CommandCompleted {
        result: (&outcome).into(), origin: ViewOrigin::Preview,
    }));
    assert!(!view.preview_pending()); // Pas de boucle source → aperçu immédiate.
    assert!(matches!(view.take_editor_action(&engine), Some(ViewAction::RevealInEditor { .. })));
    assert!(view.take_editor_action(&engine).is_none());
    Ok(())
}
```

L'hôte conserve la garde avec chaque demande asynchrone et filtre les réponses
tardives avant d'installer données ou textures ; `install_render` revalide aussi.
Un échec courant produit `RenderFailed`. Le délai de l'aperçu interactif actuel
(220 ms), la coalescence et les réveils ne font pas partie de `ViewSync`.
Pour une lecture sans interface, `compile` fournit directement un rendu gardé,
puis `locate`, `selection_boxes` et `render_page_png` réutilisent ce cache.

## Fichiers, services et transports

Le verrou d'écriture et la vérification de l'empreinte à la sauvegarde appartiennent
au moteur. L'hôte gère le sélecteur de fichier, la confirmation d'abandon et
l'offre de lecture seule. Sauvegarde et export restent explicites ; un export
préparé avec `prepare_export` possède son instantané et n'exporte pas les frappes
ultérieures. La récupération utilise les références du moteur et un service
planifié/configuré par l'hôte.

La CLI autonome n'a pas besoin de coordinateur de vue. MCP stdio possède aujourd'hui
sa session indépendante ; il n'accède pas au moteur d'une fenêtre par le chemin
du fichier. Une interface web pourra réutiliser ce contrat, avec cœur côté client
ou serveur à décider ; K ne choisit ni WASM ni protocole de placement/édition web.

## Limite actuelle et phase 6

`Engine` possède encore **une seule sélection et un seul style de frappe**.
Deux `ViewSync` permettent une navigation indépendante, sans créer deux contextes
d'édition. L'origine `Agent` prépare la coordination, pas le rattachement : la
position fournie doit être celle de la vue utilisateur déjà reportée, pas le
curseur de l'agent. La conversion directe depuis `Outcome` convient au parcours
local ; elle ne réalise aucun report de position.

Les choix validés de phase 6 restent à implémenter : contextes séparés, concurrence
stricte, report des positions, maintien du focus et de la navigation, indication
« Voir », refus temporaire des mutations pendant IME/modalité et filtrage des
résultats d'une demande annulée. Transport, rattachement et décisions différées
restent à préciser ensemble. Une donnée sérialisable de vue n'autorise pas à
contourner ces contrôles.

## Références et vérification

Le guide est inclus dans la documentation du crate : ses deux exemples Rust sont
compilés et exécutés par `cargo test --locked -p typstographe-core --doc`, déjà
couvert par `cargo test --locked --workspace` dans la CI des trois systèmes.
`cargo check --locked -p typstographe-core` et le contrôle des dépendances
confirment séparément la frontière sans egui/eframe, Fluent, Tokio ni MCP.

Pour le détail :

- [Registre et schémas des opérations](commands.md), [schémas JSON](commands.json).
- [Frontière Rust et saisie native](engine-adapters.md).
- [Lectures, découverte et caches](engine-queries.md).
- [Événements et actions de `ViewSync`](view-sync.md).
- [Branchement egui, parcours et captures identiques](view-sync-egui.md).
- [CLI](cli.md) et [MCP](mcp.md), pour leurs enveloppes et politiques propres.
- [Décisions et suite de phase 6](../ROADMAP-ARCHITECTURE.md#phase-6--assistance-à-la-composition-dans-lapplication-d14).

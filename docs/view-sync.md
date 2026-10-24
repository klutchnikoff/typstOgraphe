# K.2 — Coordinateur de vue

Le [contrat général des interfaces](interface-contract.md) relie ce composant
aux commandes, lectures et services. Cette page détaille ses événements et actions.

`typstographe_core::view_sync::ViewSync` est un composant par vue, distinct
d'`Engine`. Il reprend les règles relevées en [K.1](interface-view-sync.md) et
réutilise les lectures géométriques du moteur. K.2 livre le composant et ses
tests ; le [branchement egui et sa vérification](view-sync-egui.md) sont livrés en K.3.

## Frontière

Le coordinateur reçoit des faits ordonnés, conserve des intentions de navigation
et renvoie des données. Il ne compile pas, ne lance aucun worker et ne possède
ni horloge, ni délai, ni préférence, ni focus, ni IME, ni identifiant de widget.
La temporisation de 220 ms, les réveils et la coalescence restent dans les services
existants. Les coordonnées de sortie sont des points Typst, jamais des pixels.

`preview_actions`, `preview_target` et `take_editor_action` empruntent `&Engine` :
ils réutilisent `locate`, `selection_boxes` et les contrôles de cibles sans changer
le texte, la sélection moteur, le style de frappe ou l'historique. Les mêmes
méthodes emploient la sélection explicite de leur vue, même si le moteur possède
une autre sélection courante.

Les événements et actions sont sérialisables avec Serde. Ce sont des données
d'intégration, sans ajout au catalogue des 49 commandes et 19 requêtes, ni nouveau
transport CLI/MCP. Le composant ne constitue pas une frontière de confiance : les
mutations doivent déjà avoir été validées par le moteur.

## Entrées

| Événement | Sens et garde |
| --- | --- |
| `DocumentReplaced` | Ouvre la vue sur une nouvelle sélection ou la ferme avec `None`. Efface rendu/erreur et réinitialise les demandes. Seule cette substitution explicite peut changer d'identité documentaire. |
| `DocumentPublished` | Met à jour la sélection de la vue après publication native ou modification externe ; refuse identité étrangère et révision antérieure. Ne demande pas à lui seul de révélation. |
| `CommandCompleted` | Adapte le résultat d'une commande ou d'un lot réussi, y compris `DocumentReplaced`. `ViewCommandResult::from(&outcome)` projette les données utiles au parcours local ; valeurs métier et planification de `Render` restent à l'appelant. |
| `SelectionChanged` | Signale une intention à garde égale. Origine `User` : suivi avant ; `Preview` : révélation éditeur et annulation du retour immédiat. Origine `Agent` refusée : son curseur n'est pas la sélection de cette vue utilisateur. |
| `RevealRequested` | Demande explicitement de révéler la sélection actuelle dans l'éditeur, l'aperçu ou les deux ; garde égale obligatoire. Permet les compléments recherche/P.-S./P.J. sans inventer un effet documentaire. |
| `RenderInstalled` | Enregistre la référence après installation réussie dans le moteur ; garde complète égale, erreur effacée, suivi réarmé pour le parcours utilisateur actuel. |
| `RenderFailed` | Rend la synchronisation indisponible pour la garde courante, sans message localisé ni modification des textures. Une erreur périmée est ignorée. |

`apply` renvoie `false` pour un fait incohérent, périmé ou étranger et conserve
alors tout son état. Les événements ne sont pas réordonnés et ne constituent pas
une file réseau : l'adaptateur les transmet dans l'ordre où il valide/applique
les opérations. Plusieurs intentions à révision égale conservent la dernière
sélection. Une nouvelle révision ne suffit pas à décrire cette chronologie.

## Sorties et consommation

| Méthode | Résultat | Consommation |
| --- | --- | --- |
| `preview_actions(&engine)` | Toujours une mise à jour `Highlights`, éventuellement vide ; au plus un `RevealInPreview` avec référence, cible et point. | Le suivi reste en attente si le rendu est périmé/échoué. Sur un rendu courant, il est consommé même si la localisation ne trouve aucun point. |
| `preview_target(&engine, point)` | Cible logique gardée éventuelle, sans mutation ni nouvelle intention. | Aucune consommation. Une décoration, un point invalide ou un rendu indisponible ne sélectionne rien et n'annule pas le suivi. |
| `take_editor_action(&engine)` | Au plus un `RevealInEditor` avec la sélection actuelle de la vue. | Consomme une demande lorsque le document et la cible sont valides ; l'appelant choisit le moment où le placement peut la réaliser. Aucun focus n'est demandé par cette donnée. |

Les actions appartiennent au cycle visuel courant. Un adaptateur qui attend un
placement ne doit pas rejouer aveuglément une ancienne action après une nouvelle
intention, y compris à révision égale ; l'attente et sa revalidation restent locales.

La validité d'un rendu combine la garde complète de la vue, celle de l'Engine,
la référence installée et le statut d'erreur. Les lectures du moteur contrôlent
aussi l'identifiant de rendu et les positions. Aucune correspondance n'est déduite
d'une simple égalité de révision entre deux documents.

## Traduction du parcours egui livrée en K.3

| Site egui | Adaptation |
| --- | --- |
| Initialisation sans document | `ViewSync::default()` ; aucune action géométrique tant que la vue est fermée. |
| `execute_document_command` | Un `CommandCompleted` après succès, avec origine `User` dans le parcours local. La conversion traite les effets de remplacement ; éviter une seconde réinitialisation du coordinateur pour le même résultat. |
| Publication native | `DocumentPublished` après validation ; `SelectionChanged(User)` si le curseur a changé. La préédition IME reste locale. |
| Commit IME | Après tentative de validation, demande explicite de suivi de la sélection publiée, comme le drapeau actuel ; aucune prédiction n'est transmise. Un rejet ne doit pas publier le brouillon. |
| Recherche cliquée, P.-S./P.J. | Après commande réussie, `RevealRequested(Editor)` pour conserver les compléments actuels. |
| Installation/échec de rendu | `RenderInstalled` seulement après `Engine::install_render` réussi ; `RenderFailed` pour une erreur courante. Textures, durée et message restent egui. |
| Dessin de l'aperçu | Consommer `preview_actions`, peindre les boîtes et appliquer le défilement seulement si le petit rectangle cible n'est pas déjà visible. |
| Double-clic aperçu | `preview_target`, puis commande `text.select` ; seulement après succès, `CommandCompleted(Preview)` ou signalement de sélection `Preview`. Ne pas appliquer deux fois la commande. |
| Retour de l'éditeur | Consommer `take_editor_action` lorsque le placement est prêt ; conserver le focus recherche, les conversions de curseur et `force_selection` dans egui. |
| Erreur native ou fermeture de dialogue | Restaurer le widget/focus local ; aucun événement documentaire fictif. |

Préserver l'ordre observable : aperçu traité avant l'éditeur, demande inverse
consommable dans la même frame, publication native et nouveaux suivis visibles
dans la suite du cycle. K.3 vérifie les parcours graphiques et les captures ;
ces tests restent nécessaires même si le coordinateur passe ses tests purs.
Le [bilan du branchement](view-sync-egui.md) précise le placement du curseur natif
avant publication et les huit comparaisons de captures réussies.

Deux particularités sont volontairement conservées : une commande réussie sans
changement et sans `Saved` demande le suivi avant ; une sauvegarde seule n'en
ajoute pas et n'efface pas une demande existante. Une installation de rendu dans
le parcours local réarme aussi le suivi après une navigation inverse antérieure.

## Préparation de la phase 6

L'origine est conservée sans introduire de canal d'agent. Une publication marquée
`Agent` reçoit la position **de la vue utilisateur déjà reportée**, ne demande ni
révélation ni suivi supplémentaire et ne réarme pas ce dernier à l'installation
de son rendu. Une demande volontaire de l'utilisateur déjà en attente demeure.

La conversion directe depuis `Outcome` est celle du parcours local. Dans une
session partagée, l'adaptateur devra substituer la sélection reportée de la vue à
la sélection de l'agent avant de transmettre le résultat. K.2 ne calcule pas ce
report, ne crée pas les contextes d'édition et ne gère pas les interruptions,
notifications, annulations ou le bouton « Voir ». Ces travaux restent en phase 6
(D32/D33) ; le branchement de K.3 conserve le parcours utilisateur existant.

## Vérifications

Les [tests de transitions](../crates/core/src/view_sync/tests/transitions.rs)
contrôlent commandes sans changement, sauvegardes, publications natives, origines,
effets et demandes explicites, dernière intention à révision égale, rejet des faits
périmés/étrangers, fermeture/remplacement et sérialisation stricte.

Les [tests de géométrie](../crates/core/src/view_sync/tests/geometry.rs) compilent
réellement du Typst sans egui et vérifient points et surbrillances, consommation
unique, rendu périmé/échoué, absence de position éditable, double sens, indépendance
des vues et absence de mutation du document, du style, de la sélection et de
l'historique du moteur. Le contrôle des dépendances garde le cœur sans bibliothèque
graphique, Fluent, Tokio ou MCP.

Validation du lot : `cargo test --locked --workspace` (451 tests, dont 14 nouveaux),
`cargo clippy --locked --workspace --all-targets -- -D warnings`,
`cargo fmt --all --check`, `python3 scripts/check_core_dependencies.py` et
`scripts/check-file-length.sh --root crates/core/src`. Journaux locaux des tests
et de Clippy dans `output/k2-view-sync/`.

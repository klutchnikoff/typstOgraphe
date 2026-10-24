# K.3 — Branchement egui et vérification des parcours

Le branchement de `ViewSync` est livré sans modification volontaire du comportement
graphique. Le [composant et son API](view-sync.md) restent dans le cœur ; aucun
canal d'agent ni contexte d'édition supplémentaire n'est ajouté ici.

## Adaptation livrée

`PreviewState::sync` remplace `forward_pending` et `editor_scroll_pending`. Les
conditions de suivi, validité géométrique, attente du rendu et annulation du retour
inverse appartiennent désormais au composant commun. Il ne reste aucun de ces
deux anciens indicateurs dans le code de l'application, tests compris.

| Chemin | Adaptation |
| --- | --- |
| Commande/lot | `CommandCompleted` projette `Outcome` une seule fois. Le remplacement réinitialise les caches locaux ; son effet réinitialise la coordination. Le chargement direct des fixtures de démonstration envoie explicitement `DocumentReplaced`. |
| Saisie native | `accept_edit` transmet la sélection après publication validée. Le déplacement du curseur ajoute `SelectionChanged(User)` ; une édition refusée conserve le texte publié et restaure le curseur natif. |
| IME | Aucune prédiction dans le coordinateur. Le Commit valide le document, puis demande le suivi de la sélection publiée ; l'annulation et le filtrage des événements tardifs restent locaux. |
| Recherche et P.-S./P.J. | `RevealRequested(Editor)` conserve leurs demandes supplémentaires, distinctes d'un effet documentaire. |
| Rendu | Une installation réussie dans le moteur produit `RenderInstalled`. Une erreur affichée ou l'arrêt du service produit `RenderFailed`. Textures, dimensions, durée et message restent dans egui. |
| Aperçu | `preview_actions` fournit boîtes et point éventuel ; egui convertit l'échelle, dessine et ne défile que si le rectangle cible n'est pas déjà visible. |
| Double-clic | `preview_target` localise sans mutation ; après sélection réussie, `SelectionChanged(Preview)` demande la révélation éditeur et supprime le retour immédiat. Un clic simple ne navigue pas. |
| Révélation éditeur | `take_editor_action` consomme la demande lorsque le `TextEdit` est placé. egui conserve le focus de recherche et utilise le curseur natif de cette frame pour le rectangle à révéler. |

[view_coordination.rs](../crates/app/src/app/view_coordination.rs) convertit les
curseurs et signale les faits. L'actualisation silencieuse de la position avant
le rendu, avant l'aperçu et en fin de frame n'ajoute pas de demande de navigation.
Les règles communes ne sont pas recalculées dans cet adaptateur.

L'ordre de la frame reste inchangé : aperçu avant éditeur, révélation inverse
consommable dans la même frame, publication native ensuite. Un point particulier
est conservé : `TextEdit` peut déjà avoir déplacé son curseur dans son tampon
avant la publication de la frappe. La demande commune utilise une cible publiée ;
le calcul local du rectangle utilise le galley et le curseur natifs courants.
Il ne faut pas remplacer ce rectangle par celui de l'ancien curseur du moteur.
Le test dédié vérifie le défilement d'un document long dans ce cas.

La temporisation interactive de 220 ms, le worker Typst, le statut, les réveils,
le focus, les fenêtres, la préédition et les textures restent dans leurs
adaptateurs/services existants. Le cœur et le catalogue CLI/MCP ne changent pas.

## Parcours automatisés

Les 139 tests existants de l'application passent après le branchement. Leurs
montages signalent maintenant les identités neuves et les rendus installés ; les
tests de péremption utilisent une vraie édition plutôt qu'une référence altérée.
Leurs contrôles couvrent notamment :

- double-clic sur une page redimensionnée, retour du focus et saisie au bon endroit ;
- défilements des deux panneaux, refus d'un rendu périmé/échoué, absence de boucle ;
- surbrillance de la sélection et des résultats de recherche, sélection unique ;
- Tab/Shift-Tab, annotations, images, descriptions et P.-S./P.J. ;
- navigation orthographique, métadonnées, annulation/rétablissement ;
- sauvegarde sans suivi ajouté, lot modifié et sauvegardé, remplacement et récupération ;
- préédition IME inter-frames, abandon, Commit tardif et publication atomique.

Trois [tests du branchement](../crates/app/src/app/tests/view_coordination.rs)
complètent cette couverture :

1. La préédition ne change pas `ViewSync` ; après Commit, un ancien rendu reçu
   par le canal du worker est ignoré et seul le rendu courant consomme le suivi.
2. Une révélation en attente pendant une saisie au curseur natif révèle le bon
   passage, même avant publication de ce curseur au moteur. Le test laisse egui
   appliquer la cible de défilement à la frame suivante et vérifie l'offset réel.
3. Révéler un résultat de recherche conserve le focus du champ : la frappe suivante
   remplace la requête sélectionnée sans modifier le corps ni son historique.

## Captures natives

Les références existantes ont été préservées. Deux comparaisons avec
`scripts/capture-baseline.sh --compare --binary target/debug/typstographe --output output/baseline`
ont réussi :

- avant branchement, code `e42a21d`, résultats dans `output/baseline/run-7190jqqf/` ;
- après branchement, code de production `4eb5ec1`, résultats dans
  `output/baseline/run-y_41vvqy/`.

Les huit PNG de chaque série sont strictement identiques aux références (SHA-256
complets comparés, mêmes dimensions 2480 × 1800). Environnement : Darwin 25.6.0,
arm64, profil déterministe `native-light-fr-v1`. Contrôle visuel complémentaire
des vues de rédaction et de recherche effectué.

| Scénario | Préfixe SHA-256 identique avant/après/référence |
| --- | --- |
| Accueil | `7b17c863cf94a0ac` |
| Rédaction | `4ffb11db23b2e43c` |
| Création | `8a59dc545ecdba5b` |
| Recherche | `943856c64e2a9b9f` |
| Orthographe | `5c26ca72ee0db6dd` |
| Profil | `b748b1a33f6b9279` |
| Courrier issu du profil | `60ab220c2ceb4f96` |
| Récupération | `f77fbf2d2b3be381` |

Les journaux sont conservés dans `output/k3-view-sync/` ; les fichiers de capture
restent locaux et ignorés par Git. Cette comparaison native est faite sur macOS ;
les tests egui sans fenêtre restent exécutables par la CI des trois systèmes.

## Vérifications du lot

- `cargo test --locked --workspace` : 454 tests réussis, aucun échec ni test ignoré.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` : réussi.
- `cargo fmt --all --check` : réussi.
- `python3 scripts/check_core_dependencies.py` : aucune dépendance interdite.
- `scripts/check-file-length.sh --root crates/app/src/app` : limites respectées.
- Captures natives : huit comparaisons exactes avant et après le branchement.

Les fichiers `workspace-tests.log`, `clippy.log`, `coordination-tests.log`,
`before-captures.log` et `after-captures.log` sont dans `output/k3-view-sync/`.

## Suite

Le [contrat général des interfaces](interface-contract.md), livré en K.4, réunit
appels, lectures, effets et coordination. Les contextes d'édition séparés, le
report des positions et les interactions d'agent validées en D32/D33 restent
des travaux de phase 6.

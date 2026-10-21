# Phase 1 — Rangement de l’application et décision sur l’éditeur

Les tâches 1.1 à 1.5 sont réalisées. Le prototype 1.6 est disponible et l’orientation
vers des blocs/cellules explicites en mémoire a été validée ensemble le 22 octobre
2026. Le dernier essai de saisie du système reste ouvert avant de clore la phase 1.
L’inventaire de phase 2 précédera le gel des signatures du moteur.

## Application

Les 93 méthodes initiales ont d’abord été déplacées sans modification de logique,
dans un commit par responsabilité. Les tests ont été répartis dans
`src/app/tests/` ; leurs assistants et leurs amorces communes sont dans `support.rs`.
Les assertions des 143 tests existants sont conservées.

Les fonctions longues ont ensuite été découpées dans des commits distincts des
déplacements : raccourcis par comportement, une fonction par dialogue, rangées de
recherche et de remplacement, préparation du texte, rendu et retour d’éditeur.
Les coordonnées et les identifiants egui conservent leur rôle et leur ordre d’usage.

Huit structures dans `src/app/state.rs` regroupent maintenant l’état :
`DocumentState`, `FileState`, `SearchState`, `SpellingState`, `RecoveryState`,
`PreviewState`, `DialogState` et `CaptureState`. Chaque regroupement a son commit
et son passage des tests. La construction est dans `setup.rs`, pour que `mod.rs`
reste un court coordinateur. Les préférences persistantes, leurs services, le cache
des textures et les menus natifs restent des champs propres à l’application.

| Vérification locale | Résultat |
| --- | --- |
| `cargo test --locked -q` | 143 tests réussis, aucun ignoré |
| `cargo fmt --check` | Conforme |
| `cargo clippy --locked --all-targets -- -D warnings` | Aucun avertissement |
| `python3 -m unittest discover -s scripts/tests` | 11 tests réussis |
| `scripts/check-file-length.sh --root src/app` | Tous les fichiers sous 800 lignes |
| `scripts/capture-baseline.sh --compare` | Huit captures strictement identiques à la phase 0 |

`app/mod.rs` fait 341 lignes physiques ; le plus grand module est `text_input.rs`
(506 lignes). Les captures ont été comparées une fois après les déplacements
(`output/baseline/run-n_a5mndu/`), puis après les réorganisations
(`output/baseline/run-5o21qaps/`), sur le même environnement que la phase 0.

La [CI Rust sur `b829f61`](https://github.com/klutchnikoff/typstOgraphe/actions/runs/39691516869)
est verte sur macOS, Windows et Linux. La
[CI Coverage correspondante](https://github.com/klutchnikoff/typstOgraphe/actions/runs/39691516825)
est également verte. Ces exécutions couvrent le rangement de l’application, avant
l’ajout du diagnostic isolé et de sa tâche de CI dédiée.

## Seuils et garde-fous

`clippy.toml` fixe le seuil à 120 lignes. Le lint est activé pour les modules de
production d’`app/` ; seuls les scénarios de tests sont exemptés, afin de conserver
leurs assertions regroupées. Il est vérifié par la commande Clippy habituelle avec
`-D warnings`. Son activation globale reste prévue en phase 7 : aucun masquage
nouveau n’est ajouté aux fonctions de production des autres modules.

`scripts/check-file-length.sh` échoue au-delà de 800 lignes de production, hors
fichiers et éléments `cfg(test)`. La CI contrôle `app/` de façon bloquante et le reste
de `src/` en avertissement. Les deux dépassements restants sont `document.rs`
(1 376 lignes de production selon le script) et `editor.rs` (1 044), dont le
découpage est déjà prévu en 3.8. Six tests du compteur évitent notamment qu’un
assistant de test ou une variante d’énumération masque la suite d’un fichier.

## Prototype 1.6 et choix validé

Voir [le diagnostic du modèle d’édition](../experiments/editor-model/README.md),
son périmètre, ses six essais ciblés et ses commandes de lancement. Il réutilise
le vrai tampon de l’éditeur, sans dépendance supplémentaire et sans modifier
l’application. Une capture native a été inspectée dans
`output/architecture-phase-1/editor-model-spike.png`.

Les essais montrent qu’une grille simple et la saisie IME sont possibles, mais
qu’une mutation native peut enlever une frontière de cellule et que les règles
de normalisation des paragraphes ne conviennent pas à toutes les structures.
Cela justifie une structure explicite ou des frontières protégées, pas l’abandon
d’egui ni de Typst comme source de référence.

**Choix validé le 22 octobre 2026 :** des blocs et cellules explicites en mémoire, avec la
projection actuelle réutilisée pour les zones de texte. L’alternative du tampon
unique protégé a été examinée. Les paramètres des commandes ne sont pas figés ;
les conséquences du choix figurent désormais en entrée de l’inventaire 2.1. Les touches mortes et l’IME via une
méthode de saisie du système restent à essayer : les événements egui simulés ne
sont pas présentés comme des essais matériels.

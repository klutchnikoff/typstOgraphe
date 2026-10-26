# Phase 0 — Base avant la restructuration

Les décisions D1 à D20 de [ROADMAP-ARCHITECTURE.md](https://github.com/klutchnikoff/typstOgraphe/blob/7b0d6fd3e6ae25e5704ed43ce2f4efefa9c2fa0b/ROADMAP-ARCHITECTURE.md)
sont validées. La phase 0 est terminée ; la phase 1 n’a pas commencé.
Le tag annoté `pre-refactor-architecture` identifie le dépôt prêt à être restructuré.

## Tests et analyse statique

Base initiale : `f0c81a0a057ed952558eaf183cf67c2d420e3a5d`.
Les vérifications locales après les changements de code Rust portent sur
`03f05363a8153f609ac71f0b5cae423bb2456e97` : macOS, Darwin 25.6.0,
arm64, Rust et Cargo 1.98.1 (Homebrew).

| Vérification | Résultat |
| --- | --- |
| `cargo test --locked` | 143 tests réussis, aucun ignoré, 6,66 s hors compilation |
| `cargo fmt --check` | Conforme |
| `cargo clippy --locked --all-targets -- -D warnings` | Aucun avertissement |
| `python3 -m unittest discover -s scripts/tests -v` | 5 tests réussis sur le script de captures |
| CI Rust macOS / Windows / Linux | Verte sur les trois systèmes |

La [CI Rust sur `03f0536`](https://github.com/klutchnikoff/typstOgraphe/actions/runs/39685126935)
exécute formatage, tests et Clippy sur les trois systèmes. Le décompte varie selon
la plateforme : 143 tests sur macOS, 142 sur Linux et Windows (test spécifique macOS).
Ces exécutions de CI ne sont pas des essais manuels de l’interface.

La CI initiale échouait déjà sur Windows : deux tests injectaient une dépendance
image dans un modèle Typst en recherchant une chaîne avec des fins de ligne LF.
La conversion CRLF du checkout empêchait cette injection. Le commit `83bc117`
impose LF pour nos fichiers `.typ`, préserve les octets des paquets tiers et ajoute
une assertion explicite sur l’injection dans les deux tests. Leur fonction reste
de vérifier l’élagage des images inutilisées et la conservation des dépendances.

Les journaux locaux sont conservés dans `output/architecture-phase-0/`
(`final-tests.log`, `final-clippy.log`). Ce dossier est ignoré par Git.

## Couverture

La mesure de référence provient de la
[CI Coverage sur `03f0536`](https://github.com/klutchnikoff/typstOgraphe/actions/runs/39685126985),
sous Linux avec `cargo +nightly llvm-cov --locked --workspace --no-report`.
Les implémentations des tests et les dépendances sont exclues par la configuration
existante ; aucun seuil bloquant n’est introduit.

| Mesure | Couvert / total | Pourcentage |
| --- | --- | --- |
| Lignes | 15 404 / 17 991 | **85,62 %** |
| Fonctions | 1 628 / 1 919 | 84,84 % |
| Régions | 16 582 / 19 618 | 84,52 % |

L’artefact `coverage-report` a été téléchargé dans
`output/architecture-phase-0/coverage-final/` : `summary.json`, rapport HTML et
`lcov.info`. La mesure initiale sur `f0c81a0` était de 85,67 % des lignes
(15 372 / 17 943). La nouvelle base comprend les garde-fous du mode capture.
La couverture n’a pas été mesurée localement : cette installation Homebrew
ne dispose ni de rustup ni de cargo-llvm-cov.

## Captures de référence

Le script [capture-baseline.sh](../scripts/capture-baseline.sh) utilise Python 3
et sa bibliothèque standard, sans paquet supplémentaire. Depuis la phase 4, il
compile les deux binaires avec `cargo build --locked --workspace --bins`. La CLI
prépare le courrier de référence avec `new` et `export --format typst`, à partir
des valeurs explicites de `fixtures/capture-letter-params.json` ; aucun profil
personnel ni ancien diagnostic graphique n’est utilisé. L’application lance
ensuite les huit scénarios :

| Identifiant | Vue |
| --- | --- |
| `welcome` | Accueil sans document |
| `editor` | Courrier autonome : texte enrichi, descriptions, citation, image, P.-S. et P.J. |
| `creation` | Création d’un document |
| `search` | Recherche et occurrences |
| `spelling` | Correction orthographique |
| `profile` | Préférences du profil personnel |
| `profile-letter` | Courrier issu du profil |
| `recovery` | Proposition de récupération après incident |

Les deux passages consécutifs sur le commit
`d99d44afb39852ddbc21e49bd7e99dad1f715c69` (arbre de travail propre) ont produit
**huit images strictement identiques**, dimensions et empreintes SHA-256 comprises.
Toutes mesurent 2 480 × 1 800 pixels, pour une fenêtre de 1 240 × 900 points.
Environnement du manifeste : Darwin 25.6.0, arm64, profil `native-light-fr-v1`.
Les vues du courrier et de la récupération ont aussi été inspectées visuellement.

```sh
# Enregistrer une fois ; refuse d’écraser des références existantes.
scripts/capture-baseline.sh --record

# Refaire les huit vues et les comparer sans toucher aux références.
scripts/capture-baseline.sh --compare

# Autre environnement ou changement visuel décidé : créer un jeu distinct.
scripts/capture-baseline.sh --record --output output/baseline-nouvelle-reference

# Réutiliser des binaires déjà construits (CLI voisine de l’application par défaut).
scripts/capture-baseline.sh --compare --binary target/debug/typstographe --cli-binary target/debug/typstographe-cli
```

Les références et leur manifeste restent dans `output/baseline/reference/`.
Chaque comparaison conserve ses images, journaux et manifeste dans un nouveau
`output/baseline/run-*/`. Les fichiers sont ignorés par Git ; conserver ces dossiers
localement pendant la restructuration. Une autre machine doit enregistrer sa propre
référence : polices, densité d’écran et moteur graphique peuvent différer.
Après un changement volontaire d’interface, utiliser un nouveau dossier de référence
et consigner la décision, sans remplacer silencieusement la base précédente.

Le mode capture fixe le français, le thème clair, la géométrie, les animations,
le clignotement du curseur, les survols et le temps de rendu affiché. Il réutilise
les attentes existantes pour les traitements asynchrones. Ces réglages sont limités
au mode diagnostic ; le fonctionnement normal conserve ses préférences.
Les données du profil et de récupération sont isolées. Le script fabrique un
`.typsto` autonome avec les ressources locales à partir des fixtures versionnées,
et une copie de récupération temporaire datée du 21 octobre 2026 à 12 h UTC.

Les tests Python vérifient qu’une différence d’un pixel, une image manquante et
un environnement différent font échouer la comparaison, que le manifeste de
référence reste intact et que la boucle de captures accepte son répertoire préparé.

## Suite

Commencer par la tâche 1.1 : déplacer les tests de `app.rs` dans `crates/app/src/app/tests/`,
sans changer les comportements. Les limites de taille, les regroupements d’état et
le prototype du modèle d’édition sont détaillés dans la feuille de route technique.

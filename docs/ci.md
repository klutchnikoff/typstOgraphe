# Vérifications et consommation de ressources

La CI courante vise un retour en quelques minutes avec les caches disponibles.
Une compilation à froid peut prendre davantage de temps. Les garanties de
conservation des documents restent vérifiées sur chaque PR.

| Déclencheur | Vérifications |
| --- | --- |
| PR | Ubuntu : formats, architecture, Clippy, tous les tests Rust et web, contrats Rust ↔ Yjs, 7 parcours graphiques critiques dans Chromium et WebKit. Audits si les dépendances ou leur configuration changent. |
| Poussée sur `main` | Tests Rust, Clippy, compilation native et contrats sur Ubuntu, macOS et Windows. Tests web sur Ubuntu seulement. Couverture dans un workflow séparé. |
| Validation complète manuelle | Même chaîne sur les trois systèmes, tous les parcours graphiques, audits et compatibilité WebAssembly. Paquets natifs en option. |
| Lundi à 03:41 UTC | Validation complète uniquement si du code a changé depuis la dernière exécution réussie sur `main`. |
| Tag `v*` | Validation complète, puis construction et inspection des paquets natifs. |

## Une seule chaîne de contrôle

`ci.yml` se déclenche sur les PR et sur les poussées vers `main`, jamais sur une
poussée vers une branche de travail : une PR ouverte ne double donc plus les
exécutions. `desktop.yml` contient les étapes communes réutilisées par la CI
courante et `exhaustive.yml`, sans déclencheur indépendant. Les nouveaux commits
de code annulent les vérifications courantes devenues obsolètes ; une modification
documentaire ne coupe pas une validation de code ou une couverture encore en cours.
Les validations de tags ne sont pas annulées automatiquement.

Une modification purement documentaire ne démarre pas les compilations, les
audits ou la couverture. Le catalogue généré `docs/commands.json` reste vérifié.
Un historique indisponible déclenche les contrôles par précaution. Les suppressions
et renommages de fichiers de code restent pris en compte. Le même planificateur
sert pour les PR, la couverture et la comparaison hebdomadaire.

Le job **Résultat** termine aussi les PR documentaires et échoue si un contrôle
requis échoue. Si une protection de branche exige un contrôle unique, choisir
ce job ; les anciens noms `check (macos-latest)`, etc. ne figurent plus sur chaque
PR. La configuration des protections du dépôt n'est pas modifiée automatiquement.

## Parcours critiques

Le tag Playwright `@smoke` identifie les tests, sans les dupliquer ni modifier
leur comportement. La sélection actuelle couvre le rendu et la saisie dans les
widgets, la palette dans un champ riche, la sauvegarde/réouverture/export, la
sauvegarde concurrente, l'annulation/rétablissement par participant, les lots de
mise en forme et la conservation d'un brouillon refusé. Les tests Rust de
récupération après incident et de conservation des fichiers sont exécutés en
entier sur chaque PR.

```sh
npm run build:web
cargo build --locked -p typstographe --features recipe-driver
npm run test:browser:smoke
```

Les deux moteurs restent séquentiels et partagent le port/session de recette,
jamais ceux de la rédaction. Aucun screenshot, vidéo ou trace n'est activé.
Les résultats graphiques ne sont téléversés qu'en cas d'échec, pendant 7 jours.
La suite exhaustive reste disponible avec `npm run test:browser`.

## Contrôles occasionnels

Depuis l'onglet Actions, lancer **Validation complète** sur la branche voulue,
ou, après installation de ce workflow sur la branche par défaut :

```sh
gh workflow run exhaustive.yml --ref main
gh workflow run exhaustive.yml --ref main -f packages=true
```

Le contrôle hebdomadaire compare les révisions, sans relancer une suite déjà
validée lorsque seuls des documents ont changé. En cas d'échec, sa référence
reste la dernière exécution réussie : la prochaine échéance vérifiera à nouveau
le code en défaut. Les audits sont donc aussi renouvelés lors de ces validations
avec activité, même sans changement de lockfile. Pour contrôler des avis de
sécurité apparus pendant une période sans activité, lancer la validation manuelle.

Sur un tag, les trois validations doivent réussir avant le packaging : `.deb`
Linux, `.app` macOS conservée dans une archive tar pour ses permissions, et
installateur NSIS Windows. Le packaging utilise le lanceur Tauri habituel avec
les langues générées depuis Fluent, un binaire release sans `recipe-driver`,
et les versions verrouillées. Les paquets sont inspectés puis conservés comme
artefacts pendant 14 jours. Les tests d'archives et d'exports autonomes hors ligne
font partie de la validation Rust commune.

Ces artefacts sont des paquets de test : l'installation interactive, la signature,
la notarisation et la publication d'une version ne sont pas automatisées ici.
Les mesures de latence et de mémoire sur un vrai bureau restent des recettes
manuelles décrites dans [la méthode de mesure](tauri-preview-performance.md).

# Vérifications et consommation de ressources

Les vérifications sont réparties par objectif. Une PR ne lance plus la chaîne
complète qui sera répétée après fusion. Les durées de quelques minutes supposent
les caches disponibles ; une compilation Rust à froid peut rester plus longue.

| Déclencheur | Vérifications |
| --- | --- |
| PR ouverte ou mise à jour | Linux : formats, TypeScript, scripts, architecture statique et contrats de publication. Clippy, frontend natif requis et frontière des dépendances du cœur uniquement si Rust, Cargo ou la configuration CI changent. Pas de tests Rust/web, navigateur, couverture ou installateur. |
| Fusion/poussée sur `main` | Linux seulement : tests Rust du workspace (récupération et conservation comprises), tests web/locales, frontend de distribution et exclusion des diagnostics. Pas de répétition de Clippy ni de navigateur. |
| Label `full-ci` ajouté à une PR | Validation complète explicite sur les trois systèmes. Retirer puis remettre le label pour la relancer après de nouveaux commits ; sa présence ne relance pas la suite à chaque poussée. |
| Lancement manuel « Validation complète » | Trois systèmes, contrats Rust/Yjs, Chromium/WebKit, recettes natives, audits et WASM. Installateurs optionnels. |
| Tag `v*` | Validation complète puis installateurs joints à une release en brouillon, avec les garde-fous de publication. |
| Couverture | Manuelle, ou le lundi à 03:41 UTC si du code a changé depuis la dernière couverture réussie sur `main`. Plus de calcul après chaque fusion. |

Aucune validation exhaustive hebdomadaire n’est lancée automatiquement. Les avis
récents de sécurité sont contrôlés lors d’une validation complète ; en lancer
une explicitement avant une diffusion ou après une période sans activité.

## Organisation des workflows

`ci.yml` déclenche `routine.yml` sur PR et `main`. Le job stable **Résultat**
synthétise ces contrôles, y compris les changements purement documentaires.
`desktop.yml` conserve les contrôles complets et n’est appelé que par
`exhaustive.yml`. Il n’a pas de déclencheur indépendant. Le label `full-ci`
déclenche ce dernier séparément : son résultat complet doit aussi être consulté
avant une fusion pour laquelle il a été demandé.

Les nouvelles révisions de code annulent les vérifications courantes devenues
obsolètes. Les validations de tags ne sont pas annulées automatiquement. Le
planificateur `.github/scripts/ci_plan.py` conserve les suppressions/renommages
et exécute les contrôles en cas d’historique inconnu. Les documents seuls
n’entraînent pas de compilation ; le catalogue et les schémas générés restent
des entrées exécutables. Les protections de branche ne sont pas modifiées :
**Résultat** reste le contrôle requis conseillé pour la CI courante.

Les commandes courantes sont mutualisées dans `scripts/verify.py` :

```sh
npm run verify:quick  # contrôles statiques, sans compilation native
npm run verify:lint   # frontière du cœur et Clippy avec la feature de recette
npm run verify:test   # tests Rust/web et frontend de distribution
npm run verify:push   # contrôles locaux renforcés, contrats et tout Chromium
```

La compilation native courante conserve un seul graphe avec `recipe-driver`.
La compilation distribuée sans cette feature et les deux variantes de Clippy
restent vérifiées dans la suite complète et au packaging.

## Hooks locaux

Installer une fois dans chaque clone :

```sh
npm run hooks:install
```

Cette commande règle `core.hooksPath=.githooks` localement au dépôt. Elle refuse
de remplacer une configuration ou des hooks existants. La configuration Git
peut être partagée par les worktrees ; les scripts restent versionnés avec le
code. Python 3, Node/npm et les outils Rust du projet doivent être disponibles.

**Avant commit :** contrôle des espaces fautifs dans le diff, formats des seuls
blobs indexés Rust/TypeScript/JavaScript/CSS/JSON, syntaxe Python et shell.
Les fichiers partiellement indexés sont vérifiés dans leur état réellement
commité, sans reformater ni indexer la copie de travail. Aucun test ou Clippy
n’est exécuté à cette étape.

**Avant poussée :** chaque commit de tête envoyé est vérifié dans un worktree
éphémère détaché. Les dépendances npm y sont installées depuis son lockfile ; les
caches npm et Cargo sont réutilisés. La copie de travail, ses modifications non
commitées et son index ne sont pas utilisés comme résultat du contrôle. Le hook
exécute les contrôles statiques, Clippy, tests Rust/web, contrats Yjs/Yrs et
parcours Chromium critiques puis ceux des zones modifiées. Si la comparaison
est inconnue, tous les parcours Chromium passent par précaution. Les suppressions
de branches et les poussées purement documentaires n’exécutent pas ces tests.

Le hook est synchrone : la poussée attend la réussite. On peut continuer à éditer
dans la fenêtre de travail pendant ce temps. Les commandes Cargo partagent le
cache de compilation et peuvent attendre son verrou si une compilation est déjà
en cours. Le port navigateur local est **1437**, distinct du développement et
des recettes ordinaires ; `TOG_BROWSER_PORT` peut le remplacer. Éviter plusieurs
vérifications locales simultanées sur ce même port/cache de recette.
Le worktree temporaire est retiré après réussite ou échec normal.

Sur macOS, cela vérifie réellement le Rust et les intégrations de cette
plateforme. Cela ne simule pas Windows/Linux ni les tests natifs qui prennent
le contrôle d’une fenêtre. Ces derniers restent manuels ou dans la validation
complète. Chromium est installé automatiquement si nécessaire ; sur Linux,
installer préalablement les bibliothèques Tauri et les dépendances système
Playwright. Aucune capture, vidéo ou trace n’est activée.

Pour désactiver les hooks locaux : `git config --local --unset core.hooksPath`.
Pour un contournement ponctuel et conscient : `git commit --no-verify` ou
`git push --no-verify`. Les contrôles distants continuent à s’appliquer.

## Validations complètes et publications

Depuis Actions, lancer **Validation complète** sur la branche souhaitée, ou :

```sh
gh workflow run exhaustive.yml --ref main
gh workflow run exhaustive.yml --ref main -f packages=true
gh workflow run coverage.yml --ref main
```

Sur une branche, `packages=true` produit uniquement des artefacts de test. Sur un
tag `vX.Y.Z`, les versions Cargo/npm/Tauri doivent correspondre au tag et au
commit vérifié. Après les validations, les installateurs Linux, macOS universel
et Windows sont inspectés puis joints à une release en brouillon. Une release
publiée et un tag déplacé sont refusés. Le mode `acceptance_only=true` reste
réservé aux recettes natives ; il n’emballe ni ne publie de release.

Les résultats graphiques ne sont conservés qu’en cas d’échec (7 jours), les
rapports de couverture 30 jours et les installateurs 14 jours. La
[procédure de publication](releases.md) décrit les vérifications manuelles,
la signature et les évolutions de distribution. Les mesures sur un vrai bureau
restent décrites dans la [méthode de mesure](tauri-preview-performance.md).

# Documentation de typstOgraphe

Les pages ci-dessous décrivent l’application courante. Un numéro de lot dans un
nom de fichier indique son origine, pas une fonctionnalité encore à développer.
Les feuilles de route ne portent que le travail restant ; les bilans conservent
les décisions, mesures et limites des étapes livrées.

## Démarrer et contribuer

- [Guide de développement](../app/README.md) : prérequis, lancement, constructions
  normales et recettes isolées, commandes de vérification.
- [CLI](cli.md), [MCP autonome](mcp.md), [MCP rattaché à Tauri](mcp-local.md).
- [Exemples](../examples/README.md) et [politique de CI](ci.md).
- [Feuille de route produit](../ROADMAP.md) et [plan technique](../ROADMAP-ARCHITECTURE.md).

## Contrats courants

| Sujet | Référence |
| --- | --- |
| Architecture, adaptateurs et autorité | [Contrat général](interface-contract.md), [contrat structuré](structured-contract.md), [adaptateurs](structured-adapters.md) |
| Commandes et paramètres exécutables | [Catalogue généré](commands.md), [schémas JSON](commands.json), `typstographe-cli commands describe ID` |
| Cibles, texte, structures et contextes | [Commandes structurées](structured-commands.md), [contextes privés](structured-editing-contexts.md), [transformations](a2-transformations-inventory.md) |
| Projet autonome et Versions | [Format CAS 11](a4-project-format.md), [comparateur et validation contextualisée](a4-comparison-contract.md) |
| Conservation et récupération | [Contrat de conservation](a3-conservation.md), [parcours de reprise](a3-recovery-interface.md) |
| Éditeur et interactions | [Zones riches](rich-zones.md), [widgets/bandeaux](editor-widgets.md), [navigation clavier](keyboard-navigation.md), [dialogues](metadata-dialogs.md), [retours/jointures](paragraph-join.md) |
| Types, modèles et bibliographie | [Article](article-contract.md), [citations et bibliographie](article-bibliography.md), [dépendances Typst et wrappers](typst-dependencies.md) |
| Performances et synchronisation | [Mode individuel](individual-mode.md), [rendu et méthode de mesure](tauri-preview-performance.md) |

Le cœur Rust possède l’état confirmé et valide les commandes. Tiptap/Yjs fournit
l’édition immédiate ; Yrs confirme, garde les identités et l’historique par
participant. Le Typst produit est **dérivé** de cet état structuré. Les modèles
restent écrits en Typst. Aucune deuxième source de vérité textuelle ne sert de
solution de repli. L’interface est traduite par Fluent ; la langue du document
reste indépendante. Les noms publics de capacités sont anglais ou latins.

## Bilans et références historiques

- [A.1 — application principale](application-layout.md), [retrait d’egui](egui-retirement-results.md).
- [A.2 — convergence des adaptateurs](a2-closure-results.md), [transformations](a2-transformations-results.md).
- [A.3 — conservation après incident](a3-closure-results.md), avec résultats CI natifs multisystème.
- [A.4 — CAS évalué](a4-snapshot-storage-evaluation.md), [stockage](a4-storage-core.md),
  [adaptateurs](a4-storage-adapters.md), [Versions](a4-versions-interface.md),
  [comparateur et recette](a4-comparison-results.md).
- [A.5 — documentation, isolation et recette finale](a5-closure-results.md).

`architecture-phase-*`, `architecture-chantier-l.md`, les bilans de migration,
`docs/archive/`, les expériences et les maquettes sont des preuves historiques,
à lire avec leur date. Ils ne remplacent pas les contrats ci-dessus. Les anciens
catalogues plats restent dans `docs/archive/`. Les rapports et références visuelles
existants sont conservés ; les recettes actives ne produisent ni captures, ni
vidéos, ni traces graphiques.

## Licences et provenance

Le code de typstOgraphe est sous [MIT](../LICENSE). Les dépendances et ressources
conservent leurs licences : [Inter](../assets/ui/fonts/inter/LICENSE.txt),
[Noto CJK](../crates/core/assets/fonts/noto-cjk/LICENSE),
[Lucide](../assets/ui/icons/lucide/README.md) et paquets Typst inventoriés dans
le [contrat des dépendances](typst-dependencies.md). `Cargo.lock`, le lock npm et
`deny.toml` conservent les versions et la politique de contrôle des licences.
Aucune ressource tierce n’est relicenciée par le fichier MIT du projet.

## Limites suivies

Le [sélecteur de fichiers macOS](native-localization-investigation.md) peut encore
présenter ses boutons en anglais ; ce diagnostic est reporté. Les menus de
l’application et l’interface suivent Fluent. Le contrat des Versions ne garantit
pas un PDF identique entre versions différentes de typstOgraphe. Les relectures
par annotations, ACP, la collaboration réseau et la distribution signée sont des
chantiers distincts, pas des fonctionnalités livrées par le socle A.

# Bilan du prototype Tauri / Tiptap / Yjs

Recette locale du 26 octobre 2026, macOS arm64, branche
`codex/spike-tauri-editor`. Les [décisions](architecture-web-editor-spike.md)
et les [commandes de lancement](../app/README.md) sont séparées
de ce bilan. L'application egui et le cœur Rust de production n'ont pas été modifiés.

## Ce qui fonctionne

Le diagnostic possède deux colonnes, un véritable éditeur ProseMirror et un rendu
Typst composé par `typstographe-core`. Paragraphes, titres, styles, liens, listes,
tableaux simples, images légendées et notes sont des structures éditables.
Les notes et textes alternatifs s'éditent dans un bandeau, avec Entrée/Échap.
Les métadonnées et le modèle de courrier personnel restent fixes.

Deux fenêtres Tauri possèdent chacune leur `Y.Doc` et échangent des mises à jour
binaires via Rust. Une fenêtre déconnectée peut continuer à écrire ; à sa reconnexion,
les deux contributions convergent. Annuler retire le travail du participant sans
retirer celui de l'autre. L'insertion préparée par Rust entre dans ce même circuit.

Rust valide l'instantané structuré et les ressources avant de générer le Typst.
Le texte est encodé comme texte, jamais interprété comme du code fourni par le client.
Les structures non prises en charge restent visibles et corrigeables ; elles ne
remplacent pas le dernier rendu valide. Les demandes de rendu/enregistrement avec
une révision périmée sont refusées.

La sauvegarde atomique expérimentale conserve instantané, Typst, ressources et état
Yjs versionné. La recette ferme réellement le processus et rouvre cette session.
L'export produit PDF, sources, paquets et polices ; son `main.typ` a aussi été
recompilé avec la CLI Typst, en ignorant les polices système et en utilisant celles
du dossier exporté.

## Vérifications et portée

| Vérification | Résultat local | Portée |
| --- | --- | --- |
| Vitest | 6 tests réussis | Deux éditeurs/Y.Doc, initialisation, concurrence, annulation locale, reprise, HTML de presse-papier et retours à la ligne |
| Rust | 7 tests réussis | Validation, refus, sauvegarde, vraie compilation/export et styles des annotations |
| Playwright | 6 parcours réussis | Chromium et WebKit : bandeaux, légende, cellule/Tab, sélection entre paragraphes, collaboration, reprise et export |
| Tauri natif | Recette complète puis reprise réussies | Deux vraies fenêtres macOS et événements Tauri ; éditions pilotées pour la recette |
| Qualité | Build TypeScript/Vite, rustfmt, Clippy avec refus des avertissements, Prettier et contrôle du diff réussis | Projet isolé ; rustfmt du workspace principal également vérifié |
| CI | Workflow dédié ajouté | Builds/tests Rust et JavaScript sur trois OS ; parcours de navigateur sous Linux, résultats à consulter après la poussée |

La saisie Unicode et les textes japonais sont couverts ; les séquences d'un clavier
réel, les touches mortes et une composition IME avec choix dans la liste des candidats
**restent à essayer manuellement dans ce nouveau prototype**. Un test d'éditeur ou
de navigateur ne remplace pas cette vérification. Même distinction pour le
presse-papier du système et les parcours graphiques natifs Windows/Linux.

## Mesures

Les mesures finales de la recette de cent paragraphes sont consignées ci-dessous.
Le temps de saisie va du `keydown` à la prochaine frame après la transaction ;
il exclut les échanges de Playwright. C'est un indicateur local, sans seuil
de performance arbitraire bloquant la CI. La composition Typst tourne séparément.

Document : **100 paragraphes, 26 615 unités UTF-16** selon `getText().length`,
7 pages Typst ; 40 événements de frappe mesurés par moteur. Ce compteur technique
sert à dimensionner le corpus, pas à remplacer le comptage Unicode du produit.

| Indicateur | Chromium 156 | WebKit 27.2 |
| --- | ---: | ---: |
| Frappe → frame, médiane | 11,8 ms | 14 ms |
| Frappe → frame, 95e centile | 16,4 ms | 27 ms |
| Maximum observé | 18,4 ms | 27 ms |
| Composition et conversion PNG des 7 pages, dernière mise à jour | 6 377 ms | 6 552 ms |
| Heap JavaScript indicatif après la recette | 37,8 Mio | Non disponible |

La saisie reste réactive, mais l'attente de l'aperçu complet est nettement visible.
Ces temps incluent la conversion PNG de **toutes** les pages, avec les options par
défaut de `typst-render` (2 pixels par point), dans un build Rust de développement
non optimisé. Ce n'est pas une mesure du seul compilateur Typst. Le heap Chromium
est une estimation de `performance.memory`, hors DOM, images décodées et processus
natifs ; il ne permet pas de comparer la mémoire totale des deux moteurs.
Les JSON détaillés sont conservés dans `experiments/web-editor/test-results/`,
avec les captures ; la CI les publie comme artefact de recette.

La recette native complète (deux vues, fusion, annulation, annotations, sauvegarde
et export) a duré **8,8 s**, puis la recette de reprise et nouvel enregistrement
**1,9 s**. Ce ne sont pas des mesures isolées du démarrage. Le pic RSS échantillonné
du seul processus Rust était de **192,2 Mio**, puis **168,8 Mio** à la reprise.
**Les processus WebKit sont exclus** : ces nombres ne représentent pas la mémoire
totale de l'application. Build de développement, sans profilage release.

Les rapports natifs, sources et PDF sont conservés dans
`output/web-editor/native-1792974382885/`. Les journaux globaux sont
`output/web-editor-native-recipe.log` et `output/web-editor-browser-tests.log`.
Chaque nouvelle recette crée une session distincte ; elle ne modifie pas les
documents `.typsto` de l'utilisateur.

## Relecture de l'interface

| Avant | Après | Pourquoi |
| --- | --- | --- |
| La barre du tableau apparaissait en déplaçant verticalement le texte sous le pointeur | Emplacement réservé, une seule ligne avec défilement horizontal | Stabiliser la cellule ciblée et la saisie dès les premières touches ; petite hauteur réservée même hors tableau |
| Les retours standards de Tiptap créaient immédiatement un paragraphe | Dans le corps libre, retour simple interne et double retour pour un paragraphe | Conserver la règle de rédaction du produit ; titres/listes/cellules gardent leur navigation structurelle |

Références vérifiées visuellement : [vue d'ensemble claire](mockups/web-editor-spike/overview.png)
et [tableau et légende en mode sombre](mockups/web-editor-spike/editor-dark.png).
La présentation reste celle d'un diagnostic : boutons simples et interface française, sans
transposition complète du chantier L ni des menus et préférences de production.

## Limites et coût d'une migration

- **Adaptateur documentaire expérimental.** Il génère le Typst depuis le schéma
  structuré et réutilise le monde, les polices, les paquets, le modèle et l'export
  PDF du cœur. Il ne fait pas passer les nouvelles structures par le projecteur
  textuel de production, qui ignore encore tableaux et notes. Le registre typé,
  les sélections/cibles et les contrats CLI/MCP restent à adapter avant une migration.
- **Source externe.** Aucun import arbitraire de Typst ni aller-retour général
  Typst → schéma web n'est livré. Le format expérimental n'est pas un `.typsto`.
  Les identités CRDT sont persistées ; les piles d'annulation des vues ne le sont pas.
- **Frontière Yjs.** Rust relaie des octets Yjs et valide les instantanés, sans
  décoder lui-même le CRDT. À la reprise, le client compare son document reconstruit
  à l'instantané enregistré. Une autorité serveur sur le CRDT, la résistance à un
  client incohérent et les migrations de schéma nécessiteraient un travail distinct.
- **Collaboration locale.** Relais dans un seul processus, sans authentification,
  réseau distant ni verrou entre plusieurs processus ouverts sur le même dossier
  expérimental. La reprise d'initialisation après disparition du tout premier client
  avant publication de la fixture reste à durcir. Aucun agent ACP/MCP n'est raccordé.
- **Journal et ressources.** Journal borné à 32 Mio, mise à jour individuelle à
  4 Mio, session à 64 Mio. Reconnexion par rejeu, sans compactage continu ; toutes
  les images sont gardées pour les annulations et clients différés. Ce n'est pas
  encore une politique de rétention destinée à de longues sessions de travail.
- **Éditeur/rendu.** Pas de synchronisation avant/arrière ni de surlignage de sélection
  dans l'aperçu. Les listes imbriquées, cellules fusionnées, notes riches et capacités
  hors périmètre sont refusées. Les raccourcis, l'orthographe, l'accessibilité et
  l'internationalisation du produit ne sont pas intégralement portés.
- **Performance.** Instantanés complets et PNG de toutes les pages restent coûteux.
  Le bundle principal est d'environ 601 ko minifié, 188 ko compressé ; Vite émet
  son avertissement informatif de taille. Virtualisation, rendu différentiel,
  mesures mémoire globales et essais de mémoire plus longs restent à prévoir.

## Lecture des résultats

Le prototype montre que l'édition structurée et la collaboration peuvent s'appuyer
sur des composants existants tout en gardant le moteur de composition Rust.
Il mérite un essai ergonomique comparé à egui, surtout pour les tableaux, les
sélections et la saisie IME. **Il ne tranche pas encore une migration.** Celle-ci
demanderait une décision explicite sur l'adaptateur des commandes, le format durable,
la reprise des fichiers existants et le portage des fonctions déjà présentes.

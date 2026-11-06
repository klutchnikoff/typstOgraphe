# Plan technique de développement

Révisé le **6 novembre 2026**. Ce document accompagne la [roadmap produit](ROADMAP.md)
et distingue le socle livré des décisions et travaux techniques restants.
Les réalisations détaillées et les inventaires historiques sont
dans les bilans de docs/ et l’historique Git.

## Cadre à préserver

- Tauri pour l’application, Tiptap/ProseMirror pour l’édition, Yjs pour l’état
  partagé et l’historique des participants, Rust et Typst pour la validation et le rendu.
- Contrats documentaires communs, identifiants publics anglais/latins et catalogue
  de capacités partagé ; l’interface et les agents ne définissent pas leur propre vocabulaire.
- Les modèles produisent la présentation en Typst ; la présentation de l’éditeur
  reste indépendante du type et du modèle.
- Fichiers locaux autonomes et compilation hors ligne, modèles et ressources
  embarqués, écritures atomiques et protection contre les écrasements concurrents.
- Un projet correspond à un .typsto autonome : document, modèle, dépendances,
  ressources et snapshots conservés. L’organisation technique de l’archive
  reste masquée ; le catalogue de projets de l’application ne contient que
  leurs références et les informations nécessaires à leur accès.
- Préférences et profil locaux, distincts du contenu partagé ; les valeurs
  incorporées au document deviennent ses données propres.
- Fluent fournit les langues de l’interface et son repli. Langue documentaire,
  langue d’interface et langue du correcteur sont distinctes.
- Contextes d’édition séparés ; une préédition IME n’est jamais publiée comme
  une modification validée. Les actions distantes préservent le focus humain.
- Propositions MCP isolées, gardées par document/révision, acceptées ou refusées
  par décisions locales dans le comparateur commun d’A.4. Les groupes dépendants
  restent atomiques ; F.3 complète les reçus et le cycle de vie,
  sans second moteur. Sauvegarde et export ont leurs autorisations propres.
- Les nouvelles interfaces sont maquettées et les choix structurants validés
  ensemble ; rechercher les outils maintenus avant de réimplémenter une brique.

Le [contrat structuré](docs/structured-contract.md) et l’[index documentaire](docs/README.md)
sont les références du socle. Aucun maintien artificiel des anciens formats ou
de la projection plate n’est demandé. Les bilans d’A.1–A.4 restent consultables.

## Socle livré — références

A.1–A.5 sont implémentés et recettés localement. Les [contrats courants](docs/README.md)
et le [bilan A.5](docs/a5-closure-results.md) décrivent les constructions normale/de
recette, les gardes et le corpus Courrier/Article entre adaptateurs. La qualification
CI complète du candidat doit encore être vérifiée avant fusion. Les contrôles
matériels, de signature et de distribution restent dans H.

Les contrats du fichier CAS, de récupération et du comparateur restent courants,
même lorsque leur nom conserve le numéro du lot qui les a définis. Le travail
livré ne figure plus comme tâche à faire dans ces roadmaps.

B.1 est clôturé après relecture le 3 novembre 2026 ; voir le
[bilan et les critères vérifiés](docs/b1-closure-results.md). Déclarations YAML,
structures partagées, régimes de modèles, dates et références d’images alimentent
le contrat commun de Courrier, Article et du contrat interne Compte-rendu.
Les codecs et comportements restent intégrés. Les contrats préparatoires de
C.1–C.4 sont consignés ; les nouveaux types et leurs modèles restent à livrer.

## B. Types, paramètres et manifestes — P1

**Catalogue validé le 1er novembre 2026 :**
[groupes et fiches](docs/document-type-catalogue.md), avec un registre de
découverte commun aux adaptateurs. Garder les profils prévus séparés des
contrats exécutables : disponibilité dérivée des modèles, aucune extension
factice de l’énumération des types. Un type peut appartenir à plusieurs groupes.
Un seul type Mémoire et thèse ; la lettre de motivation reste un Courrier.
Valider le contrat de chaque fiche avant son développement, puis fournir un
premier modèle utilisable. La galerie de création réutilise ces registres :
filtres cumulables, tuiles et vignettes de modèles calculées à la demande par
Typst, indépendamment du document ouvert. Garder le contenu inchangé au
changement de modèle.

Préciser le schéma de chaque type, les genres de valeurs, les groupes de champs,
les capacités et les signatures à implémenter. La note
[Types, modèles et libertés](docs/types-modeles-libertes.md) est une proposition,
avec une clarification validée sur les paramètres et l’interchangeabilité des
modèles ; les autres arbitrages ne remplacent pas encore les contrats actuels.

**Décision validée le 26 octobre 2026 :** le type définit les capacités ; tous
ses modèles les implémentent. Les options de présentation ont un régime libre,
restreint ou imposé, déclaré dans le manifeste, y compris pour les options des
capacités. Les contrôles des valeurs imposées sont masqués. Les métadonnées
de contenu ne sont pas remplacées par le modèle. Séparer choix enregistré et
valeur effective : un choix incompatible utilise le défaut avec avertissement,
sans écrasement ; un choix imposé reste temporaire tant que ce modèle est actif.
Vérifier notamment brouillon → revue à style imposé → brouillon : les citations
restent intactes et le choix initial redevient effectif s’il est autorisé.

### Extensions du socle déclaratif — différées

B.1 est clôturé ; les extensions suivantes ne sont pas des critères de sortie
restants. Leur besoin et leur contrat doivent être validés avant développement.

- Types externes, codecs arbitraires, installation et rechargement ; paramètres
  personnalisables des fragments partagés, aujourd’hui réutilisés à l’identique.
  Les identités et comportements restent enregistrés en Rust.
- Paramètres propres aux modèles : étendre les régimes communs avec identités,
  genres et contrôles dans D ; conserver contenu de l’auteur, choix enregistré
  et valeur de rendu distincts.
- Dates ou ressources dans les collections, précision calendaire supplémentaire
  et genres autres que les images : enregistrer contrôles et validateurs avant
  de les autoriser. Les déclarations non prises en charge sont refusées.
- Paquets installables de modèles, versions, provenance, ressources et licences
  de polices : chantier D, avec les contrôles de distribution dans H.

**Suite du chantier B :** compléter les types, leurs fiches et la matrice
capacités/modèles à mesure des besoins. La clôture du socle déclaratif ne livre
pas les types encore annoncés ni leur premier modèle.

## C. Capacités et interactions — P1/P2

Pour chaque capacité retenue dans l’[inventaire](docs/capabilities-inventory.md),
définir une fois son contrat, sa disponibilité, ses cibles, son contenu principal,
ses annotations et ses paramètres. Dériver les contrôles de paramètres réellement
disponibles ; ne pas afficher un panneau vide ou un bouton actif sans action.

Réutiliser le cadre de widget et le panneau contextuel communs : insertion,
frontières, brouillon, focus, navigation, validation, annulation, suppression et
positionnement interligne. Tester dans le corps et dans les structures autorisées.
Les éléments de présentation ne font pas partie du document partagé.

Pour une dépendance Typst, conserver un adaptateur public stable et mettre à jour
l’inventaire des paquets, versions, ressources et licences. Vérifier la compilation
de l’export déplacé dans un environnement sans cache de paquets.

### C.1–C.4 — Ordre et périmètres validés le 3 novembre 2026

**Dépendance satisfaite :** B.1 est clôturé et ses contrats préparatoires sont
consignés. C.1 (renvois) et le premier périmètre de C.2 (mathématiques) sont
implémentés et vérifiés localement, comme C.3 (gestionnaire complet, profils
natifs Hayagriva et **CAS 16**). La CI et la recette utilisateur restent requises
avant fusion. La consolidation frontend / E.1 est également livrée localement ;
le prochain lot est C.4 (bibliothèque personnelle), avant les extensions lourdes
de D, F et J. Préciser ensemble les paramètres,
commandes/cibles et maquettes avant chaque implémentation ;
la livraison de tous les types de B n’est pas un prérequis.

- **C.1 — Cibles et renvois — livré localement.** Le [contrat](docs/c1-cross-references.md)
  décrit les ancres automatiques, la copie et la protection des cibles, les
  composantes libellé/page, les callbacks de présentation Typst et l’extraction
  des libellés pendant la compilation existante. Sélecteur, badges, formulaires
  récupérables et commandes réutilisent les mécanismes communs Tauri/CLI/MCP.
  Le sélecteur vise les titres, images et tableaux du corps ; C.2 l’étend aux
  équations. Les théorèmes reprendront ce contrat. Reste différé le mode explicite « Choisir dans
  l’aperçu », réutilisant la correspondance backward sans sélection arbitraire
  d’une cible voisine. Qualification CI et recette utilisateur avant fusion.
- **C.2 — Mathématiques — livré localement.** Le [contrat](docs/c2-mathematics.md)
  décrit les nœuds `mathInline`/`mathBlock`, leurs zones de code partagées,
  l’adaptateur Typst/MiTeX et les imports dérivés conservés dans les projets.
  Le [benchmark MiTeX natif/WASM](experiments/mitex-comparison/RESULTS.md)
  ne montre pas de gain robuste à chaud ; le chemin WASM est conservé.
  Le [profilage détaillé](experiments/preview-profiling/RESULTS.md) confirme la
  réutilisation des sources et des caches. Les mises à jour différentielles des
  pixels et des canvas sont la priorité proposée. L’[évaluation native RGBA / Reflexo](experiments/preview-renderers/RESULTS.md)
  est terminée sur les trois articles : 18 sessions, pixels RGBA identiques,
  reconstructions complètes et synchronisations vérifiées. Sur 24 pages,
  édition de formule : 270 ms en RGBA complet, 102 ms pour les deux candidats
  différentiels ; premier rendu : 713 ms en RGBA différentiel et 455 ms pour
  Reflexo. La [contre-épreuve VS Code/Tinymist et parcours commun](experiments/tinymist-pipeline/RESULTS.md)
  affine l’arbitrage : sur 24 pages, deux frames après une édition de formule,
  47 ms p50 dans Tinymist avec rendu partiel, 123 ms dans notre candidat RGBA ;
  p95 respectifs 168 et 126 ms. Les premières mutations SVG précèdent parfois
  le post-traitement : ne pas les comparer à notre callback de publication.
  Les 24 sessions vérifient 792 états de source identiques ; six comparaisons
  natives de synchronisation et les reconstructions/erreurs passent.
  [Déduplication de l’interface implémentée et mesurée](experiments/ui-refresh/RESULTS.md) :
  microtâche commune sans attente de frame, disponibilité rafraîchie une fois,
  comptages selon texte/sélection/langue et index de références selon
  génération/valeurs. Sur 24 pages, édition de formule : commande 22 → 7 ms,
  publication RGBA complet 271 → 254 ms, candidat différentiel 102 → 87 ms ;
  p95 complet encore variable, 296 → 307 ms. Les 24 sessions vérifient
  396 états avant/après identiques en source, pixels et navigation ; badges,
  commandes, sélection et synchronisation sont vérifiés aussi dans les tests.
  [Cartes de source et synchronisation optimisées](experiments/source-sync/RESULTS.md) :
  règles d’échappement communes sans chaîne temporaire par caractère, index des
  balises numérotées par parcours syntaxique linéaire dans chaque compilation.
  Aucun cache de plages entre révisions ni coût différé au clic.
  Sur 24 pages : carte 11,93 → 2,01 ms, construction RenderSync hors empreinte
  12,81 → 4,82 ms (mesure dérivée), publication 258 → 244 ms RGBA complet et
  93 → 76 ms candidat différentiel. Les 24 contrôles d’identité du cœur compilé
  et 396 comparaisons de cartes UTF-16/ancres complètes passent, ainsi que les
  pixels, reconstructions et synchronisations. Un pilote au cache Cargo invalide
  est exclu ; reconstruction ciblée et empreinte du cœur font partie du protocole.
  [Latence de saisie continue](experiments/typing-latency/RESULTS.md) : comparaison
  native avec/sans actualisation acquise. Les uploads de pages consécutifs
  bloquent le fil de l’interface ; préparation fractionnée, annulation et
  libération des canvas sont implémentées, avec publication atomique conservée.
  459 tests, 14 parcours navigateur et 24 sessions natives passent ; 1 152
  entrées conservées, états finaux texte/source/pixels/navigation identiques.
  Sur 24 pages à 250 ms/caractère : attente d’entrée p95 25 → 3 ms, prochain
  créneau d’affichage p95 40 → 21 ms, intervalle maximal entre frames 123 → 67 ms.
  Les publications restent actives (44 contre 46). Les pointes à cadence 100 ms
  sans upload restent à isoler : la correction ne garantit pas toutes les frames.
  Garder la fluidité de saisie comme critère pour réévaluer les renderers WebKit.
  Préserver UTF-16, échappements, zones et gardes ; mesurer chaque gain avant
  l’arbitrage final RGBA / SVG partiel, idéalement dans le même hôte WebKit.
  Le contrôle automatisé source→aperçu dans VS Code reste non concluant ;
  les retours par Cmd-clic et les parcours natifs passent. Le premier chargement
  VS Code est exploratoire et non monotone ; aucun classement sur ce point.
  RGBA différentiel et Reflexo restent des candidats évalués ; le format
  Rust/WASM doit être assorti et ses adaptations à Typst officiel, notamment
  les PDF importés, demandent une qualification de production.
  À valider avant intégration : protocole et cache par document/génération,
  options et flux/epoch de vue, contrôle de la base réellement publiée,
  réinitialisation, publication atomique, refus des résultats périmés et cartes
  complètes de synchronisation à chaque révision. Ajouter les garanties
  multi-vues, remplacement de document, erreurs et révisions sans changement
  de pixels. Les candidats restent isolés ; aucune dépendance ni politique
  CSP de l’application n’a changé.
  Un contrat de formule avec source, langage Typst ou
  LaTeX, description accessible et présentation en ligne/en bloc. Les blocs
  peuvent être ciblés par C.1 ; les modèles règlent numérotation et présentation.
  Saisie directe du source en premier, éditeur visuel ensuite, sans conversion
  bidirectionnelle implicite. Aucun aperçu mathématique supplémentaire dans
  l’éditeur : réutiliser le rendu temps réel de la colonne droite.
  MiTeX passe par un adaptateur Typst public stable ;
  inventorier et embarquer sa version et ses dépendances pour les fichiers et
  exports hors ligne. Les diagnostics proviennent du moteur concerné, sans
  liste de formules autorisées propre à typstOgraphe ; conserver les limites
  réelles du support Typst/MiTeX explicites. Le source représente une formule,
  éventuellement complexe ou multiligne, et non un document LaTeX complet.
  **Déclencheurs validés :** `$` seul sur une ligne puis Entrée crée le widget
  Typst ; `$$` seuls puis Entrée créent le widget LaTeX. Dans une ligne de texte,
  `$cos(x)$` est Typst et `$$\cos(x)$$` est LaTeX. Les doubles dollars désignent
  le langage, pas une mise en bloc implicite. Ces conventions appartiennent
  à la saisie ; conserver séparément langage, source et présentation.
  **Différé explicitement :** macros LaTeX et fonctions Typst personnalisées.
  Préciser ultérieurement leur définition, leur portée, leur disponibilité au
  rendu et leur conservation autonome dans le projet et les exports.
  **Ajustement du 5 novembre 2026 :** `language` est un paramètre de création
  stocké comme attribut, exclu des propriétés et des écritures `node.set_attrs`.
  Le champ de label dans l’en-tête réutilise schéma, attribut et commande du
  panneau ; le contrôle générique conserve validation et concurrence, sans
  écraser les autres brouillons. Le texte alternatif rejoint le `alt` natif de
  `math.equation` ; le cœur vérifie les exports PDF/UA-1 des deux langages avec
  et sans description, sans modifier la politique d’export de l’application.
  **Labels validés :** séparer l’ancre stable automatique de l’alias humain
  facultatif des équations en bloc. Le champ affiche un préfixe `eq:` grisé,
  non éditable ; suffixe vide signifie absence d’alias. Une modification du
  suffixe préserve les renvois par identité. Le sélecteur reconnaît `eq:` et
  `$` comme filtres d’équations ; le badge utilise le contenu composé par
  Typst, par exemple `(12)`, sans reconstruire son numéro dans l’adaptateur.
- **C.3 — Notices du projet — livré localement (CAS 16).** Remplacer les multiples fichiers originaux par
  une bibliothèque Hayagriva unique, matérialisée dans le `.typsto` et l’export.
  Réutiliser la crate Hayagriva pour import et sérialisation ; évaluer la
  conversion BibLaTeX (champs, macros, relations, accents et contenu riche),
  signaler les pertes et arbitrer les collisions sans écrasement silencieux.
  Conserver les sources importées intactes en lecture seule comme provenance,
  sans seconde autorité active. Préserver les identités/clés des notices et les
  liens des citations ; remplacer les cibles actuelles `file_id`/`key` par le
  contrat retenu, sans couche de compatibilité artificielle.
  Le gestionnaire utilise une grande modale commune : liste filtrée et recherche
  fuzzy titre/auteurs à gauche, formulaire par genre à droite, création par +.
  Les éditions restent des brouillons récupérables avant validation ; changement
  de notice ou fermeture propose valider, abandonner ou continuer. Valider est
  une édition annulable et ne sauvegarde pas immédiatement le fichier.
  « Créer et citer » valide la notice et l’insertion dans un seul lot annulable.
  La suppression d’une notice citée est protégée. UI, CLI et MCP partagent
  les validations et opérations ; aucun second formulaire pour l’insertion.
  **Contrat du 6 novembre 2026 :** [crate bibliographique](docs/c3-bibliography-contract.md),
  [corpus de qualification](fixtures/bibliography/README.md). Créer
  `typstographe-bibliography`, dépendante de Hayagriva et indépendante du cœur,
  des interfaces, du stockage et de Fluent. `core → bibliography → hayagriva` ;
  les mutations documentaires restent dans le moteur commun. Modèle natif
  intégral, `RecordId` opaque, clé lisible distincte, provenance et profils de
  formulaire déduits des types/parents natifs ; les citations ciblent l'identité. État structuré confirmé
  comme autorité, un seul Hayagriva dérivé actif pour Typst, originaux hors de la
  compilation et conservés avec leurs références documentaires/historiques.
  Descripteurs de genre et diagnostics typés communs : erreurs structurelles,
  complétude non bloquante, pertes de conversion explicites. Hayagriva 0.10.1
  n'exige pas titre/auteur/date par genre ; son parseur exige le type racine.
  Inclure prépublications et logiciels ; qualifier les composites, `@software`
  importé en `misc` et `repository` composé comme CSL logiciel, avec profils
  déduits du type et des parents natifs, sans genre persistant indépendant.
  Plans purs avec empreinte attendue et décisions ciblées avant publication
  atomique ; pas de fusion par clé, score fuzzy ou transitivité des candidats.
  Porter les garanties de Citekeep une seule fois, avec les adaptations explicites
  pour les états de publication et versions de logiciels. Le corpus documentaire
  contient des entrées et assertions ; les chemins CAS 16 sont raccordés.
  Le [premier lot](docs/c3-bibliography-foundation.md) livre la crate pure, les
  24 scénarios d'identité exécutables, ses tests complémentaires et le partage
  de lecture/synthèse/recherche avec le cœur. Le format et les cibles de citations
  utilisés à cette étape ont depuis été remplacés par les identités de notices.
  Le [lot de préparation](docs/c3-bibliography-preparation.md) ajoute les requêtes
  communes `bibliography.genres`, `bibliography.prepare_record` et
  `bibliography.prepare_import`, découvertes et exécutées par CLI/MCP, avec
  schémas de paramètres/résultats et erreurs typées présentées par Fluent.
  Les préparations restent inactives et ne modifient ni révision ni historique ;
  leurs empreintes ne valent pas autorisation de publication.
  La [présentation validée](docs/c3-bibliography-interface.md) utilise une liste
  compacte à deux lignes et les composants communs de dialogues, champs,
  collections et navigation. Le [raccordement documentaire](docs/c3-bibliography-document-state.md)
  livre l’état partagé par notice/source, `bibliography.apply`, les gardes, les
  citations par identité et le CAS, désormais en version 16, avec contrôle de la projection Hayagriva.
  Le [gestionnaire et ses codecs](docs/c3-bibliography-manager.md) sont raccordés :
  champs natifs préservés, décisions d’import explicites, formulaires privés
  récupérables, création/citation atomique et garde de notice concurrente. Les
  requêtes communes projettent et préparent les formulaires, parcourent toute
  la bibliothèque, localisent les citations et revoient les imports sans
  publication. Les profils sont déduits des types/parents natifs ; les
  transitions conservent champs et conteneurs inactifs, avec remplacement de
  notice gardé, annulation, récupération et projection autonome CAS 16.
  **Classification native :** [décision C.3](docs/c3-native-bibliography-profiles.md).
  Retirer le genre persistant ; déduire les profils du type/parents Hayagriva.
  Mutualiser transitions natives, conservation des conteneurs inactifs et
  publication gardée dans la crate commune ; qualifier CAS, reprise et rendu.
- **C.4 — Bibliothèque personnelle.** Recherche locale au projet avant recherche
  personnelle ; copie d’une notice lors de son utilisation, liens de provenance
  conservés et échanges de corrections explicites. Les deux stockages ont leurs
  révisions et protections contre l’écrasement ; aucune transaction atomique entre
  eux n’est présumée. Aucune modification personnelle ne réécrit silencieusement
  un projet. Le gestionnaire de C.3 est réutilisé avec un choix de bibliothèque.
  Réutiliser la crate et les tests C.3 pour les principes de Citekeep (identités,
  clés locales stables, conflits explicites, planification avant écriture).
  Son moteur Python et son stockage BibTeX ne sont pas retenus ; le stockage et
  le format personnels restent à décider. Ne pas refaire le rapprochement en C.4.

Le contrat bibliographique C.3 et le CAS 16 sont courants ; les anciens lots
de raccordement restent des bilans datés. Recherche en ligne et gestion de PDF
restent hors de ce premier parcours.

**Sortie C :** contrat partagé GUI/CLI/MCP, recette clavier et aller-retour du
fichier ; absence de logique spécifique dupliquée dans les déclencheurs.

### Consolidation frontend et E.1 — 6 novembre 2026

Avant C.4, séparer les responsabilités des contrôleurs existants : rendu et
liaisons des champs d’en-tête, liste bibliographique et navigation, requêtes
asynchrones liées à une génération documentaire, préparation pure des lots,
puis orchestration des formulaires et de leur récupération. Réutiliser les
composants de dialogue/champ/clavier ; ne pas introduire de framework ni de
seconde logique métier bibliographique dans le frontend.

Étendre le contrôle bloquant des **800 lignes** au TypeScript de production
(`app/src/**/*.ts` et `.tsx`), hors tests, fixtures et code généré. Découper par
responsabilité, sans déplacer artificiellement du code pour satisfaire le seuil.
`provider.ts` conserve la coordination des révisions, de la composition et des
contextes d’édition ; `main.ts` reste le point de composition des services.
Leur taille seule ne justifie pas une extraction couplée à leurs états internes.

E.1 utilise le catalogue commun pour exposer les capacités du type actif dans
la barre d’outils et la palette, y compris les actions dérivées. Aucun nom de
type dans ce filtrage : un futur type composé de capacités existantes hérite
du comportement. Capacités absentes masquées, groupes vides masqués, commandes
prises en charge mais indisponibles au curseur visibles et désactivées ;
commandes générales conservées. Qualifier les changements de type, un type
fictif, les brouillons, les réponses tardives et le clavier sans captures.

## D. Modèles et bibliothèque — P1

Compléter les identités, versions et paquets installables des modèles à partir
du manifeste `model.yaml` livré en B.1 ; définir leur stockage global et la copie
autonome dans chaque document. Distinguer modèle installé et modèle embarqué ;
aucune mise à jour distante implicite.

Faire découler création, sélection et commandes des catalogues disponibles.
Ajouter validation/import, provenance, retrait, aperçu et remplacement explicite.
Pour chaque type livré : modèle neutre utile, corpus de test, export hors ligne
et changement de modèle sans modification du contenu.

**Accessibilité des exports (C/D) :** partager les diagnostics de description
d’image entre les adaptateurs ; conserver et récupérer les brouillons incomplets,
mais bloquer leur export accessible tant que la description manque. Activer les
standards PDF dans l’export commun, vérifier un corpus avec veraPDF en CI et
compléter par une recette humaine des modèles (ordre de lecture, descriptions,
titres et tableaux), les contrôles automatiques ne suffisant pas à tout vérifier.

## E. Interface et réactivité — continu

Conserver les conventions validées des widgets/bandeaux, menus, actions et
raccourcis ; maqueter les extensions sur l’interface Tauri réelle. Examiner
focus, lecture d’écran, contraste, mouvement réduit, petite fenêtre et saisie IME.

**À reprendre — localisation des sélecteurs macOS (31 octobre 2026) :** le menu
Tauri est correctement traduit par Fluent, mais le sélecteur ouvert avec
`npm start` garde ses boutons anglais. Investigation reportée à la demande de
l’utilisateur ; [constats, essais retirés et piste dbv-typst-editor](docs/native-localization-investigation.md).
Comparer les parcours de développement et de bundle et le service natif réel.
Un contrôle de métadonnées ou du titre du panneau ne valide pas ses boutons.

**E.1 — adaptation de la barre d’outils et de la palette au type — livré localement :** filtrer
les capacités exposées à partir du catalogue du type actif et supprimer les
groupes sans entrée. Ne pas coder « article » ou « courrier » dans la
présentation : « Fin de lettre » disparaît automatiquement d’un article parce
que ses capacités P.-S./P.J. n’y sont pas disponibles. Distinguer cette absence
structurelle de l’indisponibilité contextuelle d’une capacité du type, qui reste
visible mais désactivée par le registre commun. Actualiser la visibilité à la
création/ouverture sans perdre la recherche ni le focus ; vérifier le passage courrier → article → courrier et la
navigation au clavier sans groupe vide ni commande étrangère au type.
La palette réutilise ce même filtrage du catalogue et la même disponibilité
contextuelle, sans maintenir sa propre liste de capacités. Ses résultats
excluent les capacités étrangères au type actif ; les commandes générales de
l’application restent accessibles. Vérifier également les résultats de palette
lors du passage courrier → article → courrier.

**Séparation validée le 27 octobre 2026 :** l’accès **Projets** présente le
catalogue global (récents, épinglés, recherche, création et ouverture) ; l’accès
**Document** présente le plan, les références, les ressources, les versions et les relectures du
projet actif, selon son type. Le catalogue référence les fichiers ; retirer une
entrée ne les supprime pas. Préférences et bibliothèque globales restent
distinctes des données du projet.

Maquetter cette séparation sur la barre verticale existante ; un seul panneau
ouvert à la fois. **Versions** et **Relectures** sont des entrées distinctes.
La consultation de Versions et des propositions MCP partage une modale à 90 %,
avec onglets Contenu/Aperçu/Comparer et confirmations intégrées ; Relectures
garde les interactions prévues dans J. Le choix fenêtres/onglets pour
plusieurs projets ouverts reste lui aussi distinct de cette séparation.

Les tests graphiques Chromium/WebKit vérifient le frontend ; les recettes natives
doivent aussi couvrir WebKit macOS, WebView2 Windows et WebKitGTK Linux.
Vérifier les métadonnées de langue et les menus effectifs dans les parcours de
développement et d’application installée, sans confondre déclaration et traduction.

La [référence de performance](docs/tauri-preview-performance.md) sert de départ :
mesurer saisie, validation, compilation, transport, affichage et mémoire séparément.
Préserver la dernière page valide pendant un rendu, refuser les résultats périmés
et éviter une file de compilations obsolètes. Définir ensemble les budgets sur
documents courts, longs, illustrés et avec plusieurs participants.

**Flux desktop (5 novembre 2026) :** le [contrat de rendu différentiel](docs/preview-stream.md)
annule les travaux périmés aux frontières coûteuses et ne transporte que les
pages dont les pixels changent. Base explicite par fenêtre/génération/époque,
acquittée uniquement après publication ; un rendu retourné mais abandonné ne
remplace pas la base affichée. Les cartes immuables sont renouvelées à chaque
révision et adressées par identifiant du rendu affiché. Repli complet après perte
de base, publication atomique et réemploi des canvas inchangés. Les exports et
le format documentaire restent indépendants de ce protocole interne. Mesurer la
saisie sur les trois articles avec/sans aperçu, sans confondre réduction du
transfert et disparition de toutes les pointes de latence. La
[recette native complétée](experiments/preview-delta/INPUT-RESULTS.md) conserve
24 sessions, 1 152 entrées et les états finaux identiques. Sur 24 pages à
100 ms/caractère : deuxième RAF p95 80 → 34 ms, intervalle maximal 69 → 44 ms.
Le rapport distingue les gains, les métriques dégradées et le témoin sans aperçu ;
il n’isole pas le bénéfice de l’annulation, non sollicitée dans ces phases après.

**Priorité à la frappe (5 novembre 2026) :** ne déplacer dans le DOM que les
pages remplacées/retirées, conserver les autres en place et éviter les écritures
sur des surlignages vides. Les événements des surfaces de commandes et le
rafraîchissement général partagent une mise à jour secondaire après une
possibilité de rendu du texte (callback de frame puis nouvelle tâche, avec repli
borné pour fenêtre masquée). La disponibilité est toujours contrôlée à
l’invocation ; la présentation différée n’accorde aucun droit supplémentaire.
La [recette en trois variantes](experiments/editor-priority/RESULTS.md) conserve
36 sessions et des états finaux identiques. Elle mesure directement entrée →
frame du texte et entrée → publication/frame du nouvel aperçu, sans les
confondre avec l’aller-retour IPC ni prétendre mesurer la présentation physique.
Sur 24 pages à 250 ms/caractère, frappe p95 25 → 16 ms ; aperçu 90 → 80 ms.

**Budget relatif :** exprimer la cible comme un facteur (1,5 à 2, en p95) de la
compilation incrémentale de Typst seul sur le même document, avec une référence
distincte pour le natif et pour le WebAssembly, un plancher absolu pour les petits
documents et un suivi du surcoût par étape. La cible suit ainsi les évolutions de
Typst et détecte les régressions propres à typstOgraphe.

## F. Assistance ACP

**Orientation retenue :** client Rust agent-client-protocol, Codex via codex-acp
pour le premier parcours ; MCP reste le contrat documentaire. Aucun couplage à
ACP ou à un fournisseur d’agent dans le cœur documentaire.

Conserver la [découverte MCP progressive](docs/mcp-discovery.md) : vues compactes
partagées, détails canoniques à la demande, définitions de schémas accessibles
par référence et support de l’adaptateur distinct des permissions. Reprendre
ces générateurs pour ACP, sans ajouter de catalogue parallèle. Le contrat
CLI/cœur reste complet ; les vues MCP allègent sa présentation.

### F.1 Design à valider

Maquette du panneau ouvert depuis la barre latérale : question de hauteur
adaptative, réponse dessous, pied d’actions stable, historique consultable et
brouillon conservé. Une conversation libre n’impose pas de mode discuter/modifier.
L’emplacement, les dimensions et la place de l’aperçu restent à valider.

Échap masque sans terminer la session ; Tab garde la navigation standard.
L’envoi par Entrée et la saisie multiligne sont à préciser avec l’IME.
Une Entrée globale n’accepte jamais un lot de modifications.

### F.2 Processus, droits et session

Démarrage volontaire, choix de l’agent dans les préférences, disponibilité visible
et connexion au compte par l’outil de l’agent. Aucune bascule implicite vers une
API facturée. Vérifier versions, authentification, distribution et mise à jour
du client ACP, de l’adaptateur et de l’agent.

Créer et fournir automatiquement un raccordement MCP privé au document courant ;
ne pas demander une configuration manuelle à chaque invitation. Conserver les
outils documentaires seuls dans le premier périmètre, y compris face à la
configuration héritée de l’agent. Préférences globales et autres documents restent
hors du contexte autorisé ; document et sélection confiés sont explicités.

Une conversation par document et agent initialement ; aucun historique ajouté
au .typsto dans le premier périmètre. La conservation éventuelle par l’agent
doit être expliquée. Changer d’agent commence une nouvelle conversation.

Un seul lot en attente, consultation et acceptation/refus explicites, révision
revalidée à l’acceptation. Distinguer interruption, masquage et terminaison ;
terminer/fermer/remplacer révoque l’accès et retire les propositions en attente.
Une reprise ne renvoie ni une demande ni une édition automatiquement.

Le lot soumis prend la forme d’une paire privée base/candidat, avec garde et
opérations documentaires sous-jacentes, telle que définie dans A.4. La soumission
ne crée pas une version visible, n’ouvre pas une relecture J et ne sauvegarde
pas implicitement le projet. Accepter publie via le moteur commun, sous contrôle
de révision ; les écritures de fichiers gardent leurs permissions propres.
La conservation durable éventuelle après décision reste à préciser dans F,
sans introduire par défaut une nouvelle origine de version « agent ».

**Sortie F :** deux agents compatibles vérifiés sur discussion, lecture, proposition,
refus/acceptation, péremption, annulation, interruption et révocation. Ne pas
supposer que les fonctions, forfaits et quotas sont identiques entre agents.

### F.3 Cycle de vie et reçus des propositions

La comparaison humaine et ses reçus par groupes sont livrés : voir le
[contrat courant](docs/a4-comparison-contract.md). Il reste à préciser ensemble
la conservation durable après décision, les reçus supplémentaires requis par ACP
et le parcours de nouvelle proposition sur une base relue. Conserver les gardes,
les groupes dépendants indivisibles et l’autorité humaine ; ne pas construire
un second moteur ni accorder de droits implicites de sauvegarde/export.

## G. Collaboration et hébergement — P2

Étudier Hocuspocus et son connecteur avant de figer le transport distant.
Son protocole n’est pas interchangeable avec un simple serveur y-websocket.
Réutiliser le même document Yjs et l’historique par participant.

**Arbitrage à deux niveaux :** un arbitre local optimiste (frappe immédiate, sans
autorité sur les autres) et un arbitre d’autorité (hôte ou serveur) qui valide
avant admission et diffusion du candidat fusionné avec son état confirmé courant.
Réutiliser `EditingSession` et `SharedSession`, déjà dans `core`, et extraire la
coordination encore liée à Tauri en laissant fichiers, fenêtres et transport dans
les adaptateurs. La validation native convient au desktop ; évaluer compilation
`wasm32`, poids et performances pour un client navigateur avant de retenir WASM.
Comparer un serveur Rust/Yrs à Hocuspocus raccordé au même moteur Rust : appel
distant, module natif, WebAssembly ou processus séparé restent à évaluer.
Intercepter les messages avant application à l’état d’autorité, pas dans un
observateur après modification. Concevoir la reconstruction de l’état local et
la conservation du brouillon après refus distant, en suspendant les mises à jour
qui dépendent de la branche refusée. L’annulation Yjs n’efface pas cette branche
de l’historique CRDT.

Deux parcours à distinguer :

- **Session hébergée depuis le desktop :** processus serveur avec runtime embarqué,
  invitation limitée et révocable, droits, arrêt explicite ; tester d’abord le réseau
  local. Définir les effets de la veille, de la fermeture et de la perte de l’hôte.
- **Service d’organisation :** authentification, autorisation par document,
  stockage durable, ressources et disponibilité indépendante des postes utilisateurs.

Pour un lien Internet vers un poste, évaluer un relais/tunnel ou une autre
stratégie de traversée réseau ; ne pas supposer qu’ouvrir un port local suffit.
Pour Cloudflare, évaluer séparément un relais et un hébergement Workers/Durable
Objects : cycle de vie, état rechargé, persistance, connecteur, coûts et limites.
Le support d’un runtime ne prouve pas l’adéquation de tout le déploiement.

Sources de départ, pas choix d’infrastructure définitifs :
[Hocuspocus](https://tiptap.dev/docs/hocuspocus/server/usage),
[WebSockets et Durable Objects](https://developers.cloudflare.com/durable-objects/best-practices/websockets/).

Décider la propriété/sauvegarde des documents partagés, le transfert des images,
les modèles et schémas compatibles, le travail hors connexion et la réconciliation.
Préciser les droits et l’autorité pour les snapshots partagés, la capture d’une
révision cohérente, l’annonce des restaurations et la gestion des éditions
concurrentes ; conserver l’historique lors d’un retour à un état antérieur.
Prévoir un test de charge représentatif à cent utilisateurs sur plusieurs documents,
avec participants lents, collages volumineux, reconnexions et reprise du serveur.

## H. Distribution et vérification — continu

**Découpage validé le 2 novembre 2026 — publication des applications :**

H.1 dispose désormais de la chaîne de packaging/dépôt, de tests de provenance
et d'un essai natif macOS ARM64 ; voir les [résultats et limites](docs/releases.md#vérification-locale-du-lot-h1).
La matrice complète et le premier dépôt réel restent à qualifier en CI.

- **H.1 :** conserver la validation complète comme préalable, puis réutiliser
  le packaging Tauri pour macOS universel (`.dmg`), Linux x86_64 (`.deb`,
  AppImage) et Windows x64 (NSIS). Vérifier la concordance tag/versions/commit,
  inspecter et collecter les paquets, puis les joindre à une release unique en
  brouillon, avec empreintes SHA-256. N'accorder l'écriture GitHub qu'au job
  de dépôt des fichiers, une fois toutes les constructions réussies.
  La relance opère sur le tag immuable ; elle peut compléter un brouillon,
  mais ne modifie pas une release publiée. La publication est manuelle.
  Les constructions manuelles de branche restent des artefacts de test.
- **H.2 :** vérifier le contenu final puis l'installation et le fonctionnement
  des paquets sur les trois systèmes, avec modèles, polices, langues et
  compilation/export hors ligne. Distinguer inspection du paquet et recette
  de l'application réellement installée.
- **H.3 :** signature/notarisation, tap Homebrew après publication et manifeste
  de mise à jour signé ; comptes, clés, canaux et reprise sur échec à valider.
  Aucun contournement de sidecars n'est nécessaire pour le moteur Typst intégré.

Ne pas relancer les tests complets par format : une validation du commit suivie
des constructions/inspections natives et d'un dépôt central des fichiers suffit.
Le premier lot prépare les installateurs non signés, pas une mise à jour automatique.

**Jalon d’export volontaire à développer :** figer l’état ayant produit un PDF
réussi, puis proposer de créer sa version sans bloquer la rédaction ni l’export.
La capture ne doit pas prendre un état ultérieur ; annulation/échec ne déclenche
rien et le quota visible n’empêche pas l’export. Comparaison entre fichiers différée.

Préparer builds reproductibles, paquets et identité stable sur les trois systèmes,
signature/notarisation, associations de fichiers, invitations, mise à jour et retour
sur échec. Vérifier les ressources réellement embarquées et leurs licences.

Maintenir CI et couverture Rust/web, tests de contrats, recettes d’archives/exports,
contrôles des chemins et limites d’import, tests natifs et mesures de performance.
Prévoir corpus de documents réels, documentation utilisateur et diagnostics de
support sans diffusion implicite du contenu des documents.

## Condition de fin de lot

Le comportement et ses limites sont validés, le parcours utilisateur fonctionne,
les changements sont vérifiés et documentés au bon endroit. Une tâche terminée
sort de la roadmap ; ses preuves et décisions vont dans une note ou dans Git.
Une nouvelle dépendance, un changement de contrat ou un choix graphique structurant
revient dans la discussion avant d’engager l’implémentation concernée.

## J. Relecture par suggestions — P2

Représenter les suggestions par des annotations ancrées séparées du contenu,
applicables par les commandes gardées existantes. Conserver les outils dédiés de
relecture ; l’interface commune d’A.4 sert aux snapshots et propositions MCP,
sans imposer sa présentation à J. Mutualiser les composants pertinents ; le
moteur de décisions locales d’A.4 préserve l’atomicité des opérations liées,
et F.3 complète le protocole MCP de ce moteur. La première
relecture humaine textuelle ne dépend pas d’ACP. Distinguer aperçu confirmé et
aperçu candidat explicitement signalé ; revérifier le contenu cible avant toute
acceptation si l’auteur l’a modifié.

A.4 prépare la référence de chaque relecture : créer automatiquement un snapshot
au démarrage d’une nouvelle relecture, le lui rattacher et le conserver inchangé
pendant toute sa durée, en empêchant sa suppression tant qu’elle est en cours.
La reprise ne recrée pas de snapshot ; aucun déclencheur heuristique n’est prévu.
Cette référence est interne à Relectures, hors liste Versions et quota des
100 versions affichées. Son cycle de vie relève de J, sous les limites de volume
du projet ; ne pas dériver son interface des actions de comparaison de snapshots.
La conservation des suggestions, l’effet d’une restauration et la rétention de la
référence après clôture restent à préciser ensemble dans J. Le calcul des
différences entre snapshots et les suggestions ancrées ont des rôles distincts ;
la relecture ne se
réduit pas à un retour global à une ancienne version.

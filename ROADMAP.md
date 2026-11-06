# Feuille de route de typstOgraphe

Révisée le **6 novembre 2026**. Objectif : transformer le prototype validé en une
application de rédaction utilisable au quotidien, pour l’administration,
l’enseignement et la recherche en sciences humaines et sociales.

Cette feuille distingue le socle livré des travaux restants. Les réalisations
détaillées, mesures et décisions historiques restent dans Git et les bilans de docs/.
Le [plan technique](ROADMAP-ARCHITECTURE.md) précise les dépendances et les
conditions de sortie ; il ne constitue pas une seconde liste de fonctionnalités.

## Manière de travailler

**Nous validons ensemble les choix principaux avant chaque implémentation** :
besoin, périmètre, contrat documentaire et comportement ; maquette pour une
interaction nouvelle. Une piste inscrite ici n’est pas une décision technique.

Livrer par lots courts et utilisables, vérifier les parcours réels, puis retirer
du plan ce qui est terminé. Rechercher les bibliothèques et services existants
avant de créer une nouvelle brique. Faire plusieurs commits cohérents et une
seule poussée par lot, sauf correction nécessaire d’une vérification distante.

**Socle disponible :** application Tauri, contrat documentaire commun,
conservation des brouillons, fichiers CAS et Versions avec comparaison locale.
Les [contrats courants et bilans](docs/README.md) décrivent les garanties livrées ;
les propositions MCP rattachées restent limitées au corps/cellules/annotations.
La [clôture A.5](docs/a5-closure-results.md) documente les contrats, l’isolation
des recettes et la recette finale depuis un checkout propre. La qualification
CI complète du candidat reste à vérifier avant fusion. Les contrôles matériels
et de distribution restent dans H.

**Aucune capture d’écran pour le moment** : les tests fonctionnels restent actifs,
sans captures automatiques ni références au pixel près. L’interface n’est pas figée.

## Un projet typstOgraphe — définition retenue

Un **projet est le fichier .typsto autonome**, équivalent d’un répertoire de
travail LaTeX dont l’organisation technique est masquée. Il rassemble un
document, éventuellement structuré en parties, ses métadonnées et réglages,
le modèle, les dépendances, les images, les références bibliographiques et les
autres ressources nécessaires, ainsi que les snapshots conservés. Un courrier,
un article ou un mémoire constitue chacun un projet ; le projet n’est pas un
regroupement de plusieurs documents indépendants.

Le mot « document » peut rester celui des actions courantes dans l’interface.
Le catalogue global référence les projets : il ne remplace pas leurs fichiers
par un espace de stockage imposé.

**Séparation d’interface validée le 27 octobre 2026 :**

- **Projets** : retrouver les fichiers récents ou épinglés, rechercher par nom,
  créer et ouvrir un projet. Retirer une entrée des récents ne supprime pas
  le fichier.
- **Document** : naviguer dans le projet actif, avec son plan, ses références
  bibliographiques, ses ressources, ses versions conservées et ses relectures. Les rubriques
  s’adaptent aux capacités du type ; l’arborescence technique reste masquée.

La piste à maquetter s’appuie sur la barre verticale existante et un seul panneau
latéral ouvert à la fois, avec les accès aux outils et aux préférences globales.
**Versions** et **Relectures** sont deux entrées distinctes de la barre latérale.
Versions expose les états conservés pour consultation/restauration ; Relectures
conserve le parcours par annotations du chantier J. Les références techniques
de relecture ne sont pas des lignes de Versions. La modale commune de consultation
et comparaison sert aux versions et aux propositions MCP, avec des actions adaptées.

## Priorités et dépendances

| Priorité | Chantier | Résultat attendu |
| --- | --- | --- |
| **P1** | B — Types et contrats | Une liste limitée de types utiles, leurs métadonnées et leurs capacités |
| **P1** | C — Capacités documentaires | Un périmètre quotidien complet et cohérent, puis les outils nécessaires aux documents SHS |
| **P1** | D — Modèles et bibliothèque | Au moins un modèle réellement utilisable pour chaque type livré |
| **P1, continu** | E — Interface et rédaction | Une édition prévisible, rapide et entièrement utilisable au clavier |
| **P1** | F — Assistance ACP | Une aide à la rédaction intégrée, avec propositions soumises à validation |
| **P2** | G — Collaboration | Partage ponctuel depuis un poste, puis service durable d’organisation |
| **P2** | J — Relecture par suggestions | Corriger et commenter un document sans le modifier, pour les humains comme pour les agents |
| **P1, continu** | H — Diffusion et qualité | Une application installable, testée et documentée sur les trois systèmes |
| **P3** | I — Extensions | Grammaire, conversions et capacités spécialisées justifiées par les usages |

L’accessibilité, la protection du travail et les performances accompagnent chaque
lot. La collaboration distante et l’assistance restent facultatives ; la
rédaction individuelle, l’ouverture et l’export doivent fonctionner hors ligne.

## Socle livré — références

A.1–A.5 sont implémentés et recettés localement ; voir l’[index documentaire](docs/README.md)
et le [bilan de clôture](docs/a5-closure-results.md). La qualification CI du
candidat sur les trois systèmes reste distincte des résultats macOS locaux.
Le socle ne constitue pas encore un outil de rédaction complet : les lots
ci-dessous portent les fonctionnalités restantes et leurs arbitrages.

B.1 est clôturé après relecture le 3 novembre 2026 ; le
[bilan de clôture](docs/b1-closure-results.md) rassemble les critères acquis,
les contrats, les preuves et les limites. Déclarations de types, `authorship`,
options de modèles, dates et références d’images sont raccordés aux adaptateurs.
Courrier et Article sont créables ; le Compte-rendu reste un contrat interne
sans modèle fourni. Le chantier B continue pour les autres types.

## B — Définir les types utiles et leurs contrats

Le [catalogue validé et les fiches par type](docs/document-type-catalogue.md)
guident ce chantier. Les groupes sont des portes d’entrée, pas des types ni des
contraintes de modèle ; un type peut appartenir à plusieurs groupes.

| Groupe validé | Types |
| --- | --- |
| Correspondance et administration | Courrier ; compte rendu ; note ; rapport ; présentation |
| Recherche et publications | Article scientifique ; mémoire et thèse ; projet de recherche ; poster ; rapport ; livre ; présentation |
| Parcours et candidatures | CV ; projet de recherche ; courrier (dont lettre de motivation) |
| Enseignement | Support de cours ; exercices et évaluations ; présentation |
| Livres et communication | Livre ; présentation ; rapport ; poster |
| Transversal | Document libre — périmètre à définir |

Courrier et Article sont disponibles ; les autres fiches décrivent des **types
à venir**, sans promettre des commandes déjà exécutables. Mémoire et thèse
restent un seul type. Une lettre de motivation est un Courrier. Le Projet de
recherche vise notamment demandes de financement et projets doctoraux ; il ne
désigne pas le fichier-projet `.typsto`. La fiche Document libre reste ouverte.

Le parcours de création présente les groupes en filtres cumulables, les types
en tuiles et les modèles en **vignettes Typst**, dans la grande modale décrite
au [catalogue](docs/document-type-catalogue.md#parcours-de-création). Aucun type
annoncé ne devient créable sans contrat validé et modèle réellement utilisable.

Pour chaque type : définir les métadonnées, leurs genres, les groupes d’en-tête,
les champs obligatoires à l’export, les capacités autorisées et les contrats
Typst à satisfaire. Distinguer paramètres du type, paramètres généraux et
paramètres propres au modèle. Prévoir les auteurs multiples comme données
structurées, sans les transformer prématurément en une chaîne de texte.

**Organisation d’en-tête validée le 27 octobre 2026 :** un seul accès par onglets.
Pour le Courrier : Général, Expéditeur, En-tête. Les deux premiers sont des
formulaires ; le dernier est une zone composée de sections décrites par le type,
avec les informations propres au document. Ces contrôles éditent les valeurs
structurées existantes, sans duplication dans le corps ni dépendance aux fonctions
internes des modèles. Pour l’Article : Général, Titres (titre, sous-titre et titre
court par langue), Résumés (résumé riche et mots-clés par langue), Affiliations,
Auteurs et Bibliographie. Les mêmes enregistrements multilingues alimentent
Titres et Résumés.
Les ajouts par + sont provisoires jusqu’à validation et annulables par Échap ;
ils ne sont pas diffusés ni sauvegardés avant confirmation. La
[décision détaillée](docs/types-modeles-libertes.md#interface-de-len-tête--décision-du-27-octobre-2026)
et le [contrat Article](docs/article-contract.md) servent de référence.

### Extensions des contrats — différées

B.1 est clôturé. Les extensions suivantes restent à examiner selon les besoins,
sans bloquer les capacités suivantes :

- Types installables, codecs de données arbitraires et personnalisation des
  fragments partagés : les identités et comportements restent intégrés ;
  modifier une déclaration livrée demande une reconstruction.
- Paramètres entièrement propres à un modèle : étendre les régimes et contrôles
  communs après validation dans D.
- Dates ou ressources en collections, nouveaux genres de ressources et précision
  calendaire supplémentaire : valider leurs contrôles avant d’étendre le
  vocabulaire ; les formes non prises en charge sont refusées.

**Suite du chantier B :** définir les prochains types et fournir une fiche et
une matrice type/capacité/modèle, au rythme des besoins et des capacités livrées.
Le Document libre et les nouveaux contrats de types restent à préciser.

## C — Compléter les capacités à partir des besoins des types

L’[inventaire des capacités](docs/capabilities-inventory.md) contient les besoins
restants et les pistes Typst/Universe. Le catalogue exécutable du code fait foi
sur ce qui est réellement disponible ; une entrée de cette liste ne suffit pas.

- **Premier périmètre :** compléter les tableaux simples selon les
  [décisions validées pour les figures et tableaux](docs/structured-commands.md#figures-et-tableaux--contrat-mis-en-place) :
  enveloppe `figure` commune avec les images, légende facultative, en-têtes
  sémantiques, ajout/suppression de lignes et colonnes autour de la cellule active.
  La présentation et les défauts de placement appartiennent au modèle. La fusion
  de cellules est différée. Terminer aussi la recette des notes de bas de page ;
  leur base existe : ne pas les réimplémenter.
- **Documents SHS :** citations bibliographiques et bibliothèque de références,
  renvois vers titres/figures/tableaux, numérotation, sommaire et annexes.
- **Selon les types retenus :** encadrés sémantiques, listes de tâches et listes
  imbriquées ; conserver un widget unique pour une liste entière.
- **Enrichissements ponctuels :** barré, souligné, exposants, indices et séparation
  thématique, sans transformer la barre d’outils en catalogue permanent.
- **Accessibilité des images — décision du 28 octobre 2026 :** le **texte
  alternatif est obligatoire** pour toute image. Aujourd’hui le champ est
  facultatif et vide par défaut ; avec PDF/UA-1, une image sans description fait
  échouer l’export (« missing alt text »). L’insertion, l’édition, les commandes
  CLI/MCP et la validation documentaire partagent le même diagnostic. Un brouillon
  incomplet doit rester insérable, enregistrable et récupérable ; la description
  manquante est signalée, puis bloque l’export accessible tant qu’elle manque.
  Un statut « image décorative » n’est pas décidé : à examiner avec la maquette.

Une capacité partagée doit avoir le même contrat et les mêmes interactions
dans tous les types où elle est disponible. Les modèles en définissent la
présentation et les défauts. Les identifiants publics restent anglais ou latins.
Une dépendance ou une logique Typst spécifique passe par un adaptateur stable ;
les primitives natives simples n’ont pas besoin d’un wrapper systématique.

### Lots validés le 3 novembre 2026 — après les contrats de B.1

B.1 est clôturé. L’ordre de réalisation validé est désormais
**C.1 → C.2 → C.3 → consolidation frontend / E.1 → C.4**, à partir des contrats
préparatoires consignés en B.1. C.1, le premier périmètre de C.2 et C.3 sont
implémentés et vérifiés localement. C.3 comprend le gestionnaire compact complet,
les profils natifs Hayagriva et le **CAS 16** ; sa fusion reste soumise à la CI
et à la recette utilisateur. E.1 précède les extensions plus lourdes de D, F et J.
Valider ensemble les choix détaillés et les interfaces avant de coder chaque
lot ; ne pas attendre l’achèvement de tous les types de B pour avancer dans C.

1. **C.1 — Labels et références croisées — livré localement.** Ancres stables
   automatiques, sélecteur et badges communs, composantes libellé/page, rendu
   Typst défini par le modèle et commandes partagées Tauri/CLI/MCP. Le
   [contrat et la recette](docs/c1-cross-references.md) précisent les garanties.
   La sélection par clic dans l’aperçu est une amélioration différée, avec un
   mode explicite et une résolution des ambiguïtés ; elle ne bloque pas C.2.
2. **C.2 — Formules mathématiques — livré localement.** Saisie de code Typst ou LaTeX via MiTeX,
   dans le texte ou en bloc ; conserver le langage et la source, prévoir la
   description accessible et les références aux blocs. L’éditeur visuel viendra
   ensuite sur le même objet, sans promettre une conversion réversible des
   syntaxes.
   [Comparaison native/WASM réalisée](experiments/mitex-comparison/RESULTS.md) :
   WASM conservé, aucun gain robuste de latence à chaud sur les trois articles.
   Le [profilage de l’aperçu long](experiments/preview-profiling/RESULTS.md) confirme
   les caches Typst et identifie les pixels transférés et les canvas recréés pour
   les pages inchangées comme priorité. L’[évaluation RGBA différentiel / Reflexo](experiments/preview-renderers/RESULTS.md)
   est terminée : 18 sessions natives, synchronisations conservées, 270 → 102 ms
   pour une édition de formule sur 24 pages. Reflexo atteint aussi 102 ms et
   accélère le premier rendu complet (455 ms contre 713 ms), avec un transfert
   plus petit mais des adaptations et une qualification supplémentaires.
   La [contre-épreuve dans VS Code/Tinymist](experiments/tinymist-pipeline/RESULTS.md)
   complète cette conclusion : sur 24 pages, deux frames après une édition de
   formule, p50 de 47 ms dans Tinymist avec rendu partiel, contre 123 ms dans
   notre candidat RGBA (p95 respectifs 168 et 126 ms). La première étude ne
   comparait que les renderers dans notre parcours commun.
   [Optimisation de l’interface mesurée](experiments/ui-refresh/RESULTS.md) :
   rafraîchissements regroupés sans délai de frappe, disponibilité actualisée
   une fois, comptages inchangés et transactions de références dédupliqués.
   Sur 24 pages, une édition de formule rend la main en 7 ms contre 22 ms ;
   publication 271 → 254 ms en RGBA complet et 102 → 87 ms dans le candidat
   différentiel. Le p95 du RGBA complet reste variable (296 → 307 ms).
   Les 24 sessions préservent 396 états de source, pixels et navigation identiques.
   [Cartes optimisées et mesures vérifiées](experiments/source-sync/RESULTS.md) :
   largeurs échappées sans allocation par caractère et index syntaxique construit
   une fois par compilation, sans réutiliser de plages entre révisions.
   Sur 24 pages, carte de source 11,93 → 2,01 ms ; construction RenderSync
   hors vérification 12,81 → 4,82 ms (mesure dérivée). Publication de formule
   258 → 244 ms en RGBA complet, 93 → 76 ms dans le candidat différentiel.
   Les 24 sessions vérifient le cœur réellement compilé et 396 états identiques
   en source, pixels, cartes UTF-16 complètes et ancres transformées.
   [Saisie continue mesurée avant/après](experiments/typing-latency/RESULTS.md) :
   les uploads successifs de toutes les pages peuvent bloquer l’éditeur.
   Préparation désormais fractionnée en tâches courtes, annulation des états
   périmés et libération des canvas ; publication toujours atomique.
   459 tests, 14 parcours navigateur et 24 sessions natives passent, avec
   1 152 entrées conservées et des états finaux identiques. Sur 24 pages à
   250 ms/caractère : file d’entrée p95 25 → 3 ms, prochain créneau d’affichage
   p95 40 → 21 ms, intervalle maximal entre frames 123 → 67 ms. L’aperçu reste
   actif (44 publications contre 46). Des pointes subsistent à 100 ms/caractère,
   sans uploads pendant la frappe ; ne pas annoncer leur élimination générale.
   **Suite (5 novembre 2026) :** [flux RGBA différentiel de production](docs/preview-stream.md)
   désormais raccordé : annulation après compilation, entre pages et avant le
   paquet ; cache par fenêtre/document/génération/époque ; base acquittée seulement
   après affichage ; transfert et canvas réservés aux pages visuellement modifiées.
   Chaque révision reconstruit ses cartes de synchronisation, y compris sans
   changement de pixels. Repli complet en cas de base absente, publication
   atomique et parcours erreurs/rechargement/multi-fenêtres testés. Réévaluation
   native de la saisie dans une série isolée `preview-delta`, sans remplacer les
   mesures précédentes. [Vérification des trois articles](experiments/preview-delta/README.md) :
   pixels/cartes identiques ; sur 24 pages, une page modifiée transporte
   8,13 Mo contre 192,65 Mo. [Mesure native de saisie complétée](experiments/preview-delta/INPUT-RESULTS.md) :
   24 sessions et 1 152 entrées conservées, états finaux identiques. Sur 24 pages
   à 100 ms/caractère, deuxième créneau d’affichage p95 80 → 34 ms et intervalle
   maximal entre frames 69 → 44 ms ; l’aperçu suit désormais la frappe rapide.
   Les premiers créneaux p95 à 250 ms se dégradent légèrement (24 pages :
   20 → 30 ms) ; conserver les contrôles sans aperçu et les pointes restantes,
   sans promettre une amélioration uniforme ni attribuer le gain à l’annulation
   seule, qui n’a pas été sollicitée dans les phases chronométrées après.
   **Priorité à l’éditeur :** [comparaison en trois étapes](experiments/editor-priority/RESULTS.md)
   après suppression des déplacements de pages inchangées puis regroupement des
   mises à jour secondaires après une possibilité d’afficher le caractère.
   Les rafraîchissements de commandes passent par le même ordonnanceur ; les
   contrôles d’exécution restent immédiats. 479 tests frontend, 30 parcours
   navigateur et 36 sessions natives valident 1 728 entrées et les états finaux.
   Sur 24 pages : frappe p95 17 → 13 ms à 100 ms/caractère et 25 → 16 ms à
   250 ms/caractère. Le premier créneau du nouvel aperçu, désormais mesuré
   directement de bout en bout, passe respectivement de 70 à 60 ms et de 90 à
   80 ms. Aucun délai de frappe ajouté. Les petits documents restent variables
   (7 pages à 100 ms : 13 → 17 ms) ; conserver tous les cas et les pointes.
   **Saisie validée :** `$` seul sur une ligne suivi d’Entrée crée un widget
   Typst ; `$$` seuls suivis d’Entrée créent un widget LaTeX. Dans le texte,
   `$cos(x)$` désigne une formule Typst et `$$\cos(x)$$` une formule LaTeX.
   Ces délimiteurs choisissent le langage ; le double dollar n’impose pas
   une présentation en bloc. On édite directement le source mathématique,
   comme dans Typst ou LaTeX, sans liste de formules autorisées propre à
   l’application. Le moteur concerné fournit les diagnostics dans les limites
   de son support. Aucun aperçu supplémentaire dans l’éditeur : le rendu
   temps réel de la colonne droite suffit. Le source est celui d’une formule,
   éventuellement complexe ou multiligne, sans document LaTeX complet.
   **À traiter ultérieurement :** macros LaTeX et fonctions Typst personnalisées,
   leur définition, leur portée et leur conservation dans le projet et l’export.
   Le [contrat C.2](docs/c2-mathematics.md) décrit les zones de code partagées,
   l’adaptateur MiTeX autonome et les garanties de conservation.
   **Widgets ajustés (5 novembre 2026) :** langage fixé à la création, sans
   changement Typst/LaTeX dans les propriétés ; champ `eq:` dans l’en-tête,
   synchronisé avec les propriétés via le même attribut et la même commande.
   Description accessible transmise à `math.equation(alt: ...)` ; test réel
   PDF/UA-1 avec et sans description dans les deux langages. Les brouillons
   restent éditables sans description et les réglages d’export ne changent pas.
   **Labels validés :** les équations en bloc conservent une identité automatique
   stable et peuvent recevoir un alias lisible facultatif. L’interface présente
   le préfixe `eq:` grisé et non éditable, suivi du nom libre ; un nom vide ne
   crée aucun alias. Modifier cet alias ne casse pas les renvois. Dans le
   sélecteur, `eq:` ou son raccourci `$` filtre les équations. Le badge reprend
   le libellé réellement composé par Typst, par exemple `(12)` ; le modèle
   conserve la maîtrise de la numérotation et de sa présentation.
3. **C.3 — Bibliographie du projet — livré localement (CAS 16).** Consolider les imports BibLaTeX et Hayagriva
   dans un unique fichier Hayagriva interne au `.typsto` et à l’export. Le
   contrat de plusieurs fichiers originaux a été remplacé après évaluation des pertes
   de conversion, des collisions et de la conservation des citations.
   Ajouter un gestionnaire dans une grande modale : à gauche, recherche fuzzy
   sur titres et auteurs, notices filtrées et bouton + ; à droite, formulaire
   adapté au genre de la notice. Création, modification et suppression protégée
   des notices citées ; brouillon local avec Annuler/Valider et choix explicite
   avant de quitter une notice modifiée. Réutiliser le formulaire depuis
   l’insertion de citation avec « Créer et citer », en une opération annulable.
   **Contrat validé le 6 novembre 2026 :** créer la crate commune
   `typstographe-bibliography`, réutilisée ensuite en C.4. Elle porte les notices
   natives Hayagriva, identités stables distinctes des clés, profils/descripteurs natifs,
   import, recherche et plans de rapprochement ; le cœur garde les citations,
   l'annulation et la publication documentaire. Conserver les originaux importés
   comme provenance en lecture seule, sans deuxième bibliothèque active.
   Prépublication et Logiciel font partie des profils natifs ; les champs manquants
   donnent des avertissements de complétude, sans rendre les notices invalides.
   Hayagriva n'impose pas de champs bibliographiques par genre. Qualifier les
   correspondances natives et les pertes de conversion avant livraison.
   Le [contrat C.3](docs/c3-bibliography-contract.md) et son
   [corpus issu de Citekeep](fixtures/bibliography/README.md) distinguent
   candidature, contradictions et décisions ; prépublication/publié et versions
   de logiciel ne sont pas fusionnés silencieusement. Valider une notice est une
   édition annulable, sans écriture immédiate du fichier ; les conflits se
   résolvent avant publication du lot d'import. Le
   [premier lot de crate](docs/c3-bibliography-foundation.md) est implémenté,
   avec les 24 scénarios d'identité exécutés et le partage de lecture/recherche.
   Le [lot de préparation commune](docs/c3-bibliography-preparation.md) expose
   profils natifs, préparation de notices et rapports d'import dans la CLI et MCP, avec
   schémas et diagnostics typés, sans publier d'édition ni changer le format.
   Le [raccordement documentaire](docs/c3-bibliography-document-state.md) ajoute
   `bibliography.apply`, les notices/sources partagées, les citations par identité,
   l’annulation et un Hayagriva actif unique ; le format courant est le CAS 16.
   **Présentation validée :** [gestionnaire commun](docs/c3-bibliography-interface.md),
   liste compacte à deux lignes (auteur/année puis titre tronqué), défilement
   indépendant, détails et formulaire à droite. Le [gestionnaire implémenté](docs/c3-bibliography-manager.md)
   raccorde formulaires, récupération privée et décisions d’import au moteur
   commun, avec création/citation atomique, gardes concurrentes et navigation
   clavier. Les codecs préservent parents, valeurs composées et champs non
   exposés ; les requêtes de formulaire, revue, parcours et usages sont communes
   à Tauri, CLI et MCP. Les profils sont maintenant déduits de Hayagriva : le
   changement de type reste privé jusqu’à Valider, conserve champs masqués et
   conteneurs inactifs, puis se publie en une édition annulable. Le CAS 16 et
   l’export gardent ces données hors de la bibliothèque active.
   **Classification native — décision du 6 novembre 2026 :**
   [supprimer le genre indépendant](docs/c3-native-bibliography-profiles.md).
   Déduire les profils de formulaire du type et des parents Hayagriva ; changer
   de type sans perdre champs, conteneurs inactifs, provenance ou citations.
   Publier une édition gardée et annulable, avec les mêmes règles dans les
   adaptateurs et les exports.
4. **C.4 — Bibliothèque bibliographique personnelle.** Recherche projet d’abord,
   bibliothèque personnelle ensuite ; copier la notice dans le projet lors de
   l’insertion. Échanges et corrections explicites, identités et clés stables,
   conflits soumis à décision, aucune mise à jour silencieuse des documents.
   Réutiliser le gestionnaire avec un choix Projet/Bibliothèque personnelle.
   Réutiliser aussi la crate de C.3 et son corpus : ajouter le stockage personnel
   et les échanges à révisions propres, sans second moteur de rapprochement.

La séparation bibliothèque personnelle/projet et les garanties de Citekeep
servent de référence de conception. Les règles seront portées une seule fois
dans la crate Rust commune ; le moteur Python et son stockage BibTeX ne sont
pas retenus. Le stockage personnel reste à décider en C.4. Chaque projet reste
autonome et utilisable hors ligne, indépendamment de cette bibliothèque.

**Choix à valider :** périmètre par capacité, paramètres, contenus et imbrications
autorisés, saisie, clavier et cibles MCP ; fonctions natives ou paquet externe.

**Sortie :** insertion, édition, annulation, sauvegarde/réouverture et exports
vérifiés, avec la même disponibilité dans les interfaces qui exposent la capacité.

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

## D — Un vrai modèle par type, puis une bibliothèque

- Accompagner chaque type livré d’un modèle généraliste sobre, utile et sans
  coordonnées ni contenu d’organisation fictifs imposés.
- Étendre le modèle « Article de travail » aux prochaines capacités validées,
  puis reprendre compte-rendu et note/mémo selon le catalogue validé ; vérifier
  le courrier personnel sur les mêmes critères.
- Pour l’article, le rapport, le cours et le syllabus : définir une recette
  représentative, puis le modèle en même temps que les capacités nécessaires.
- Réutiliser et étendre la bibliothèque Typst éditoriale commune, sans imposer
  une présentation unique aux modèles. Les premiers helpers d’auteurs,
  affiliations et résumés sont définis dans le [contrat Article](docs/article-contract.md).
  Valider ensemble les signatures des nouvelles fonctions et leur recette ;
  inventorier et embarquer chaque dépendance réellement utilisée.
- Définir le manifeste du modèle : identité, description, mots-clés, type,
  fonction publique, paramètres, défauts, ressources et dépendances.
- Installer, choisir et retirer des modèles personnels ou d’organisation dans
  une bibliothèque persistante ; prévoir l’aperçu et la provenance.
- Définir les mises à jour et le remplacement d’un modèle embarqué dans un
  document, sans modification implicite de documents déjà rédigés.
- Documenter la création d’un modèle et proposer un exemple minimal par type ;
  donner des diagnostics utiles lorsque son contrat est invalide.
- Vérifier les conventions typographiques et les langues sur des exemples réels ;
  garder leur composition dans Typst et les règles de modèles.

**Choix à valider :** format des paquets installables, extensions du manifeste
`model.yaml` livré en B.1 et paramètres propres aux modèles,
polices et licences d’incorporation, import, stockage et politique de mise à jour.

**Accessibilité des exports :** viser PDF/UA-1, avec un profil PDF/A pour
l’archivage. Les standards de `typst_pdf` ne sont pas encore activés dans l’export
(options par défaut) ; les fixtures et l’article d’exemple compilent avec
`ua-1,a-2b` en ligne de commande. Activer les standards dans l’export, rendre le
texte alternatif des images obligatoire (voir C) et valider un corpus avec
veraPDF dans la CI : aucun outil externe n’a encore confirmé la conformité.
Compléter les contrôles automatiques par une recette humaine de chaque modèle :
ordre de lecture, descriptions pertinentes, structure des titres et des tableaux.
Un résultat veraPDF favorable ne vérifie que les critères automatisables.

**Sortie :** création d’un document utilisable avec chaque modèle livré ;
changement de modèle d’un même type sans altération du contenu ; archive et
export autonomes, compilables sans téléchargement de paquets ; export PDF/UA-1
validé sur la recette représentative de chaque modèle livré.

## E — Peaufiner l’interface et les outils de rédaction

- **E.1 — barre d’outils et palette adaptées au type — livré localement :** afficher
  uniquement les capacités du type de document actif, à partir du catalogue
  commun ; masquer les groupes devenus vides. Exemple : « Fin de lettre » et
  ses commandes P.-S./P.J. sont présents pour un courrier et absents pour un
  article. Une commande prise en charge par le type mais temporairement
  indisponible dans la sélection reste visible et désactivée. Actualiser la
  barre lors de la création et de l’ouverture d’un document, sans règles
  spécifiques dupliquées dans les boutons. Appliquer le même filtrage à la
  palette de commandes : ses propositions ne contiennent que les capacités du
  type actif, avec la même distinction entre absence du type et indisponibilité
  contextuelle. Les commandes générales de l’application restent accessibles.
- Revoir les parcours réels : accueil, création, en-tête, rédaction, recherche,
  sélection, correction, insertion, sauvegarde et export.
- Achever l’harmonisation des widgets et bandeaux : même cadre et mêmes règles
  de focus, validation, annulation et suppression ; contenus spécifiques par capacité.
- Étendre le [contrat clavier commun](docs/keyboard-navigation.md) aux nouveaux
  contrôles via leurs générateurs : listes, menus, boutons, formulaires et bandeaux.
- Renforcer les parcours clavier, les lecteurs d’écran, les contrastes, l’IME,
  les petits écrans et les états sans document ou sans résultat.
- Maquetter les deux accès validés **Projets** (catalogue global) et **Document**
  (navigation dans le projet actif), définis plus haut. Clarifier leur
  articulation avec bibliothèque globale, préférences et assistance, en
  conservant la place de l’éditeur et de l’aperçu. Valider la disposition
  détaillée et les parcours clavier, notamment la consultation d’une ancienne
  version en lecture seule et le retour explicite au travail courant.
- Définir l’ouverture de plusieurs documents (fenêtres ou onglets), l’impression
  et l’accès direct aux fichiers/PDF produits.
- Définir le déplacement/duplication de blocs, le collage structuré et le collage
  d’images ; traiter les incompatibilités sans perte silencieuse.
- Ajouter les raccourcis personnalisables et le choix de polices système en
  distinguant la police de lecture de l’éditeur de celles du modèle.
- Vérifier les langues des menus et dialogues natifs dans l’application installée,
  la couverture des glyphes et l’absence de corrections concurrentes avec Spellbook.
  **Problème reporté le 31 octobre 2026 :** sous macOS avec `npm start`, le menu
  de l’application est français mais les boutons du sélecteur de fichiers
  restent anglais. Reprendre le [diagnostic consigné](docs/native-localization-investigation.md)
  et comparer la solution éventuelle de `dbv-typst-editor` avant une nouvelle
  expérimentation. Les essais non concluants ont été retirés ; la traduction
  visible de ces boutons n’est pas considérée comme validée.
- Tenir un budget de latence de saisie et de rendu sur des documents représentatifs,
  en build natif optimisé ; réutiliser les mesures et références existantes.
  **Budget relatif (29 octobre 2026) :** la latence de bout en bout de l'aperçu
  doit rester comparable, de 1,5 à 2 fois en p95, à la compilation incrémentale
  de Typst seul sur le même document et la même machine (référence mesurée
  séparément pour le natif et pour Typst en WebAssembly). Y ajouter un plancher
  absolu pour les petits documents et suivre le surcoût de chaque étape (saisie,
  validation, génération, compilation, transport, affichage), pas seulement le total.

**Choix à valider :** les nouvelles interactions et maquettes, les préférences
réellement utiles, les priorités de raccourcis et les objectifs de performance.

Les auteurs et affiliations utilisent désormais le dialogue réutilisable validé,
centré sur un arrière-plan atténué et légèrement flouté. Le contrat de saisie,
les brouillons isolés et les contrôles de concurrence sont décrits dans
[`docs/metadata-dialogs.md`](docs/metadata-dialogs.md).

## F — Intégrer l’assistance ACP

L’orientation ACP reste retenue : un client commun aux agents dans l’application,
MCP pour les capacités documentaires et l’acceptation explicite des propositions.
La [découverte MCP progressive](docs/mcp-discovery.md), commune aux modes autonome
et rattaché, est en place : aperçu compact, index puis signatures et exemples à
la demande. Préserver ce parcours lors des ajouts de capacités ; une nouvelle
recette avec un agent sans contexte pourra en évaluer la clarté avant le parcours ACP.

- Maquetter et valider le parcours de conversation sur l’interface Tauri actuelle.
  Piste principale : panneau ouvert depuis la barre latérale, question de hauteur
  adaptative, réponse dessous et actions dans un pied stable.
- Réaliser le premier parcours avec Codex via codex-acp et le client Rust ACP ;
  vérifier ensuite le même parcours avec un second agent compatible.
- Automatiser le raccordement MCP privé au document courant. Afficher clairement
  le document et la sélection confiés, sans suivre ensuite le curseur humain.
- Distinguer masquer le panneau, interrompre une demande et terminer l’assistance ;
  préciser les erreurs, coupures et reprises sans renvoi automatique d’une demande.
- Conserver un seul lot proposé en attente, sa consultation, sa péremption et son
  acceptation/refus explicites. L’assistance ne doit pas déplacer le focus de rédaction.
- Représenter le lot soumis par une base figée et un candidat explicitement fondé
  sur cette base. Réutiliser la consultation/comparaison de Versions, avec les
  actions propres aux propositions. Les snapshots privés MCP ne deviennent ni
  des versions conservées ni une relecture J par leur seule création. Conserver
  garde stricte, commandes autorisées et autorisations de fichiers distinctes.
- Préciser installation, authentification et distribution des agents/adaptateurs ;
  annoncer uniquement les fonctions et droits effectivement disponibles par agent.

**Choix à valider :** maquette et clavier, contexte fourni, conversation par document
et agent, conservation de l’historique, permissions et modèle de distribution.
Les choix fonctionnels détaillés sont dans le [plan technique](ROADMAP-ARCHITECTURE.md#f-assistance-acp).

**Sortie :** discuter, reformuler une sélection et préparer un document structuré,
puis accepter/refuser un lot, avec au moins deux agents et sans configuration MCP manuelle.

**Cycle de vie des propositions restant à préciser :** la comparaison et les
reçus locaux sont livrés ; le [contrat courant](docs/a4-comparison-contract.md)
est conservé. F doit décider de leur conservation durable après décision,
des enrichissements de reçus utiles à ACP et du parcours de nouvelle proposition
sur une base actualisée. Aucun second moteur de validation ni droit implicite
de sauvegarde/export n’est à développer.

## G — Ajouter la collaboration distante

La collaboration entre fenêtres sert de base technique. Le réseau distant,
les droits et le cycle de vie du document partagé restent à construire.

- **G.1 — Partage ponctuel :** héberger une session depuis typstOgraphe pour deux
  ou trois personnes ; invitation temporaire/révocable, lecture ou édition,
  liste des participants et fin de session explicite.
- **G.2 — Partage sur Internet :** choisir un relais/tunnel ou une autre stratégie
  adaptée aux box et pare-feu. Un lien local ne devient pas public automatiquement.
- **G.3 — Service durable :** documents partagés sur un serveur auto-hébergeable
  d’organisation, accès même lorsque le poste de l’hôte est fermé.
- **G.4 — Fiabilité :** sauvegarde et autorité du document partagé, copies locales,
  travail hors connexion, reconnexion, annulation propre à chaque participant,
  ressources, modèles et versions de schéma cohérents. Définir l’autorité et
  les droits de création, restauration et suppression des snapshots ; restaurer
  un état doit être explicite et annoncé aux participants, avec traitement des
  éditions concurrentes et conservation de l’historique.
- **G.5 — Charge :** mesurer une centaine de collaborateurs répartis sur plusieurs
  documents, avec collages, images, coupures et reconnexions ; ne pas transformer
  une estimation de matériel en capacité garantie.

**Piste de serveur : Hocuspocus**, avec son connecteur Yjs. Évaluer son exécution
embarquée avec runtime, le déploiement sur serveur et l’authentification.
**Cloudflare reste une option à étudier**, pas une dépendance choisie : distinguer
relais d’accès au poste et hébergement d’un service persistant.

**Point d’attention : deux niveaux d’arbitrage.** La convergence CRDT ne garantit
pas la validité documentaire. Valider un candidat fusionné avec l’état confirmé
courant avant de l’admettre dans l’état d’autorité et de le diffuser : deux
éditions valides séparément peuvent devenir incohérentes après fusion.

- **Arbitre local (client) :** validation optimiste pendant la frappe, sans
  attendre le réseau ; elle garde l’édition immédiate et évite d’envoyer la
  plupart des mises à jour invalides. Elle n’a pas d’autorité sur les autres.
- **Arbitre d’autorité :** poste hôte en session ponctuelle, serveur en service
  durable. Il valide avant de diffuser ; les autres participants ne voient que
  l’état validé.

Les deux niveaux réutilisent le **même moteur Rust**, indépendant de Tauri :
`EditingSession` et `SharedSession` sont déjà dans `core`. Extraire seulement la
coordination encore liée à Tauri ; fichiers, fenêtres et transport restent des
adaptateurs. Le client desktop peut conserver sa validation native. Évaluer la
compilation `wasm32` et le coût du composant pour un client navigateur avant de
retenir son déploiement WebAssembly. La compilation Typst reste côté client.
Comparer un serveur Rust/Yrs à Hocuspocus raccordé à la validation Rust, sans
imposer un appel distant : module natif, WebAssembly ou processus séparé sont
à évaluer. Avec Hocuspocus, intercepter avant application du message, et non
dans un observateur du document déjà modifié.

**À concevoir : refus distant d’une mise à jour déjà appliquée localement.**
L’annulation Yjs produit une opération inverse ; elle ne retire pas une édition
refusée de l’historique comme si elle n’avait jamais existé. Prévoir la reconstruction de l’état local
à partir de l’état de l’arbitre d’autorité, la conservation du brouillon refusé
(comme pour le refus local) et l’information de l’utilisateur. Ce cas est
censé rester rare mais doit être correct. Suspendre les mises à jour dépendant
de la branche refusée jusqu’au rétablissement explicite de l’état confirmé.

**Choix à valider :** premier parcours local/Internet, hôte et disponibilité,
identités et droits, transport, stockage, confidentialité, coûts et maintenance.

## H — Préparer une première diffusion

**Lot Publication des applications — validé le 2 novembre 2026 :**

La chaîne H.1 est implémentée ; la [procédure et les vérifications locales](docs/releases.md)
sont disponibles. Sa première qualification complète des installateurs sur les
runners et le dépôt réel sur un nouveau tag restent à effectuer.

1. **H.1 — Installateurs et release en brouillon.** Sur un tag de version,
   attendre la validation complète du même commit, construire les paquets et les
   joindre à une seule release GitHub en brouillon. Première matrice : `.dmg`
   macOS universel (Intel et Apple Silicon), `.deb` et AppImage Linux x86_64,
   installateur NSIS Windows x64. Une relance explicite reprend le tag exact,
   sans le déplacer ni construire une autre révision sous le même nom.
   La publication reste manuelle ; une release publiée n'est pas modifiée par
   cette voie. Les premiers paquets ne sont pas signés/notarisés pour diffusion.
2. **H.2 — Vérification des paquets installés.** Contrôler ressources, polices,
   modèles et langues dans les paquets finaux, puis installation, lancement,
   ouverture d'un `.typsto` et export hors ligne sur les trois systèmes.
3. **H.3 — Canaux de diffusion.** Décider signature et notarisation, puis tap
   Homebrew et mises à jour. Actualiser le tap seulement après publication des
   fichiers définitifs, avec leurs empreintes ; distinguer signature du système
   et signature des mises à jour. Microsoft Store reste une piste distincte.

Réutiliser la chaîne Tauri et la validation existantes, sans refaire les suites
coûteuses pour chaque format de paquet. Les workflows de DBV Typst Editor
servent de référence ; son organisation autour de sidecars ne s'applique pas
à notre moteur Typst intégré en Rust.

**Versions après export PDF — à implémenter :** après un export réussi, proposer
« Créer une version de cet export », volontairement et sans modale. Capturer
l’état exact qui a produit le PDF même si la rédaction a continué. Aucun jalon
sur échec/annulation ; le quota de versions ne bloque jamais l’export.
La comparaison entre fichiers reste une piste ultérieure.

- Définir le périmètre de la première version utilisable et ses limites annoncées.
- Produire des paquets macOS, Windows et Linux, l’association .typsto, l’ouverture
  par un lien d’invitation et une identité d’application stable.
- Décider signature, notarisation, distribution, mises à jour et reprise sur échec.
- Généraliser les erreurs typées et leur présentation Fluent dans la langue de
  l’interface, avec repli anglais. Garder les détails techniques dans les diagnostics.
  L’ouverture des archives structurées ne doit pas masquer sa cause par un essai
  systématique de l’ancien lecteur.
- Faire les recettes natives des trois systèmes : installation, menus, dialogues,
  clavier/IME, presse-papiers, ouverture, fermeture et récupération.
- Maintenir la CI Rust/web, les tests de contrat et les rapports de couverture ;
  ajouter les recettes requises par les nouveaux chantiers, sans multiplier les
  tests qui ne vérifient que leur propre implémentation.
- Rédiger une documentation courte : démarrage, fichiers, modèles, assistance,
  collaboration, limites, confidentialité et dépannage ; garder le README bref.
- Préparer quelques essais avec des utilisateurs administratifs et SHS sur leurs
  vrais documents, puis traiter les blocages avant d’élargir le catalogue.

**Sortie :** une personne extérieure au projet installe l’application, produit
un document utile et retrouve son travail, sans lire le dépôt ni installer Rust/Node.

## I — Extensions à réexaminer après les premiers usages

- Correction grammaticale locale : orientation LanguageTool avec runtime Java
  embarqué, processus persistant et API HTTP officielle sur la boucle locale ;
  mesurer mémoire, démarrage et pertinence avant intégration.
- Conversion entre types : correspondances explicites et bilan préalable,
  sans suppression silencieuse des contenus incompatibles.
- Éditeur mathématique visuel, macros LaTeX et fonctions Typst personnalisées
  (saisie mathématique en ligne/bloc livrée en C.2), code, diagrammes et graphiques de données ;
  glossaire, acronymes et index selon les besoins des types livrés.
- Import de contenus externes et exports supplémentaires justifiés par les usages.

La reformulation par petit LLM local n’est pas réactivée par ce plan. Un créateur
graphique de types/modèles, les diaporamas et les posters ne font pas partie du
premier périmètre. Les propositions non retenues ne deviennent pas des engagements.

## J — Relecture par suggestions — articulée aux types, snapshots et à F

**Piste retenue le 28 octobre 2026, à maquetter avant tout codage.** Un mode
relecture n’édite pas le texte directement : le relecteur sélectionne une plage ou
place le curseur, choisit un outil (proposition, correction, remplacement,
suppression, commentaire/discussion) et saisit son texte. Chaque suggestion est
un **objet d’annotation ancré**, distinct du contenu ; l’accepter exécute une
commande documentaire ordinaire sous garde de révision, la refuser la retire
sans modifier le contenu documentaire partagé. Cette voie évite d’annuler des modifications
concurrentes dans le CRDT. Elle doit être imparfaite mais utile dès sa première
version, puis être peaufinée.

- Conserver les outils dédiés de relecture par annotations ; l’interface commune
  de comparaison d’A.4 vise les snapshots et les propositions MCP et ne remplace
  pas ces outils. Mutualiser les composants pertinents sans imposer la même
  présentation. La validation locale des propositions MCP réutilisera le moteur
  commun d’A.4, avec décisions groupées pour les opérations liées ; l’acceptation
  commune à A.4 conserve l’atomicité des groupes dépendants.
- **Relectures** est une entrée propre de la barre latérale, distincte de Versions.
  Son snapshot initial est un support interne protégé, absent des versions affichées
  et de leur quota de 100. Sa rétention relève de la relecture et des budgets de
  volume du projet. Ne pas remplacer les outils déjà définis ici par une simple
  consultation de snapshots, ni ajouter un bouton de comparaison non validé.
- Limiter le premier périmètre de relecture humaine aux suggestions de texte dans une zone (insertion,
  suppression, remplacement, commentaire). Les suggestions structurelles (blocs,
  tableaux, notes, citations) et le suivi de formatage viennent plus tard.
- Évaluer comme geste principal « sélectionner puis taper » créant directement
  une suggestion de remplacement dans un mode relecture clairement signalé ;
  les outils servent aux autres cas.
- L’aperçu ordinaire montre le document confirmé, dont les suggestions acceptées
  font déjà partie. Un aperçu candidat distinct et explicitement identifié peut
  appliquer les suggestions encore en attente sur un état dérivé, sans les
  accepter. Un aperçu marqué pour l’auteur reste à décider.
- Créer automatiquement un snapshot au démarrage d’une nouvelle relecture et
  le lui rattacher comme référence initiale, conservée sans modification pendant
  toute sa durée et protégée de la suppression tant qu’elle est en cours. Reprendre
  une relecture ne recrée pas ce snapshot ; aucun déclencheur heuristique n’est
  prévu. A.4 fournit cette référence durable et sa comparaison, tandis que les
  suggestions restent des objets distincts. Leur conservation, leur devenir après
  reprise de version et la rétention de la référence après clôture restent à
  valider ensemble dans J.
- La relecture humaine ne dépend pas de la livraison d’ACP : son contrat commun
  de suggestions peut être préparé indépendamment et raccordé ensuite aux agents.
- Une ancre déplacée peut suivre le texte ; une cible dont le contenu a changé
  doit être revérifiée avant acceptation, sans réapplication aveugle de la suggestion.

**Choix à valider :** stockage des suggestions (état partagé, `.typsto`, export
qui ignore les suggestions en attente), ancres devenues orphelines après édition de
l’auteur, suggestions qui se chevauchent, portée par participant ou partagée,
droits en collaboration, parcours de saisie sur maquette avec de vrais relecteurs.

**Sortie :** un article relu par deux personnes et un agent, suggestions
acceptées ou refusées, aperçu corrigé compilé, sans modification directe du texte.

## Prochain lot proposé

**C.4 — Bibliothèque bibliographique personnelle**, après qualification du lot
frontend / E.1 et de C.3 (gestionnaire complet, profils natifs, CAS 16).

1. Valider ensemble le stockage personnel, ses révisions, les échanges explicites
   avec les projets et la maquette Projet/Bibliothèque personnelle.
2. Réutiliser la crate, le corpus de rapprochement et le gestionnaire C.3 ;
   aucune seconde implémentation des identités ou de la fusion des notices.
3. Évaluer progressivement les modèles de D sur leur corpus, après E.1 ; les
   chantiers F (ACP) et J (relecture) conservent leurs validations propres.

Les tests locaux ne remplacent pas la qualification CI avant fusion ni la
recette utilisateur. Aucun choix encore ouvert de C.4 n’est implémenté d’avance.

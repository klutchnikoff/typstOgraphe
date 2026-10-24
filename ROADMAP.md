# Feuille de route de typstOgraphe

Document de travail établi le **21 octobre 2026**, à faire évoluer au fil des
décisions et des réalisations. Le POC est validé ; l’application continue à être
construite dans ce dépôt. Cette feuille de route ne constitue pas un calendrier.
Le [README](README.md) présente l’objectif et le lancement de l’application.

## Comment nous avançons

**À chaque étape, les choix principaux sont discutés et validés ensemble avant
leur implémentation.** L’accord sur cette feuille de route ne vaut pas validation
anticipée des interfaces, des dépendances ou des comportements encore à définir.

Pour chaque étape :

1. Choisir ensemble un périmètre limité et un résultat concret pour l’utilisateur.
2. Présenter les décisions nécessaires : comportement, interface ou maquette,
   conséquences sur les documents et principales options techniques.
3. Consigner les choix retenus ici ou dans une note liée, puis implémenter.
4. Vérifier le fonctionnement et les cas importants, faire un essai ensemble,
   puis actualiser l’état du chantier et choisir la suite.

Les détails ordinaires de mise en œuvre restent du ressort du développement.
Avant de construire une nouvelle brique, rechercher les outils existants et
évaluer leur entretien, leur adéquation et leur intégration. Réutiliser une
solution appropriée plutôt que réimplémenter ses fonctions.
Une découverte qui change sensiblement le périmètre ou les choix validés doit
revenir dans la discussion avant de poursuivre la partie concernée.

## Principes retenus

- Une application macOS, Windows et Linux, d’abord individuelle et locale.
- Un utilisateur rédige sans voir de code ; les créateurs de modèles travaillent
  en pur Typst, avec une interface publique simple et une implémentation libre.
- Les types sont intégrés en Rust : métadonnées, capacités et contrat des modèles.
- Le source Typst restreint reste la référence du contenu, avec des métadonnées
  séparées et un modèle embarqué dans un fichier unique `.typsto`.
- La présentation de l’éditeur reste cohérente entre les types et indépendante
  de la présentation finale, qui appartient au modèle.
- Les commandes publiques passent par un registre commun aux menus, à la palette,
  aux boutons et aux raccourcis, complété selon le type de document.
- La langue du document est indépendante de celle de l’interface. L’interface
  suit par défaut le système, avec l’anglais comme langue de repli.
- La composition typographique appartient à Typst et aux modèles. Nous vérifions
  les besoins non couverts avant d’ajouter des corrections dans l’éditeur.
- Le collaboratif reste une évolution possible, secondaire. Nous préservons la
  séparation entre contenu, opérations et interface, sans construire dès maintenant
  une infrastructure de collaboration.
- Aucun mécanisme de rétrocompatibilité n’est requis pendant cette phase
  préliminaire. Le format et les contrats peuvent évoluer pour rester simples.

## Point de départ au moment de l’établissement de la feuille de route

Les éléments suivants existent déjà ; leur présence ne signifie pas que tous
les cas d’usage sont aboutis :

- Éditeur visuel egui, rendu Typst réel, navigation dans les deux sens et
  surlignage de la sélection dans l’aperçu.
- Paragraphes, titres 1–3, gras, italique, liens, trois formes de listes,
  citations en bloc avec attribution, images avec légende et texte alternatif.
- Enregistrement et réouverture d’un fichier autonome, export PDF et sources Typst.
- Session documentaire et historique commun au texte, aux métadonnées et au modèle.
- Types Courrier et Compte-rendu de réunion, création vierge et premiers modèles.
- Import d’un modèle autonome dans le document courant et validation de son appel.
- Menus, palette, registre de commandes, préférences visuelles et internationalisation.
- Contrôles automatisés sur macOS, Windows et Linux ; les essais manuels Windows
  et Linux sont faits.

Au départ, la récupération après incident, la recherche, les correcteurs, les tableaux,
les notes, les références bibliographiques, les profils et la bibliothèque
persistante de modèles restent à développer.

### Suivi de réalisation

- **Architecture — phase 4 terminée le 24 octobre 2026.** Premier lot CLI : catalogue, inspection,
  vérification et exports sans fenêtre, avec protection des écritures concurrentes
  commune à l’application. Deuxième lot : création, `query`/`run`, cibles résolues
  dans la session et enregistrement explicite. Troisième lot : scripts JSON atomiques,
  avec cibles portables et sauvegarde finale facultative ; parcours courrier/P.-S./P.J.,
  exports puis réouverture native vérifiés. Dernier lot : rendu d’une page en PNG,
  retrait du diagnostic `--render-demo` et préparation des références par la CLI ;
  les huit captures restent strictement identiques. La phase suivante est MCP,
  dont les choix ont été validés : outils issus du registre, découverte documentée
  sans skill requis, modèles décrits par nom/description/mots-clés, session autonome
  et éditions gardées. **Phase 5 terminée le 24 octobre 2026 : serveur `rmcp` stdio livré**, avec
  découverte, éditions/lots et aperçu en mémoire. **Deuxième lot livré :** racines
  autorisées, ouvertures sous verrou, imports, sauvegarde explicite et export autonome.
  **Troisième lot livré :** fiches des capacités, exemples à références liées,
  schémas MCP vérifiés et cinq ressources, dont le document courant et les modèles.
  La [recette finale](docs/mcp-acceptance.md) valide l'inspecteur officiel et un agent
  sans skill ni lecture du dépôt jusqu'à l'export du courrier et sa réouverture.
  Deux corrections isolées portent sur le schéma des lots et la langue MCP par défaut ;
  le courrier réel devient un scénario de régression. **K.1 terminé le 24 octobre 2026 :**
  [inventaire des interactions](docs/interface-interactions.md) et
  [relevé des états et transitions](docs/interface-view-sync.md), sans changement
  de code. **Choix K.2 et interactions de phase 6 validés :** coordination pure,
  contextes d'édition séparés, concurrence stricte, conservation du focus et de
  la navigation, refus temporaire des mutations pendant IME/modalité.
  **K.2 livré :** [coordinateur de vue pur et ses tests](docs/view-sync.md), sans
  changement du branchement egui dans ce premier lot. **K.3 livré :**
  [branchement egui, parcours et captures](docs/view-sync-egui.md), avec 454 tests
  réussis et huit captures strictement identiques. Prochaine étape : documentation
  commune K.4. La phase 6
  réalisera l'assistance à la composition dans la session de l'application ;
  le transport et les modalités de rattachement restent à préciser.
  État de la CLI dans [le bilan de phase 4](docs/architecture-phase-4.md),
  décisions et lots MCP dans [celui de phase 5](docs/architecture-phase-5.md).

- **Étape 1 — Récupération : implémentée ; essai utilisateur fait.** Les choix
  validés et le périmètre livré sont précisés dans le chantier A ci-dessous.
- **Étape 2 — Rechercher/remplacer : implémentée ; essai utilisateur fait.**
  Les choix validés et le périmètre livré sont précisés dans le chantier B.
- **Étape 3 — Orthographe : implémentée ; essai utilisateur fait.** Spellbook,
  dictionnaires embarqués français/anglais, suggestions et dictionnaire personnel ;
  le périmètre livré est précisé dans le chantier B.
- **Étape 4 — Profil/signature et premier modèle personnel implémentés (D, H).**
  Le courrier français pour enveloppe DL est le seul modèle fourni. L'accueil
  démarre sans document ; création, sélecteurs et commandes suivent le catalogue
  intégré des types et modèles. Essai utilisateur et essai papier faits.
  Prochaine suite à définir ensemble : modèle de compte-rendu et premier périmètre
  de bibliothèque persistante (H, E).
- **Reformulation locale : essais terminés, piste mise de côté.** Le bilan est
  conservé dans le chantier B et le rapport lié.
- Les étapes 5 à 8 ne sont pas implémentées ; les types, premiers modèles et
  l’import ponctuel déjà présents constituent leur point de départ.

## Priorités proposées

Ces priorités servent à choisir les prochaines étapes ensemble. Elles ne fixent
pas encore l’ordre de chaque fonctionnalité au sein d’un chantier.

| Priorité | But | Chantiers concernés |
| --- | --- | --- |
| **P0 — Protection du travail** | Pouvoir reprendre son travail après un incident | A ; vérifications transversales I |
| **P1 — Usage quotidien utile** | Rédiger, corriger et produire un vrai courrier ou compte-rendu | B, C pour les tableaux et notes, D, premier périmètre E et H |
| **P2 — Organisation et documents longs** | Retrouver ses documents et structurer des contenus plus riches | G, suite C et E, rapport puis mémoire dans H |
| **P3 — Extensions** | Convertir entre types et approfondir l’assistance | F, correction grammaticale avancée dans B, capacités spécialisées dans C |

L’accessibilité, les performances, les traductions et les vérifications
multiplateformes accompagnent chaque étape ; elles ne sont pas reportées en bloc
à la fin du projet.

## A. Fiabilité et cycle de vie des documents — P0, puis P1/P2

**Première étape implémentée, choix validés le 21 octobre 2026 :**

- Copie locale indépendante, toutes les cinq secondes si le document est modifié,
  en arrière-plan, y compris avant son premier enregistrement.
- Texte, métadonnées, modèle et images conservés ; historique non conservé.
- Au redémarrage : Récupérer, Plus tard ou Supprimer la copie ; conservation sans
  expiration automatique et accès ultérieur par le menu Fichier ou la palette.
- Reprise comme document récupéré à enregistrer séparément ; fichier original
  jamais remplacé implicitement.
- Retrait après enregistrement réussi ou abandon explicite ; fermeture annulée
  et échec d’enregistrement préservent la copie.
- `atomic-write-file` pour les écritures atomiques communes aux documents et à
  la récupération ; eframe pour le dossier applicatif et les échéances.
- Vérifications locales : sortie brutale d’un processus, session encore ouverte,
  copie illisible, erreurs d’écriture et parcours d’interface. L’essai utilisateur
  et les essais natifs Windows/Linux sont faits.

**Objectif :** conserver le travail, reprendre un document et comprendre les
éventuels incidents.

- **P0 :** récupération des documents modifiés après arrêt inattendu, y compris
  d’un document qui n’a encore jamais été enregistré.
- **P0 :** restauration proposée au redémarrage, erreurs d’écriture explicites,
  vérification des scénarios d’échec et de fermeture.
- **P1 :** enregistrement automatique configurable, en distinguant sauvegarde du
  fichier et copie de récupération.
- **P1 :** détection des modifications externes et gestion des conflits d’accès.
- **P1 :** export PDF et Typst plus direct, impression et accès au fichier enregistré.
- **P2 :** plusieurs documents ouverts, avec une gestion claire des modifications.

**Choix ensemble :** emplacement et fréquence de récupération, durée de conservation,
comportement de restauration, éventuelle sauvegarde automatique du fichier,
réaction aux modifications externes et fenêtres ou onglets pour plusieurs documents.

**Validation :** une interruption ou une erreur d’écriture ne remplace pas le
dernier fichier valide ; la reprise est compréhensible et le travail récupéré
conserve texte, métadonnées, modèle et ressources.

## B. Outils de rédaction — P1, puis P2/P3

**Première étape implémentée, choix validés le 21 octobre 2026 :**

- Barre compacte au-dessus du texte, recherche exacte et instantanée dans le corps
  entier ; casse ignorée par défaut, accents significatifs, option de casse.
- Toutes les occurrences surlignées ; chaque clic remplace l’unique sélection
  native. L’ancienne occurrence conserve sa surbrillance de recherche.
- Navigation suivante/précédente et réutilisation de la synchronisation existante
  avec l’aperçu. Fermer la barre conserve la sélection et retire les surbrillances.
- Remplacer l’occurrence active et Tout remplacer ; historique commun et une
  seule transaction pour le remplacement global, style du premier caractère.
- Corps, titres, descriptions, légendes et attributions inclus ; métadonnées,
  textes alternatifs et contenus du modèle exclus.
- Réutilisation de `regex` pour la recherche Unicode strictement littérale.
- Aucun mode approximatif, aucune regex utilisateur ni sélection discontinue.
  Recherche limitée à une ligne ; restriction à la sélection et mots entiers
  restent des extensions éventuelles à discuter, sans les ajouter par défaut.

**Objectif :** faciliter la saisie et la révision sans exposer le source.

**Orthographe implémentée, choix validés le 21 octobre 2026 — essai utilisateur fait :**

- Spellbook avec les dictionnaires Hunspell embarqués français classique 7.7 et
  anglais américain de LibreOffice ; aucune transmission de texte ni compte.
- Analyse en arrière-plan après une pause de saisie, cache des lignes inchangées,
  positions Unicode et rejet des résultats et suggestions périmés.
- Soulignement ondulé dans l’éditeur, suggestions au clic droit ou depuis les
  commandes d’Édition et la palette ; navigation entre les mots signalés.
- Correction explicite conservant les styles, avec l’historique commun.
- Mots ignorés par document et langue dans ses métadonnées ; dictionnaire
  personnel par langue dans les préférences, avec possibilité de retirer un mot.
- Corps et annotations vérifiés ; métadonnées, source masqué et destinations des
  liens exclus. Les adresses web, courriels et identifiants numériques sont ignorés.
- La langue suit le document. Les autres langues de composition restent
  disponibles ; l’absence de dictionnaire est indiquée, sans repli sur le français.

- **P1 :** rechercher/remplacer dans le document ou la sélection, recherche
  suivante/précédente, casse et mots entiers, annulation des remplacements.
- **P1 :** correction orthographique selon la langue du document, suggestions,
  mots ignorés et dictionnaire personnel.
- **P1/P2 :** copier/coller structuré pour les styles et blocs pris en charge,
  avec une option pour coller du texte seul ; collage d’images.
- **P2 :** harmoniser la présentation des citations, images, P.-S. et P.J. dans
  l’éditeur : cadres, fonds, bordures, espacements, libellés et textes indicatifs.
  Factoriser les composants de présentation et les comportements réellement
  communs (focus, navigation, défilement et annotations facultatives), en
  conservant les dispositions et interactions propres à chaque bloc. Réutiliser
  la gestion commune des annotations déjà présente. Valider les choix visuels
  ensemble avant l’implémentation ; vérifier les thèmes clair/sombre, les curseurs,
  les sélections et les lignes repliées. Le rendu final reste défini par le modèle.
  **À entreprendre après la phase 2 du chantier J** (commandes strictes) : elle
  migre vers des commandes les annotations, le focus, la navigation et les images,
  que cette harmonisation touche aussi ; les mener ensemble obligerait à refaire
  l’un des deux travaux.
- **P2 :** déplacer et dupliquer des paragraphes ou des blocs, navigation par titres,
  affichage facultatif des caractères invisibles.
- **P2 :** personnalisation persistante des raccourcis dans les préférences et
  accès aux caractères utiles lorsque la saisie ordinaire ne suffit pas.
- **P3 :** correction grammaticale et suggestions de rédaction, avec acceptation
  explicite ; le texte de l’utilisateur ne doit pas être réécrit silencieusement.
  **Orientation technique retenue le 24 octobre 2026 :** LanguageTool local,
  exécuté dans un processus Java persistant piloté par Rust, avec son serveur
  HTTP officiel limité à la boucle locale. Le runtime Java et les JAR seront
  livrés dans le paquet d’application ; aucune installation Java ni compte
  utilisateur requis. Pas d’extraction de l’ensemble depuis `include_bytes!`.
  Le processus démarrera à la demande, restera en arrière-plan pendant l’usage
  et sera supervisé (délais, arrêt propre, reprise après incident).
  Commencer par un essai français : qualité des règles locales, démarrage froid,
  latence à chaud, mémoire réelle et poids de distribution. Réduire ensuite le
  runtime avec `jlink` et les dépendances de langues sélectionnées ; aucun budget
  de taille/mémoire n’est encore promis. Spellbook reste chargé de l’orthographe
  immédiate ; éviter les diagnostics en double. La langue suit le document.
  Transmettre le texte rédigé et des cibles gardées ; convertir explicitement les
  offsets UTF-16 de Java en positions Unicode du moteur, ignorer les réponses
  périmées et appliquer une correction acceptée via une transaction annulable.
  Versionner Java, LanguageTool et leurs dépendances dans l’inventaire des
  composants. L’essai ne suppose ni règles d’IA cloud ni réécriture automatique.
  Cette orientation ne lance aucune implémentation dans la phase 2 du chantier J.
- **P3, piste mise de côté après essais :** reformulation d’une sélection avec
  un petit modèle local, chantier distinct de l’orthographe. Les
  [essais du 21 octobre 2026](experiments/reformulation/report.html) comparent
  CATIE FLAN-T5 et Qwen2.5 1,5B/3B (qualité, latence et mémoire), puis plusieurs
  consignes sur un courrier fautif fourni par l’utilisateur. Le dernier ajout
  au prompt réduit l’emphase du 3B, mais des maladresses et des écarts de
  fidélité subsistent ; le gain est insuffisant pour retenir la fonction.
  Aucun modèle n’est intégré. Les scripts, mesures et sorties brutes sont
  conservés ; une éventuelle reprise sera décidée ensemble.

**Choix ensemble :** interface et périmètre de recherche (corps, annotations,
métadonnées), présentation des suggestions, langues et dictionnaires,
fonctionnement local ou service facultatif pour les correcteurs, formats de collage.
Tout recours à un service externe doit faire l’objet d’un choix explicite.

**Validation :** les opérations respectent Unicode, les frontières des structures,
les mises en forme et l’historique ; le correcteur suit la langue du document,
indépendamment de la langue de l’interface.

## C. Capacités documentaires Typst — P1 à P3

**Objectif :** proposer des structures simples à éditer, composées par le modèle.

- **P1 :** tableaux simples : en-tête, lignes et colonnes, édition au clavier,
  ajout/suppression, alignement et légende facultative.
- **P1 :** notes de bas de page : insertion, édition, suppression et navigation
  entre l’appel et le contenu de la note.
- **P2 :** citations bibliographiques et bibliographie, distinctes des citations
  en bloc déjà présentes ; import et conservation des références nécessaires.
- **P2 :** renvois internes vers titres, figures, tableaux et autres éléments
  identifiables ; numérotation calculée sans saisie manuelle.
- **P2 :** table des matières et, selon les types, listes des figures et tableaux.
- **P2/P3 :** saut de page explicite et autres contrôles de composition réellement
  nécessaires, à exposer avec parcimonie.
- **P3 :** équations en ligne et en bloc, blocs de code et structures spécialisées
  selon les besoins des types scientifiques ou techniques.

### Décision sur le modèle d’édition — validée le 22 octobre 2026

L’orientation retenue après le prototype du chantier J est de décrire les blocs et
cellules explicitement **en mémoire**, avec des zones de texte réutilisant la
`Projection` et la saisie native d’egui. Typst reste la référence du contenu et du
stockage ; la structure en mémoire est reconstruite depuis le source et sérialisée
en Typst. Elle ne constitue pas un second format documentaire.

- **État actuel :** la production utilise encore le tampon plat du `TextEdit`, ses
  styles par caractère et ses types par ligne. La migration structurelle relève
  de la phase 2 du chantier J, avant le gel des paramètres publics du moteur.
- **Prototype réalisé :** grille simple, navigation, sélection, lignes/colonnes et
  historique ; six essais ciblés, dont la composition IME via les événements egui.
  L’utilisateur a confirmé les accents et touches mortes sur macOS. Le chemin
  d’une véritable méthode IME du système reste à couvrir dans les essais multilingues.
- **Conséquence pour les commandes :** prévoir des cibles par zone de texte et
  position Unicode, avec validation structurelle avant publication. Les identifiants,
  leur durée de vie et la sélection entre zones sont à préciser ensemble en 2.1.
- **Limites observées :** un remplacement natif peut enlever une frontière entre
  cellules ; la normalisation des paragraphes ne convient pas à toutes les cellules.
  Le tampon unique protégé reste une alternative examinée, mais l’orientation
  choisie est celle des blocs/cellules explicites.
- **Éléments à instruire ensuite :** migration des structures existantes, aller-retour
  du source, sélection, historique, tests aléatoires (dont les sentinelles collées)
  et essais avec des méthodes de saisie du système.
- **Critères pour rouvrir le choix du composant d’édition :** échecs de saisie ou
  limites de présentation impossibles à corriger avec egui ; besoin de co-édition
  en temps réel ou priorité à une version web/Nextcloud. Un éditeur Rust propre
  ou une vue web restent des possibilités à réexaminer à ce moment-là.

La livraison des tableaux à l’utilisateur reste à l’étape 5. Voir le
[bilan du diagnostic](experiments/editor-model/README.md).

### Capacités communes et activation par type

Une capacité doit être implémentée une seule fois et réutilisée dans les types
qui la proposent. Les commandes, paramètres et éditions visuelles restent partagés
côté Rust ; l’activation par type reste à préciser pour les prochaines structures.

**Convention Typst retenue :** chaque capacité ayant une logique commune ou un
paquet externe fournit une interface stable dans un fichier dédié sous
`crates/core/templates/capabilities/`. Les modèles importent cette interface et choisissent
sa présentation. Seul l’adaptateur connaît le paquet et son API : une mise à jour
ou un remplacement du paquet doit conserver le contrat public et ses tests.
Les fonctions natives de Typst (`strong`, `emph`, `quote`, `figure`, etc.) restent
les primitives de composition ; cette couche accueille les règles propres à
l’application et les adaptations nécessaires, sans doubler le moteur Typst.

**Convention de nommage :** les identifiants des capacités, leurs fichiers,
fonctions Typst, paramètres publics et commandes associées portent des noms
anglais ou latins. Utiliser des identifiants ASCII en minuscules, avec des tirets
pour séparer les mots (`kebab-case`) côté Typst. Les libellés et les messages
restent traduits dans les catalogues de l’interface ; la documentation peut
être rédigée en français. Exemples : `format-date`, `postscript`, `attachment`,
`letter-notes(postscript: ..., attachments: ...)`.

**Premier adaptateur implémenté : Dates.**
`capabilities/dates.typ` expose `format-date(value, lang: "fr") -> content`.
Il accepte une date ISO fixe, un `datetime` ou un libellé libre. Il compose une
date longue selon la langue, conserve le « 1er » français en exposant et ne
consulte jamais l’horloge. Le modèle de courrier utilise cette interface ; il ne
connaît plus Datify. Les paramètres supplémentaires seront choisis ensemble
lorsqu’un besoin concret de modèle les justifiera.

`crates/core/templates/dependencies.json` est l’inventaire de référence des capacités et des
paquets : signature, point d’entrée, version, licence, archive, empreinte SHA-256
et dépendances transitives. Le chargement s’appuie sur cet inventaire, sans
condition Rust spécifique à Datify. Chaque document et export reçoit son propre
`dependencies.json`, limité aux capacités et paquets effectivement embarqués.
Le wrapper, les paquets et les données associées sont copiés avec le modèle.

**Capacité Courrier implémentée : Post-scriptum et pièces jointes.**
Deux commandes propres au type Courrier, disponibles dans Insertion, la palette
et la barre d’outils. Un cadre discret distingue le P.-S. ; un second rassemble
les mentions de P.J., sans incorporer de fichiers. Le contenu conserve le gras,
l’italique, les liens et la synchronisation avec l’aperçu. Entrée ajoute une
ligne ; sur une ligne vide elle quitte le bloc. Tab permet également d’en sortir.
Le bouton P.-S. rejoint le bloc existant, ou le crée avant les P.J. ; le bouton
P.J. ajoute une mention en fin de liste.

L’interface Typst est `postscript(body)` et `attachment(body)`, fournie par
`capabilities/correspondence.typ`, sans paquet externe. Le source ajoute un
import explicite de ces deux fonctions depuis le modèle, masqué dans l’éditeur.
Les modèles du type Courrier doivent exposer ces fonctions lorsqu’elles sont
utilisées ; le modèle fourni les réexporte depuis cet adaptateur. Son point
`letter-notes(postscript: ..., attachments: ...)` collecte les contenus et
applique les deux présentations données par le modèle, après la signature.
Les blocs vides ne produisent aucune mention dans le rendu. Les interfaces,
l’archivage, l’export autonome et les sélections restent testés ensemble.

Pour toute nouvelle capacité : convenir du contrat public minimal, écrire son
adaptateur Typst si nécessaire, déclarer ses dépendances, puis vérifier le même
contrat dans les types concernés, l’archivage et la compilation hors ligne.

| Niveau | Responsabilité |
| --- | --- |
| Application | Commandes, paramètres, édition visuelle et production du source Typst de la capacité |
| Type intégré | Capacités disponibles, métadonnées et contrat à respecter par les modèles |
| Adaptateur Typst commun | Interface stable de la capacité, règles communes et appel aux paquets privés |
| Modèle Typst | Présentation, numérotation, valeurs par défaut et personnalisations autorisées |
| Document | Contenu et paramètres propres à son usage |

Une note ou une citation bibliographique doit donc avoir la même interface dans
les types où elle est présente, avec un rendu défini par chaque modèle. Ajouter
une capacité ne signifie pas la rendre disponible dans tous les types.

**Choix ensemble :** liste des capacités par type, paramètres publics minimaux,
édition des structures imbriquées, numérotation et options laissées au modèle.
Pour les tableaux : limites initiales, cellules riches ou simples, fusions éventuelles.
Pour la bibliographie : format des références, saisie/import et éventuelle intégration
à un gestionnaire de références. Pour les équations : mode de saisie accessible.

**Validation :** une structure s’édite, se sélectionne, s’annule, se sauvegarde et
se rouvre sans perte ; son aperçu et ses exports respectent le modèle. La
synchronisation doit prendre en compte les nouveaux contenus.

## D. Profils personnels et signatures — P1

**Objectif :** créer un document avec ses coordonnées sans les ressaisir.

**Première étape implémentée, choix validés le 21 octobre 2026 — essai utilisateur fait :**

- Un profil global dans les préférences : nom, organisme, fonction, adresse
  postale, courriel, téléphone et image de signature facultative (PNG/JPEG/SVG).
- L’image est copiée dans le dossier applicatif ; déplacer ou retirer son
  original ne change pas la signature conservée par l’application.
- Chaque nouveau courrier reçoit une copie des coordonnées et de l’image dans
  son archive autonome. Le compte-rendu reprend le nom comme rédacteur.
- Les documents déjà ouverts ou enregistrés ne sont pas actualisés quand le
  profil change. Les champs d’expéditeur et la signature restent modifiables
  par courrier ; remplacement et retrait participent à son historique commun.
- Dans les paramètres d'expéditeur, « Signature du profil » permet de reprendre
  à la demande l'image actuelle des préférences, après remplacement ou retrait.
  La copie reste indépendante et l'action s'annule ; sans image disponible dans
  le profil, le bouton est désactivé. Reprendre une image déjà identique ne
  modifie pas le courrier et ne crée pas de ressource supplémentaire.
- Sans image de signature, le modèle de courrier réserve une zone blanche de
  14 mm de haut pour signer à la main, au-dessus du nom et de la fonction.
  Le bloc reste indivisible pour conserver cet espace avec le nom en fin de page.
- Le modèle Typst choisit la place et la présentation de la signature. Le
  contrat Courrier ajoute `organisme`, `adresse_expediteur`, `telephone` et
  `signature` (chemin embarqué ou `none`). Le format `.typsto` passe en version 7,
  sans migration, conformément au choix de ne pas maintenir de rétrocompatibilité.
- Préférences réparties en trois rubriques : Profil personnel, Apparence,
  Orthographe. Enregistrement après modification, avec 500 ms de stabilisation
  de saisie, et à la fermeture des préférences ou de l’application. Aucune
  écriture périodique des préférences ; la récupération documentaire conserve
  son échéance indépendante de cinq secondes.
- Vérifications : indépendance profil/document, image originale absente,
  annulation/rétablissement, conservation des seules ressources utilisées,
  réouverture/export PDF et captures natives isolées des réglages réels.

**Suites à discuter :** plusieurs identités et logo. Le premier profil et les
copies indépendantes dans les documents sont désormais retenus et implémentés.

- Profil personnel, puis plusieurs identités : personnelle, professionnelle,
  associative, par exemple.
- Coordonnées, fonction, organisme, logo et image de signature.
- Valeurs proposées à la création et modifiables dans les paramètres du document.
- Ressources nécessaires copiées dans le document pour préserver son autonomie.

**Choix ensemble :** champs, nombre de profils au premier périmètre, sélection du
profil, insertion de signature et comportement lors d’un changement d’identité.
La copie des valeurs dans le document, sans mise à jour rétroactive après une
modification du profil, est validée pour le premier périmètre.

La signature envisagée ici est une image ; une signature électronique
cryptographique constituerait un chantier distinct.

## E. Bibliothèque persistante de modèles — P1, puis P2

**Objectif :** réutiliser des modèles personnels ou d’organisation au-delà d’un
seul document. L’import dans le document courant existe déjà.

**Socle intégré implémenté :** le [catalogue](crates/core/src/document_types/models.rs)
associe chaque modèle à son type, son source Typst, ses libellés et sa commande.
Les types de la création viennent de `DocumentType::ALL` ; un type sans modèle
est affiché indisponible. Les modèles proposés à la création, dans les paramètres
et dans la palette viennent de ce catalogue, avec le premier comme défaut.
Ajouter une entrée ne nécessite pas de modifier ces interfaces. Les anciens
modèles fictifs ont été retirés ; le type Compte-rendu reste intégré et attend
son prochain modèle. Ce catalogue fourni avec l'application précède la
bibliothèque installable et persistante décrite ci-dessous.

**Polices — orientation validée le 22 octobre 2026 :** conserver un socle libre
embarqué à large couverture, puis permettre le choix de polices système.
Le catalogue partagé fournit les fontes latines et mathématiques de `typst-assets`
et le secours Noto Sans CJK 2.004 en normal et gras. L’éditeur et l’aperçu utilisent
ces données hors ligne ; les exports de sources joignent les faces CJK utilisées
et leur licence. Leur provenance est dans [le catalogue des fontes](crates/core/assets/fonts/noto-cjk/README.md).

Le sélecteur de polices système reste à réaliser. Distinguer la préférence de
lecture de l’éditeur des polices de présentation choisies par le modèle ; définir
leur identification, les droits d’incorporation, le transport des fontes utiles
et le comportement en cas d’absence, avant de dépendre d’une police locale pour
un document d’organisation.

- **P1 :** installer, retrouver, choisir et retirer un modèle ; description,
  type associé, provenance et modèle par défaut.
- **P1 :** importer ses ressources et vérifier son contrat avant utilisation.
- **Implémenté pour le modèle fourni :** copie autonome des dépendances Typst.
  Datify 1.3.0 et Datify-core 2.1.0 sont embarqués avec toutes leurs locales et
  licences. Imports relatifs, aucun téléchargement ni cache externe pendant le
  rendu. Les `.typsto` (format 9), copies de récupération et exports Typst
  conservent ces fichiers, l’adaptateur et l’inventaire, y compris les langues
  non utilisées actuellement.
  Les dépendances suivent le modèle dans l’historique ; les images gardent
  leur mécanisme distinct de suppression des ressources inutilisées.
  Provenance et modification d’import dans `crates/core/templates/packages/PROVENANCE.md`.
  Vérifié : réouverture avec les données propres à l’archive, changement de langue,
  annulation d’un changement de modèle et recompilation d’un export déplacé avec
  le CLI Typst, répertoires de paquets et de cache vides.
- **P1 :** généraliser l’import des ressources aux modèles installables.
- **P2 :** aperçu, favoris, variantes, remplacement et gestion des mises à jour.
- **P2 :** documentation pour les créateurs, exemple minimal par type et diagnostics
  compréhensibles lors d’un contrat invalide.

**Choix ensemble :** format de distribution des modèles et ressources, stockage
local, identification des versions, polices et imports autorisés, comportement
lors d’une mise à jour. Proposition : mettre à jour un modèle installé ne change
les documents existants que sur demande.

**Validation typographique :** vérifier des exemples concrets selon les langues
(guillemets, ponctuation double, espaces, nombres et unités). Utiliser Typst et
des règles de modèle réutilisables ; compléter l’éditeur uniquement pour des
besoins non couverts identifiés. Ne pas recréer un moteur de composition en Rust.

## F. Conversion entre types — P3

**Objectif :** réutiliser un contenu dans un autre cadre documentaire.

- Distinguer le changement de modèle au sein du même type, déjà possible, de la
  conversion vers un autre type.
- Définir les correspondances entre métadonnées et conserver les capacités communes.
- Expliquer les structures non prises en charge par la cible et proposer leur
  transformation sans suppression silencieuse.
- Présenter un bilan avant validation.

**Choix ensemble :** premiers couples de types convertibles, correspondances,
traitement des contenus incompatibles et création d’un nouveau document plutôt
que remplacement de l’original. Une conversion sans perte n’est pas garantie
pour tous les couples de types.

**Dépendances :** capacités des types et métadonnées suffisamment stabilisées ;
au moins deux usages réels permettant de justifier une conversion.

## G. Barre latérale et organisation — P2

**Objectif :** retrouver rapidement ses documents et les réglages du logiciel.

- Documents récents, favoris, création et accès au fichier ou dossier d’origine.
- Dossiers de travail ou projets légers, sans imposer la gestion d’un projet informatique.
- Accès à la bibliothèque de modèles, aux profils et aux préférences globales.
- Plan du document et navigation par titres, dans un contexte clairement identifiable.
- Panneau repliable pour préserver la place de rédaction et d’aperçu.

**Choix ensemble :** sens de « projet », regroupement des documents, distinction
entre navigation globale et plan du document, maquette et comportement sur petite
fenêtre. L’organisation par dossiers ou projets reste à décider.

## H. Types et modèles utiles — P1, puis P2

**Objectif :** disposer de quelques parcours complets avec un modèle utilisable,
au-delà des démonstrations fictives.

- **P1 : Courrier**, avec un modèle personnel utile et un modèle professionnel
  ou administratif ; raccordement au profil et à la signature.
- **Courrier personnel : présentation validée et modèle intégré.** La présentation retenue
  est composée en pur Typst dans [le modèle](crates/core/templates/courrier-personnel.typ),
  avec [un exemple autonome](crates/core/examples/courrier-personnel.typ). Cible : A4 portrait,
  plis à 99/198 mm, enveloppe DL 110 × 220 mm, fenêtre 100 × 35 mm à 20 mm du bas
  et de droite ([GPV 60013 / ES35F110](https://myprint.gpvfrance.fr/products/60013-boite-de-500-enveloppes-blanches-dl-110x220-90-g-m2-fenetre-35x100-bande-de-protection-es35f110)).
  Zone d'adresse de 82 × 18 mm, centrée verticalement à x=104/y=58 mm ; marges
  pour le jeu théorique de la feuille pliée. Expéditeur à gauche, date sous
  l'adresse, objet/référence facultatifs, signature à droite et pagination à
  partir du deuxième feuillet. Composition noire en Libertinus Serif 11 pt,
  champs vides masqués, repères de pliage discrets. Une adresse qui déborde est
  signalée. La maquette se compile dans le moteur embarqué et dispose d'un mode
  de contrôle avec fenêtre et plis. Il est proposé à la création et dans les
  paramètres du courrier. L'A4 portrait est imposé par le modèle dans l'interface.
  L'accueil ne crée aucun document et Fermer y revient, avec confirmation des
  modifications éventuelles. Validations faites : essai utilisateur et essai
  papier à 100 % dans une enveloppe réelle.
- **En-tête du courrier :** Général, Destinataire (identité/adresse), Informations
  (objet, référence, lieu, date) et Expéditeur. La date reste dans Informations ;
  son format relève du modèle, sans onglet de réglages typographiques.
  « Aujourd'hui » insère la date locale calculée par Chrono ; `today`,
  `aujourd'hui` et `aujourd’hui` sont des raccourcis de saisie résolus aussitôt en
  date fixe, jamais des valeurs évolutives. Les dates numériques reconnues
  (`JJ/MM/AAAA` ou ISO) sont conservées en ISO ; les libellés libres restent
  possibles. Le modèle personnel appelle l’adaptateur commun `format-date` pour
  la présentation locale des dates ISO. Cet adaptateur utilise Datify en privé
  (format `long`), avec l’ordre et la ponctuation de chaque langue et la règle
  « 1er » en exposant en français, sans table manuelle des mois. L’annulation, l’enregistrement et la réouverture conservent
  la date choisie. Les dépendances locales complètes assurent le rendu hors ligne
  et l’autonomie des exports (voir E).
- **P1 : Compte-rendu de réunion**, avec un modèle utilisable à titre personnel :
  intitulé, date, lieu/visioconférence, rédacteur, participants, personnes excusées
  et corps libre. Ces métadonnées ont déjà été validées et sont implémentées.
- **P2 : Note ou rapport court**, pour éprouver les tableaux, notes et renvois.
- **P2 : Mémoire ou rapport long**, après les références bibliographiques, renvois,
  sommaire et vérifications sur des documents longs.

« Personnel », « professionnel » ou « institutionnel » peut désigner un modèle
d’un même type. Nous ne créons un nouveau type que si ses métadonnées, ses
capacités ou son contrat le justifient.

**Choix ensemble :** usage précis, métadonnées, champs obligatoires, capacités,
paramètres publics et maquette du modèle pour chaque type. Le périmètre du
mémoire et sa proximité avec le rapport restent à définir.

**Validation :** créer un document vierge, le rédiger, le reprendre, changer de
modèle compatible et produire un PDF utilisable ; les modèles fonctionnent sans
coordonnées ni ressources fictives imposées.

## I. Qualité transversale — à chaque étape

- Couverture mesurée en CI avec `cargo-llvm-cov` sous Linux : rapport HTML/LCOV,
  résumé et badge des lignes exécutées, hors code des tests et dépendances.
  Le premier résultat sert de référence ; un éventuel seuil reste à choisir.
- Vérifications adaptées aux changements : contenu Unicode, aller-retour du
  fichier, historique, compilation Typst et export.
- Essais manuels macOS, Windows et Linux : menus, dialogues, presse-papiers,
  claviers, saisie IME et fermeture. La CI ne remplace pas ces essais. Les accents
  et touches mortes sont confirmés sur macOS pendant la phase 1 ; compléter le
  test de composition avec une véritable méthode IME lors des essais multilingues.
- Accessibilité : noms des commandes, navigation clavier, focus, contraste,
  lecteurs d’écran et textes alternatifs ; identifier les limites d’egui.
- Traductions maintenues avec les nouvelles fonctions ; séparation entre langues
  de l’interface, du document et du correcteur.
- Mesures sur documents représentatifs : latence de saisie, compilation,
  défilement, mémoire et traitement des images ; optimiser selon les résultats.
- Préparation d’une distribution installable : paquets par système, identité
  de l’application, association `.typsto`, puis signature et mises à jour selon
  le mode de diffusion choisi ensemble.

## J. Architecture, commandes, CLI et MCP — avant l’étape 5

**État :** plan établi le 21 octobre 2026, décisions D1 à D20 validées après relecture ;
phase 0 terminée : tests et Clippy verts, couverture mesurée, huit captures
reproductibles ([bilan](docs/architecture-phase-0.md)).
Phase 1 terminée : rangement, prototype de tableau et orientation vers des
blocs/cellules explicites en mémoire validés ([bilan](docs/architecture-phase-1.md)).
Accents et touches mortes confirmés sur macOS ; l’essai d’une méthode IME réelle
reste dans les vérifications multilingues du chantier I.
Le détail, les tâches et les critères de sortie sont dans
[ROADMAP-ARCHITECTURE.md](ROADMAP-ARCHITECTURE.md).
Phase 2.1 : [inventaire des commandes et cibles](docs/commands-inventory.md) préparé ;
choix V1 (cibles et identités), V2 (sélections entre structures), V3 (appels communs
aux interfaces) et V4 (saisie et préédition) validés le 22 octobre 2026.
V5 (réglages globaux hors du premier périmètre CLI/MCP) et V6 (création, dates,
exports) validés également ; implémentation autorisée, en commençant par les blocs
et zones de texte en mémoire.
Premiers lots réalisés : blocs et zones avec identités, aller-retour vers le
sérialiseur, positions logiques et gardes de révision, restauration des sélections
par l’historique. Frontières de zones protégées et voie commune de validation,
normalisation et publication des éditions en place. La préédition IME reste
isolée entre frames, avec une publication unique à validation, vérifiée par les
événements egui et une saisie japonaise confirmée ; la matrice complète des
méthodes système reste au chantier I.
Le registre typé, ses schémas et un premier moteur possédant la session et sa
sélection sont en place : 49 commandes et dix-neuf requêtes, décrites par un
[catalogue généré](docs/commands.md). Mise en forme, blocs, P.-S./P.J. et historique
sont raccordés à l’interface. Les légendes/attributions et leurs raccourcis
Tab, Maj+Tab et Entrée passent également par le moteur, comme les descriptions,
listes, notes et transformations de frappe. Les images et signatures utilisent
aussi les commandes typées et la publication atomique des ressources. Recherche,
navigation entre occurrences, clic sur un résultat et remplacements sont également
raccordés au moteur, avec cibles gardées et remplacement global en une transaction.
L’orthographe documentaire passe aussi par le moteur : navigation, correction,
exceptions du document, réinitialisation et lectures des fautes/suggestions.
L’apprentissage d’un mot reste global, avec configuration de lecture explicite
pour le moteur. Les champs et options du document passent désormais par
`metadata.set` et `document.set_option`, avec regroupement de la saisie par champ,
dates fixes et contraintes du modèle vérifiées dans le moteur. Import et choix du
modèle utilisent une même opération validée, avec dépendances et historique communs.
Création, ouverture, fermeture, sauvegarde, export et restauration/suppression de
copies de récupération passent également par le moteur. La création reçoit une
copie explicite du profil ; l’export PDF/Typst/complet respecte V6 et reste asynchrone
dans l’interface. Les [lots atomiques](docs/atomic-batches.md) sont en place :
préparation sur une session isolée, une seule annulation et une sauvegarde finale
facultative. L’accès transitoire à la session est supprimé : accesseurs immuables
pour l’affichage, brouillons natifs opaques et publications contrôlées par le moteur.
Les [lectures de rendu et compteurs](docs/read-queries.md) passent aussi par le
moteur : positions logiques, quadrilatères en points Typst, référence de rendu
gardée et comptage Unicode sur le document publié. La disponibilité est unifiée
et les routes des déclencheurs sont explicites et testées ; les règles
documentaires viennent du moteur, les états de fenêtres/préférences restent
dans l’interface ([contrat d’aiguillage](docs/command-routing.md)). La relecture
finale est consignée dans le [bilan de sortie](docs/architecture-phase-2-review.md).
Les cinq lectures complémentaires validées sont livrées : types, source, ressources,
validation et compilation sans interface. `commands.list` expose la disponibilité
contextuelle ; `document.inspect` le type, les capacités et le style de frappe.
Le [contrat des lectures](docs/engine-queries.md) précise leurs réponses et limites.
**Phase 2 terminée le 24 octobre 2026 :** les 68 identifiants et contrats d’entrée
sont stabilisés, avec ajouts compatibles possibles et décision explicite pour tout
changement incompatible.
**Phase 3 réalisée le 24 octobre 2026 :** workspace `core` / `i18n` / application,
cœur compilable indépendamment sans dépendances graphiques ou Fluent, profil
conservé côté application, ressources et tests répartis. Le repli anglais est
appliqué dans un commit distinct ; la langue documentaire reste indépendante.
Les 356 tests des trois crates et les 6 tests du diagnostic passent ; les huit
captures natives sont identiques. La CLI (phase 4) est maintenant réalisée ;
son [bilan](docs/architecture-phase-4.md) consigne les lots et leur validation.
Voir [le suivi de phase 3](docs/architecture-phase-3.md).

**Objectif :** un cœur indépendant de l’interface, dont toute action est une
commande à identifiant stable, pilotable par une ligne de commande et par MCP.

- **Phase 0 :** garde-fous, base chiffrée et comparaison de captures d’écran.
- **Phase 1 :** découper `app.rs` (7 223 lignes) en modules, dans le même crate ;
  prototype du modèle d’édition réalisé et choix des blocs/cellules en mémoire validé.
- **Phase 2 :** commandes strictes, paramètres typés, lectures séparées des
  écritures, moteur sans egui. Historique limité aux mutations documentaires,
  voie de commit commune à la saisie native et aux commandes.
- **Phase 3 :** espace de travail en quatre crates (`core`, `i18n`, application, CLI).
- **Phase 4 :** CLI (commandes, lots d’édition atomiques d’un document, vérification,
  exports séparés). Les lots excluent les écritures intermédiaires et les effets globaux.
- **Phase 5 :** serveur MCP, accès aux fichiers refusé par défaut.
- **Phase 6 :** assistance à la composition dans le document ouvert, en rejoignant
  le moteur de l’application par un canal local contrôlé.
- **Phase 7 :** consolidation et documentation.

**Ordre avec les autres chantiers :** ce chantier passe avant l’étape 5.
L’harmonisation de la présentation des blocs (chantier B) attend la fin de sa
phase 2. La décision sur le modèle d’édition (chantier C) précède le gel des
paramètres publics du moteur. Les déplacements restent séparés par commit ; les PR
peuvent regrouper plusieurs déplacements cohérents. Les limites globales de taille
incluent un découpage explicite de `document.rs` et d’`editor.rs` avant la phase 7.

## Ordre de réalisation proposé

| Étape | Résultat attendu | Décisions principales avant de coder |
| --- | --- | --- |
| 1 | Reprendre un travail après incident (A) | Politique de récupération et parcours de restauration |
| 2 | Rechercher/remplacer confortablement (B) | Périmètre, interface, raccourcis et remplacements globaux |
| 3 | Corriger l’orthographe (B) | Moteur, dictionnaires, langues et interface des suggestions |
| 4 | Produire un courrier et un compte-rendu utiles (D, E, H) | Profils/signature, modèles réels et premier périmètre de bibliothèque |
| 4 bis | Architecture, commandes, CLI et MCP (J) | Décisions D1 à D20 validées ; voir [ROADMAP-ARCHITECTURE.md](ROADMAP-ARCHITECTURE.md) |
| 5 | Enrichir les documents courants (C, B) | Modèle d’édition (voir « Décision préalable à l’étape 5 » du chantier C), architecture des capacités, tableaux, notes et collage structuré |
| 6 | Retrouver et organiser son travail (G, A) | Barre latérale, dossiers/projets et plusieurs documents |
| 7 | Rédiger des documents longs (C, H) | Bibliographie, renvois, sommaire, rapport et mémoire |
| 8 | Étendre les usages (F, B, C) | Conversions, grammaire avancée et besoins spécialisés |

Chaque ligne sera découpée en étapes plus petites. Les dépendances concrètes
peuvent conduire à revoir cet ordre ensemble. Après l’essai de rechercher/remplacer
et du correcteur orthographique, choisir ensemble le prochain périmètre.

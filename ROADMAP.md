# Feuille de route de typstOgraphe

Révisée le **28 octobre 2026**. Objectif : transformer le prototype validé en une
application de rédaction utilisable au quotidien, pour l’administration,
l’enseignement et la recherche en sciences humaines et sociales.

Cette feuille de route ne contient que le travail restant. Les réalisations,
mesures et décisions historiques restent dans Git et les bilans de docs/.
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

**Séquence immédiate :** poursuivre l’unification du contrat structuré entre
adaptateurs (**A.2**), à partir du [contrat Article](docs/article-contract.md) et
des [citations et de la bibliothèque locale](docs/article-bibliography.md).
Les [décisions de contrat et les lots techniques](docs/structured-contract.md)
sont validés : Yrs côté Rust, cibles structurées, en-tête commun, lots atomiques
et frappe locale immédiate. Mode individuel par défaut, avec services
collaboratifs activés à la demande et moteur commun aux deux modes.
Les [commandes multizones et structurelles du cœur](docs/structured-commands.md)
sont en place, avec les enregistrements d’en-tête ordonnés. Les tableaux avec
légende et les opérations de lignes/colonnes sont raccordés au cœur depuis Tauri.
Poursuivre les contextes/historiques et le raccordement des autres opérations
et adaptateurs.
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
  bibliographiques, ses ressources et ses versions conservées. Les rubriques
  s’adaptent aux capacités du type ; l’arborescence technique reste masquée.

La piste à maquetter s’appuie sur la barre verticale existante et un seul panneau
latéral ouvert à la fois, avec les accès aux outils et aux préférences globales.
« Versions » est le libellé proposé pour les snapshots. Leur consultation,
leur comparaison et leurs actions demandent une maquette spécifique ; la
séparation Projets/Document n’en fige pas encore la présentation détaillée.

## Priorités et dépendances

| Priorité | Chantier | Résultat attendu |
| --- | --- | --- |
| **P0** | A — Socle d’application | Une application Tauri de référence, un contrat documentaire commun et des fichiers fiables |
| **P1** | B — Types et contrats | Une liste limitée de types utiles, leurs métadonnées et leurs capacités |
| **P1** | C — Capacités documentaires | Un périmètre quotidien complet et cohérent, puis les outils nécessaires aux documents SHS |
| **P1** | D — Modèles et bibliothèque | Au moins un modèle réellement utilisable pour chaque type livré |
| **P1, continu** | E — Interface et rédaction | Une édition prévisible, rapide et entièrement utilisable au clavier |
| **P1** | F — Assistance ACP | Une aide à la rédaction intégrée, avec propositions soumises à validation |
| **P2** | G — Collaboration | Partage ponctuel depuis un poste, puis service durable d’organisation |
| **P1, continu** | H — Diffusion et qualité | Une application installable, testée et documentée sur les trois systèmes |
| **P3** | I — Extensions | Grammaire, conversions et capacités spécialisées justifiées par les usages |

L’accessibilité, la protection du travail et les performances accompagnent chaque
lot. La collaboration distante et l’assistance restent facultatives ; la
rédaction individuelle, l’ouverture et l’export doivent fonctionner hors ligne.

## A — Sortir le code du statut de prototype

- Unifier le contrat structuré entre l’interface, la CLI et MCP : capacités,
  cibles, erreurs, disponibilité, fichiers et exports. Retirer les chemins
  historiques devenus inutiles après vérification de la nouvelle voie.
- Séparer les diagnostics, fixtures et outils de mesure de l’application livrée.
- Consolider le cycle de vie : sauvegarde, récupération, fermeture, verrou,
  modification externe, réouverture et ressources réellement utilisées.
  Distinguer la copie de récupération d’un éventuel enregistrement automatique
  du fichier, dont le comportement reste à choisir.
- Préciser le contrat de conservation de l’arbre documentaire, de Yjs et du
  Typst généré avant de stabiliser le format public du fichier .typsto.
- **Snapshots — P1, à préparer dès le contrat du fichier :** conserver dans le
  projet des états cohérents et identifiables, consultables hors ligne. Chaque
  état doit retrouver son contenu, ses métadonnées et réglages, son modèle,
  ses dépendances et les ressources nécessaires au rendu, y compris une image
  remplacée ou supprimée depuis. Prévoir nom, date, auteur du snapshot et
  description facultative ; ne pas confondre création du snapshot et attribution
  de toutes les modifications à cette personne.
- Distinguer snapshots durables, annuler/rétablir en séance et récupération
  après incident. Prévoir consultation en lecture seule, comparaison et retour
  à un état conservé sans effacer l’historique. Maquetter et valider le parcours
  de restauration, de copie indépendante et de suppression des snapshots.
- Séparer stockage des états et calcul des différences. **Git local embarqué
  est une piste à évaluer, pas une dépendance choisie**, sans serveur GitHub ni
  connaissance de Git demandée à l’utilisateur. Comparer cette piste à un
  stockage de snapshots et une bibliothèque de diff existante. Git éventuel
  ne remplace pas Yjs pour l’édition collaborative.
- Maîtriser le poids du projet : étudier la déduplication des ressources et
  leur référencement immuable par snapshot. Exclure assets/ d’un éventuel dépôt
  Git ne doit pas permettre de perdre les images d’un état conservé. Définir
  rétention, suppression et nettoyage ; une ressource utile à un snapshot
  conservé reste dans le .typsto. L’export Typst/PDF porte sur l’état choisi,
  sans embarquer inutilement les ressources des autres états.

**Choix à valider :** migration du contrat de la CLI et du MCP autonome ;
périmètre et moment de stabilisation du format.
Pour les snapshots : déclenchement manuel/automatique, stockage, comparaison,
rétention et restauration, puis droits et coordination en collaboration.
Aucune couche de rétrocompatibilité préventive pendant cette phase préliminaire.

**Sortie :** un même document passe de l’interface aux opérations documentaires
et aux exports sans voie spéciale de prototype ni perte de contenu.

## B — Définir les types utiles et leurs contrats

Partir de la proposition [Types, modèles et libertés](docs/types-modeles-libertes.md),
puis arrêter ensemble le catalogue et sa matrice de capacités. Les groupes
ci-dessous sont une proposition d’ordre, pas sept développements lancés d’un coup.

| Groupe proposé | Types candidats | Usage à éprouver |
| --- | --- | --- |
| Premier périmètre administratif | Courrier ; compte-rendu de réunion ; note/mémo | Correspondre, restituer une réunion, diffuser une information courte |
| Recherche SHS | Article ; rapport/mémoire | Auteurs, résumé, notes, références, bibliographie et documents longs |
| Enseignement | Support de cours ; syllabus | Structurer un cours, ses objectifs et ses modalités |

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
court par langue), Affiliations, Auteurs, Résumés (résumé riche et mots-clés par
langue) et Bibliographie. Les mêmes enregistrements multilingues alimentent
Titres et Résumés.
Les ajouts par + sont provisoires jusqu’à validation et annulables par Échap ;
ils ne sont pas diffusés ni sauvegardés avant confirmation. La
[décision détaillée](docs/types-modeles-libertes.md#interface-de-len-tête--décision-du-27-octobre-2026)
et le [contrat Article](docs/article-contract.md) servent de référence.

**Décisions à mettre en œuvre et arbitrages avant refonte :**

- Types intégrés en Rust ou définition déclarative : la note propose une évolution,
  qui doit être validée avant de remplacer l’organisation actuelle.
- Mettre en œuvre le choix validé pour les modèles : options de présentation
  libres, restreintes ou imposées, y compris pour les capacités du type. Le type
  définit les capacités disponibles ; tous ses modèles doivent les présenter,
  sans suppression ni masquage de contenu. Les métadonnées de contenu restent
  celles de l’auteur.
- Dates : conserver une valeur structurée et la langue du document ; décider
  de l’évolution éventuelle du contrat en tenant compte de l’adaptateur Typst
  et de Datify. Ne pas déplacer implicitement la composition vers Fluent.
- Listes d’enregistrements et champs facultatifs : préciser les genres et les
  valeurs absentes. Pour les options de présentation, distinguer choix conservé
  et valeur effective : un modèle imposé ne réécrit pas le choix ; une valeur
  exclue utilise le défaut avec avertissement. Le retour au modèle précédent
  retrouve le choix conservé lorsqu’il est de nouveau autorisé.

**Sortie :** une fiche par type retenu et une matrice type/capacité/modèle,
compréhensibles pour l’utilisateur, le créateur de modèles et les agents.

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

Une capacité partagée doit avoir le même contrat et les mêmes interactions
dans tous les types où elle est disponible. Les modèles en définissent la
présentation et les défauts. Les identifiants publics restent anglais ou latins.
Une dépendance ou une logique Typst spécifique passe par un adaptateur stable ;
les primitives natives simples n’ont pas besoin d’un wrapper systématique.

**Choix à valider :** périmètre par capacité, paramètres, contenus et imbrications
autorisés, saisie, clavier et cibles MCP ; fonctions natives ou paquet externe.

**Sortie :** insertion, édition, annulation, sauvegarde/réouverture et exports
vérifiés, avec la même disponibilité dans les interfaces qui exposent la capacité.

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

**Choix à valider :** format des manifestes et des paquets, paramètres exposés,
polices et licences d’incorporation, import, stockage et politique de mise à jour.

**Sortie :** création d’un document utilisable avec chaque modèle livré ;
changement de modèle d’un même type sans altération du contenu ; archive et
export autonomes, compilables sans téléchargement de paquets.

## E — Peaufiner l’interface et les outils de rédaction

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
- Tenir un budget de latence de saisie et de rendu sur des documents représentatifs,
  en build natif optimisé ; réutiliser les mesures et références existantes.

**Choix à valider :** les nouvelles interactions et maquettes, les préférences
réellement utiles, les priorités de raccourcis et les objectifs de performance.

Les auteurs et affiliations utilisent désormais le dialogue réutilisable validé,
centré sur un arrière-plan atténué et légèrement flouté. Le contrat de saisie,
les brouillons isolés et les contrôles de concurrence sont décrits dans
[`docs/metadata-dialogs.md`](docs/metadata-dialogs.md).

## F — Intégrer l’assistance ACP

L’orientation ACP reste retenue : un client commun aux agents dans l’application,
MCP pour les capacités documentaires et l’acceptation explicite des propositions.

- Maquetter et valider le parcours de conversation sur l’interface Tauri actuelle.
  Piste principale : panneau ouvert depuis la barre latérale, question de hauteur
  adaptative, réponse dessous et actions dans un pied stable.
- Réaliser le premier parcours avec Codex via codex-acp et le client Rust ACP ;
  vérifier ensuite le même parcours avec un second agent compatible.
- Automatiser le raccordement MCP privé au document courant. Afficher clairement
  le document et la sélection confiés, sans suivre ensuite le curseur humain.
- Distinguer masquer le panneau, interrompre une demande et terminer l’assistance ;
  préciser les erreurs, coupures et reprises sans renvoi automatique d’une demande.
- Conserver un seul lot proposé en attente, sa relecture, sa péremption et son
  acceptation/refus explicites. L’assistance ne doit pas déplacer le focus de rédaction.
- Préciser installation, authentification et distribution des agents/adaptateurs ;
  annoncer uniquement les fonctions et droits effectivement disponibles par agent.

**Choix à valider :** maquette et clavier, contexte fourni, conversation par document
et agent, conservation de l’historique, permissions et modèle de distribution.
Les choix fonctionnels détaillés sont dans le [plan technique](ROADMAP-ARCHITECTURE.md#f-assistance-acp).

**Sortie :** discuter, reformuler une sélection et préparer un document structuré,
puis accepter/refuser un lot, avec au moins deux agents et sans configuration MCP manuelle.

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

**Choix à valider :** premier parcours local/Internet, hôte et disponibilité,
identités et droits, transport, stockage, confidentialité, coûts et maintenance.

## H — Préparer une première diffusion

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
- Mathématiques en ligne/bloc, code, diagrammes et graphiques de données ;
  glossaire, acronymes et index selon les besoins des types livrés.
- Import de contenus externes et exports supplémentaires justifiés par les usages.

La reformulation par petit LLM local n’est pas réactivée par ce plan. Un créateur
graphique de types/modèles, les diaporamas et les posters ne font pas partie du
premier périmètre. Les propositions non retenues ne deviennent pas des engagements.

## Prochain lot proposé

1. Poursuivre l’unification du contrat structuré entre les adaptateurs (**A.2**),
   à partir du [rangement de l’application](docs/application-layout.md).
   Les schémas de métadonnées Article sont partagés ; les propositions MCP
   rattachées restent actuellement limitées au corps du document.
2. Reprendre ensuite le périmètre administratif restant, un type à la fois,
   avec un modèle utile et une recette représentative.
3. Engager ACP après stabilisation du contrat documentaire et validation de sa
   maquette (**F**), sans attendre la livraison de tous les types SHS/enseignement.

Ce prochain lot est une proposition à discuter ; cette réécriture documentaire
n’autorise pas à implémenter d’avance les choix encore ouverts.

# Plan technique de développement

Révisé le **31 octobre 2026**. Ce document accompagne la [roadmap produit](ROADMAP.md)
et ne conserve que les décisions à prendre et les travaux techniques restants.
Les anciennes phases, leurs réalisations et leurs inventaires historiques sont
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
  actuellement par lot entier. Le lot dédié F.3 préparera les décisions locales
  sûres ; sauvegarde et export ont leurs autorisations propres.
- Les nouvelles interfaces sont maquettées et les choix structurants validés
  ensemble ; rechercher les outils maintenus avant de réimplémenter une brique.

Le [contrat structuré](docs/structured-contract.md) est le socle documentaire
commun. La conservation après incident est livrée dans A.3 ; les snapshots restent
à préciser dans A.4, puis A.5 clôt le socle par la documentation et le nettoyage.
Aucun maintien artificiel de la projection plate ou des anciens formats n’est demandé.

## A. Socle et convergence — P0

L’application principale et le retrait d’egui sont décrits dans les bilans
[A.1](docs/application-layout.md) et [nettoyage](docs/egui-retirement-results.md).
**Aucune capture d’écran pour le moment** ; conserver les recettes fonctionnelles,
sans outillage de capture ni références graphiques figées.
La présentation de l’interface peut maintenant être retravaillée.

**A.2 clos le 31 octobre 2026 :** le [bilan de clôture](docs/a2-closure-results.md)
conserve les recettes et leurs résultats. Tauri, CLI et MCP utilisent le contrat
structuré commun ; le mode individuel garde ce même moteur. Les propositions
rattachées restent limitées au corps/cellules/annotations. Les claviers/IME et
presse-papiers physiques des trois systèmes restent une recette manuelle ; cette
clôture n’en revendique pas une validation exhaustive. Le réseau reste dans G,
la fusion de cellules et les listes imbriquées dans C.

### A.3 Conservation et publication — clôture livrée

**Décisions validées le 31 octobre 2026 :** [contrat de conservation et reprise](docs/a3-conservation.md).
Enregistrement explicite du `.typsto` ; secours indépendant après deux secondes
sans modification, échéance de dix secondes en rédaction continue. Protéger aussi
les saisies privées, champs en attente et formulaires ouverts, avec leurs ressources,
sans publication automatique. Une copie par document/fenêtre ; reprise isolée et
fermeture explicite livrées. L’annulation reste limitée à la séance.
Premier lot livré : collecte commune des formulaires et zones riches, copies privées
atomiques par document/fenêtre, conservation des refus et ressources, échéances et
reprises d’écriture. La recette native macOS vérifie un arrêt brutal sans publication
d’un formulaire ouvert. Voir le [contrat et les limites](docs/a3-conservation.md) et le
[bilan du premier lot](docs/a3-conservation-results.md).
Le [parcours de reprise et de fermeture](docs/a3-recovery-interface.md) est validé :
accueil sans ouverture automatique, copie indépendante, saisies privées reprises
dans les interfaces existantes, conservation à la fermeture par défaut. La maquette
est validée et son implémentation livrée, avec une barrière de conservation de
toutes les fenêtres avant remplacement du document. Voir le [bilan de reprise](docs/a3-recovery-results.md).
Les recettes d’incident, de changement externe et de publication Courrier/Article
sont livrées, avec comparaison des états et exports Tauri/CLI. La [recette de
clôture](docs/a3-closure-results.md) précise la qualification native des trois systèmes,
réussie lors de la première passe CI, les contrôles finaux du commit courant et
les contrôles matériels de distribution restant dans H.

Fixer les responsabilités de l’état Yjs, de l’arbre validé et du Typst généré.
Tester identité/révision, sauvegarde pendant des éditions, récupération après
incident, fermeture, changement externe et refus d’une archive incohérente.
Les copies de récupération Tauri sont désormais isolées par processus,
protégées par un verrou et restaurées explicitement ; ne pas les confondre avec
les snapshots durables. Les recettes natives sont exécutables sur les trois systèmes dans la validation
complète ; les rapports locaux et les résultats CI sont distingués.
Conserver en séance les ressources utiles à l’annulation ; conserver dans
l’archive celles requises par l’état courant et tous les snapshots retenus.
Un export Typst/PDF de l’état choisi ne requiert pas les ressources propres
aux autres états conservés.

### A.4 Snapshots durables — P1

**Décisions validées le 31 octobre 2026 :** capturer uniquement l’état documentaire
confirmé complet, modèle/dépendances/ressources compris ; conserver les formulaires
et saisies non validées dans la récupération privée A.3. La création d’une version
enregistre atomiquement l’état courant et le snapshot dans le `.typsto`, après
choix d’emplacement si nécessaire. Une sauvegarde ordinaire ne crée pas de version.

Avant toute restauration, créer une version automatique de sauvegarde de l’état
courant ; publier explicitement l’état restauré sans supprimer les autres versions.
En collaboration, passer par l’autorité commune et les gardes documentaires,
jamais par un remplacement silencieux chez les participants. Pas de suppression
automatique de versions dans le premier périmètre ; le nettoyage des ressources
doit tenir compte de tous les états retenus, dont les références de relecture.

L’[expérience isolée de stockage](docs/a4-snapshot-storage-evaluation.md) est réalisée :
Git embarqué et stockage d’états à ressources dédupliquées, mêmes corpus Courrier,
Article et Article allongé, 20/100 versions, taille, création, ouverture,
suppression et intégrité. Les deux candidats utilisent une lecture sélective et
des manifestes de ressources partagés. **Choix validé après examen du bilan : CAS**,
plus rapide à publier et ouvrir, en acceptant des historiques plus volumineux
qu’avec Git. Le format et les maquettes précèdent l’intégration de production,
encore à faire. Git reste dans l’expérience isolée. Cette décision ne clôt pas A.4.

Le [format cible validé](docs/a4-project-format.md) précise
un ZIP entièrement CAS, un manifeste pointant sur l’état courant, des descripteurs
immuables communs aux snapshots et un index de versions distinct. Il décrit
la vérification Yjs/projection/sources, les ressources et inventaires embarqués,
les métadonnées modifiables des snapshots, la lecture sélective et les budgets.
Retenir **100 versions affichées maximum** (`manual`, `before_restore`), hors
état courant, références internes de relecture et candidats MCP privés. Cette
règle remplace le quota initial toutes origines confondues. Une sauvegarde
ordinaire ne consomme pas de place. Une restauration nécessitant `before_restore`
à la limite est refusée avant mutation, sans suppression automatique. Le quota
visible ne bloque pas à lui seul une nouvelle relecture : ses références
internes restent soumises aux budgets de volume et à la rétention de J.
Point à intégrer : la validation des packages doit s’appuyer sur l’inventaire
de l’état conservé, sans le rendre dépendant du catalogue installé. Ne pas
confondre ce document de conception avec le schéma de production déjà livré.

Définir un état cohérent incluant contenu structuré, métadonnées, réglages,
modèle, dépendances et ressources, avec identité stable, date de création
automatique, nom libre modifiable, tags facultatifs, auteur du snapshot et
description facultative. Proposer un nom daté par défaut sans imposer de
numérotation de version ; permettre le classement chronologique indépendamment
du nom et des tags. Conserver l’instant de création sans ambiguïté de fuseau
horaire et l’afficher selon la locale de l’interface. L’auteur du snapshot n’est
pas nécessairement celui de toutes les modifications conservées.
Préciser les données Yjs nécessaires à sa
conservation ; une capture de l’état collaboratif binaire et une comparaison
lisible du contenu répondent à des besoins distincts. Ne pas assimiler le
snapshot à une commande d’annulation ou à la copie de récupération.

Le CAS stocke les états et ressources immuables dans le `.typsto`, avec lecture
sélective et manifestes de ressources dédupliqués. Séparer ce stockage du moteur
de différences ; aucune dépendance Git dans la production, aucun changement du
moteur Yjs/Yrs pour la synchronisation.
Évaluer la comparaison documentaire par blocs, texte, annotations, métadonnées
et ressources, sans exposer les différences d’octets Yjs à l’utilisateur.

Livrer deux entrées latérales distinctes : **Versions** et **Relectures**. Versions
groupe les origines visibles (manuelles, avant restauration), avec ordre
chronologique dans chaque groupe. Le clic ouvre une modale centrée à 90 % de
la fenêtre, sur fond atténué/flouté : Contenu, Aperçu, Comparer et choix de l’état
comparé dans ce dernier mode. Confirmer restauration/suppression dans la même
modale. Échap annule d’abord la confirmation, puis ferme la consultation avec
retour du focus à la ligne d’origine. Les références internes de relecture ne
s’affichent pas dans Versions ; conserver les outils dédiés de J.

Partager entre snapshots et propositions MCP un contrat de différences et une
interface de comparaison, en séparant calcul, présentation et décisions. Comparer
deux snapshots ou un snapshot et l’état courant ; présenter des groupes de
modifications proches avec contexte et navigation. Préparer leur identification
et leur correspondance aux cibles/opérations documentaires sans confondre
contiguïté et indépendance. Évaluer les briques existantes, dont `similar`
anciennement utilisé sous egui, avant de retenir un moteur ; il n’est pas une
dépendance actuelle à considérer comme déjà disponible.

La consultation de différences entre snapshots ne publie aucune modification.
Une éventuelle reprise locale doit être une commande explicite sur l’état
courant, sans modifier les snapshots. A.4 prépare les données nécessaires à la
validation locale MCP, livrée séparément en F.3 et non requise pour clôturer A.4.
Les outils d’annotations de J restent dédiés au mode relecture.

**Convergence MCP :** une proposition associe deux états figés complets et
vérifiés, la base depuis laquelle l’agent a travaillé et le candidat figé à sa
soumission. Garder leur lien de provenance et la garde document/révision source.
Réutiliser la même modale de consultation et comparaison, avec la paire
« État de départ → Proposition » par défaut, et Accepter/Refuser comme décisions.
Chaque aperçu utilise les ressources et le modèle de son propre état. La
comparaison n’applique rien et ne produit pas implicitement un patch autorisé.
Une révision courante différente rend la proposition périmée pour publication,
même si les états figés restent consultables ; une égalité des empreintes après
Annuler ne réactive pas une ancienne garde. La régénération passe par une base
actualisée, pas par une fusion silencieuse. Le candidat et ses ressources restent
privés jusqu’à acceptation, hors Versions et du `.typsto` confirmé ; voir le
[contrat de format](docs/a4-project-format.md#propositions-mcp--paire-détats-privés).

Étudier un stockage de ressources immuables identifiées par leur contenu,
partagées entre états : une image remplacée doit rester accessible depuis
son ancien snapshot. Le nettoyage prend en compte l’état courant et tous les
snapshots retenus, indépendamment des ressources visibles dans l’état courant.
Préciser les parcours de suppression explicite avant de stabiliser le schéma de
l’archive ; aucune rétention automatique ne supprime de version au départ.

La restauration doit produire un nouvel état courant sans effacer les états
conservés ; son interaction avec les révisions Yjs, les participants distants
et les propositions MCP gardées par révision reste à spécifier avant codage.
Vérifier hors ligne l’ouverture et le rendu d’un état ancien après changement
de modèle, remplacement/suppression d’image, sauvegarde et réouverture ; tester
aussi archive incomplète, suppression d’un snapshot et absence de nettoyage
des ressources encore référencées. Une politique de nettoyage de l’historique
ne doit pas dépendre du seul état courant.

**Articulation avec J — règle retenue :** le démarrage d’une nouvelle relecture
crée automatiquement un snapshot de l’état initial et rattache la relecture à
son identité stable, dans une collection interne distincte des versions affichées.
Conserver cette référence inchangée pendant toute la
relecture et empêcher sa suppression tant que celle-ci est en cours. La reprise
d’une relecture existante ou la réouverture de son panneau ne crée pas un nouveau
snapshot. Ce déclencheur explicite n’introduit aucune création automatique
heuristique. La référence durable et comparable ne remplace ni les annotations
ancrées de suggestion, ni leurs commandes d’acceptation/refus sous garde.
Cette base commune n’impose pas de bouton « Comparer à l’état initial » ou de
parcours Versions dans Relectures.

Avant de coder, préciser ensemble la conservation des suggestions/commentaires
et de leurs ressources, leur devenir après restauration, ainsi que la rétention
du snapshot de référence après clôture de la relecture. Distinguer
ces données du contenu confirmé. Prévoir les cas de cibles modifiées, d’ancres
orphelines et de propositions MCP périmées, sans inventer dès A.4 une politique
de réapplication automatique. Ces arbitrages et la maquette du mode restent dans
A.4/J ; livrer les snapshots ne suppose pas de livrer J ni ACP.

### A.5 Documentation, nettoyage et recette finale — clôture de A

Après A.4, vérifier le socle complet avant de quitter le chantier A :

1. **Documentation de référence :** mettre à jour contrats et instructions,
   retirer des deux roadmaps les tâches terminées et distinguer clairement bilans
   historiques et documentation courante. Conserver un README court et les preuves
   utiles, sans perdre les décisions d’architecture.
2. **Application livrée :** retirer code mort, anciens chemins et duplications
   vérifiées, examiner les dépendances et isoler tous les diagnostics, fixtures et
   outils de mesure dans des constructions/cibles de recette adaptées. Le binaire
   normal ne doit plus embarquer les comportements de prototype ; les tests et
   recettes restent disponibles. Le choix concret d’isolation est à valider.
3. **Organisation du dépôt :** ranger exemples et outils, retirer les fichiers
   temporaires et branches fusionnées inutiles ; conserver résultats, rapports,
   licences, provenances et références visuelles historiques. Ne pas réintroduire
   d’outillage de capture.
4. **Contrat de conservation :** documenter le format versionné retenu après A.4,
   ses invariants et limites, sans couche de rétrocompatibilité préventive.
5. **Recette depuis un checkout propre :** vérifier les instructions de dépendances
   et de build, tests appropriés, lancement normal et accès CLI/MCP. Faire passer le
   corpus Courrier/Article par édition, sauvegarde/réouverture, consultation et
   restauration de snapshots, compilation et export hors ligne de l’état choisi.
   Vérifier contenu, métadonnées, modèle, dépendances et ressources sans voie de
   prototype ni duplication du contrat entre adaptateurs.

Consigner les résultats et les limites résiduelles. Les qualifications matérielles,
paquets, signature/notarisation et mises à jour restent dans H. Une nouvelle
dépendance, une réécriture ou un choix graphique structurant demande un arbitrage
distinct ; ce lot ne les autorise pas implicitement.

**Sortie A :** même corpus relu et modifié par les adaptateurs concernés,
sauvegardé, rouvert, restauré depuis un snapshot, rendu et exporté sans branche
documentaire spéciale ; constructions et instructions vérifiées depuis un checkout propre.

## B. Types, paramètres et manifestes — P1

Préciser le schéma de chaque type, les genres de valeurs, les groupes de champs,
les capacités et les signatures à implémenter. La note
[Types, modèles et libertés](docs/types-modeles-libertes.md) est une proposition,
avec une clarification validée sur les paramètres et l’interchangeabilité des
modèles ; les autres arbitrages ne remplacent pas encore les contrats actuels.

**Décision validée le 26 octobre 2026 :** le type définit les capacités ; tous
ses modèles les implémentent. Les options de présentation ont un régime libre,
restreint ou imposé, déclaré dans le manifeste, y compris pour les options des
capacités. Une valeur imposée peut être indiquée sans sélecteur. Les métadonnées
de contenu ne sont pas remplacées par le modèle. Séparer choix enregistré et
valeur effective : un choix incompatible utilise le défaut avec avertissement,
sans écrasement ; un choix imposé reste temporaire tant que ce modèle est actif.
Vérifier notamment brouillon → revue à style imposé → brouillon : les citations
restent intactes et le choix initial redevient effectif s’il est autorisé.

Décider avant implémentation :

1. Types Rust ou définition déclarative ; migration et limites d’un type installable.
2. Champs simples et listes d’enregistrements, valeurs absentes et défauts ;
   conserver les genres jusqu’à la sérialisation Typst et empêcher l’injection
   de source par les valeurs saisies.
3. Représentation des paramètres du type, des paramètres généraux et des options
   du modèle ; schéma des trois régimes validés et exposition des contrôles.
4. Contrôle de conformité des modèles et diagnostic d’incompatibilité, sans
   perte ou réécriture implicite du contenu ; conservation des choix lors des
   changements de régime.
5. Dates : valeur structurée/ISO, insertion fixe d’aujourd’hui, langue du document,
   format demandé au modèle et rôle de l’adaptateur Typst/Datify.
6. Format et emplacement du manifeste, validation des signatures, ressources,
   dépendances et licence des polices.

**Sortie B :** matrice de types/capacités, schémas et contrats testables, avec
un exemple minimal de modèle et des diagnostics compréhensibles.

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

**Sortie C :** contrat partagé GUI/CLI/MCP, recette clavier et aller-retour du
fichier ; absence de logique spécifique dupliquée dans les déclencheurs.

## D. Modèles et bibliothèque — P1

Définir les identités, versions, manifestes et paquets de ressources des modèles,
leur stockage global et la copie autonome dans chaque document. Distinguer
modèle installé et modèle embarqué ; aucune mise à jour distante implicite.

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

**À faire — adaptation de la barre d’outils et de la palette au type (30 octobre 2026) :** filtrer
les capacités exposées à partir du catalogue du type actif et supprimer les
groupes sans entrée. Ne pas coder « article » ou « courrier » dans la
présentation : « Fin de lettre » disparaît automatiquement d’un article parce
que ses capacités P.-S./P.J. n’y sont pas disponibles. Distinguer cette absence
structurelle de l’indisponibilité contextuelle d’une capacité du type, qui reste
visible mais désactivée par le registre commun. Reconstruire la barre à la
création/ouverture ; vérifier le passage courrier → article → courrier et la
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

**Budget relatif :** exprimer la cible comme un facteur (1,5 à 2, en p95) de la
compilation incrémentale de Typst seul sur le même document, avec une référence
distincte pour le natif et pour le WebAssembly, un plancher absolu pour les petits
documents et un suivi du surcoût par étape. La cible suit ainsi les évolutions de
Typst et détecte les régressions propres à typstOgraphe.

## F. Assistance ACP

**Orientation retenue :** client Rust agent-client-protocol, Codex via codex-acp
pour le premier parcours ; MCP reste le contrat documentaire. Aucun couplage à
ACP ou à un fournisseur d’agent dans le cœur documentaire.

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

### F.3 Validation locale des propositions MCP — lot distinct après A.4

Réutiliser le contrat et l’interface de comparaison d’A.4 pour accepter/refuser
des groupes indépendants à partir de la paire base/candidat. Ces états restent
immuables ; le suivi du reliquat et de ses gardes ne se déduit pas d’un simple
retour au même contenu. Le calcul de différences ne suffit pas à prouver cette
indépendance : relier les groupes aux commandes et à leurs dépendances, y compris
structurelles et de ressources. Des changements éloignés peuvent être liés
(citation et référence bibliographique). Conserver une décision groupée lorsque
l’indépendance n’est pas garantie, avec la voie actuelle par lot entier.

Chaque acceptation locale doit revérifier les cibles et publier atomiquement les
opérations retenues, avec une étape d’annulation cohérente. Définir le suivi des
groupes acceptés/refusés/restants, la révision de référence et la revalidation ou
péremption du reliquat après publication ou édition concurrente. Ne pas permettre
à l’agent d’accepter, ni rejouer implicitement une opération déjà publiée.

Préciser ensemble le protocole MCP, les garanties d’indépendance et la maquette
avant de coder. Vérifier dépendances éloignées, cible périmée, acceptations
successives, refus sans modification et annulation. L’acceptation par lot entier
reste le contrat actif jusque-là ; F.3 ne conditionne pas la clôture d’A.4.

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
sans imposer sa présentation à J. Mutualiser les composants pertinents ; F.3
traitera les décisions locales MCP et préservera l’atomicité des opérations liées.
L’acceptation actuelle reste par lot entier jusqu’à ce lot. La première
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
référence après clôture restent à préciser ensemble dans A.4/J. Le calcul de différences entre
snapshots et les suggestions ancrées ont des rôles distincts ; la relecture ne se
réduit pas à un retour global à une ancienne version.

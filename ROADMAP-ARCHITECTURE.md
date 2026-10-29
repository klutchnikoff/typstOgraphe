# Plan technique de développement

Révisé le **29 octobre 2026**. Ce document accompagne la [roadmap produit](ROADMAP.md)
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
  par lot entier ; sauvegarde et export ont leurs autorisations propres.
- Les nouvelles interfaces sont maquettées et les choix structurants validés
  ensemble ; rechercher les outils maintenus avant de réimplémenter une brique.

La représentation documentaire de session et le format public restent à préciser
dans A. Aucun maintien artificiel de la projection plate ou des anciens formats
n’est demandé pendant cette phase préliminaire.

## A. Socle et convergence — P0

L’application principale et le retrait d’egui sont décrits dans les bilans
[A.1](docs/application-layout.md) et [nettoyage](docs/egui-retirement-results.md).
**Aucune capture d’écran pour le moment** ; conserver les recettes fonctionnelles,
sans outillage de capture ni références graphiques figées.
La présentation de l’interface peut maintenant être retravaillée.

### A.2 Un contrat documentaire structuré

**Décisions validées le 28 octobre 2026 :**
[contrat structuré et séquence de migration](docs/structured-contract.md).
Yrs est retenu côté Rust après évaluation ; Yjs reste dans l'éditeur. Cibles par
zones, garde document/révision pour les commandes différées, en-tête entièrement
éditable, lots isolés et publication atomique. La frappe locale reste immédiate
avec confirmation asynchrone et conservation du brouillon en cas de refus.
Mode individuel par défaut, services collaboratifs activés à la demande ;
le moteur documentaire reste commun.

Le [socle Yrs et de conservation](docs/structured-contract-foundation.md) est en
place, ainsi que les [commandes multizones et structurelles](docs/structured-commands.md)
dans le cœur : insertions/remplacements/suppressions de blocs et items, séparation
de paragraphes, titres, enregistrements d’en-tête ordonnés et normalisation des
lignes vides avant publication. Les recettes Rust ↔ Tiptap vérifient les identités
conservées et une seule étape d’annulation pour chaque lot. Le
[socle de contextes et d’historique par participant](docs/structured-editing-contexts.md)
ajoute des répliques privées durables, des styles/sélections isolés et la validation
des annulations avant publication. Il est vérifié avec Yjs/Tiptap et raccordé à
l’application : frappe immédiate,
validation asynchrone, groupement et historique Rust unique par fenêtre,
accusés/refus et export explicite du brouillon avant abandon. Restent les autres
commandes typées, le mode individuel allégé, puis
la migration CLI/MCP autonome. Les brouillons refusés restent en mémoire avant
leur export JSON ; leur conservation automatique durable reste à traiter.

Les sélections et styles de frappe Tauri sont désormais raccordés à ces contextes :
points UTF-16 multizones et direction, sélections de widgets/badges et de cellules,
styles privés par fenêtre et conservation pendant les changements distants.
Le corps et les zones riches d’en-tête partagent ce mécanisme ; les brouillons
restent isolés. Une frontière causale compacte garde les positions sans renvoyer
le document pour chaque mouvement, sans publication ni compilation. Voir le
[contrat des contextes](docs/structured-editing-contexts.md#sélections-et-styles-raccordés-dans-tauri--29-octobre-2026).

**Lot en place :** [figures et tableaux](docs/structured-commands.md#figures-et-tableaux--contrat-mis-en-place),
avec une enveloppe Typst `figure` commune aux images/tableaux, légende facultative,
édition des lignes/colonnes depuis la cellule active et conservation de la
sémantique accessible. Le parcours Tauri prépare puis publie sous garde dans
le propriétaire, avec annulation et diffusion aux autres fenêtres. Fusion des
cellules différée ; migration des autres commandes et CLI/MCP encore à réaliser.

La recherche/remplacement et les corrections orthographiques Tauri rejoignent
aussi les commandes typées : lots textuels par zones, styles conservés,
exceptions orthographiques documentaires gardées et annulation unique. Les
widgets et les remplacements partagent la publication de leur plan et le
rétablissement de la sélection. Le dictionnaire personnel reste une préférence
globale. Voir la [recette et les limites](docs/structured-commands.md#rechercheremplacement-et-corrections-raccordés-dans-lapplication).

Les champs simples et les collections d’en-tête Tauri utilisent les commandes
communes : saisie native immédiate avec confirmation asynchrone, formulaires
privés puis publication atomique, déplacement par identifiant et suppression
avec détachement explicite des références. La sauvegarde attend les champs en
cours de confirmation et conserve les valeurs refusées jusqu’à décision.
Les onglets restent stables pendant les mises à jour. Voir le
[contrat de publication de l’en-tête](docs/structured-commands.md#en-tête-raccordé-dans-lapplication).
La récupération protège l’état confirmé ; l’export JSON du brouillon comprend
les champs natifs en attente. Leur conservation durable et leur réimportation
restent à traiter avec A.3.

Les ressources, les options générales et les modèles Tauri rejoignent aussi ce
contrat : images et signature admises avec leur lot, préparation sans effet de
bord, ressources immuables conservées pour l’annulation et modèles vérifiés hors
ligne sans reconstruire l’en-tête. Les sélecteurs utilisent la confirmation et
les refus communs ; changer la langue conserve les zones riches existantes.
Le profil reste global jusqu’à son incorporation explicite. Voir les
[garanties et recettes](docs/structured-commands.md#ressources-options-et-modèles-raccordés-dans-lapplication).
Les régimes de paramètres de modèle de B et les snapshots de A.3/A.4 restent distincts.

Converger entre le contenu structuré du nouvel éditeur et les adaptateurs
historiques : catalogue, commandes, requêtes, erreurs typées et schémas de fichiers.
Les cibles du nouvel éditeur sont des chemins de blocs/cellules/annotations,
avec offsets UTF-16 dans une zone. Conserver les noms d’opérations lorsque leur
sens reste identique, sans simuler les positions de la projection plate.

Migrer la CLI et le MCP autonome vers ce contrat, retirer les implémentations
dupliquées et annoncer honnêtement ce que chaque adaptateur sait exécuter.
Centraliser la disponibilité, la validation des paramètres et les garanties des
transactions ; aucun canal ne contourne les frontières entre zones.

### A.3 Conservation et publication

Fixer les responsabilités de l’état Yjs, de l’arbre validé et du Typst généré.
Tester identité/révision, sauvegarde pendant des éditions, récupération après
incident, fermeture, changement externe et refus d’une archive incohérente.
Les copies de récupération Tauri sont désormais isolées par processus,
protégées par un verrou et restaurées explicitement ; ne pas les confondre avec
les snapshots durables. Étendre les recettes aux systèmes et situations réelles
encore non couverts.
Conserver en séance les ressources utiles à l’annulation ; conserver dans
l’archive celles requises par l’état courant et tous les snapshots retenus.
Un export Typst/PDF de l’état choisi ne requiert pas les ressources propres
aux autres états conservés.

### A.4 Snapshots durables — P1, contrat à préparer avec A.3

Définir un état cohérent incluant contenu structuré, métadonnées, réglages,
modèle, dépendances et ressources, avec identité, nom, date, auteur du snapshot
et description facultative. Préciser les données Yjs nécessaires à sa
conservation ; une capture de l’état collaboratif binaire et une comparaison
lisible du contenu répondent à des besoins distincts. Ne pas assimiler le
snapshot à une commande d’annulation ou à la copie de récupération.

Comparer Git embarqué local à un stockage de snapshots avec bibliothèque de
diff : dépendances, taille, temps de création/lecture, empaquetage dans le
.typsto et intégrité après incident. Aucun moteur n’est choisi ; ne pas
introduire de service Git distant ou remplacer la synchronisation Yjs.
Évaluer la comparaison documentaire par blocs, texte, annotations, métadonnées
et ressources, sans exposer les différences d’octets Yjs à l’utilisateur.

Étudier un stockage de ressources immuables identifiées par leur contenu,
partagées entre états : une image remplacée doit rester accessible depuis
son ancien snapshot. Le nettoyage prend en compte l’état courant et tous les
snapshots retenus ; ignorer assets/ dans Git ne dispense pas de cette garantie.
Définir rétention et suppression avant de stabiliser le schéma de l’archive.

La restauration doit produire un nouvel état courant sans effacer les états
conservés ; son interaction avec les révisions Yjs, les participants distants
et les propositions MCP gardées par révision reste à spécifier avant codage.
Vérifier hors ligne l’ouverture et le rendu d’un état ancien après changement
de modèle, remplacement/suppression d’image, sauvegarde et réouverture ; tester
aussi archive incomplète, suppression d’un snapshot et absence de nettoyage
des ressources encore référencées. Une politique de nettoyage de l’historique
ne doit pas dépendre du seul état courant.

**Sortie A :** même corpus relu et modifié par les adaptateurs concernés,
sauvegardé, rouvert, rendu et exporté sans branche documentaire spéciale.

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

**Séparation validée le 27 octobre 2026 :** l’accès **Projets** présente le
catalogue global (récents, épinglés, recherche, création et ouverture) ; l’accès
**Document** présente le plan, les références, les ressources et les versions du
projet actif, selon son type. Le catalogue référence les fichiers ; retirer une
entrée ne les supprime pas. Préférences et bibliothèque globales restent
distinctes des données du projet.

Maquetter cette séparation sur la barre verticale existante ; un seul panneau
ouvert à la fois et le libellé « Versions » sont les pistes proposées. La
disposition détaillée, la consultation en lecture seule, la comparaison et les
actions de restauration restent à valider. Le choix fenêtres/onglets pour
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

Un seul lot en attente, relecture et acceptation/refus explicites, révision
revalidée à l’acceptation. Distinguer interruption, masquage et terminaison ;
terminer/fermer/remplacer révoque l’accès et retire les propositions en attente.
Une reprise ne renvoie ni une demande ni une édition automatiquement.

**Sortie F :** deux agents compatibles vérifiés sur discussion, lecture, proposition,
refus/acceptation, péremption, annulation, interruption et révocation. Ne pas
supposer que les fonctions, forfaits et quotas sont identiques entre agents.

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

 annotations ancrées séparées du contenu, applicables par les
commandes gardées existantes. Mutualiser la présentation des propositions sans
perdre l’acceptation atomique des lots d’agent, y compris structurels. La première
relecture humaine textuelle ne dépend pas d’ACP. Distinguer aperçu confirmé et
aperçu candidat explicitement signalé ; revérifier le contenu cible avant toute
acceptation si l’auteur l’a modifié.

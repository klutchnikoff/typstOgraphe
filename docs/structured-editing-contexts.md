# Contextes d’édition et historique — A.2

Ce lot ajoute `content::editing::EditingSession` au cœur Rust. Il complète les
[commandes communes](structured-commands.md) et respecte le
[contrat structuré](structured-contract.md). Il est exécuté dans les recettes
Rust et Rust ↔ Yjs/Tiptap, puis raccordé au relais de frappe et à l’historique de
l’application Tauri et aux adaptateurs CLI/MCP autonomes.

## Propriété des données

`EditingSession` possède un `SharedSession` confirmé et des contextes privés.
Chaque contexte conserve une réplique Yrs durable, son gestionnaire d’annulation,
une sélection et ses styles de frappe. Rien de cela n’ajoute de clés privées à
l’état partagé ou au `.typsto`. Les contextes sont bornés à 64 par session et leurs
identifiants ne sont pas réutilisés après fermeture.

Un identifiant de contexte n’est pas une autorisation. L’hôte associe les
participants authentifiés à leurs contextes, vérifie leurs permissions et
sérialise les appels. Le focus, les panneaux et la préédition IME restent à
l’adaptateur. Aucun contexte ne peut demander de prendre le focus humain.

L’API commune propose :

- `open_context`, `close_context`, `context` : cycle de vie et état privé ;
- `set_context` : remplacement atomique de la sélection et des styles de frappe ;
- `select`, `typing_style`, `insert` : raccourcis de saisie dans une seule zone,
  avec les styles propres au contexte ;
- `check_frontier` : vérification d’un checkpoint causal compact Yjs/Yrs ;
- `execute` : lot de commandes documentaires explicites ;
- `execute_updates` : lot de mises à jour attribué au contexte par l’hôte ;
- `execute_grouped_update` : mise à jour de frappe avec groupe explicite et
  conservation d’un brouillon privé en cas de refus ;
- `publish_updates` : publications sans attribution à un historique local ;
- `history` : annulation ou rétablissement du contexte ;
- `rejected_draft` : consultation d’une proposition privée refusée.

Tous ces appels vérifient la garde document/révision. L’ouverture d’un contexte,
la navigation et les styles de frappe ne modifient ni la révision documentaire,
ni l’historique, ni la possibilité de rétablir un lot. Les sélections de texte conservent leurs extrémités `anchor`/`head`, y compris
à rebours et entre plusieurs blocs du même arbre. Les sélections d’un nœud,
de cellules d’un même tableau et de tout un arbre sont explicites ; elles ne
sont jamais aplaties en fausses plages textuelles. La sélection discontinue
reste hors périmètre. `insert` exige encore une sélection textuelle dans une
seule zone ; les transformations multizones utilisent les commandes explicites.

## Sélection et styles

Le résolveur de zones, les frontières et les offsets UTF-16 sont ceux des
commandes communes. Les ancres internes utilisent les positions relatives de
Yrs, avec l’identité du bloc ou du champ ; aucun identifiant Yrs n’est exposé
comme cible publique.

Une insertion de bloc avant la zone ou une modification du texte peut déplacer
les offsets et chemins présentés par `context`. Cela ne réancre jamais une
commande préparée : sa garde stricte reste nécessaire. Une zone reconstruite ou
supprimée n’est pas remplacée par son nouveau voisin. La sélection devient
indisponible et `selection_lost` l’indique ; une insertion exige alors une
nouvelle sélection explicite. Un découpage qui reconstruit une partie du texte
ne transporte pas automatiquement la sélection vers une autre zone.

Pour un champ scalaire ou le texte d’une note actuellement conservé comme
attribut, on refuse de réutiliser une sélection si sa valeur a changé. Le contexte
ne devine pas des positions relatives que cette représentation ne fournit pas.
Les textes partagés et les champs riches utilisent leurs ancres Yrs.

Les styles suivent le contexte, jamais un autre participant. `insert` les utilise
uniquement dans les zones riches et replace le point d’insertion après le texte.
Dans un champ simple, l’insertion reste en texte brut ; demander un nouveau style
y est refusé. Les styles explicites des commandes `text.insert` restent explicites.

## Historique par participant

Le gestionnaire d’annulation est celui de **Yrs 0.28**, déjà utilisé par le projet.
Chaque réplique suit uniquement les publications attribuées à son contexte. Les
mises à jour des autres participants sont reçues sans capture ; elles ne vident
pas sa pile de rétablissement. Un nouveau lot local suit les règles de Yrs pour
la pile de rétablissement.

Un lot confirmé est importé dans la réplique en une transaction suivie : une
seule étape d’annulation couvre toutes ses zones. Les commandes sans changement
et les refus avant publication ne créent pas d’étape. Le relais de l’application fournit un identifiant de groupe privé au contexte :
une pause de 500 ms, un déplacement explicite du curseur, un changement de zone
ou une action distincte termine le groupe. Le cœur regroupe ces publications
avec Yrs, sans décider d’un délai de saisie.

La réplique est durable parce qu’un état Yjs transmis ne conserve pas les liens
internes `redone` de Yrs. Reconstruire le gestionnaire à chaque commande à partir
d’un checkpoint et de ses piles casserait certaines suites « insertion,
suppression, annuler deux fois, rétablir deux fois ». La recette couvre cette
suite avec une contribution distante intercalée.

La synchronisation privée est évitée si la révision n’a pas changé. Sinon elle
importe le différentiel nécessaire, sans reconstruire la réplique ni son
historique. La frappe reste immédiate dans Yjs ; le propriétaire Rust confirme ensuite les
mises à jour. La validation et les historiques ont un coût en arrière-plan,
à mesurer sur les documents longs ; aucune latence n’est ajoutée volontairement
avant l’affichage d’un caractère.

## Annulation refusée et brouillon privé

Une annulation est calculée dans la réplique du contexte. Son différentiel
repasse par la préparation et la validation finale de `SharedSession`, puis par
sa publication gardée. La révision augmente seulement pour une modification
confirmée.

Une protection supplémentaire refuse les suppressions d’objets introduits par
un autre participant. Par exemple : A insère un tableau, B écrit dans une cellule,
puis A annule l’insertion du tableau. Annuler le conteneur ferait aussi disparaître
le texte de B ; le moteur refuse au lieu de le masquer silencieusement. Cette
protection peut également concerner une première saisie dans une zone vide,
lorsqu’elle crée un conteneur de texte ensuite enrichi par quelqu’un d’autre.

La validation refuse aussi un résultat documentaire invalide : par exemple,
retirer une affiliation qu’un autre auteur utilise désormais.

En cas de refus :

- le document confirmé, sa révision et les autres contextes restent intacts ;
- le contexte conserve le différentiel proposé, sa garde de départ, la cause typée
  et, si sa projection est possible, son état documentaire privé ;
- il est marqué `pending` et bloque les nouvelles éditions jusqu’à traitement ;
- sa sélection privée n’est pas présentée comme une cible du document confirmé ;
- les autres participants peuvent poursuivre leur travail.

Aucun rétablissement forcé ou abandon de brouillon n’est automatique. Fermer un
contexte en abandonne explicitement les données ; l’hôte doit faire confirmer
cette décision si nécessaire. Le parcours initial validé conserve la réplique complète dans la fenêtre,
suspend les envois et propose un export JSON du brouillon, puis un retour à la
version confirmée après confirmation explicite d’abandon. Ce mécanisme ne constitue pas un contournement de l’acceptation
humaine des propositions MCP.

Les erreurs de contexte ont le code `editing_context`, avec les motifs `closed`,
`pending_draft`, `selection_required`, `text_selection_required` et `history_conflict`. Leur présentation
utilise Fluent, avec repli anglais, sans traduction dans le cœur.

## Recette et suite

Les tests couvrent sélections et styles isolés, Unicode et surrogates, champs
simples et riches, notes, cellules et légendes, déplacement des chemins, perte de
cible, lots multizones, annulations successives, rétablissement après une édition
distante, mises à jour attribuées, refus structurels et dépassement de révision.

Le transport de diagnostic exécute un scénario de deux participants dans cette
API. Ses publications sont appliquées au véritable document Yjs/Tiptap ; chaque
état est relu par Rust et le curseur graphique reste indépendant.

## Raccordement Tauri livré

Les appels Tauri du propriétaire utilisent le dispatch asynchrone fourni par
Tauri (`#[tauri::command(async)]`) : validation, verrou et sérialisation de la
réponse s’exécutent sur ses workers, sans bloquer le thread de l’interface.

La session native conserve un `EditingSession` durable et associe ses contextes
aux identités des fenêtres inscrites. Le premier état complet contient à la fois
le corps et les réglages ; son initialisation ne crée pas d’étape d’annulation.
Changer de document retire le propriétaire précédent. Fermer une fenêtre retire
son contexte. Le journal de transport peut être compacté sans recréer les
répliques privées ni perdre les liens `redone` de leurs historiques.

L’insertion native de tableau réutilise aussi la règle commune de continuation
des widgets : le lot ajoute sa ligne de saisie si nécessaire, avec une seule
étape d’annulation.

Le relais collecte les transactions Yjs synchrones d’un geste dans une microtâche :
la validation voit aussi sa réparation d’espacement, sans ajouter de temporisation
à la frappe. Les lots typés des tableaux publient dans ce même propriétaire. Les
propositions MCP rattachées restent soumises à acceptation ; leur publication
rejoint l’historique de la personne qui accepte le lot. Les réparations marquées
`addToHistory: false` passent par la validation commune sans étape d’annulation.

La préédition IME est retenue localement jusqu’à `compositionend`, y compris ses
réparations synchrones. Les publications distantes attendent sa fin dans cette
fenêtre. `flush` refuse de produire un instantané tant qu’une composition est en
cours, même si elle commence pendant l’attente d’un accusé de réception. Les
recettes simulent ces événements ; les claviers et IME natifs des trois systèmes
restent à vérifier manuellement.

La zone principale et les champs riches liés à l’en-tête n’instancient plus de
`UndoManager` Yjs dans l’application native. Leurs raccourcis, boutons et limites
de capture passent par `editing-history.ts`, vers l’historique Rust. Les éditeurs
de brouillons non publiés gardent leur historique local indépendant. Les recettes
unitaires qui utilisent un faux transport sans propriétaire natif conservent
également leur gestionnaire Yjs ; ce n’est pas un second historique de production.

Pendant une annulation/rétablissement, les zones et contrôles documentaires sont
brièvement désactivés, puis le focus de la zone active est restauré s’il n’a pas
été déplacé ailleurs. Une sauvegarde attend cette publication comme celles de
la frappe. Un accusé de réception rafraîchit les commandes, sans demander une
nouvelle compilation du contenu inchangé. Les événements natifs portent aussi
la génération du document, pour ignorer un événement tardif de l’ancien projet.

Après un refus du relais de frappe ou de l’historique, les envois déjà en attente
sont arrêtés. Un refus de commande préparée, avant toute mutation locale, conserve
son fonctionnement gardé sans suspendre la fenêtre. Le document local reste
éditable, mais sa sauvegarde/export habituels et sa reconnexion automatique sont
bloqués. Les autres fenêtres continuent à publier. Le bandeau propose :

- **Exporter le brouillon** : JSON explicitement distinct du `.typsto`, avec état
  Yjs complet, projection lisible, réglages et ressources ; l’export natif ajoute
  aussi la proposition privée du moteur si elle existe. Une boîte native choisit
  son emplacement. Ce format de secours n’a pas encore d’importeur graphique.
- **Revenir à la version confirmée** : confirmation d’abandon, fermeture du contexte
  privé puis rechargement d’une nouvelle réplique. On ne fusionne jamais l’ancienne
  réplique refusée dans le document confirmé.

Le brouillon complet reste en mémoire dans la fenêtre tant qu’il n’est pas exporté.
Ce parcours ne remplace pas une conservation automatique durable des brouillons
refusés, ni les snapshots de projet. Les historiques ne sont pas rétablis à la
réouverture du `.typsto`.

Les recettes de navigateur utilisent le vrai propriétaire Rust, y compris la
création d’un nouveau document et sa nouvelle génération. Elles vérifient le
corps, les champs riches Article, les tableaux, la séparation des contributions,
le refus, l’export JSON et la reprise après abandon, sans captures d’écran.

La recherche/remplacement et les corrections orthographiques utilisent désormais
les commandes typées, y compris l’exception documentaire `spelling.ignore`.
Leur publication et celle des widgets partagent `applyPlannedEdit` : un seul
delta confirmé et une étape d’historique, sans relais brut supplémentaire.

Les champs natifs d’en-tête et les formulaires de collections utilisent également
le propriétaire typé. La saisie native reste locale jusqu’à confirmation et ses
valeurs intermédiaires sont regroupées ; la préédition IME n’est pas publiée.
La garde de document/révision s’applique au champ ciblé, sans dépendre du curseur
du corps. Une publication scalaire ne verrouille pas les zones riches. Les
autres commandes différées attendent les champs en cours avant de préparer leur
candidat, en vérifiant que leur sélection et leur texte n’ont pas changé.
Les opérations de collections partagent préparation gardée, publication et
historique ; un formulaire modifié est un seul lot. Les instantanés manuels
attendent les confirmations, et un refus reste soumis à une décision explicite.
La récupération automatique peut prendre l’état confirmé pendant ce refus ;
l’export JSON du brouillon inclut les champs natifs en attente dans `headerDrafts`.
La conservation durable et la réimportation de ces valeurs restent distinctes.

Restent la migration des autres commandes vers leurs cibles typées communes,
le mode individuel allégé et les adaptateurs CLI/MCP autonomes. Le relais des
modifications Yjs de l’interface est déjà validé par le moteur, même lorsque le geste n’utilise
pas encore une commande typée.

Lors du premier raccordement Tauri, les contrôles comprenaient les tests du
workspace Rust, les 168 tests web unitaires et les 3 contrôles de langues natives,
les 13 recettes de contrat
Rust ↔ Yjs/Tiptap, Clippy sans avertissement et le formatage. Les 190 scénarios
Chromium/WebKit ont été vérifiés, avec reprise ciblée des insertions après
correction du tableau et des contrôles IME après ajout de la recette d’en-tête.
La compilation native est aussi vérifiée ; ce bilan ne remplace pas une recette
manuelle des méthodes de saisie et des boîtes système sur les trois OS.

## Sélections et styles raccordés dans Tauri — 29 octobre 2026

`editing-context.ts` traduit la sélection ProseMirror en cibles typées : points
UTF-16 dans leurs zones, direction conservée, badges et widgets sélectionnés
comme nœuds, cellules identifiées séparément. Toutes les zones riches liées au
projet utilisent ce même raccordement via `bindEditingHistory` : corps,
cellules, légendes, attributions et champs riches d’en-tête. Un seul contexte
par fenêtre conserve la dernière zone riche active. Le focus réel reste local ;
les champs HTML simples et les brouillons de formulaires ne deviennent pas des
éditeurs documentaires implicites.

Les styles de frappe distinguent l’héritage des marques au curseur d’une liste
explicitement vide qui les arrête. Pour remplacer une sélection, ils suivent
l’héritage réellement utilisé par ProseMirror, indépendamment de sa direction.
Les boutons, raccourcis et palette passent
par le même traducteur de mise en forme. Au curseur, « Effacer la mise en forme »
arrête maintenant les styles sans modifier le texte voisin. Les marques
explicitement préparées sont conservées pendant une transaction distante,
y compris l’arrêt explicite des styles. Elles ne viennent jamais d’un autre
participant et ne sont pas partagées dans l’awareness.

Le fournisseur regroupe les changements privés et attend les éditions locales
déjà envoyées avant `set_editing_context`. La requête comporte la génération,
la révision et une frontière causale Yjs/Yrs (`encodeSnapshot`), qui inclut
les intervalles supprimés : un simple vecteur d’état ne détecterait pas une
suppression sans insertion. Elle ne transporte ni texte complet ni ressources.
Cette frontière est réutilisée tant que le document ne change pas. Le cœur
réutilise aussi sa projection confirmée pour résoudre les cibles ; il ne
reconstruit pas un état documentaire complet pour chaque déplacement.

La validation du couple sélection/styles est atomique, sans publication,
compilation Typst, révision documentaire, entrée d’annulation ni effacement du
rétablissement. Elle ne verrouille pas l’éditeur et ne retarde pas l’affichage
des frappes. La préédition IME reste locale et son contexte n’est envoyé qu’après
validation. Les réponses tardives d’une ancienne zone ou génération ne peuvent
remplacer le contexte ou les compteurs plus récents de la fenêtre.

Une requête de navigation concurrencée est refusée sans mettre la fenêtre en
quarantaine et sans rejouer son ancienne cible sur une autre révision. Une action
locale ultérieure fournit une sélection fraîche. Les réponses du moteur ne
déplacent jamais le focus ou la sélection graphique. Une synchronisation
distante laisse les ancres du moteur suivre leur identité ; elle ne remplace
pas une zone supprimée par la position voisine choisie par ProseMirror.

Le contexte et ses styles ne sont pas enregistrés dans le `.typsto`. La frappe
continue à produire immédiatement les marques dans Yjs, puis le propriétaire
Rust valide sa publication ; le raccordement des styles ne remplace pas cette
validation par une dépendance synchrone à un IPC pour chaque caractère.


Validation de ce raccordement :

- workspace Rust complet, dont 360 tests du cœur ; le benchmark natif déjà
  ignoré reste informatif ;
- 198 tests web et 3 contrôles de langues natives ;
- 19 contrats exécutés entre Rust et le véritable éditeur Yjs/Tiptap ;
- 70 parcours Chromium/WebKit : sélections, styles, champs riches, concurrence,
  perte de cible, objets en ligne, clavier, IME, bandeaux et historique ;
- compilations web/native, Clippy avec `-D warnings`, formatage et gardes
  d’architecture.

Les compositions sont simulées dans ces parcours automatisés ; la recette des
claviers et IME physiques des trois systèmes reste manuelle. La conservation
durable des contextes, le mode individuel allégé et les adaptateurs autonomes
ne font pas partie de ce raccordement.


## Disponibilité des interactions — 29 octobre 2026

`ContextSnapshot.availability` associe chaque capacité publique à `enabled` et,
si elle est indisponible, à une raison stable en anglais. Cette réponse est commune
à Tauri, à `context.get` en CLI/MCP autonome et aux invitations MCP rattachées.
Le catalogue expose la politique de cible dans `editor.availability`. Le calcul
lit les nœuds concernés ; il ne clone pas Yrs, ne compile pas Typst, ne publie rien
et ne crée pas d’étape d’historique. Tauri réutilise la réponse de contexte et sa
garde causale existantes, sans nouvelle requête lors des déplacements du curseur.

Cette disponibilité concerne **l’interaction dans la sélection** : par exemple,
activer un style de frappe au curseur ou ouvrir la saisie des paramètres d’une
image. Elle ne valide pas les paramètres encore absents d’un formulaire.
`commands.available` conserve son sens : vérifier une commande documentaire
complète par la même préparation que son exécution. Un gras au curseur peut donc
être disponible comme style de frappe, tandis que `text.bold` exige une plage
non vide. Aucun résultat ne dispense de la garde ni des permissions de
l’adaptateur ; les propositions MCP rattachées restent limitées à leur périmètre.

Les contrôles web donnent immédiatement un retour à partir de la politique du
catalogue et de la projection locale. Une réponse native ne les affine que si
elle correspond au document, à la révision, à la sélection et à la zone active,
après confirmation des éditions locales. Une réponse périmée ne peut pas
réactiver une commande. La composition IME, le verrou temporaire d’édition,
le brouillon refusé et la déconnexion relèvent des contraintes de l’interface.

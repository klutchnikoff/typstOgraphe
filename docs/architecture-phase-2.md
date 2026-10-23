# Phase 2 — Mise en place du modèle structuré et du moteur

**État au 24 octobre 2026 : phase commencée, pas terminée.** Les orientations
V1–V6 de [l’inventaire](commands-inventory.md) sont validées. Les premiers lots
portent sur les blocs, les zones, leurs identités, les cibles de sélection, puis
la protection des frontières et la publication commune des éditions.
La préédition IME est maintenant isolée entre frames, avec une publication à validation.
Le registre typé et le premier moteur sont implémentés : 49 commandes, quatorze
requêtes et leur catalogue généré. La consolidation de la phase n’est pas terminée.

## Blocs et zones en mémoire

`src/document/structure.rs` décrit les blocs existants : paragraphes et titres,
éléments de listes, descriptions, citations, images, P.-S. et mentions de P.J.
Les descriptions possèdent des zones terme/définition ; les citations des zones
corps/attribution ; les images une zone de légende et leurs propriétés d’image.
Chaque zone réutilise la projection textuelle et ses styles. Les annotations
vides ont une identité, sans texte indicatif enregistré. Les lignes vides du
tampon restent représentées pour préserver exactement ses séparateurs actuels.

La structure est reconstruite à l’ouverture depuis la projection du Typst.
Lors d’un commit, elle est actualisée après la normalisation existante, puis
alimente le sérialiseur Typst via un adaptateur vers la projection. Les sources,
cartes de source, styles et ressources restent identiques. Aucun nouveau format
disque, aucune dépendance et aucune fonctionnalité d’édition ne sont ajoutés.

Pendant cette migration, le tampon plat reste l’adaptateur de saisie d’egui.
Des ancres de lignes suivent ses insertions, suppressions et transformations.
La réconciliation des identités utilise ces ancres et les rôles, jamais la
ressemblance du texte ni le numéro de ligne. Les identifiants sont opaques,
distincts pour les blocs et les zones, et alloués sans réutilisation pendant le
processus. Ils ne sont pas sérialisés et ne participent pas à l’état modifié.

Annuler/rétablir restaure les identités avec les instantanés. Une nouvelle branche
d’historique reçoit de nouvelles identités. Scinder une citation conserve
l’identité de son attribution avec le texte correspondant, même si elle passe
dans le second bloc. Un remplacement sans changement conserve les ancres et la
révision précédentes, sans créer d’étape d’historique.

## Positions et garde de révision

Une position désigne une zone et un offset en valeurs scalaires Unicode, ou une
frontière avant/après un bloc. La conversion vers le tampon graphique préserve
les deux extrémités et le sens de la sélection, sans exposer ses sentinelles.
Les positions hors limites et les identités inconnues sont refusées.

La session fournit une garde contenant son identité et sa révision. Une cible
provenant d’une autre session ou d’une ancienne révision est refusée, y compris
après une annulation qui restaure sa zone. Les lectures de sélection ne changent
pas la révision ; les modifications de métadonnées l’invalident également.
Les instantanés conservent désormais leur sélection logique pour la restaurer
avec les objets et les affinités de curseur. Cette restauration interne actualise
la révision de l’instantané ; aucune actualisation n’est faite sur une cible externe.

L’adaptateur graphique conserve encore ses indices globaux. Si une sélection
graphique transitoire n’est pas adressable dans un instantané, sa restauration
conserve le comportement numérique antérieur ; les nouvelles cibles explicites
restent strictement validées. Les types Rust sont internes : les signatures et
schémas JSON du registre ne sont pas gelés.

## Protection des frontières et publication commune

Le découpage en blocs et zones sert maintenant aussi à la validation du tampon,
sans rapprochement approximatif de chaînes avant/après. Une suppression ou un
remplacement est accepté dans une zone textuelle, entre paragraphes du même
corps ordinaire, ou lorsque les structures traversées sont entièrement couvertes.
Les séparateurs terme/définition, corps/attribution, objet/légende et les jonctions
avec un bloc composé partiellement sélectionné sont protégés. Les refus sont
signalés dans la barre d’état, en français ou en anglais.

Une suppression totale laisse un paragraphe vide. Effacer seulement une légende
ou une attribution conserve son parent ; supprimer tout le bloc retire aussi ses
annotations. L’insertion d’une image ne scinde plus implicitement une description,
une citation ou une légende : elle doit cibler un paragraphe ordinaire, une frontière
de bloc ou une structure entièrement sélectionnée. L’insertion avant un bloc et
après une attribution vide conserve les identités de ses zones existantes.

Le tampon `TextInput` est partagé par la saisie egui et les remplacements explicites.
Il conserve la première erreur et refuse les gestes suivants du même brouillon :
une suppression refusée ne peut donc pas être suivie d’une insertion publiée.
Les séparateurs collés dans les zones qui n’acceptent qu’une ligne (terme,
définition, attribution et légende) sont refusés avec un message, sans perte de
la sélection. Le caractère objet interne ne devient pas une insertion textuelle.

La saisie native prépare désormais une copie, au lieu de modifier le document
publié pendant `TextEdit`. Les éditions de mise en forme, les remplacements
simples/multiples et les corrections orthographiques rejoignent la même voie de
`Session` : vérifier la session/révision et le brouillon, normaliser une copie,
puis publier contenu, sélection, révision et historique ensemble. Les préparations
par mutation de champs suivie de restauration ne sont plus compilées en production.
Leur usage est limité aux fixtures de tests historiques ; la saisie et les commandes
réelles préparent leur candidat sans modifier la session publiée.

Un brouillon invalide restaure aussi les métadonnées, le modèle, les dépendances
et les ressources ; un brouillon périmé ne remplace pas un commit intervenu entre
temps. Un remplacement multiple est annulé en entier si une occurrence échoue.
Aucune erreur ne modifie la révision, les piles d’annulation/rétablissement ou
l’horodatage d’édition. Une mise en forme au curseur seul met à jour le contexte
de frappe, sans marquer le document modifié ni ajouter d’étape d’historique.
Le remplacement d’un texte reprend son style de départ, au clavier comme par
appel explicite. Remplacer tout le texte d’un titre conserve son niveau ; effacer
la sélection totale du document laisse un paragraphe vide. Les compteurs et le
rendu continuent de lire le document publié.

## Préédition IME entre frames

L’adaptateur `app/ime.rs` conserve l’instantané d’origine, sa garde de session et
révision, le tampon `TextInput` et la sélection temporaire. Chaque prédiction
remplace la sélection d’origine dans ce brouillon ; les prédictions successives
ne réaffectent pas les ancres du document. Le texte marqué reste visible dans
l’éditeur, avec le curseur et le défilement egui, sans publier sa sélection.

Un `Commit` non vide passe une seule fois par `Session::commit_projection`, en
groupe discret. Une composition validée forme donc une étape d’annulation,
distincte des frappes précédentes et suivantes. Le remplacement reste littéral,
reprend le style de départ et subit les mêmes contrôles de frontières et de
contenu que les appels explicites. Une erreur du brouillon reste bloquante
jusqu’à son abandon ou sa validation refusée.

`Preedit("")` reste transitoire. Une validation vide, `Disabled`, Échap, une perte
de focus, un clic, un changement de cible ou de document, et une commande
abandonnent la composition. Le document publié ne nécessite aucun retour arrière.
Les événements terminaux d’un flux abandonné sont écartés pour éviter leur
insertion dans une nouvelle cible. Un nouveau `Enabled` ou la reprise d’une
saisie directe autorise un nouveau flux ; un `Commit` sans préédition est pris
en charge, notamment pour les touches mortes. `Enabled` seul ne met pas la
saisie ordinaire en attente.

Les événements de l’éditeur sont adaptés avant le `TextEdit` ; aucun champ privé
d’egui n’est utilisé. Les recherches, champs de métadonnées et dialogues gardent
leur traitement IME natif. Pendant la composition, le Typst, les cartes de source,
les identités, les ressources, les compteurs, le rendu, la sauvegarde et la
récupération continuent de lire uniquement la session publiée.

Le test manuel macOS `Option E`, puis `e`, confirme les touches mortes après ce
lot. Le premier essai japonais ouvre la fenêtre de candidats mais affiche des
carrés dans l’éditeur : les tests précédents vérifiaient le texte Unicode sans
vérifier la présence des glyphes. Le défaut a été reproduit dans egui et dans
Typst, indépendamment de la conservation correcte du texte par le commit.

Le catalogue `src/fonts.rs` fournit maintenant aux deux moteurs les mêmes
polices embarquées, avec Noto Sans CJK JP 2.004 normal et gras en secours.
Le Latin conserve ses familles et variantes existantes ; les petites capitales
et les mathématiques restent composées par Typst avec les fontes déjà embarquées.
L’export des sources joint uniquement les faces CJK effectivement utilisées et
leur licence, avec l’option `--font-path fonts` indiquée pour le CLI Typst.
Le format `.typsto` reste inchangé. Le choix ultérieur de polices système est
consigné dans le chantier E ; aucune découverte de polices système n’est ajoutée ici.

## Registre typé et premier moteur

`src/engine/` sépare les commandes avec paramètres, les requêtes en lecture seule,
les descriptions et les schémas JSON, les règles de disponibilité, et les erreurs
structurées. `schemars` 1.2.3 produit les schémas depuis les paramètres Rust et
leurs attributs Serde ; `thiserror` fournit l’erreur standard. Le moteur n’importe
ni egui ni Fluent et ne produit aucune phrase d’interface.

Une déclaration commune produit chaque enum, son identifiant, sa conversion
`from_call`/`to_call` et sa spécification. Les identifiants sont validés et uniques
au démarrage. Ils acceptent les espaces de noms hiérarchiques pour préserver
`courrier.model.personal`. Seuls les deux renommages D2 sont appliqués :
`letter.insert_postscript` et `letter.insert_attachment`.

Les paramètres refusent les champs inconnus, les types incorrects et les variantes
inconnues. Une cible est `current`, `all`, ou une sélection explicite comprenant
sa garde et ses positions logiques. Les identités sont des nombres opaques pendant
la session, sans signification de rôle ni persistance dans les archives. Les
offsets sont des valeurs scalaires Unicode. Aucun indice graphique global ne
devient un paramètre du protocole. Les cibles périmées ou étrangères sont refusées.

`Engine` possède la session et sa sélection publiée. `run` et `run_call` appliquent
les mutations via la publication commune de `Session`. `query(&self, ...)` et
`query_call` lisent sans changer la sélection, l’historique ou le contexte de frappe.
`Outcome` fournit la garde courante, la sélection, l’état modifié, les effets de
rendu et une valeur éventuelle. La disponibilité est évaluée par le moteur pour
les commandes migrées ; les autres états de menus restent dans l’application.

Sont raccordés : gras/italique avec `toggle/on/off`, effacement des styles, liens,
paragraphes, trois niveaux de titres, listes, descriptions, citations, insertion
de P.-S./P.J., annuler et rétablir. Les appels `text.insert`, `text.delete` et
`text.select` sont disponibles avec paramètres explicites. Les requêtes couvrent
l’inspection, le texte, les blocs/zones, les métadonnées, le catalogue et la recherche
littérale. Le catalogue décrit uniquement les opérations implémentées, sans
annoncer les familles encore absentes.

L’ancien enum graphique s’appelle `UiCommand` ; `Definition` devient
`Presentation` et garde les clés Fluent, raccourcis, menus et icônes. Les gestes
migrés préparent les paramètres et appellent le moteur par la voie JSON commune.
Le dialogue de lien capture sa cible à l’ouverture et refuse une validation après
changement de révision. La préédition reste un brouillon graphique ; sa validation
et la saisie native passent par `Engine::commit_native_edit`, puis la voie commune de `Session`.
La parité est vérifiée avec `text.insert` appelé par identifiant et JSON.

L’accès transitoire à la session par `Deref/DerefMut` est supprimé. Le curseur egui
reste un cache converti avant les commandes et à la fin de la frame ; les requêtes
lisent la sélection possédée par le moteur. Les déplacements natifs lui transmettent
aussi immédiatement le contexte de frappe, sans écriture directe par `App`.
La consolidation des autres tâches de phase 2 reste distincte de cette encapsulation.

[commands.md](commands.md) et [commands.json](commands.json) sont générés depuis
le registre et comparés aux données courantes par les tests, donc dans la CI.
La comparaison Markdown neutralise uniquement les fins de ligne CRLF des
checkouts Windows ; l’ensemble du contenu reste comparé.
`--commands-catalogue` et `--commands-catalogue-json` sont des diagnostics de
développement ; la CLI séparée et MCP restent dans leurs phases respectives.
Les identifiants et signatures ne sont pas encore gelés.

## Annotations, navigation et raccourcis

Le lot du 23 octobre migre les règles des légendes d’images et des attributions
de citations dans `src/engine/annotations.rs`. Le moteur résout la structure
ciblée et calcule la sélection ; l’application adapte uniquement les touches,
le focus et le défilement. Les événements clavier et les appels JSON rejoignent
les mêmes commandes :

- `text.select { part: "annotation" }` rejoint le début de la légende ou de
  l’attribution ; `part: "body"` revient au corps cité ou sélectionne l’image.
  Sans `part`, la sélection continue ordinaire conserve son contrat.
- `block.split` porte le comportement d’Entrée : nouvelle ligne dans le corps
  cité ; sortie sur une ligne vide ; sortie depuis la légende, l’image ou
  l’attribution sans remplacer leur contenu sélectionné.
- `block.exit` demande explicitement un paragraphe après la structure annotée.
  Le traitement de sortie est partagé avec `block.split`.

Le premier lot limitait ces deux commandes aux structures à annotation facultative.
Le lot suivant étend leur portée aux descriptions, listes et notes (voir ci-dessous).
La portée dépend de la structure ciblée, sans type Courrier imposé.

Les cibles acceptent les gardes de session/révision habituelles. Une sélection
qui traverse partiellement la frontière corps/attribution ne peut pas être
remplacée par une scission ; une citation sélectionnée en entier reste
remplaçable sans attribution orpheline. La sortie explicite conserve les deux zones.
Navigation et refus ne modifient ni contenu, révision, ressources ni piles
d’historique. Une sortie ou une scission est une transaction discrète, avec sélection restaurée à annulation
et rétablissement ; l’identité de l’attribution est conservée.

`RevealSelection` signale la cible à rendre visible : l’interface conserve les
affinités de curseur, le focus et le défilement existants. Une simple nouvelle
ligne du corps cité ne recentre pas le panneau. Le texte indicatif reste virtuel.
Les raccourcis restent écartés pendant la préédition IME.
Le comportement existant des séparateurs après une figure est conservé : son
sérialiseur peut compacter des lignes vides supplémentaires hors de la légende.

Le premier lot registre/moteur a été confirmé par la CI et par les essais
utilisateur le 23 octobre. Le présent lot est vérifié séparément ci-dessous.

## Descriptions, listes, notes et règles de saisie

`src/engine/blocks.rs` étend `text.select { part }` avec `term` et `definition`.
Tab, Maj+Tab et le clic sur un terme vide rejoignent ces commandes. Les zones
existent déjà à la publication : leur navigation n’insère pas de séparateur et
ne crée aucune transaction. Une sélection qui traverse plusieurs éléments ne
peut pas servir à une navigation relative ambiguë.

`block.split` traite désormais les listes à puces/numérotées, les descriptions,
les P.-S. et les mentions de P.J. : nouvelle ligne ou nouvel élément ; Entrée
sur un élément vide revient au paragraphe ordinaire. Dans une description,
Entrée depuis la définition prépare un nouvel élément avec ses deux zones.
Les espaces seuls restent de la saisie dans une liste ou une note ; une
description sans terme ni définition peut être quittée même avec des espaces.
`block.exit` permet aussi de sortir après le groupe de P.-S./P.J. sans remplacer
le texte sélectionné. Les gardes et protections des frontières restent communes.

`src/engine/typing.rs` porte les règles de frappe existantes : préfixes de titres,
listes, description et citation suivis d’espace ; fermetures `*…*` / `_…_` ;
« : espace » qui termine le terme d’une description. `text.insert` reste littéral
par défaut. Le paramètre explicite `input_rules: "typing"` active ces règles sur
une frappe déclenchante ; il ne parse pas un texte collé comme un script. Une
sélection non vide ou une saisie sans règle suit le remplacement littéral.
L’instantané d’annulation contient la saisie littérale complète, y compris le
caractère déclencheur, et sa sélection. Une erreur ne publie pas ce brouillon.

L’adaptateur egui ne reconnaît plus les préfixes ou délimiteurs : il demande au
moteur si la frappe doit être interceptée. Les autres frappes gardent le tampon
natif et son regroupement. La préédition IME reste isolée et sa validation
littérale n’active aucune transformation. Les gestes d’images sont migrés dans le
lot suivant ; le dernier adaptateur de focus restant concerne l’orthographe.

## Images et signatures

`src/engine/images.rs` porte désormais `insert.image`, `image.replace`,
`image.remove`, `image.set_alt` et `document.set_signature`. L’identifiant
`insert.image` est conservé. Le registre compte 27 commandes et six requêtes.

Une image est ciblée par un `BlockTarget { guard, block_id }`. Les identifiants
numériques des ressources du peintre ne sont pas des paramètres publics ; un
adaptateur les convertit en identité de bloc. Les cibles périmées, étrangères,
inconnues ou désignant un bloc textuel sont refusées avant la lecture d’un fichier.
`document.structure` expose aussi le chemin interne et le texte alternatif.
Les légendes continuent d’être éditées par les commandes textuelles communes.
L’insertion accepte en plus une légende initiale littérale sur une ligne et un
texte alternatif. Le remplacement préserve ces deux annotations et leur identité.

`ImageInput` accepte un chemin local explicite (`kind: "path"`) ou une copie
(`kind: "embedded"`, extension et données en base64 standard). La copie sert
notamment à la signature du profil ; `base64` 0.22, déjà présent transitivement,
devient une dépendance directe. Les deux voies utilisent le même décodeur typé,
normalisent JPEG en `jpg` et conservent les formats PNG/JPEG/SVG et la limite de
16 Mio. Les erreurs portent codes et données ; leur adaptation Fluent reste dans
l’application. Les fonctions historiques de vignettes et de profil gardent leur
adaptateur de messages, à séparer avec le chantier i18n de phase 3.

Les données sont lues et validées dans un candidat complet. Son budget de
ressources et dépendances est vérifié avant `Session::commit_snapshot`, nouvelle
entrée de la publication commune. Aucune erreur ne publie les nouveaux octets,
les métadonnées ou une sélection partielle. La limite de 64 Mio conserve sa
politique : les anciennes images inutilisées ne doivent pas empêcher un import ;
le modèle et ses ressources restent pris en compte. En mémoire, les anciennes
données restent disponibles pour annuler. L’archivage et l’export continuent de
n’écrire que les ressources utilisées par le document ou le modèle.

Les boutons, le dépôt d’un fichier et les raccourcis d’image appellent le moteur
par identifiant/JSON. Le dialogue de texte alternatif capture sa cible à
l’ouverture ; une validation périmée reste refusée. Après une édition réussie,
seule la garde renvoyée par ce résultat sert au retour interne vers la légende.
Fermer un dialogue périmé ne sélectionne pas une autre image. Les dialogues
natifs capturent leur cible avant le choix du fichier.

La signature reste limitée au type Courrier. Le moteur reçoit une image explicite
ou `null` pour la retirer ; il ne lit et ne modifie jamais le profil global.
« Signature du profil » transmet une copie des octets conservés par l’application.
Une image déjà identique n’ajoute ni ressource, révision ni transaction. Le retrait
reste annulable et conserve la réserve manuscrite du modèle. L’interface préserve
le focus du panneau des informations. L’initialisation du profil à la création
reste avec la famille `document.new`, encore à migrer.

## Recherche et remplacement

Le lot 2.4.4 conserve `search.find` et ajoute `edit.find_next`,
`edit.find_previous`, `edit.replace_current` et `edit.replace_all` au registre.
Le catalogue compte désormais 31 commandes et six requêtes. La recherche reste
exacte et littérale, Unicode, insensible à la casse par défaut ; accents distincts,
sans regex ni correspondance entre zones. Le périmètre actuel est tout le contenu
éditable, y compris termes, définitions, attributions et légendes ; ni propriétés
privées, ni métadonnées, ni modèle. Les séparateurs structurels sont exclus.

`search.find { query, case_sensitive: false }` est une lecture pure. Elle retourne
`SearchResults { guard, matches }`, chaque occurrence ayant une cible logique
gardée et son texte exact. Un motif dépassant les limites de compilation produit
`search_too_large`, repris par le message existant de l’interface. Une requête vide
ou contenant un séparateur retourne zéro résultat. Le cache graphique est lié à
la session, à la révision, au texte recherché et à la casse ; seuls les rectangles
de surbrillance conservent des indices dans le tampon egui. L’interface ne calcule
plus elle-même les correspondances ou la destination de navigation.

La navigation accepte `query`, `case_sensitive` et `target` (sélection courante
par défaut, ou cible logique explicite). Elle boucle en début/fin, remplace la
sélection précédente et met à jour le contexte de frappe sans modifier le document
ni son historique. Le résultat indique `index` (à partir de zéro) et `count`, avec
l’effet `RevealSelection`. Le clic sur une occurrence appelle `text.select` avec
sa garde, puis l’adaptateur affiche cette sélection unique dans l’éditeur et l’aperçu.

Remplacer une occurrence demande `query`, `case_sensitive`, `occurrence` et
`replacement`. Le moteur vérifie la session/révision, le texte attendu et que la
cible est une correspondance complète de cette recherche. Aucune référence
externe n’est actualisée implicitement, même après annulation. Le remplacement
et le choix du résultat suivant appartiennent à la même commande. Une substitution
identique peut déplacer la sélection sans créer de transaction.

Tout remplacer reçoit `guard`, `query`, `case_sensitive` et `replacement`.
Les correspondances proviennent toutes du même état et sont remplacées en ordre
inverse dans un candidat validé par la voie commune de publication. Une erreur
ne publie rien ; les nouvelles occurrences créées ne sont pas remplacées en boucle.
Les styles et ressources sont préservés, l’ensemble se défait en une étape. Les deux
commandes renvoient le nombre remplacé dans `Outcome.value.count`. Un remplacement
avec séparateur est refusé (`search_one_line`) ; une navigation ou un remplacement
sans résultat est indisponible (`search_no_results`), et une occurrence incorrecte
est refusée (`search_mismatch`). Les champs, messages Fluent, focus et défilement
restent gérés par egui ; Entrée/Maj+Entrée conservent le focus de recherche.

## Orthographe documentaire

Le lot 2.4.5 ajoute cinq commandes (`spelling.next`, `spelling.previous`,
`spelling.correct`, `spelling.ignore`, `spelling.reset`) et deux lectures
(`spelling.issues`, `spelling.suggestions`). Le catalogue compte 36 commandes
et huit requêtes. Les boutons, menus et raccourcis documentaires passent par les
mêmes appels typés/JSON ; le dernier adaptateur `focus_annotation_target` est supprimé.

`spelling.issues {}` retourne `guard`, `language`, `available` et `issues`.
Chaque `SpellingIssue` contient une cible logique gardée, le `word` exact et sa
langue. L’analyse couvre le contenu éditable, y compris les annotations et les
descriptions, sans analyser le source Typst, les propriétés privées ou le modèle.
Les adresses, URL et nombres gardent leurs exclusions existantes. Les dictionnaires
embarqués français et anglais restent les seuls disponibles. Une autre langue
produit `available: false` et une liste vide, sans modifier le document.

La navigation accepte une `target` (courante par défaut ou sélection explicite),
boucle et renvoie `RevealSelection`. Elle conserve révision, contenu et historique.
La correction accepte `issue` et `replacement` : le moteur résout la garde,
vérifie la langue et que le texte attendu est encore une faute complète de
l’analyse courante. Un fragment, une ancienne révision, une autre session ou un
mot désormais accepté sont refusés. Le remplacement est littéral, sans séparateur
structurel ; il conserve les styles et rejoint `Session::replace_text`.

Ignorer accepte `issue` et ajoute sa clé normalisée aux exceptions de la langue
du document. Réinitialiser accepte `guard` et efface les exceptions de toutes ses
langues, sans toucher au dictionnaire personnel. Les deux actions préparent un
instantané candidat avant la publication commune. Elles sont annulables ; une
réinitialisation déjà vide n’ajoute aucune étape. Les métadonnées `.typsto` et
la présentation Typst conservent leur fonctionnement existant.

`spelling.suggestions` accepte une `target` de forme `word { language, word }`
ou `issue { issue }`, avec le discriminant `kind`. La seconde forme valide la
faute avant de proposer des corrections ; la première permet une requête explicite
indépendante de la sélection. Le résultat donne langue, mot et suggestions, sans
déplacer le curseur. Les erreurs sont typées : `spelling_mismatch`,
`spelling_language`, `spelling_word`, `spelling_no_dictionary`,
`spelling_no_issues` ou `spelling_dictionary`, en plus des erreurs communes de cibles.

L’algorithme de tokenisation, les exclusions, le cache par ligne et le calcul
Spellbook sont partagés avec le worker existant. L’interface garde l’analyse
différée de 400 ms et les suggestions en arrière-plan. Le moteur conserve un cache
d’analyse lié à la session/révision, à la langue et à l’ensemble des mots acceptés ;
ses lectures ne changent que ce cache, jamais l’état documentaire ni ses groupes
de frappe. Les appels sans interface fonctionnent sans worker egui.

Conformément à V5, `configure_spelling` reçoit une copie en lecture des mots
personnels. Une modification de cette configuration invalide les résultats concernés
sans changer révision ou historique. L’activation, l’apprentissage, la persistance
et l’édition du dictionnaire personnel restent dans l’application, hors du catalogue
documentaire. Le moteur ne lit aucun fichier de préférences globales.

Le menu contextuel conserve la référence capturée ; une suggestion tardive ne peut
jamais viser un autre mot ou un autre document. Les actions clavier utilisent au
contraire la faute de la sélection actuelle, même après fermeture d’un ancien menu.
Les fenêtres, le focus, le défilement, les messages Fluent et les rectangles de
soulignement restent dans l’adaptateur graphique.

## Métadonnées et options

Le lot 2.4.6 ajoute `metadata.set`, `document.set_option` et la lecture
`models.presentation`. Le catalogue compte 38 commandes et neuf requêtes.
Les champs egui sont déplacés de `document_types` vers `app/information_fields` :
ils préparent des valeurs locales, puis appellent le moteur par identifiant/JSON.
`information_ui` ne modifie plus les métadonnées publiées et n’appelle plus
l’adaptateur transitoire `Session::record`.

Les descripteurs des types portent une identité `MetadataField` en anglais :
`sender.*`, `recipient.*`, `letter.*` et `meeting.*`, selon les clés de l’inventaire.
Les paramètres des fonctions Typst et les noms des champs d’archives restent
inchangés. `metadata.get {}` complète les données existantes avec `fields` :
identité, valeur, groupe, nature texte/date, caractère multiligne et obligation
pour l’export. Les libellés Fluent et identifiants graphiques ne sont pas sérialisés.

`metadata.set { guard, field, value, input: "discrete" }` est une mutation gardée.
Un champ connu mais absent du type actif est indisponible ; les noms inconnus,
changements de type ou accès indirects à la signature/options ne sont pas des
champs acceptés. Les textes sont littéraux, avec validation des séparateurs selon
le caractère multiligne et normalisation CRLF/CR en LF. Un champ obligatoire peut
rester vide pendant la rédaction ; sa contrainte reste vérifiée à l’export.

Le mode explicite `input: "typing"` conserve les groupes de frappe de 650 ms,
identifiés par le champ typé. Changer de champ ou effectuer une action discrète
sépare les transactions. `EditGroup::Metadata` ne contient plus d’identifiant egui.
Un appel sans changement ne crée ni révision ni étape. Corps, styles, ressources,
sélection de rédaction et préférences globales sont conservés. L’interface garde le
focus du champ, y compris à l’annulation/rétablissement des métadonnées.

Pour une date, le moteur normalise une date explicite valide en ISO, à validation
discrète ou à dix caractères pendant la frappe. Une saisie partielle ou un libellé
libre reste possible. `normalize_explicit` ne consulte jamais l’horloge : les alias
`today`/`aujourd’hui` sont refusés dans l’API (`metadata_fixed_date`). Le bouton
« Aujourd’hui » et l’alias saisi dans l’interface résolvent la date locale avant
`metadata.set`, puis cette valeur fixe est conservée. Datify et la présentation
finale restent du côté du modèle Typst.

`document.set_option { guard, option: { key, value } }` a quatre variantes strictes :
`paper` (format pris dans le catalogue), `landscape` (booléen), `language` (code
défini par l’application), `bullet_marker` (un caractère ou `null` pour automatique).
La signature, les exceptions orthographiques et le type ne passent pas par cette
commande. Les contraintes de page du modèle fourni sont vérifiées dans le moteur,
même sans UI ; un choix identique reste idempotent. La langue du document reste
indépendante de celle de l’interface.

Les marqueurs non automatiques sont validés contre `editor-options` évalué dans
le vrai modèle Typst, avec les ressources et dépendances du document, jamais contre
une permission fournie par l’appelant ou le cache graphique. Les choix libres ne
sont possibles que si le modèle les autorise. `models.presentation {}` retourne
`guard`, `fixed_page`, `bullet_markers` et `custom_bullet_marker`. Un cache interne
lié à la garde évite une seconde évaluation du même état ; la lecture ne modifie
pas le document. La publication d’une option n’exige pas de compiler tout le
document : les autres contraintes internes du Typst restent du ressort du rendu.

Une valeur personnalisée invalide reste dans un brouillon graphique gardé et
affiche le message existant ; elle n’entre ni dans les options, ni dans l’archive,
ni dans l’historique. Les erreurs métier sont codées (`metadata_field`,
`metadata_fixed_date`, `model_fixed_page`, `bullet_marker_invalid`,
`bullet_marker_policy`, `model_presentation`) et adaptées en messages dans App.
Champs et options préparent un instantané candidat puis utilisent
`Session::commit_snapshot`, avec refus sans publication partielle.

Le lot suivant migre les préparations de métadonnées liées au changement de modèle.
Celles de la création restent dans le lot 2.4.8.

## Choix et import des modèles

Le lot 2.4.7 ajoute `document.set_model`, le préréglage historique
`courrier.model.personal` et la lecture `models.list` : 40 commandes et dix requêtes.
Le moteur utilise le même catalogue de modèles intégrés que l’application.
`models.list {}` expose les identifiants des modèles du type actif, leur état
actif et leurs contraintes de page. Un compte-rendu conserve une liste vide tant
qu’aucun modèle n’est fourni ; aucun modèle factice ou changement de type n’est ajouté.

`document.set_model { guard, model }` accepte deux formes strictes :
`{ kind: "source", source }` ou `{ kind: "path", path }`. La garde est contrôlée
avant la lecture d’un fichier. La lecture UTF-8 est limitée à 1 Mio ; la limite
s’applique aussi aux sources explicites. Le dialogue natif reste dans App, capture
sa garde avant de s’ouvrir et transmet le chemin choisi. `document.import_model`
reste donc une action graphique, avec une opération documentaire distincte.

Le préréglage `courrier.model.personal { guard }` conserve son identifiant et
appelle la même préparation que l’installation de sa source. Le moteur résout les
dépendances par le catalogue de capacités/paquets et les valide. Pour un modèle
fourni reconnu exactement, il utilise les exemplaires embarqués ; pour un modèle
dérivé, il conserve les exemplaires déjà présents dans le document aux chemins
nécessaires. L’import ne parcourt pas le dossier du fichier et ne télécharge rien.

Le candidat complet contient le modèle, les dépendances et les options préparées.
Si la source ou les dépendances changent, la puce revient au mode automatique ;
les réglages de page d’un modèle fourni sont appliqués (A4 portrait pour le courrier
personnel). Les autres métadonnées, dont langue et signature, restent identiques.
Réinstaller la même source avec les mêmes dépendances conserve les options choisies.

Le moteur vérifie la fonction publique attendue par le type, puis compile le corps
avec les métadonnées et les ressources du candidat, et lit sa politique d’éditeur.
Un modèle incompatible, mal formé ou impossible à composer est refusé avant toute
publication. Le budget de ressources est partagé avec les opérations d’images et
signatures dans `engine/resources.rs`. Les anciennes ressources inutilisées restent
en mémoire pour annuler ; sauvegarde et export gardent leur filtrage existant.

Après validation, `Session::commit_snapshot` publie tout en une transaction
discrète. Corps, styles et sélection sont conservés. Une installation identique
est validée sans ajouter de révision ou de transaction. `Outcome.value.presentation`
renvoie la politique déjà évaluée ; le cache du moteur et l’interface l’utilisent
immédiatement. Annuler/rétablir restaure modèle, dépendances et options ; l’interface
relit aussi la présentation quand la source ou les dépendances changent.

Les erreurs sont codées (`model_io`, `model_utf8`, `model_too_large`,
`model_dependencies`, `model_validation`) avec les erreurs communes de garde et de
budget. La validation fournit le contrat attendu et le détail du compilateur ;
leur présentation reste dans App. Le validateur Typst existant est réutilisé, avec
ses diagnostics ; sa séparation complète des adaptations i18n relève de la phase 3.

## Fichiers, création et récupération — lot 2.4.8

Les commandes `document.new`, `document.open`, `document.close`, `document.save`,
`document.save_as` et `document.export` sont typées et annoncées dans le catalogue.
Les parcours des menus, raccourcis et dialogues appellent ces opérations. Le moteur
possède l’état ouvert/fermé ; l’application en conserve un cache pour l’affichage.
`document.inspect` expose cet état, le chemin et l’origine récupérée.

Toutes ces opérations portent une garde documentaire. Créer, ouvrir, fermer ou
restaurer un autre document refuse un brouillon non enregistré, sauf décision
explicite `discard_changes: true`. L’UI conserve son dialogue enregistrer/abandonner/
annuler. Une ouverture réussie installe une nouvelle identité de document et une
sélection initiale, sans historique ; les erreurs de fichier conservent la session
publiée. Les caches et dialogues documentaires sont réinitialisés seulement après
réussite. L’ancien chemin reste sans effet sur les cibles gardées du nouveau document.

`document.new` reçoit le type, l’identifiant du modèle fourni et, facultativement,
`initial_values: [{ field, value }]` et une signature `ImageInput`. Le corps est vide.
Les champs sont validés par la même voie que `metadata.set` ; champs hors type,
doublons, contrôles et date dynamique `today` sont refusés. Le moteur ne lit ni le
profil global ni l’horloge. L’application fournit une copie des valeurs du profil
et de l’image déjà chargée. Les préférences restent indépendantes du document.

La sauvegarde réutilise `storage::save` et `atomic-write-file`. Elle ne change ni
révision, ni sélection, ni historique ; son repère en mémoire conserve les ressources
nécessaires pour annuler, tandis que le fichier filtre les images inutilisées.
`document.save` peut omettre le chemin si la session en possède un. Une première
sauvegarde vers un autre chemin refuse de remplacer un fichier existant.
`document.save_as` exige un chemin et refuse l’écrasement par défaut ; l’UI transmet
`overwrite: true` après le choix et la confirmation du dialogue natif. La publication
sans remplacement utilise un fichier temporaire voisin et `persist_noclobber`, après
l’écriture atomique commune, pour protéger aussi une cible apparue pendant l’écriture.
Une erreur ne modifie pas le chemin, le repère enregistré ou l’état récupéré.

L’export reçoit `format: pdf | typst | both`, un dossier neuf/vide et `page_images`.
Les défauts restent `both` et `true`. Le dialogue expose ces choix. Le cœur vérifie
la garde, les champs obligatoires et les dépendances, puis capture un instantané
immuable. L’appel synchrone et l’export en arrière-plan de l’interface consomment la même
`ExportTask`, le même compilateur hors ligne et le même writer (`exporting.rs`).
Le dossier est revérifié avant écriture et les fichiers sont créés sans remplacement.
Le format Typst contient sources, métadonnées, modèle et ressources autonomes,
dont les polices CJK utilisées ; PDF seul ne copie pas ces sources. Les PNG sont
indépendants du format. L’export ne rend pas le document enregistré et n’ajoute pas
d’historique. L’export multi-fichiers ne prétend pas à l’atomicité de `.typsto` (D16).

`recovery.list`, `recovery.restore` et `recovery.delete` utilisent les entrées du
service explicitement configuré ; leurs références opaques ne sont pas des chemins,
ni des indices de liste. Elles ne se réutilisent pas entre services. La restauration
prépare et valide la copie avant publication, sans chemin d’enregistrement et avec
l’obligation d’enregistrer. Le verrou reste détenu jusqu’au transfert au worker de
récupération existant. L’UI garde l’échéancier des copies, le dialogue et les messages ;
les entrées qu’elle affiche sont un cache du résultat de `recovery.list`. La suppression
explicite n’ajoute aucune transaction. Aucune préférence globale ou racine implicite
n’est introduite dans le protocole documentaire.

Le lot est réparti en commits locaux, avec une seule poussée finale. Aucun format
d’archive ni aucune dépendance Cargo n’est changé. Les diagnostics des services
historiques de stockage/rendu seront séparés des libellés Fluent en phase 3.

## Lots atomiques — D15/D16

`document.batch` exécute des appels du registre sur une session privée avec la même
identité et la même révision initiales. Toutes les étapes sont décodées et leur
admissibilité (`CommandSpec.batch_step`) est vérifiée avant exécution ou lecture
d’imports. Ouverture/fermeture/création, sauvegarde/export, récupération, historique,
requêtes et imbrication sont exclus. Les gardes explicites suivent strictement les
révisions du brouillon, sans réancrage ; les cibles symboliques se résolvent à chaque
étape. Le contrat détaillé est dans [atomic-batches.md](atomic-batches.md).

Les éditions utilisent les commandes et validations communes. L’historique provisoire
est libéré entre étapes, et les imports sont bornés par rapport aux ressources de
départ. La configuration orthographique personnelle est copiée en lecture ; les
services globaux et la récupération ne sont pas transférés au brouillon. Limites :
1–256 étapes, 4 Mio de JSON sérialisé, 64 Mio de ressources supplémentaires.

Le candidat final passe par `Session::commit_snapshot` sur une seconde session
préparée, avec une unique transaction discrète. Une sauvegarde finale explicite
réutilise les opérations de fichiers validées, avant toute publication de la session.
Après sa réussite, `Session::adopt_batch` transfère l’état sans IO ni validation
faillible et raccorde l’historique antérieur. Un refus, même de sauvegarde, ne publie
ni contenu, ni ressources, ni sélection, ni repère enregistré. Sans changement de
contenu, aucune révision ou transaction n’est ajoutée et le rétablissement reste intact.

L’adaptateur graphique applique uniquement les effets finaux : politique du modèle,
sélection, synchronisation et nettoyage de récupération après sauvegarde réussie.
Les erreurs conservent leur cause structurée et l’index de l’étape ; l’interface
traduit le message avec une numérotation à partir de un. Aucun transport CLI/MCP ni
nouvelle action de menu n’est ajouté dans ce lot.

## Session encapsulée et saisie native opaque

`Engine` ne se convertit plus en `Session` par `Deref` ou `DerefMut` et ne fournit
aucun getter de cette session. `engine/state.rs` définit des lectures explicites :
document, métadonnées, modèle, dépendances, chemin, révision, horodatage, état
récupéré/modifié et disponibilité de l’historique. Les références rendues sont
immuables ; les compteurs ou gardes sont copiés. Ces lectures permettent de peindre
l’interface sans sérialiser des données JSON à chaque frame. Les requêtes du registre
restent la voie d’accès sans interface ; les requêtes d’aperçu/compteurs sont encore
à consolider.

`begin_native_edit` prépare un `NativeEdit` opaque. L’adaptateur ne peut lire que
sa projection initiale, sa sélection et sa garde ; il ne peut modifier l’instantané
conservé dans le handle. `commit_native_edit` reçoit la nouvelle projection et la
sélection, puis applique la garde et la validation commune. La frappe conserve ses
groupes naturels ; une composition validée constitue une transaction discrète.
Un handle périmé, étranger ou utilisé sans document ouvert est refusé sans publication.
L’IME garde son brouillon entre frames et restaure chaque prédiction depuis la
projection du handle, sans l’inscrire dans le document.

Les déplacements du curseur par egui ou par retour depuis l’aperçu appellent
`update_adapter_cursor`. Le moteur contrôle les bornes, met à jour sélection et
style de frappe et clôt le groupe de frappe, sans modifier le document, sa révision
ou l’historique. La simple synchronisation du cache de curseur ne réinitialise pas
un gras/italique activé au point d’insertion.

Les scénarios natifs de capture utilisent également les commandes de métadonnées
et la publication contrôlée de projection. `App` ne reçoit plus de `Snapshot` de
session et n’appelle plus `record/remember`. `Engine::commit_projection` est privé
au moteur. Les anciens helpers de mutation/restauration de `Session` ne sont compilés
que sous `cfg(test)` pour arranger des scénarios historiques.

Les assertions internes du moteur examinent ses champs privés depuis son module
de tests. Les tests de l’application utilisent les lectures explicites et un helper
de fixture à portée fermée dans `engine/test_support.rs`, lui aussi exclusivement
sous `cfg(test)`, lorsqu’ils doivent construire un état volontairement invalide ou
manipuler une révision pour vérifier les refus. Aucun retour de session mutable ni
proxy de compatibilité n’est ajouté à l’API de production. Aucun identifiant du
catalogue, format d’archive, dépendance Cargo ou fonctionnalité produit n’est changé.

## Vérifications

- 321 tests de l’application réussis, dont 178 tests ajoutés depuis le début de
  phase 2 : identités et cibles, refus des frontières, suppressions complètes,
  refus atomiques, brouillons périmés, ressources, contexte de frappe et parité
  des remplacements Unicode entre événements egui et appels explicites.
- Les 14 nouveaux scénarios IME couvrent les prédictions sur plusieurs frames,
  leur affichage, les validations sans préédition et dans une même frame,
  l’annulation, les événements tardifs, les frontières, les styles et identités,
  l’historique, les pertes de focus et les changements de document. La parité avec
  un remplacement explicite et les archives de sauvegarde/récupération sont testées.
- Trois tests supplémentaires reproduisent les glyphes manquants, puis vérifient
  le texte validé et rouvert, les glyphes japonais/chinois dans toutes les familles
  et variantes de l’éditeur, les glyphes réellement composés par Typst (dont gras,
  petites capitales et maths), et l’export des faces utilisées avec leur licence.
- Clippy sur toutes les cibles, avec le diagnostic `editor-model-spike`, sans
  avertissement ; ses 194 tests partagés réussissent aussi.
- Dix tests du moteur couvrent les schémas, conversions, données invalides,
  cibles périmées/étrangères, refus atomiques, styles explicites, historique,
  lectures seules, portée Courrier et fichiers de catalogue générés. Quatre essais
  d’intégration comparent les gestes graphiques aux appels JSON, la sélection
  publiée pendant l’IME et la validation différée du dialogue de lien.
- Six nouveaux tests du moteur vérifient navigation, sorties et scissions,
  annotations vides ou remplies, Unicode et styles, ressources, refus atomiques,
  identité de l’attribution et sélection/historique. Trois essais d’intégration
  comparent Tab, Maj+Tab et Entrée aux appels JSON, avec annulation/rétablissement.
- Six tests du lot descriptions/listes/notes contrôlent navigation sans historique,
  scissions/sorties, lignes vides ou composées d’espaces, saisie littérale/interactive,
  annulation du littéral, cibles périmées et frontières. Quatre tests d’interface
  comparent les gestes aux appels JSON et vérifient une validation IME littérale.
- Six tests du moteur couvrent les imports PNG/JPEG/SVG, les annotations et leurs
  identités, la disparition du fichier original, les cibles invalides/périmées,
  les données invalides, les budgets, la signature et la publication d’un candidat
  complet. Trois essais d’interface comparent dépôt/import, remplacement, texte
  alternatif, suppression et copie de signature aux appels JSON ; ils vérifient
  aussi le refus d’un dialogue périmé, l’archivage et l’indépendance du profil.
- Formatage conforme et nouveaux fichiers de production sous 800 lignes.
- Trois nouveaux tests du contrat natif vérifient l’isolation du handle, la publication
  et l’historique communs, les refus après changement de révision ou de document et
  sans document ouvert, puis les déplacements de curseur, le style de frappe et la
  conservation du rétablissement. Les tests de saisie/IME, des blocs et des lots
  continuent de passer après adaptation des fixtures aux accès explicites.
- Deux tests de session et douze tests du moteur couvrent les lots : isolation,
  publication unique, limites, erreurs tardives, gardes périmées/étrangères, étapes
  interdites, imports et autonomie, échec de sauvegarde, conservation du rétablissement,
  lots sans changement et configuration orthographique de lecture. Quatre essais
  d’interface vérifient la parité JSON, l’annulation globale, la politique du modèle,
  le focus, la synchronisation et le nettoyage de récupération après réussite seulement.
- Onze tests du moteur vérifient création et valeurs initiales, signatures, catalogues,
  gardes, décisions sur le brouillon, erreurs atomiques, historique, sauvegarde sans
  écrasement, archives corrompues, récupération par références, formats d’export et
  instantané asynchrone. Quatre essais d’interface comparent la copie du profil,
  sauvegarde/ouverture et récupération aux appels typés/JSON, conservent sélection,
  focus, caches et préférences, puis vérifient un export pendant une nouvelle édition.
- Huit tests du moteur couvrent catalogue et portée des types, contrat et compilation,
  limites et erreurs de fichiers, gardes, idempotence, réglages du préréglage, refus
  atomiques, budget et autonomie après suppression du fichier importé. Trois essais
  d’interface comparent import de source/fichier et choix du modèle aux appels JSON,
  puis contrôlent erreurs, cibles périmées, sélection, préférences et restauration
  des options et de la politique d’éditeur avec l’historique.
- Huit tests du moteur couvrent les champs des deux types, dates sans horloge,
  caractères et types invalides, gardes périmées/étrangères, regroupement par champ,
  absence de mutation du corps/ressources, page fixe, charte des puces évaluée en
  Typst, lectures pures, annulation/rétablissement et persistance autonome.
  Trois essais egui comparent la frappe réelle, le bouton « Aujourd’hui » et
  l’édition d’un marqueur aux appels JSON, avec focus, sélection, brouillon invalide
  et historique. Les tests existants du panneau et des choix de modèles restent verts.
- Huit nouveaux tests du moteur vérifient lectures pures, parité avec le worker,
  suggestions, navigation dans les deux sens, six rôles de zones, styles, historique,
  normalisation des exceptions, configuration personnelle, refus atomiques et
  dictionnaires indisponibles. Quatre essais d’interface comparent les commandes aux
  appels JSON, contrôlent les suggestions communes, les cibles de menus périmées,
  la sélection après fermeture d’un menu et la séparation des préférences globales.
  Les deux scénarios existants conservent aussi le clic réel dans un menu de
  suggestions et la persistance des exceptions dans les archives.
- Neuf nouveaux tests du moteur couvrent navigation, casse Unicode, cibles
  explicites, six zones éditables, styles, textes privés exclus, historique,
  gardes périmées/étrangères, texte attendu, remplacements identiques ou vides,
  nouvelles occurrences non récursives, paramètres invalides et motif trop grand.
  Deux essais egui ajoutent la parité JSON pour Entrée/Maj+Entrée, remplacer et
  tout remplacer, annuler/rétablir, maintien du focus, invalidation du cache entre
  documents et refus d’un clic périmé. Les essais existants de clic réel et de
  surlignage dans l’aperçu continuent de réussir.
- Huit captures strictement identiques après les premiers lots, dans
  `output/baseline/run-ioma5t0l/` et `output/baseline/run-uvqd1xwk/`, puis après la
  voie commune dans `output/baseline/run-rot7603t/`. La comparaison finale sur le
  code de ce lot est également identique, dans `output/baseline/run-do7d7apa/`.
  Après l’isolation IME, les huit captures restent strictement identiques dans
  `output/baseline/run-t2opy5ig/`.
  L’ajout du secours CJK conserve également les huit captures dans
  `output/baseline/run-wuwd1114/`. La capture supplémentaire
  `output/font-diagnostics/cjk-4jxcam3g/cjk.png` a été vérifiée visuellement :
  hiragana, kanji et chinois sont lisibles dans les deux panneaux, en normal et gras.
- Après le raccordement du registre et du moteur, les huit captures restent
  strictement identiques dans `output/baseline/run-uwa0olfg/`. Le lot des
  annotations conserve aussi les huit références dans `output/baseline/run-ip5gvm0m/`.
  Les règles de saisie et de blocs conservent les huit captures dans
  `output/baseline/run-s0447__w/`.
- Les huit captures du lot images/signatures sont strictement identiques dans
  `output/baseline/run-oor20pp6/`. Un premier passage avait produit un PNG de profil
  vide ; la relance complète conserve les références sans les modifier.
- Le lot recherche/remplacement conserve les huit captures strictement identiques
  dans `output/baseline/run-qaqb4mgo/`.
- Le lot orthographique conserve également les huit captures strictement identiques
  dans `output/baseline/run-z0q4zynu/`.
- Le lot métadonnées/options conserve les huit captures strictement identiques
  dans `output/baseline/run-zekzbxe_/`.
- Le lot modèles conserve les huit captures strictement identiques dans
  `output/baseline/run-zs_6e669/`. Un premier passage
  (`output/baseline/run-bsfjew61/`) ne différait que par le curseur de saisie du
  scénario orthographe : 233 pixels dans un rectangle de 4 × 59 pixels.
  La relance utilise le même binaire et ne modifie pas les références.
- Le lot fichiers conserve les huit captures strictement identiques dans
  `output/baseline/run-elkplziq/`, sans modifier les références.
- Les lots atomiques conservent les huit captures strictement identiques dans
  `output/baseline/run-oni7qo73/`, sans modifier les références.
- La suppression des accès transitoires conserve les huit captures strictement
  identiques dans `output/baseline/run-kndfgtd7/`, sans modifier les références.
- Le lot de lectures de rendu et compteurs conserve les huit captures strictement
  identiques dans `output/baseline/run-87yhkaa6/`, sans modifier les références.
- La fixture multilingue et les fontes livrées ont été recompilées avec le CLI
  Typst 0.15.1, `--ignore-system-fonts --font-path fonts`, et des répertoires de
  paquets/cache vides. Les tests d’export contrôlent séparément les faces copiées.
- Le scénario existant qui essayait une insertion d’image à chaque caractère
  conserve ses contrôles de validité. Il vérifie désormais soit l’insertion aux
  positions autorisées, soit le refus sans mutation dans une zone protégée.

La CI du lot précédent a signalé la dépréciation de `AtomicU64::fetch_update`
par sa version de Rust stable, plus récente que l’installation locale. Les deux
allocateurs utilisent désormais `identity::next`, fondé sur
`compare_exchange_weak`, sans changer le minimum Rust déclaré. Deux tests
vérifient l’unicité concurrente et le refus de boucler au dépassement de capacité.

## Lectures de rendu et compteurs

Le lot du 24 octobre 2026 ajoute trois requêtes au registre : `render.locate`,
`render.selection_boxes` et `document.statistics`. Le catalogue généré compte
49 commandes et quatorze requêtes. Le [contrat détaillé](read-queries.md) précise
les cibles, les réponses, les unités et les erreurs.

Le moteur possède les correspondances immuables `RenderSync`. Il résout les cibles
logiques vers la carte de source Typst pour la navigation avant et le surlignage ;
la navigation inverse renvoie une cible logique gardée, puis l’interface appelle
`text.select`. Le focus et le défilement restent graphiques. Les algorithmes de
cartographie, transformations du modèle et quadrilatères sont conservés.

Chaque rendu accepté reçoit une référence opaque avec identité de document,
révision et identité propre du rendu. `document.inspect` expose cette référence
et les dimensions des pages, ou `render: null` sans rendu courant. Un nouveau
rendu remplace la référence précédente, même à révision identique. Le worker
conserve la garde complète dans son enveloppe et dans sa réponse ; un résultat
tardif d’un autre document ne peut pas remplacer l’aperçu, même si les révisions
coïncident. Le remplacement et la fermeture de session libèrent le rendu du moteur.

Le compteur Unicode existant et ses conventions sont conservés. Son cache est
désormais dans le moteur, indexé par document/révision et plage de sélection. Les
métadonnées, textes alternatifs, signatures et textes du modèle ne sont pas comptés.
Le brouillon IME reste exclu jusqu’à sa validation. Ces lectures ne publient aucune
transaction et ne modifient pas le contexte de frappe, la sélection ou l’historique.

Sept nouveaux tests couvrent les compteurs document/cible, Unicode, les gardes,
le cache lors d’un remplacement à révision identique, les correspondances des
zones (dont légendes, attributions, descriptions et notes), les quadrilatères
sur plusieurs pages, les points invalides ou sans contenu éditable, le remplacement
des références et les réponses tardives du worker. Les scénarios graphiques existants
restent utilisés pour le défilement, le focus, le clic sur les résultats de recherche,
le comptage pendant l’IME et la reprise de saisie après double-clic dans l’aperçu.

Validation : 328 tests de l’application et 200 du diagnostic isolé passent avec
`cargo test --locked --all-targets --features editor-model-spike`. Clippy sur les
mêmes cibles passe avec `-D warnings` ; formatage, différences et seuil de taille
des modules `app` et `engine` sont vérifiés. Aucun format documentaire, dépendance
Cargo ou transport CLI/MCP n’est ajouté.

## Suite de la phase

Les lectures de synchronisation et de comptage sont migrées vers le moteur. Les
cartes de source Typst sont conservées derrière les requêtes ; la peinture de la
recherche reste également un adaptateur du tampon graphique. Les règles des blocs
et de transformation de frappe, les images, signatures, actions de recherche/remplacement,
l’orthographe documentaire, les champs, les options et les modèles sont migrés.

L’utilisateur confirme maintenant l’affichage japonais après ajout des fontes.
Les essais complets de composition, choix des candidats, annulation et historique
avec les méthodes système restent à poursuivre dans le chantier I. Le contrat natif
D17 est implémenté ; les autres critères de 2.1 bis restent à relire avant clôture.

Les familles documentaires prévues en 2.4 sont migrées, y compris les fichiers,
la récupération et les lots atomiques D16. L’accès transitoire à la session est
supprimé. Restent l’unification de la disponibilité, l’aiguillage final des
déclencheurs, la relecture exhaustive de l’inventaire et le gel du catalogue.
V5 conserve les réglages globaux dans l’application ; les formats d’export séparés
et la création sans interface de V6 sont implémentés et testés. Le lot de lectures
ne livre pas de `render.compile` public ; son service et les éventuelles lectures
complémentaires seront examinés lors de la relecture de l’inventaire.

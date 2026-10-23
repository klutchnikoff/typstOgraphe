# Phase 2 — Mise en place du modèle structuré et du moteur

**État au 23 octobre 2026 : phase commencée, pas terminée.** Les orientations
V1–V6 de [l’inventaire](commands-inventory.md) sont validées. Les premiers lots
portent sur les blocs, les zones, leurs identités, les cibles de sélection, puis
la protection des frontières et la publication commune des éditions.
La préédition IME est maintenant isolée entre frames, avec une publication à validation.
Le registre typé et le premier moteur sont implémentés : 22 commandes, six
requêtes et leur catalogue généré. La migration des familles n’est pas terminée.

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
puis publier contenu, sélection, révision et historique ensemble. Les commandes
qui préparent encore directement leurs champs dans `App` passent par un adaptateur
transitoire : il rétablit l’instantané avant de valider leur candidat. Ce passage
ne constitue pas le moteur typé ni la fin de leur migration.

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
et la saisie native passent par `Engine::commit_projection`, puis `Session`.
La parité est vérifiée avec `text.insert` appelé par identifiant et JSON.

La migration conserve un accès transitoire à la session via `Deref/DerefMut` pour
les préparations historiques non migrées. Le curseur egui reste un cache converti
avant les commandes et à la fin de la frame ; les requêtes lisent la sélection du
moteur. Ce passage ne constitue pas la suppression de toutes les écritures depuis
`App`, ni l’achèvement des tâches 2.1 bis et 2.4.

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

Ces deux commandes de bloc ne prennent en charge que les structures à annotation
facultative dans ce lot. Leur extension aux descriptions, listes et notes sera
faite avec la migration de leurs règles. La portée dépend de la structure ciblée,
sans type Courrier imposé.

Les cibles acceptent les gardes de session/révision habituelles. Une sélection
qui traverse partiellement la frontière corps/attribution ne peut pas être
remplacée par une scission ; une citation sélectionnée en entier reste
remplaçable sans attribution orpheline. La sortie explicite conserve les deux zones.
Navigation et refus ne modifient ni contenu, révision, ressources ni piles d’historique. Une sortie ou
une scission est une transaction discrète, avec sélection restaurée à annulation
et rétablissement ; l’identité de l’attribution est conservée.

`RevealSelection` signale la cible à rendre visible : l’interface conserve les
affinités de curseur, le focus et le défilement existants. Une simple nouvelle
ligne du corps cité ne recentre pas le panneau. Le texte indicatif reste virtuel.
Les raccourcis restent écartés pendant la préédition IME.
Le comportement existant des séparateurs après une figure est conservé : son
sérialiseur peut compacter des lignes vides supplémentaires hors de la légende.

Le premier lot registre/moteur a été confirmé par la CI et par les essais
utilisateur le 23 octobre. Le présent lot est vérifié séparément ci-dessous.

## Vérifications

- 221 tests de l’application réussis, dont 78 tests ajoutés depuis le début de
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
  avertissement ; ses 121 tests partagés réussissent aussi.
- Dix tests du moteur couvrent les schémas, conversions, données invalides,
  cibles périmées/étrangères, refus atomiques, styles explicites, historique,
  lectures seules, portée Courrier et fichiers de catalogue générés. Quatre essais
  d’intégration comparent les gestes graphiques aux appels JSON, la sélection
  publiée pendant l’IME et la validation différée du dialogue de lien.
- Six nouveaux tests du moteur vérifient navigation, sorties et scissions,
  annotations vides ou remplies, Unicode et styles, ressources, refus atomiques,
  identité de l’attribution et sélection/historique. Trois essais d’intégration
  comparent Tab, Maj+Tab et Entrée aux appels JSON, avec annulation/rétablissement.
- Formatage conforme et nouveaux fichiers de production sous 800 lignes.
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

## Suite de la phase

Les indices de recherche et la synchronisation d’aperçu fonctionnent encore via
le tampon graphique et les cartes de source existants. Leur migration n’est pas
déclarée terminée par les tests de conversion des positions. Les préfixes de
transformation, le gras/italique saisis avec délimiteurs et les règles des
descriptions/listes/notes restent préparés par l’interface, puis validés par la
voie commune ; leur déplacement dans le cœur reste à faire (V4).

L’utilisateur confirme maintenant l’affichage japonais après ajout des fontes.
Les essais complets de composition, choix des candidats, annulation et historique
avec les méthodes système restent à poursuivre dans le chantier I. D17 et la tâche
2.1 bis restent ouverts pour la suite de la migration.

Restent les règles de descriptions/listes/notes, les images et signatures, les actions
de recherche/remplacement, l’orthographe documentaire, l’écriture des métadonnées
et options, les modèles et les fichiers. Les lots atomiques (D16), la suppression
de l’accès transitoire à la session et le gel du catalogue restent à faire. La
séparation des réglages globaux (V5) et les formats d’export séparés (V6) restent à
implémenter et à tester. Aucun transport CLI/MCP n’est ajouté par ce lot.

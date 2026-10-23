# Inventaire des commandes et des cibles — phase 2.1

**État : orientations V1–V6 et démarrage de la phase 2 validés ensemble le 22 octobre 2026.**
Relevé du 22 octobre 2026 sur `b3e1e986c250a80f70f65899f35d010145d63d26`.
L’implémentation a commencé par les blocs, leurs zones, leurs identités et les
cibles de sélection, puis la protection des frontières et la voie commune de
publication des éditions, puis l’isolation IME entre frames ; voir
[le suivi de phase 2](architecture-phase-2.md).
Le premier moteur, 31 commandes documentaires et six requêtes sont maintenant
implémentés avec leurs schémas JSON ; voir le [catalogue généré](commands.md).
Le lot du 23 octobre migre Tab/Maj+Tab vers `text.select { part }`, puis Entrée
vers `block.split` et sa sortie commune `block.exit` pour légendes et attributions.
Le lot suivant ajoute les descriptions (`term`/`definition`), listes et notes à
ces règles, puis migre les transformations de frappe via
`text.insert { input_rules: "typing" }` (défaut littéral). Tests de parité :
[annotations_engine.rs](../src/app/tests/annotations_engine.rs) ; refus atomiques,
cibles et historique : [annotations.rs](../src/engine/tests/annotations.rs).
Les tests complémentaires de ce lot sont dans
[blocks_engine.rs](../src/app/tests/blocks_engine.rs) et
[blocks.rs](../src/engine/tests/blocks.rs).
Le lot images/signatures ajoute `insert.image`, `image.replace`, `image.remove`,
`image.set_alt` et `document.set_signature` : blocs gardés, ressources préparées
avant publication, import local ou copie base64 explicite. Les dialogues et le
profil global restent dans l’application. Voir les essais
[images_engine.rs](../src/app/tests/images_engine.rs) et
[images.rs](../src/engine/tests/images.rs).
La recherche passe aussi par le moteur : `search.find`, `edit.find_next`,
`edit.find_previous`, `edit.replace_current`, `edit.replace_all` et `text.select`
pour le clic sur une occurrence. Les paramètres actuels et les gardes sont décrits
dans [le suivi de phase 2](architecture-phase-2.md#recherche-et-remplacement).
Les chemins des tableaux ci-dessous restent le relevé historique à migrer ; les
signatures réellement implémentées sont celles du [catalogue généré](commands.json).
Les tableaux ci-dessous restent l’inventaire de migration de toutes les actions,
pas la liste des opérations déjà disponibles sans interface.
Les règles déjà validées sont dans [la feuille de route technique](../ROADMAP-ARCHITECTURE.md),
D1–D20. Les propositions qui les précisent ou les font évoluer sont signalées.

## 1. Périmètre et relevé

Le registre graphique contient **31 variantes de `UiCommand`** (ancien `Command`), dont `File`, `Block` et `Model`
paramétrées : **41 définitions communes**, **2 commandes Courrier**, **1 modèle fourni**,
soit **44 entrées concrètes** pour un courrier. Un compte-rendu possède les 41
entrées communes ; son type est défini mais aucun modèle n’est actuellement fourni,
donc sa création depuis le catalogue est indisponible. Les variantes d’un enum
ne sont pas le nombre d’actions du catalogue. L’ancien relevé « 33 » était inexact.

L’inventaire couvre les entrées du registre, les gestes de saisie et les actions
des champs, images, annotations, recherches, préférences et copies de récupération.
Il distingue l’action demandée de la préparation de ses paramètres par l’interface.
Les outils de diagnostic et les traitements automatiques sont recensés séparément.
Tableaux, notes de bas de page, bibliographie, conversion de type et collaboration
ne deviennent pas des commandes publiques dans cette phase.

### Lecture des tableaux

- **C-doc** : commande modifiant le document ; transaction d’historique.
- **C-sel** : commande modifiant sa sélection, pas son contenu ; aucune entrée d’annulation.
- **C-IO** : opération de session ou de fichier ; pas d’annulation documentaire,
  ni d’autorisation implicite de perdre un travail non enregistré.
- **Q** : requête sans modification du document ni de sa sélection. Un calcul ou
  cache de rendu/orthographe n’en fait pas une mutation documentaire.
- **UI** : action prise en charge par l’application, absente du catalogue CLI/MCP.
  **UI-global** signale un véritable effet sur les préférences ou les ressources
  globales : ce n’est pas une action visuelle sans effet. Cette frontière est validée en V5.
- **Dialogue** : nécessaire à la préparation graphique actuelle, jamais un argument
  obligatoire d’une commande utilisable sans interface. « Cond. » = seulement si le
  chemin manque ou s’il faut traiter du travail non enregistré.
- **Sans UI** indique l’exposition proposée après migration, pas une API existante.
- Les chemins de fonctions partent de `src/`. `App::` est implicite pour `app/*.rs`.
  Les codes Txx renvoient aux noms exacts des tests de la section 8. Ce sont des tests
  **du comportement actuel**, pas des tests d’un moteur inexistant. « À ajouter »
  signale une couverture directe absente ; les nouveaux contrats auront leurs tests propres.

## 2. Choix principaux et état de validation

Tous les paramètres ci-dessous restent des descriptions de contrat, pas des signatures Rust figées.
V1–V6 sont validés dans leur orientation ; la forme exacte des identifiants,
les schémas Rust/JSON et les jonctions entre types de paragraphes restent à préciser.
La publication unique après composition IME est vérifiée par les événements egui
dans l’adaptateur actuel ; l’essai d’une méthode IME réelle reste au chantier I.
Le classement détaillé sert de référence de migration ; les signatures exactes seront
précisées et testées progressivement avant le gel du catalogue.

| Choix | Orientation | Conséquences / points à préciser |
| --- | --- | --- |
| **V1 — Cibles et identités — validé le 22 octobre 2026** | Positions par zone de texte + indice Unicode ; références de blocs distinctes. Identifiants opaques stables pendant une session, restaurés par annuler/rétablir et jamais réaffectés à un autre objet. Réouverture = nouvelle session et nouveaux identifiants. | Ne pas modifier l’archive pour stocker ces identifiants maintenant. Toutes les cibles externes sont liées à une session et une révision ; aucun repositionnement silencieux d’une cible périmée. |
| **V2 — Sélections entre structures — validé le 22 octobre 2026** | Garder une sélection continue orientée ; mise en forme sur les zones textuelles admissibles. Suppression/remplacement libre dans une zone et entre paragraphes d’un même corps. Refuser les remplacements partiels ambigus entre rôles ; supprimer un objet explicitement ciblé ou entièrement couvert par la sélection. | Préciser les jonctions entre types de paragraphes avant migration. C’est une restriction volontaire pour les nouveaux passages ambigus, pas une promesse de reproduire toute suppression accidentelle du tampon plat. |
| **V3 — Appels explicites et déclencheurs graphiques — validé le 22 octobre 2026** | Une action métier a une commande canonique paramétrée. Les menus et raccourcis recueillent au besoin les paramètres par un `UiCommand`, puis appellent cette même commande. Hors interface : paramètres explicites ou sélection détenue par le moteur. | Pas de dépendance au focus, à un index de menu ou à des pixels dans le cœur. Les identifiants existants sont conservés, sauf les deux corrections D2 déjà décidées. |
| **V4 — Saisie et préédition — validé le 22 octobre 2026** | Une insertion est littérale par défaut. Le mode de frappe interactif peut demander les règles textuelles existantes, à traiter dans le cœur. La préédition IME reste un brouillon de l’adaptateur, puis sa validation devient une transaction. | Ne pas interpréter du Typst collé/importé comme du code. Même validation/normalisation/commit pour saisie et commandes (D17). Le brouillon entre frames et sa publication unique sont maintenant testés dans l’adaptateur ; le diagnostic de phase 1 seul ne les démontrait pas. La migration des règles de frappe dans le cœur reste à faire. |
| **V5 — Profil et réglages globaux — validé le 22 octobre 2026** | Registre commun, mais préférences, profil et dictionnaire personnel restent des actions d’application non exposées sans interface dans ce premier moteur. Le moteur reçoit leurs valeurs nécessaires comme configuration ou données explicites. | Précision validée de D5 et de 2.4.5 : `spelling.learn` reste accessible dans l’application, sans mutation des préférences par `Engine`. Une future API de services globaux est distincte ; aucune action globale dans un lot documentaire (D16). |
| **V6 — Création, dates et export — validé le 22 octobre 2026** | Création sans interface vide, sauf valeurs initiales fournies ; l’UI transmet une copie du profil. « Aujourd’hui » est résolu en date ISO avant mutation. Export vers un dossier neuf/vide, avec `format: pdf / typst / both`. | L’export actuel produit PDF + sources + PNG : proposer `both` et `page_images: true` par défaut pour préserver cet export, avec une option explicite pour omettre les PNG de diagnostic. Les formats séparés sont un changement volontaire validé, à implémenter. Aucun export dans un lot d’édition. |

D2–D6 restent les bases : identifiants, enum typée, commandes/requêtes séparées,
UI distincte et erreurs structurées. V5 précise une frontière encore ambiguë :
un réglage global n’appartient pas à la session documentaire, même s’il est public
pour l’utilisateur de l’application. Le registre doit le décrire sans annoncer
à tort qu’il est disponible dans la CLI.

## 3. Cibles, validité et sélection

### 3.1 Vocabulaire proposé

| Cible | Description | Usage |
| --- | --- | --- |
| `DocumentRef` | Identité opaque de la session ouverte | Ne pas confondre chemin du fichier et identité d’une ouverture. Un seul document actif pour commencer. |
| `BlockRef` | Identité d’un paragraphe, d’une citation, d’une image, d’une description ou d’une note de courrier | Transformer/supprimer un bloc, remplacer une image, lire ses propriétés. |
| `ZoneRef` | Identité d’une zone textuelle, avec un rôle descriptif dans l’arbre | Corps de paragraphe, terme/texte de description, corps/attribution de citation, légende d’image, texte P.-S./P.J. |
| `TextPosition` | `zone_id` + `offset`, compté en valeurs scalaires Unicode, de 0 à la longueur | Les indices actuels de Rust/egui restent des caractères, ni des octets, ni le compte des signes affiché en barre d’état. |
| `BlockBoundary` | Position avant ou après un `block_id` dans son conteneur | Placer un objet, sélectionner un objet sans exposer le caractère sentinelle interne. |
| `Selection` | `anchor` et `head`, positions textuelles ou frontières ; une seule plage continue | Conserver le sens de la sélection et les limites structurées. L’ordre vient du document, pas du tri des identifiants. |
| `FieldRef` | Champ du catalogue du type actif | Métadonnées sans indice de widget. Pas d’écriture arbitraire dans une carte JSON. |
| `OptionRef` | Option commune typée et autorisée par le modèle | Papier, orientation, langue, puce ; distinction entre champ inconnu et option verrouillée. |
| `ImageInput` | Chemin local à importer, ou ressource déjà incluse dans le document | Copier et valider l’image ; aucune dépendance ultérieure au fichier d’origine. Le format exact sera choisi avant de coder. |
| `OccurrenceRef` / `IssueRef` | Cible textuelle + session/révision + texte attendu | Recherche et orthographe ; pas d’index nu réutilisable après une édition. |
| `RecoveryRef` | Identité de copie de récupération donnée par le service | Ne pas exposer un index de la liste graphique comme identité stable. |

Les rôles nomment des zones ; ils ne sont pas leurs identifiants. Deux paragraphes
ont le même rôle `body` et des identifiants différents. Une zone facultative vide
reste adressable logiquement : aucun texte indicatif n’entre dans le source, et une
légende vide ne devient pas une légende imprimée par défaut. Les cellules futures
pourront utiliser le même principe, sans ajouter d’action de tableau aujourd’hui.

### 3.2 Garde de révision et durée de vie (V1)

Les lectures renvoient `document_id` et `revision`. Un appel réutilisant une cible
obtenue par lecture doit préciser cette paire. L’UI les fournit automatiquement
pour sa session courante. Le cœur refuse les identités absentes, révisions périmées,
offsets hors limites et cibles de mauvais type ; il ne tronque pas silencieusement
un indice externe. L’absence de garde reste possible pour une opération qui ne
réutilise aucun relevé antérieur, par exemple créer un document.

Annuler restaure les identités du contenu restauré, mais fait avancer la révision.
Une cible lue avant annulation doit donc être relue même si son bloc existe à nouveau.
Une réouverture invalide les références de l’ancienne session.

La sélection ne fait pas avancer la révision documentaire. Une cible symbolique
« sélection courante » est donc résolue au moment de l’appel, volontairement. Pour
figer une sélection lue auparavant, transmettre ses positions explicites avec la
garde documentaire ; ne pas réinterpréter ce relevé comme la sélection du moment.
Un dialogue de lien ou d’image capture sa cible et sa révision à l’ouverture et
les conserve jusqu’à validation, plutôt que de reprendre le focus à sa fermeture.

La pérennité des identifiants dans les fichiers ou un futur protocole collaboratif n’est pas décidée
par ce choix : le contrat les traite comme opaques.

### 3.3 Règles de portée (V2)

- Une plage dans une zone est éditable selon les capacités de son rôle. Un curseur
  vide autorise les styles de frappe ; les boutons gras/italique conservent leur
  comportement de bascule. Les appels explicites peuvent aussi demander `on/off`.
- La mise en forme entre paragraphes reste disponible. URL, texte alternatif,
  métadonnées et séparateurs de structure ne deviennent pas du texte sélectionné.
- Supprimer entre paragraphes d’un même corps peut les joindre. Les conventions
  actuelles de type de paragraphe doivent être relevées dans les tests avant migration.
- Une frontière de rôle (corps → attribution, terme → définition, texte → objet)
  n’est pas un simple caractère effaçable. Un remplacement ambigu échoue en entier,
  sans effacer une première partie. Une suppression d’objet s’appuie sur son
  `BlockRef`, explicite ou résolu depuis une structure entièrement sélectionnée.
- Sélectionner puis effacer tout le document reste une opération normale : retirer
  les structures entièrement couvertes en une transaction, puis garder une zone de
  paragraphe vide. Le refus concerne les remplacements partiels ambigus, pas cette
  sélection totale ni une plage couvrant des blocs entiers.
- Effacer une attribution ou une légende n’efface pas la citation ou l’image.
  Supprimer l’image entière supprime aussi sa légende.
- Navigation, recherche et sélection ne créent pas d’étape d’annulation. « Quitter
  un bloc » peut au contraire créer un paragraphe : c’est alors une commande C-doc,
  distincte du simple changement de sélection.
- Une transaction restaure contenu, structure et sélection ensemble. Les commandes
  externes discrètes font une étape ; la frappe conserve ses groupes naturels.
  Le détail de regroupement par champ ne doit plus utiliser un `egui::Id` dans le cœur.

Exemple indicatif, non schéma JSON validé :

```json
{
  "document_id": "session-A",
  "expected_revision": 12,
  "id": "text.bold",
  "params": {
    "target": {
      "anchor": {"zone_id": "paragraph-7-body", "offset": 3},
      "head": {"zone_id": "paragraph-7-body", "offset": 8}
    },
    "value": "on"
  }
}
```

Le texte d’une insertion est du texte visible : `_bonjour_` reste littéral en mode
normal. Le mode interactif validé en V4 applique les règles existantes dans la
même transaction, en gardant l’annulation qui restitue la saisie littérale.
Les prééditions IME ne passent ni dans les requêtes sur le document validé ni dans
son historique ; l’adaptateur conserve le focus et annule/résout une composition
avant d’appliquer une mutation concurrente à sa zone. Aucun réancrage implicite.

## 4. Les 44 entrées actuelles du registre

Une ligne qui comporte une préparation UI et une opération C/Q représente **une
seule entrée publique actuelle**, pas deux nouveaux identifiants concurrents.
La préparation typée par `UiCommand` reste dans la présentation de cette action.
Un identifiant UI pur conserve sa place dans le registre mais n’est pas annoncé
comme commande sans interface.

| Entrée actuelle → cible proposée | Chemin actuel | Paramètres métier proposés | Famille cible | Dialogue actuel | Sans UI | Tests actuels |
| --- | --- | --- | --- | --- | --- | --- |
| `document.new` | `app/files::create_document`, `perform_action` | `type`, `model`, `initial_values?`, `signature?` ; refus si travail courant non enregistré sans décision explicite | Préparation UI → C-IO | Oui | Oui | T01, T04, T26 |
| `document.open` | `app/files::open_path`, `install_document` | `path`, traitement explicite du travail courant | Préparation UI → C-IO | Oui | Oui | T02, T04 |
| `document.save` | `app/files::save_document`, `save_path` | `path?` ; chemin connu requis s’il est omis | Préparation UI → C-IO | Cond. | Oui | T03, T17 |
| `document.save_as` | `app/files::save_document`, `save_path` | `path`, `overwrite: false` par défaut hors UI | Préparation UI → C-IO | Oui | Oui | T03, T47 |
| `document.export` | `app/dialogs::export_dialog`, `rendering::render` | `directory`, `format`, `page_images?` ; valeurs obligatoires du type vérifiées | Préparation UI → C-IO | Oui | Oui | T05, T14, T48 |
| `document.close` | `app/files::handle_file_command`, `close_document` | `discard_changes: false` ; sinon erreur `unsaved_changes` | C-IO | Cond. | Oui | T01, T04 |
| `application.quit` | `app/files::request_action`, `perform_action`, `app/mod::ui_frame` | Fenêtre/application courante, décision enregistrer/abandonner/annuler | UI | Cond. | Non | T04, T05 |
| `commands.palette` | `app/commands::apply`, `app/dialogs::palette_dialog` | Ouvrir/fermer ; requête et item actif locaux | UI | Palette | Non | T39, T49 |
| `application.preferences` | `app/preferences::preferences_ui`, `app/commands::apply` | Ouvrir/fermer, rubrique | UI | Oui | Non | T27, T28 |
| `document.information` | `app/information::information_ui`, `app/commands::apply` | Ouvrir/fermer, groupe d’informations | UI | Panneau | Non | T21, T49 |
| `courrier.model.personal` | `app/commands::apply(Command::Model)` | Préréglage fixe `Personal`, cible = document Courrier | C-doc | Non | Oui | T23, T25, T42 |
| `document.import_model` | `app/models::import_model`, `install_model` | `path` ; fichier Typst, contrat du type et dépendances validés | Préparation UI → C-doc avec lecture IO | Oui | Oui | T24, T25, T41 |
| `text.bold` | `app/commands::apply`, `edit`; `document::Projection::toggle_mark` | `target: selection/plage`, `value: toggle/on/off` | C-doc / contexte au curseur vide | Non | Oui | T06, T10 |
| `text.italic` | mêmes chemins, `Mark::Italic` | mêmes cibles et valeurs | C-doc / contexte au curseur vide | Non | Oui | T06, T07, T10 |
| `text.clear` | `app/commands::apply`, `Projection::clear_marks` | `target` ; styles en ligne, pas transformation du bloc | C-doc | Non | Oui | T10 indirect ; test dédié à ajouter |
| `text.link` | `app/links::open_link`, `app/dialogs::link_dialog` | `target`, `url` explicite, schémas admis `https/http/mailto` | Préparation UI → C-doc | Oui | Oui | T09, T50 |
| `text.remove_link` | `app/commands::apply`, `Projection::set_link` | `target` non vide ; conserver gras/italique et sélection | C-doc | Non | Oui | T09 |
| `block.paragraph` | `app/commands::apply`, `Projection::set_block` | `target` ; paragraphes sélectionnés | C-doc | Non | Oui | T10, T51 |
| `block.heading_1` | mêmes chemins, `Heading(1)` | `target`, niveau fixe 1 | C-doc | Non | Oui | T10, T08 |
| `block.heading_2` | mêmes chemins, `Heading(2)` | `target`, niveau fixe 2 | C-doc | Non | Oui | T10 |
| `block.heading_3` | mêmes chemins, `Heading(3)` | `target`, niveau fixe 3 | C-doc | Non | Oui | T10 |
| `block.bullet` | mêmes chemins, `Bullet` | `target` ; marqueur fourni par la politique documentaire | C-doc | Non | Oui | T11, T52 |
| `block.numbered` | mêmes chemins, `Numbered` | `target` ; numérotation composée par Typst | C-doc | Non | Oui | T11 |
| `block.description` | mêmes chemins, `Description` | `target` ; terme et définition deviennent des zones distinctes | C-doc | Non | Oui | T11, T53 |
| `block.quote` | mêmes chemins, `Quote` | `target` ; corps cité, attribution facultative | C-doc | Non | Oui | T12, T54 |
| `insert.image` | `app/images::choose_image`, `import_image` | `target` d’insertion/remplacement de sélection, `image`, `alt?`, `caption?` | Préparation UI → C-doc avec lecture IO | Oui | Oui | T14 |
| `letter.insert-postscript` → `letter.insert_postscript` | `app/commands::apply`, `Projection::insert_letter_note` | Document Courrier ; réutiliser le P.-S. existant ou créer sa zone | C-doc | Non | Oui | T13, T55 |
| `letter.insert-attachment` → `letter.insert_attachment` | mêmes chemins, `Attachment` | Document Courrier ; ajouter une mention de pièce jointe, pas un fichier | C-doc | Non | Oui | T13 |
| `edit.undo` | `app/commands::apply_history`, `session::Session::history` | Session courante ; échec explicite si historique vide | C-doc, restauration | Non | Oui | T06, T26, T43 |
| `edit.redo` | mêmes chemins | Session courante ; révision toujours croissante | C-doc, restauration | Non | Oui | T03, T43 |
| `document.recover` | `app/commands::apply`, `app/recovery::recovery_dialog` | Afficher la liste ; la restauration effective a une cible distincte §5.6 | UI → Q/C-IO distinctes | Oui | Liste UI : non ; restaurer : oui | T33, T34 |
| `edit.find` | `app/commands::apply`, `app/search::search_ui` | Afficher la recherche ; contenu de requête local | UI → `search.find` | Barre | Non (requête Q distincte) | T29 |
| `edit.find_replace` | mêmes chemins | Afficher la rangée de remplacement | UI → Q / C-doc | Barre | Non (Q/C distinctes) | T29, T30 |
| `edit.find_next` | `app/commands::apply`, `Search::next`, `select_search_result` | `query`, casse, position de départ ou sélection courante ; calcul sur état valide, bouclage actuel | C-sel ; ouverture de barre par l’UI | Non | Oui | T29, T44 |
| `edit.find_previous` | mêmes chemins | Mêmes paramètres ; occurrence précédente, bouclage actuel | C-sel ; ouverture de barre par l’UI | Non | Oui | T29, T44 |
| `edit.replace_current` | `app/search::replace_search(false)`, `Session::replace_ranges` | `occurrence`, `replacement`, garde de révision/texte attendu | C-doc | Barre de préparation | Oui | T30, T43 |
| `edit.replace_all` | `app/search::replace_search(true)`, `Session::replace_ranges` | `query`, `case_sensitive: false`, `replacement`, périmètre, révision ; résultats calculés sur le même état | C-doc, une transaction | Barre de préparation | Oui | T30, T43 |
| `spelling.toggle` | `app/commands::apply_spelling_command`, `spelling_preferences_ui` | Activation globale, `value: toggle/on/off` | UI-global (V5) | Non | Non | T32 indirect ; activation dédiée à ajouter |
| `spelling.next` | `app/commands::apply_spelling_command` | Départ explicite ou sélection courante ; prochaine faute d’une analyse valide | C-sel | Non | Oui | T31 indirect |
| `spelling.previous` | mêmes chemins | Même cible ; faute précédente | C-sel | Non | Oui | À ajouter, direction précédente |
| `spelling.suggestions` | `app/spelling::open_spelling_target`, `spelling_context_menu` | Mot/langue ou faute validée ; choix de cible et ouverture du menu par l’UI, requête sans sélection implicite | Q, préparation UI distincte | Menu contextuel | Oui | T31 |
| `spelling.ignore` | `app/commands::apply_spelling_command` | `issue`, langue ; ajouter le mot aux exceptions du document | C-doc | Non | Oui | T32 |
| `spelling.learn` | mêmes chemins, `Preferences.spelling.personal` | Mot et langue de la faute courante | UI-global (V5) | Non | Non | T32 |
| `spelling.reset` | mêmes chemins, `options.spelling_ignored.clear` | Effacer les exceptions du document, toutes langues comme actuellement | C-doc | Non | Oui | T32 |

Les deux renommages de notes sont celles déjà approuvées en D2. Le préréglage
`courrier.model.personal` reste une exception historique documentée : pas de
renommage supplémentaire imposé. Les nouveaux identifiants sont anglais.
La grammaire proposée accepte au moins deux segments ASCII minuscules en `snake_case`, séparés par des points ; elle
accepte ainsi le namespace à trois segments du préréglage existant, sans renommage.
Les libellés et messages restent des clés Fluent traduites.
Les ouvertures `edit.find/find_replace` restent des actions UI. La navigation
`edit.find_next/previous` et `spelling.next/previous` reste au contraire utilisable
sans interface : elle calcule sa cible sur un relevé valide puis modifie seulement
la sélection. L’UI fournit sa requête et décide d’afficher la barre ou le menu.
La requête `spelling.suggestions` conserve son identifiant, sans déplacer la sélection
ni ouvrir de menu lors d’un appel sans interface. Ces répartitions restent à valider
dans V3 ; il n’est pas nécessaire de renommer les identifiants pour séparer C/Q/UI.

## 5. Actions actuellement hors registre

### 5.1 Saisie, presse-papiers et sélection

| Action / cible proposée | Chemin actuel | Paramètres proposés | Famille | Dialogue | Sans UI | Tests actuels |
| --- | --- | --- | --- | --- | --- | --- |
| Insérer / remplacer une sélection → `text.insert` | `editor::TextBuffer for Projection`, `app/editor_view::editor_text`, `editor_feedback` | `target`, `text`, `input_rules: literal/typing` (V4) | C-doc | Non | Oui | T07, T08, T40 |
| Supprimer un texte → `text.delete` | `Projection::delete_char_range`, TextEdit natif | `target` validé ; pas d’offset sentinelle public | C-doc | Non | Oui | T07, T40, T56 |
| Curseur/sélection, clic/glisser, flèches, Home/End, sélection totale → `text.select` | `editor_feedback`, état natif egui | `Selection` ou sélecteur `all` ; pixels et navigation visuelle résolus par l’adaptateur | C-sel | Non | Oui | T07, T29, T38 |
| Copier | TextEdit egui, sortie presse-papiers | `document.text` sur sélection → texte au presse-papiers système | UI + Q | Non | Presse-papiers : non ; lecture : oui | Pas de test direct du presse-papiers système |
| Couper | TextEdit egui (`Event::Cut`) | Q texte + copie UI réussie, puis `text.delete` avec garde | UI + C-doc | Non | Suppression : oui | Pas de test direct Cut/clipboard ; T07 indirect |
| Coller | TextEdit egui (`Event::Paste`) | Texte du presse-papiers → `text.insert` littéral ; pas d’import HTML nouveau | UI + C-doc | Non | Insertion : oui | T07 indirect ; collage dédié à ajouter |
| Retour simple / nouveau paragraphe | TextEdit, normalisation `Projection::insert_text` | `text.insert` avec retour(s), selon règles du rôle | C-doc | Non | Oui | T57 |
| Préédition/validation/annulation IME | `app/ime`, `TextInput`, `Session::commit_projection` ; TextEdit egui pour l’affichage | Brouillon persistant de l’adaptateur, transaction finale `text.insert` après migration ; pas de commande publique par touche d’IME | UI + C-doc à validation | Non | Commit de texte : oui | T60–T63 ; 14 scénarios dans `app::tests::ime` ; méthode IME système à essayer |
| Préfixes `=`, `==`, `===`, `-`, `+`, `1.`, `/`, `>` suivis d’espace | `app/text_input::block_shortcut` | Mode `typing` ; intention de bloc typée ; historique conserve le littéral | C-doc dans l’insertion interactive | Non | Oui, mode explicite | T08 |
| Fermeture `*…*` / `_…_` | `app/text_input::inline_shortcut` | Même mode `typing`, style sur plage textuelle ; jamais exécution de code | C-doc dans l’insertion interactive | Non | Oui, mode explicite | T08 |
| Modifier les styles des frappes suivantes | `Projection::typing_style`, `editor_feedback`, `toggle_mark` | Curseur vide + style ; contexte de frappe distingué du texte déjà présent | C-sel/contexte ; C-doc seulement si contenu modifié | Non | Oui | T06 indirect ; contexte seul à préciser |

### 5.2 Zones et structures

| Action / cible proposée | Chemin actuel | Paramètres proposés | Famille | Dialogue | Sans UI | Tests actuels |
| --- | --- | --- | --- | --- | --- | --- |
| Aller au terme / texte d’une description | `description_shortcut`, clic du terme dans `editor_feedback` | `text.select` sur la zone `term` / `definition` | C-sel + focus UI | Non | Oui pour sélectionner | T11, T53 |
| « : espace » dans un terme | `description_shortcut` | Mode `typing` ; transformer le séparateur puis sélectionner la définition | C-doc si texte changé | Non | Oui, mode explicite | T11 |
| Entrée dans description, citation, note ; sortie de liste vide | `description_shortcut`, `quote_shortcut`, `letter_note_shortcut`, `list_shortcut` | `block.split` ou `block.exit`, cible et règles du type ; sélectionner le résultat | C-doc | Non | Oui | T11, T12, T13, T53 |
| Tab / Maj+Tab entre corps et annotation | `annotation_shortcuts`, `focus_annotation_target` | `text.select` sur zone de légende/attribution/corps | C-sel + focus UI | Non | Oui pour sélectionner | T18, T19, T20 |
| Entrée à la fin d’une annotation | `leave_annotation`, `Projection::leave_annotation` | `block.exit`, bloc et zone de départ ; créer le paragraphe de sortie si nécessaire | C-doc | Non | Oui | T18 |
| Éditer/vider légende ou attribution | TextEdit via l’adaptateur d’annotations | `text.insert/delete` sur `ZoneRef` ; aucun placeholder sauvegardé | C-doc | Non | Oui | T12, T14, T19 |
| Remplacer une image → `image.replace` | `app/images::choose_image`, `import_image(Some(id))` | `block_id`, `image` ; conserver légende et texte alternatif | C-doc avec lecture IO | Oui | Oui | T14 |
| Supprimer une image → `image.remove` | `remove_image`, bouton `ImageAction::Remove`, `image_shortcut` | `block_id` ; une seule opération avec sa légende | C-doc | Non | Oui | T15, T16, T17 |
| Modifier le texte alternatif → `image.set_alt` | `app/dialogs::image_dialog` | `block_id`, `alt` | Préparation UI → C-doc | Oui | Oui | T16 |
| Sélectionner l’image entière | `ImageAction::Select`, `image_shortcut` | `text.select` avec frontières du bloc ; pas U+FFFC dans l’API | C-sel | Non | Oui | T15, T16 |
| Insérer un fichier image déposé | `app/mod::ui_frame` | Même `insert.image`, cible et chemins traités dans l’ordre, une invocation par fichier | UI → C-doc avec lecture IO | Non | Oui via l’opération métier | T14 indirect ; dépôt natif à ajouter |

Le Tab de description insère parfois un séparateur technique aujourd’hui. Dans
le modèle structuré proposé, ses deux zones existent logiquement dès la création ;
le déplacement ne doit pas créer une étape d’annulation pour ce seul séparateur.
Les règles de sortie qui créent un paragraphe restent des mutations distinctes.
P.-S. et P.J. sont ordonnés par la règle du courrier et composés après signature,
pas insérés à une position arbitraire de la page. La commande P.J. ne lit aucun fichier.

### 5.3 Métadonnées, options, modèles et signature

| Action / cible proposée | Chemin actuel | Paramètres proposés | Famille | Dialogue | Sans UI | Tests actuels |
| --- | --- | --- | --- | --- | --- | --- |
| Modifier un champ → `metadata.set` | `document_types::fields_ui`, `app/information::information_ui`, `Session::record` | `field`, valeur typée ; champ du catalogue du type actif | C-doc | Panneau | Oui | T21, T43, T58 |
| Date « Aujourd’hui » | `document_types::fields_ui`, `dates::today/is_today/normalize` | UI résout la date locale puis `metadata.set` avec ISO fixe | UI → C-doc | Non | Oui avec date explicite | T22, T59 |
| Papier → `document.set_option` | `general_information_ui` | `key: paper`, valeur parmi `a3/a4/a5/us-letter/us-legal` ; charte du modèle appliquée | C-doc | Non | Oui si autorisé | T21, T60 |
| Orientation → même commande | même chemin | `key: landscape`, booléen ; refus si page verrouillée par le modèle | C-doc | Non | Oui si autorisé | T21, T60 |
| Langue du document → même commande | même chemin | `key: language`, `fr/en/de/es/it/pt/nl` ; indépendant de l’interface ; dictionnaires seulement fr/en actuellement | C-doc | Non | Oui | T21, T60 |
| Puce automatique/choisie/personnalisée → même commande | `bullet_marker_ui`, `ModelPresentation` | `key: bullet_marker`, `auto` ou un caractère autorisé ; pas de contournement du modèle | C-doc | Non | Oui si autorisé | T23, T52 |
| Lire la politique d’un modèle | `template_selector`, `rendering::model_presentation` | Requête `models.presentation` | Q | Non | Oui | T23, T41 |
| Sélection du modèle fourni | `template_selector`, `apply(Model)` | Invocation du préréglage enregistré ; ne pas réinventer une commande par clic | C-doc existante | Non | Oui | T23, T25 |
| Installer une implémentation Typst déjà lue → `document.set_model` | `app/models::install_model` | Source Typst et dépendances contrôlées ; commande partagée par l’import et les préréglages | C-doc | Non | Oui | T24, T25, T41 |
| Choisir/remplacer/enlever la signature → `document.set_signature` | `choose_signature(false)`, `set_document_signature` | `image` ou `none` ; Courrier ; conserver l’espace de signature manuscrite du modèle | C-doc avec lecture IO si image | Choix d’image | Oui | T26, T61 indirect ; réserve manuscrite à tester |
| Revenir à la signature du profil | `use_profile_signature` | UI lit la signature du profil puis appelle `document.set_signature` avec une copie | UI-global lecture → C-doc | Non | Oui avec image explicite ; lecture implicite du profil : non | T26 |

La commande générique proposée `document.set_model` ne remplace pas l’identifiant
du préréglage existant : le préréglage est une invocation fixée sur la même opération
interne. Les contrats de modèles et paramètres Typst existants ne sont pas renommés
par cette préparation d’API. Le type est choisi à la création ; aucune conversion
ni modification du type d’un document ouvert n’est ajoutée.

#### Catalogue de champs à reprendre dans l’inventaire typé

Les clés publiques proposées ci-dessous sont anglaises et mappées aux descripteurs
existants. Cela ne renomme pas les champs des archives, du Rust ou des modèles
Typst à ce stade. V3 inclut la validation de ces clés avant implémentation.

| Type | Clé proposée → paramètre actuel | Valeur / contrainte actuelle |
| --- | --- | --- |
| Courrier | `sender.name` → `expediteur` | Texte |
| Courrier | `sender.role` → `fonction` | Texte |
| Courrier | `sender.department` → `service` | Texte ; pas de valeur correspondante dans le profil global actuel |
| Courrier | `sender.email` → `email` | Texte |
| Courrier | `sender.organisation` → `organisme` | Texte |
| Courrier | `sender.address` → `adresse_expediteur` | Texte multiligne |
| Courrier | `sender.phone` → `telephone` | Texte |
| Courrier | `recipient.name` → `destinataire` | Texte, obligatoire pour l’export |
| Courrier | `recipient.address` → `adresse` | Texte multiligne |
| Courrier | `letter.subject` → `objet` | Texte, obligatoire pour l’export |
| Courrier | `letter.reference` → `reference` | Texte |
| Courrier | `letter.place` → `lieu` | Texte |
| Courrier | `letter.date` → `date` | ISO fixe ou libellé libre ; normalisation et Datify conservés |
| Compte-rendu | `meeting.title` → `titre` | Texte, obligatoire pour l’export |
| Compte-rendu | `meeting.date` → `date` | ISO fixe ou libellé libre |
| Compte-rendu | `meeting.place` → `lieu` | Texte |
| Compte-rendu | `meeting.author` → `redacteur` | Texte |
| Compte-rendu | `meeting.attendees` → `participants` | Texte multiligne, pas une nouvelle base de contacts |
| Compte-rendu | `meeting.apologies` → `excuses` | Texte multiligne |

La signature est une opération de ressource, pas un champ texte contenant un chemin
externe. Les exceptions orthographiques restent un réglage documentaire privé,
absent des options passées au modèle Typst. Une requête de schéma renverra groupes,
clés, types, caractère obligatoire et possibilités effectivement autorisées.

### 5.4 Recherche et orthographe hors boutons du registre

| Action / cible proposée | Chemin actuel | Paramètres proposés | Famille | Dialogue | Sans UI | Tests actuels |
| --- | --- | --- | --- | --- | --- | --- |
| Saisir recherche, remplacement, casse | `app/search::search_query_row`, `search_replacement_row` | État de formulaire local ; appeler `search.find` avec `query`, `case_sensitive: false`, périmètre document/zone | UI + Q | Barre | Q : oui | T29, T44 |
| Cliquer une occurrence | `select_search_result`, `editor_feedback` | `text.select` avec référence d’occurrence validée ; remplace la sélection précédente | C-sel + focus/défilement UI | Non | Oui pour sélectionner | T29 |
| Choisir une suggestion → `spelling.correct` | `correct_spelling`, `valid_spelling_target`, `Session::replace_ranges` | `issue`, `replacement` ; valider révision et mot attendu | C-doc | Menu | Oui | T31 |
| Menu par clic droit sur faute | `spelling_context_menu`, `open_spelling_target` | UI traduit le point en cible, puis Q suggestions ; jamais pixels dans la requête de correction | UI + Q | Menu | Q : oui | T31 |
| Oublier un mot appris → `application.forget_word` | `app/preferences::spelling_preferences_ui` | `language`, `word` ; dictionnaire personnel, pas exceptions du document | UI-global (V5) | Préférences | Non | T32 indirect ; oubli dédié à ajouter |

Recherche exacte, littérale, sans regex ni fuzzy dans le contrat proposé ; casse
ignorée par défaut. Le périmètre garde le texte visible des annotations, exclut
URL, alt, métadonnées et séparateurs. Un résultat ne traverse pas une frontière de
structure. Les requêtes ne modifient aucune sélection. Aucun dictionnaire personnel
n’est incorporé au document à l’exception de ses propres mots ignorés.

### 5.5 Préférences et profil (actions d’application selon V5)

| Action / cible proposée | Chemin actuel | Paramètres proposés | Famille | Dialogue | Sans UI | Tests actuels |
| --- | --- | --- | --- | --- | --- | --- |
| Police / taille / thème → `application.set_preference` | `appearance_preferences_ui` | `editor.font: libertinus/computer_modern/monospace`, taille 10–36, `appearance: system/light/dark` | UI-global | Préférences | Non | T28, T35 |
| Proportion des colonnes / double-clic de remise à moitié | `editor_view::columns_ui` | `editor_fraction`, valeur normalisée ; aucun maximum de largeur d’aperçu | UI-global | Non | Non | T37, T35 |
| Identité personnelle → `application.set_profile_field` | `profile_preferences_ui`, `profile::Profile` | `name/organisation/role/address/email/phone`, texte | UI-global | Préférences | Non | T26, T27 |
| Signature personnelle → `application.set_profile_signature` | `choose_signature(true)`, `Profile::import_signature`, bouton supprimer | Chemin à importer ou `none` ; copie globale gérée par le logiciel | UI-global avec IO | Choix d’image | Non | T26, T27, T62 |
| Activation correction / dictionnaire personnel | `spelling_preferences_ui`, `apply_spelling_command` | Réutiliser les actions `spelling.toggle/learn` et `application.forget_word` | UI-global | Non / préférences | Non | T32 |
| Raccourcis personnalisés | `Keymap::new/for_document`, `ShortcutOverrides` | Remplacer/désactiver les raccourcis par identifiant d’action ; conflits validés | Service applicatif ; pas de nouvelle UI livrée | Pas d’UI de personnalisation actuelle | Non proposé | T39, T63 |

Modifier le profil ne change pas les documents existants. À la création, l’UI
transmet les valeurs initiales et la copie de signature à `document.new` ; le cœur
ne lit pas secrètement les préférences de l’utilisateur. Le stockage global reste
sur changement stabilisé/fermeture, pas un enregistrement périodique ajouté ici.
La langue de l’interface reste celle du système avec le repli actuel ; le passage
au repli anglais prévu en D7 relève de la phase 3, pas de cet inventaire.

### 5.6 Récupération, affichage et navigation de rendu

| Action / cible proposée | Chemin actuel | Paramètres proposés | Famille | Dialogue | Sans UI | Tests actuels |
| --- | --- | --- | --- | --- | --- | --- |
| Lister les copies → `recovery.list` | `Recovery::start`, `app/recovery::start_recovery` | Racine fournie par le service ; identité des entrées et erreurs lisibles | Q avec lecture IO | Non | Oui avec service configuré | T33, T34 |
| Récupérer → `recovery.restore` | `recover_document`, `request_action`, `Entry::load`, `Recovery::adopt` | `RecoveryRef`, traitement explicite du travail courant ; ouverture comme copie à enregistrer | C-IO | Cond. | Oui avec service configuré | T33 |
| Supprimer une copie → `recovery.delete` | `recovery_dialog`, `Entry::delete` | `RecoveryRef` ; suppression explicite, aucune transaction d’édition | C-IO de service | Non | Oui avec service configuré | T34 |
| Plus tard / fermer la récupération | `recovery_dialog` | Fermer la liste sans effacer les copies | UI | Oui | Non | T33 |
| Enregistrer / abandonner / annuler lors d’un remplacement ou d’une fermeture | `app/files::file_dialogs`, `PendingAction` | UI compose sauvegarde puis nouvelle action, ou transmet `discard_changes: true`, ou ne fait rien | UI + C-IO | Oui si nécessaire | Opérations explicites : oui | T04, T34 |
| Onglet d’informations, préférences, menus Titres/Listes, fermer un dialogue ou une erreur | `information_ui`, `preferences_ui`, `toolbar::show`, `dialogs`, `file_dialogs` | États de présentation, groupes et focus | UI | Selon panneau | Non | T10, T21, T27, T49 |
| Requête/choix dans la palette, flèches, Entrée, Échap | `palette_dialog`, `dispatch_action` | Filtre local ; puis invocation de la commande choisie | UI | Palette | Commande choisie : selon son catalogue | T39, T49 |
| Navigation avant vers l’aperçu | `editor_feedback`, `preview_ui`, `RenderSync::forward` | Q `render.locate` avec position logique et référence de rendu valide | Q + défilement UI | Non | Localisation : oui ; défilement : non | T38, T64 |
| Double-clic dans l’aperçu | `preview_ui`, `navigate_backward`, `RenderSync::backward` | Q `render.locate` avec page/point d’un rendu puis C-sel | Q + C-sel + focus UI | Non | Conversion : oui ; clic : non | T38, T64 |
| Surlignage de sélection | `RenderSync::selection_highlights`, `preview_ui` | Q `render.selection_boxes`, sélection et référence de rendu | Q + peinture UI | Non | Géométrie : oui | T38, T65 |
| Lire les compteurs document/sélection | `chrome::status_ui`, `statistics::Counter` | Q `document.statistics`, cible optionnelle | Q + affichage UI | Non | Oui | T36 |

Une récupération et un enregistrement ne deviennent pas des étapes d’annulation.
Une suppression de copie et un export sont des effets externes : exclus des lots
d’édition comme les sauvegardes et réglages globaux (D16). Les services sans UI
reçoivent leurs racines/configurations explicitement ; ils ne créent pas une
nouvelle permission d’accès global, notamment pour le futur MCP (D12).

## 6. Requêtes proposées pour lire et préparer les actions

Les noms de cette section sont proposés et restent à valider ; `spelling.suggestions`
reprend un identifiant existant. Réponses structurées, erreurs et descriptions de catalogue suivront D6/D7 ; les schémas ne sont pas écrits ici.

| Requête | Données / cible | Source actuelle | Test / vérification à reprendre |
| --- | --- | --- | --- |
| `commands.list` | Actions, familles, schémas, disponibilité et motif, capacités du type ; filtre sans UI | `Registry::definitions`, `menu_statuses` | T42, T63 ; distinction C/Q/UI à tester |
| `document_types.list` | Types définis, modèles proposés, possibilité effective de création | `DocumentType::ALL/can_create` | T01, T42 |
| `models.list` | Modèles fournis du type actif, avec identifiants des préréglages | `models::ALL`, `DocumentType::models` | T42 |
| `models.presentation` | Contrat, papier verrouillé, puces et choix autorisés du modèle fourni ou courant | `model_presentation`, descripteurs | T23, T41 |
| `document.inspect` | Type, session, révision, état enregistré/récupéré, capacités ; pas le profil global | `Session`, `is_dirty`, registre | T01, T02, T43 |
| `document.structure` | Arbre logique, identités de blocs/zones, rôles, limites d’édition | `Projection` et annotations, à migrer en 2.1 bis | T12, T14, T53 ; identités à tester |
| `document.text` | Texte visible d’une sélection/zone/document ; exclure sentinelles et champs privés | `Projection`, `Search`, `statistics` | T07, T36, T44 ; découpage par zones à tester |
| `document.source` | Source Typst documentaire du relevé validé, pas une commande d’exécution de code | `Document::source` | T40, T48 |
| `document.selection` | Sélection logique orientée et contexte de frappe | `Session::Selection`, `App.document.selection` | T06, T43 |
| `document.statistics` | Mots, signes Unicode, signes hors espaces ; document et cible | `statistics::Counter` | T36 |
| `document.assets` | Inventaire des ressources du document et références utilisées, pas l’historique abandonné | `Assets`, `retained_assets` | T17, T48 |
| `metadata.get` / `metadata.fields` | Valeurs / schéma des champs du type actif, groupes et obligations | `Metadata`, `TextField`, `FieldGroup` | T21, T58 |
| `document.options` | Valeurs communes et libertés du modèle ; exceptions orthographiques documentaires séparées | `DocumentOptions`, `ModelPresentation` | T23, T60 |
| `document.validate` | Diagnostics du sous-ensemble, du contrat, des champs d’export et ressources | `Document::from_source`, `validate_model`, stockage, `ready_for_export` | T41, T47, T48 |
| `search.find` | Recherche exacte, casse et périmètre explicites ; références d’occurrence avec révision | `search::Search` | T29, T44 |
| `spelling.issues` | Faute, langue, cible et révision ; statut dictionnaire disponible/indisponible | `spelling::Checker` | T31, T32 |
| `spelling.suggestions` | Mot/langue explicites ou référence de faute ; suggestions, pas ouverture de menu | `Checker::request_suggestions` | T31 |
| `render.compile` | Compiler un relevé validé ; référence de rendu, révision et diagnostics, aucun export disque | `RenderWorker`, `TypstWorld` | T60, T64 |
| `render.locate` | `direction: to_source/to_preview`, position logique ou page/point du rendu identifié | `RenderSync` | T38, T64 |
| `render.selection_boxes` | Quadrilatères par page de la sélection sur un rendu identifié | `RenderSync::selection_highlights` | T65 |
| `recovery.list` | Copies du service de récupération configuré, sans les restaurer ou les supprimer | `Recovery::start`, entrées | T33, T34 |

Les Q asynchrones doivent identifier leur état de référence. Une géométrie de rendu
n’est utilisable qu’avec le rendu, la session et la révision qui l’ont produite ;
ni la page affichée ni sa mise à l’échelle ne sont une cible d’édition persistante.
Les numéros de page du contrat technique partent de 0 ; les points sont exprimés
en points Typst du rendu identifié, pas en pixels d’écran.
Pour `document.text`, privilégier les zones ordonnées avec leurs identifiants et
textes. Une concaténation lisible peut servir à copier/exporter du texte, mais
ses indices ne deviennent pas une seconde façon d’adresser le document.
Les résultats d’orthographe signalent « analyse en cours / non disponible » plutôt
que de faire passer un relevé périmé pour les fautes du document courant.
La pagination/streaming des réponses volumineuses sera précisée avec la CLI/MCP,
sans inventer de format de transport dans cette préparation.

## 7. Règles transversales et limites de migration

### 7.1 Disponibilité et erreurs

Chaque commande est contrôlée par le moteur, même si son bouton était activé.
Pas de document ouvert, capacité non offerte par le type, mauvais type d’objet,
champ inconnu, modèle qui verrouille une option, garde périmée et chemin manquant
sont des erreurs structurées. Proposition de codes supplémentaires à valider en
plus de D6 : `stale_revision`, `invalid_target`, `invalid_selection`, `unsaved_changes`,
`readonly_option`, `service_unavailable`, `empty_history`. Une opération sans effet
réel réussit avec `changed: false` sans historique ; une cible introuvable n’est
pas une réussite silencieuse. Le catalogue ne remplace pas cette validation.
Distinguer dans le résultat changement documentaire, changement de sélection ou
de contexte de frappe et effet IO. Déplacer le curseur ne doit pas marquer le
fichier modifié ni relancer la compilation ; cela peut mettre à jour le surlignage.

### 7.2 Transactions et effets externes

Les commandes de contenu, métadonnées, ressources, modèle et signature partagent
validation, normalisation, révision et historique. La suppression conserve les
ressources nécessaires à l’annulation ; l’enregistrement/export ne conserve que
celles utilisées par le document ou son modèle, comme actuellement.

Une opération avec import d’image ou de modèle prépare et valide ses données avant
publication ; un échec laisse la session intacte. Les lectures de dépendances restent
contrôlées. Un lot d’édition d’un document utilise une copie temporaire et une seule
écriture atomique finale (D16). Ni export, ni sauvegarde intermédiaire, ni récupération
à supprimer, ni préférences globales dans les étapes du lot. La publication d’un
export multi-fichiers ne bénéficie pas implicitement de l’atomicité de `.typsto`.
Les fichiers de travail actifs ne sont remplacés qu’après validation complète ;
les erreurs de chargement conservent la session en place.

### 7.3 Ce qui reste dans l’application, sans action publique par frame

Rafraîchissement des textures, temporisation orthographique, réveils du rendu,
instantanés de récupération, observation/enregistrement différé des préférences,
titre de fenêtre, adaptation au thème système, peinture et défilement sont des
services/effets du cycle de vie. Ils ne créent pas une commande d’édition par frame.
Les focus et données de formulaires/palette/menu relèvent de `UiCommand` ou de
leur état de présentation ; leur validation seule déclenche la commande de contenu.

`--render-demo`, `--capture*`, les scénarios d’exemple et le prototype de tableau
restent des outils de diagnostic, pas des fonctions du catalogue métier CLI/MCP.
Leurs appels internes pourront réutiliser les opérations normales ; ils ne doivent
pas devenir une voie contournant les validations du cœur.

### 7.4 Couverture à compléter avant le gel

- Aller-retour des paramètres et aiguillage par identifiant pour chaque C/Q ;
  présentation et préparation des paramètres pour chaque action UI.
- Cibles persistantes pendant une session, restauration d’identités par historique,
  rejet d’anciennes sessions/révisions et limites Unicode ; limites sans octets ni sentinelles.
- Parité de saisie native, collage, entrée interactive, préédition/annulation/commit
  IME et commandes explicites ; styles, sélection et groupe d’historique restitués.
- Sélection entre paragraphes et entre rôles ; erreurs sans mutation partielle.
- Même contrôle du type et de la politique du modèle depuis interface et sans interface.
- Image, signature et modèle : préparation, budget, ressources autonomes, échec sans
  publication, puis annuler/rétablir. Export PDF seul / Typst seul / ensemble selon V6.
- Préférences/dictionnaire global exclus du moteur et des lots selon V5 ; modification
  documentaire des exceptions bien distincte de l’apprentissage global.
- Presse-papiers et méthodes de saisie du système à vérifier lors des essais natifs,
  en complément de leurs séquences d’événements egui automatisées.

## 8. Références des tests existants

| Réf. | Fichier | Nom exact du test |
| --- | --- | --- |
| T01 | [files.rs](../src/app/tests/files.rs) | `welcome_creation_and_close_use_the_catalog_without_an_implicit_document` |
| T02 | [files.rs](../src/app/tests/files.rs) | `save_reopen_restores_the_letter_and_failed_open_preserves_current_work` |
| T03 | [files.rs](../src/app/tests/files.rs) | `save_shortcut_and_save_as_keep_both_files_and_track_clean_state` |
| T04 | [files.rs](../src/app/tests/files.rs) | `unsaved_work_blocks_closing_or_replacing_until_the_user_decides` |
| T05 | [files.rs](../src/app/tests/files.rs) | `egui_file_menu_exports_and_quit_preserves_unsaved_work_without_header_duplicates` |
| T06 | [commands.rs](../src/app/tests/commands.rs) | `cross_paragraph_formatting_undo_restores_source_and_selection` |
| T07 | [text_input.rs](../src/app/tests/text_input.rs) | `native_egui_replaces_a_unicode_selection_without_source_markers` |
| T08 | [text_input.rs](../src/app/tests/text_input.rs) | `text_shortcuts_can_be_undone_to_the_complete_literal_input` |
| T09 | [commands.rs](../src/app/tests/commands.rs) | `remove_link_keeps_other_styles_and_selection_and_can_be_undone` |
| T10 | [toolbar.rs](../src/app/tests/toolbar.rs) | `toolbar_icons_dispatch_commands_and_keep_headings_separate_from_quotes` |
| T11 | [text_input.rs](../src/app/tests/text_input.rs) | `description_editing_uses_tab_and_colon_without_exposing_typst` |
| T12 | [text_input.rs](../src/app/tests/text_input.rs) | `quote_keyboard_edits_body_and_attribution_and_exits_without_losing_source` |
| T13 | [text_input.rs](../src/app/tests/text_input.rs) | `letter_notes_insert_type_scope_editing_exit_and_undo_use_the_shared_session` |
| T14 | [images.rs](../src/app/tests/images.rs) | `images_insert_edit_caption_replace_undo_and_reopen_without_original_file` |
| T15 | [images.rs](../src/app/tests/images.rs) | `image_remove_button_deletes_the_selected_block_and_caption_with_undo` |
| T16 | [images.rs](../src/app/tests/images.rs) | `image_controls_edit_alt_and_keyboard_deletes_the_whole_block_with_undo` |
| T17 | [images.rs](../src/app/tests/images.rs) | `saving_prunes_images_on_disk_without_changing_undo_redo_or_clean_state` |
| T18 | [annotations.rs](../src/app/tests/annotations.rs) | `annotation_navigation_and_exit_preserve_content_selection_and_undo` |
| T19 | [annotations.rs](../src/app/tests/annotations.rs) | `tab_places_the_native_caret_on_each_empty_annotation_placeholder` |
| T20 | [annotations.rs](../src/app/tests/annotations.rs) | `annotation_navigation_scrolls_to_the_target_in_both_structures` |
| T21 | [information.rs](../src/app/tests/information.rs) | `inline_information_keeps_text_accessible_and_metadata_edits_refresh_the_document` |
| T22 | [information.rs](../src/app/tests/information.rs) | `today_button_keeps_a_fixed_date_through_history_and_reopening` |
| T23 | [information.rs](../src/app/tests/information.rs) | `document_information_offers_only_the_markers_allowed_by_the_template` |
| T24 | [commands.rs](../src/app/tests/commands.rs) | `creation_and_template_import_preserve_the_type_and_use_unified_history` |
| T25 | [commands.rs](../src/app/tests/commands.rs) | `model_dependencies_follow_imports_choices_and_undo_without_changing_the_body` |
| T26 | [profile.rs](../src/app/tests/profile.rs) | `personal_profile_seeds_new_documents_and_signature_edits_are_independent_and_undoable` |
| T27 | [profile.rs](../src/app/tests/profile.rs) | `global_profile_saves_after_changes_or_closing_without_modifying_the_document` |
| T28 | [profile.rs](../src/app/tests/profile.rs) | `editor_preferences_change_glyphs_and_theme_without_touching_the_letter` |
| T29 | [search.rs](../src/app/tests/search.rs) | `incremental_search_keeps_one_selection_and_clicking_another_result_updates_preview_highlights` |
| T30 | [search.rs](../src/app/tests/search.rs) | `replacement_commands_refresh_results_and_replace_all_is_a_single_document_transaction` |
| T31 | [spelling.rs](../src/app/tests/spelling.rs) | `spelling_context_menu_corrects_rich_text_with_undo_and_never_applies_a_stale_suggestion` |
| T32 | [spelling.rs](../src/app/tests/spelling.rs) | `spelling_exceptions_are_scoped_persisted_and_undoable_without_affecting_typst` |
| T33 | [recovery.rs](../src/app/tests/recovery.rs) | `recovery_dialog_defers_restores_and_saves_a_copy_without_overwriting_the_original` |
| T34 | [recovery.rs](../src/app/tests/recovery.rs) | `explicit_delete_and_discard_remove_recovery_but_cancelling_keeps_it` |
| T35 | [preferences.rs](../src/preferences.rs) | `appearance_and_column_preferences_survive_a_new_session` |
| T36 | [toolbar.rs](../src/app/tests/toolbar.rs) | `compact_toolbar_and_status_show_unicode_totals_selection_and_tooltips` |
| T37 | [preview.rs](../src/app/tests/preview.rs) | `column_drag_and_window_resize_keep_the_chosen_proportion_without_a_preview_cap` |
| T38 | [preview.rs](../src/app/tests/preview.rs) | `synchronization_scrolls_both_panes_and_rejects_outdated_or_failed_renders` |
| T39 | [commands.rs](../src/app/tests/commands.rs) | `shortcut_override_updates_dispatch_and_hints_without_changing_the_document` |
| T40 | [tests.rs](../src/tests.rs) | `literal_typst_characters_are_not_executed` |
| T41 | [courrier.rs](../src/document_types/courrier.rs) | `courrier_contract_accepts_independent_typst_models_without_presentation_flags` |
| T42 | [models.rs](../src/document_types/models.rs) | `type_availability_defaults_and_public_commands_all_follow_the_catalog` |
| T43 | [session.rs](../src/session.rs) | `unified_history_restores_content_metadata_model_selection_and_clean_state` |
| T44 | [search.rs](../src/search.rs) | `navigation_wraps_and_search_includes_annotations_but_excludes_structure_and_private_fields` |
| T47 | [storage.rs](../src/storage.rs) | `replacing_an_archive_is_atomic_and_invalid_save_preserves_existing_bytes` |
| T48 | [storage.rs](../src/storage.rs) | `archive_and_export_omit_unused_images_but_keep_document_and_model_resources` |
| T49 | [commands.rs](../src/app/tests/commands.rs) | `information_and_presentations_work_from_palette_and_custom_shortcuts` |
| T50 | [text_input.rs](../src/app/tests/text_input.rs) | `links_accept_soft_lines_and_reject_paragraph_or_list_boundaries` |
| T51 | [tests.rs](../src/tests.rs) | `paragraph_commands_include_soft_lines_but_stop_at_separators_and_structures` |
| T52 | [tests.rs](../src/tests.rs) | `model_controls_list_markers_and_language_defaults_in_actual_typst_output` |
| T53 | [tests.rs](../src/tests.rs) | `description_lists_preserve_terms_styles_unicode_and_source_positions` |
| T54 | [text_input.rs](../src/app/tests/text_input.rs) | `converting_multiple_paragraphs_to_quote_preserves_selection_and_can_be_undone` |
| T55 | [tests.rs](../src/tests.rs) | `postscript_insertion_precedes_existing_enclosures_and_reuses_its_existing_block` |
| T56 | [tests.rs](../src/tests.rs) | `image_insertions_and_cross_block_deletions_keep_the_projection_valid` |
| T57 | [text_input.rs](../src/app/tests/text_input.rs) | `native_enter_distinguishes_soft_newlines_and_paragraphs_with_undo_and_reopening` |
| T58 | [courrier.rs](../src/document_types/courrier.rs) | `courrier_field_definitions_drive_header_and_required_export_fields` |
| T59 | [dates.rs](../src/dates.rs) | `aliases_resolve_to_fixed_dates_and_explicit_dates_are_validated` |
| T60 | [ime.rs](../src/app/tests/ime.rs) | `preedit_survives_frames_and_is_visible_without_publishing_any_session_state` |
| T61 | [ime.rs](../src/app/tests/ime.rs) | `ime_commit_and_explicit_replacement_have_the_same_literal_source_styles_and_selection` |
| T62 | [ime.rs](../src/app/tests/ime.rs) | `saving_and_recovery_only_contain_committed_text` |
| T63 | [ime.rs](../src/app/tests/ime.rs) | `commands_and_new_documents_cancel_composition_and_ignore_its_late_commit` |
| T60 | [tests.rs](../src/tests.rs) | `document_options_control_real_page_dimensions_and_text_language` |
| T61 | [courrier.rs](../src/document_types/courrier.rs) | `personal_window_letter_compiles_in_app_and_keeps_addresses_on_first_page` |
| T62 | [profile.rs](../src/profile.rs) | `profile_resources_are_managed_and_documents_survive_original_and_global_image_removal` |
| T63 | [commands.rs](../src/commands.rs) | `registry_ids_are_unique_and_defaults_have_no_conflicts_on_either_platform` |
| T64 | [tests.rs](../src/tests.rs) | `bidirectional_navigation_handles_unicode_styles_links_and_repeated_passages` |
| T65 | [tests.rs](../src/tests.rs) | `selection_highlights_follow_visible_unicode_text_across_styles_and_links` |

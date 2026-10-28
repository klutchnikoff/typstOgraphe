# Commandes multizones et opérations structurelles — A.2

Ce lot prolonge le [socle Yrs](structured-contract-foundation.md) selon les
[décisions validées](structured-contract.md). Il ajoute une API Rust commune,
testée avec le véritable éditeur. Les tableaux et les commandes de mise en forme
de Tauri utilisent désormais cette API ; les autres opérations web et les
adaptateurs CLI/MCP restent à migrer.
Ce lot ne clôture pas A.2.

## Contrat exécuté

Les types publics sont dans `content::commands`. Le schéma JSON de `Command`
est produit par Schemars depuis les types réellement désérialisés. Les champs
inconnus et les offsets négatifs sont refusés. `specifications()` annonce le
périmètre et les identifiants exécutables ; `content::catalogue::describe`
annonce également le support `shared_core`, avec son périmètre d’exécution explicite.
Ce support ne signifie pas que CLI ou MCP ont déjà migré.

| Commande | Effet dans ce lot |
| --- | --- |
| `text.insert` | Remplacer une plage ou insérer du texte littéral au curseur, avec styles explicites |
| `text.delete` | Supprimer le contenu d’une plage dans une seule zone |
| `text.bold` | Appliquer ou retirer le gras d’une plage, avec `value: "on"` ou `"off"` |
| `text.italic` | Même fonctionnement pour l’italique |
| `text.link` | Appliquer un lien HTTP, HTTPS ou mailto à une plage |
| `text.remove_link` | Retirer ses liens en conservant les autres styles |
| `text.clear` | Retirer les styles d’une plage |
| `metadata.set` | Modifier un champ simple ou un champ d’enregistrement identifié, sans remplacer sa collection |
| `block.insert` | Insérer un sous-arbre à une frontière de blocs |
| `block.replace` | Remplacer explicitement un sous-arbre |
| `block.remove` | Supprimer un bloc ou un item, en respectant les frontières |
| `block.split` | Séparer un paragraphe ou un titre au curseur ; la suite devient un paragraphe |
| `block.paragraph`, `block.heading_1` à `block.heading_3` | Convertir un paragraphe/titre ou changer son niveau |
| `insert.table` | Construire un tableau avec grille et légende facultative à une frontière de blocs |
| `table.row.insert`, `table.row.remove` | Ajouter ou retirer une ligne à un indice explicite |
| `table.column.insert`, `table.column.remove` | Ajouter ou retirer une colonne à un indice explicite |
| `table.header` | Définir le rôle d’en-tête de la première ligne |
| `record.add` | Ajouter un enregistrement avec son identifiant et les valeurs par défaut du type |
| `record.move` | Changer sa position dans une collection ordonnée |
| `record.remove` | Supprimer un enregistrement sans détacher silencieusement ses références |

L’insertion n’interprète ni Markdown ni Typst. `marks` est vide par défaut ;
elle n’hérite pas d’un mode de frappe implicite ni du style voisin. Les styles
acceptés sont `{"type":"bold"}`, `{"type":"italic"}` et
`{"type":"link","url":"https://example.org"}`. Gras et italique utilisent
`"on"` par défaut, jamais un basculement dépendant d’un état caché.

## Adressage

Une cible textuelle associe une zone et une plage UTF-16 :

```json
{
  "id": "text.insert",
  "params": {
    "target": {
      "zone": { "kind": "body", "path": [2, 0, 0, 0] },
      "from": 0,
      "to": 7
    },
    "text": "Cellule revue"
  }
}
```

Le membre `zone` contient une adresse typée, discriminée par `kind`.
Le chemin désigne ici le paragraphe d’une cellule, à partir du corps documentaire.
Il n’est ni une position globale ProseMirror, ni un indice physique de Y.Xml.
Un même Y.XmlText peut produire plusieurs portions de texte stylées ; le
résolveur tient compte de cette différence.

| Zone | Adresse | Contenu accessible |
| --- | --- | --- |
| `body` | `path` dans le corps | Paragraphe, titre, cellule via son paragraphe, légende, attribution, terme/définition, P.-S. ou P.J. |
| `annotation` | `path` de l’objet dans le corps | Texte brut d’une note de bas de page |
| `metadata` | Identifiant public `field`, par exemple `letter.subject` | Valeur textuelle d’un champ du type courant |
| `record` | `collection`, `id`, `field`, et `path` pour un champ riche | Nom d’auteur, signature d’affiliation, titre, résumé, etc. |

Pour une zone riche, `from` et `to` comptent les unités UTF-16 de ses contenus
en ligne. Une note, une citation ou un retour interne compte pour une unité ;
la longueur du libellé affiché du badge ne compte pas. `inspect_zone` représente
ces objets par U+FFFC dans son texte de lecture. Pour une zone `annotation`,
les offsets portent sur le texte de la note, sans son badge.

Une cible ne traverse jamais une cellule ou une autre zone. Les plages
inversées, absentes, hors limites ou situées entre les deux unités d’un
caractère supplémentaire Unicode sont refusées. Les champs sont résolus depuis
les descripteurs du type ; une collection inconnue, un enregistrement absent,
un champ privé ou l’identifiant `id` ne sont pas des cibles éditables.

Exemple d’en-tête :

```json
{
  "id": "metadata.set",
  "params": {
    "target": {
      "kind": "record",
      "collection": "authors",
      "id": "author-1",
      "field": "email"
    },
    "value": "camille@example.org"
  }
}
```

`metadata.set` accepte les genres simples déclarés : textes, langue, booléens,
listes de mots-clés et associations. Il ne remplace ni un champ riche ni une
collection entière ; les zones riches utilisent les commandes textuelles.
Les identifiants publics de champs restent distincts des paramètres Typst et
des clés internes des valeurs sérialisées.

## Opérations structurelles

Une cible d’arbre contient `root` et `path`. La racine est `{"kind":"body"}`
ou `{"kind":"record","collection":"localizations","id":"fr","field":"abstract"}`.
Le chemin vide désigne le conteneur racine ; un chemin non vide désigne un
élément de son arbre projeté, comme pour les zones textuelles.

`block.insert` reçoit `parent`, `index` et `node`. L’indice est une frontière
entre les enfants du conteneur, dans l’état privé atteint par le lot ; `node`
est un sous-arbre du contrat documentaire `Node`, jamais du Typst exécutable.
Paragraphes, titres, listes simples, descriptions, citations, tableaux et figures
peuvent ainsi être construits. Les images doivent déjà exister dans les ressources
de la séance. Les tableaux doivent être rectangulaires et sans cellules fusionnées.

Exemple d’insertion d’une citation au milieu d’un paragraphe :

```json
[
  {
    "id": "block.split",
    "params": {
      "target": { "zone": { "kind": "body", "path": [0] }, "from": 4, "to": 4 }
    }
  },
  {
    "id": "block.insert",
    "params": {
      "parent": { "root": { "kind": "body" }, "path": [] },
      "index": 1,
      "node": {
        "type": "quote",
        "content": [
          { "type": "paragraph", "content": [{ "type": "text", "text": "Citation" }] },
          { "type": "quoteAttribution" }
        ]
      }
    }
  }
]
```

Au début d’un paragraphe, on insère directement avant celui-ci ; aucune séparation
préalable n’est nécessaire. `block.split` exige un curseur, donc `from == to`,
et refuse les annotations brutes et les zones qui ne sont pas des paragraphes/titres.
Il fonctionne aussi dans les cellules et les résumés. Un titre d’en-tête ne peut
pas devenir deux paragraphes : son profil `rich_inline` continue à s’appliquer.

`block.replace` reçoit `target` et `node`, `block.remove` seulement `target`.
Les racines, cellules, lignes de tableau, badges et zones d’attribution ne sont
pas supprimables comme des blocs ordinaires. Un item de liste peut être retiré,
mais un lot ne peut pas laisser une liste vide ; il doit alors retirer son widget.
La suppression du dernier paragraphe d’une cellule ou d’un champ riche conserve
un paragraphe vide éditable. La structure finale est contrôlée également lors de
la réception d’une mise à jour partagée brute, avant publication.

Les conversions `block.paragraph` et `block.heading_*` concernent un seul
paragraphe/titre. Elles ne transforment pas implicitement une liste ou une image
en texte. Un changement de niveau conserve le titre et son contenu partagés.
Le passage paragraphe ↔ titre, comme `block.replace`, renouvelle le sous-arbre
explicitement transformé. Une séparation conserve le paragraphe et son préfixe ;
la suite déplacée reçoit de nouveaux objets. Les voisins, cellules englobantes et
racines privées restent en place. Les styles et les annotations sont conservés
dans la projection, même lorsque leurs objets doivent être recréés.

Avant publication d’un lot typé, les lignes vides sont normalisées dans le
candidat : pas de première ligne vide devant du contenu, pas de paragraphes
vides consécutifs dans le flux, et un point de saisie final dans le corps.
Les cellules et items gardent leurs frontières. Les résumés suivent le profil
riche sans paragraphe final imposé. Cette politique Rust est confrontée à celle
de la frappe immédiate dans Tiptap ; le raccordement de production reste à faire.

## Figures et tableaux — contrat mis en place

Les commandes communes et le parcours Tauri exécutent ce contrat. Son exposition
complète en CLI/MCP dépend toujours de la migration des adaptateurs.

Images et tableaux sont toujours enveloppés dans une `figure` Typst, même sans
légende. La génération partage la légende facultative et la politique
de placement ; chaque contenu garde son édition spécialisée. L’éditeur
présente un seul widget, avec l’en-tête et les bandeaux communs existants.
Le modèle définit la présentation, la numérotation et les valeurs de placement,
sans réglage de placement par objet dans ce lot. Les références croisées restent
à réaliser. Les deux modèles fournis rendent les figures de tableaux sécables
pour les tableaux longs, sans modifier le comportement des images.

La grille conserve les lignes, colonnes, cellules et rôles sémantiques des
en-têtes. Un bandeau de création demande les dimensions et la présence d’une
ligne d’en-tête. Pendant l’édition, la roue dentée ou les commandes de la palette
ouvrent le bandeau commun sur la cellule active :

- ajouter une ligne au-dessus ou au-dessous ;
- supprimer sa ligne ;
- ajouter une colonne à gauche ou à droite ;
- supprimer sa colonne.

Les choix et leurs libellés proviennent du catalogue du cœur. Flèches, Entrée et
Échap utilisent la navigation commune des bandeaux. La corbeille reste dédiée
au widget entier ; supprimer la dernière ligne ou colonne est indisponible.
Le premier périmètre reste limité à 100 lignes et 12 colonnes, sans fusion.
Les cellules suivent les retours du corps : un retour simple reste dans le
paragraphe ; deux retours le séparent, à l’intérieur de la cellule.
La légende accepte les styles et contenus en ligne du document. Tab depuis la
dernière cellule rejoint la légende ; Maj+Tab revient dans la grille ; Tab depuis
la légende rejoint la ligne de reprise après le widget. L’image et l’attribution
de citation partagent la même résolution d’annotation et les règles de retour.

Dans le contrat structuré, `tableFigure` contient une grille `table` et une zone
`tableCaption`. Cette structure est masquée par un seul widget « Tableau » ;
aucun widget supplémentaire n’entoure la grille. Les images conservent leur
nœud et leur interface existants. Le parseur HTML construit également cette
enveloppe lors du collage d’un tableau.

Les paramètres communs sont `parent`/`index` et `rows`/`columns`/`header` pour
`insert.table`, puis `target` et `index` pour les lignes/colonnes. La cible désigne
`tableFigure` ; l’indice d’insertion va de zéro au nombre courant de lignes ou
colonnes. `table.header` reçoit `target` et `enabled`. Les insertions/suppressions
modifient directement les objets Yrs de la grille et préservent les voisins et
la légende. Un changement de rôle d’en-tête renouvelle uniquement les cellules
concernées, dont le texte et les marques sont conservés. Une insertion au-dessus
de l’en-tête crée une nouvelle ligne d’en-tête et transforme l’ancienne en données.

Le parcours Tauri prépare le lot dans un candidat privé après acquittement des
relais en cours. Le cœur vérifie que le checkpoint fourni correspond à l’historique
confirmé du propriétaire. Le client recontrôle document, révision, saisie et curseur ;
la validation finale réexécute ensuite le lot sous le verrou du propriétaire et
revérifie la garde avant publication. Un changement concurrent à cette étape
refuse tout le lot. Pendant ce court engagement final, le bandeau reste en attente
et la saisie du corps est temporairement indisponible ; la préparation laisse le
corps éditable. La mise à jour confirmée n’est pas renvoyée dans le relais par le
client, mais elle rejoint son historique : annuler le lot constitue une seule étape
et diffuse une modification documentaire normale aux autres fenêtres.

La frappe générale est également confirmée par le propriétaire Rust, selon le
[contrat des contextes d’édition](structured-editing-contexts.md). Aucun outil MCP
supplémentaire n’accorde un contournement du parcours de propositions soumises à
validation.

Accessibilité :

- Le texte alternatif d’une image est porté par `image(alt: ...)`.
- Aucun `figure(alt: ...)` n’est ajouté autour d’un tableau : cela remplacerait
  la lecture de ses cellules par la description. Les cellules et leurs en-têtes
  restent accessibles comme un véritable tableau.
- Les en-têtes de colonnes utilisent `table.header`. Les en-têtes de lignes
  nécessitent l’évaluation de l’API Typst actuellement expérimentale
  `pdf.header-cell`, sous le drapeau `a11y-extras`.
- La cible d’export à préparer est PDF/UA-1, éventuellement combinée à un profil
  PDF/A compatible pour l’archivage. Ce choix ne signifie pas que l’export courant
  vérifie déjà la conformité ; les modèles et les métadonnées devront la permettre.

Références : [figure](https://typst.app/docs/reference/model/figure/),
[table](https://typst.app/docs/reference/model/table/),
[en-têtes accessibles](https://typst.app/docs/reference/pdf/header-cell/) et
[guide d’accessibilité](https://typst.app/docs/guides/accessibility/).

## Mise en forme raccordée dans l’application

Les boutons, la palette, les raccourcis et les outils des champs riches passent
par `formatting.ts`. Les liens utilisent cette même voie depuis leur bandeau.

| Action de l’interface | Commande commune |
| --- | --- |
| Gras et italique sur une sélection | `text.bold`, `text.italic`, avec état explicite |
| Effacer la mise en forme | `text.clear` |
| Valider ou retirer un lien | `text.link`, `text.remove_link` ; `text.unlink` reste le nom du menu |
| Paragraphe et titres | `block.paragraph`, `block.heading_1` à `block.heading_3` |

`editor-targets.ts` traduit la sélection ProseMirror en chemins de l’arbre public
et plages UTF-16. Une sélection traversant plusieurs paragraphes, cellules,
légendes, attributions ou termes/définitions devient un lot atomique de plages
distinctes. Les badges et retours internes comptent chacun pour une unité ; le
texte affiché de leur libellé ne participe pas au calcul. Les champs riches
d’en-tête portent leur collection, identifiant d’enregistrement et nom de champ ;
leur position dans la liste ou leur libellé traduit ne sert jamais d’adresse.

La sélection et le focus restent locaux. Une conversion de titre préserve la
sélection même si l’identité XML du bloc change. Au curseur, gras et italique
modifient seulement le style de la frappe suivante ; aucune commande de plage
vide n’est envoyée au cœur. Les brouillons de formulaires détachés restent locaux
jusqu’à leur validation ; le raccordement de leur publication est un lot distinct.

Les commandes explicites suivent préparation et publication gardées. La frappe,
les règles textuelles et le collage restent immédiats via le relais validé :
ils ne sont pas artificiellement découpés en commandes différées de mise en forme.
Le court engagement final utilise la même file que le relais et l’historique,
verrouille toutes les zones montées de la fenêtre et retarde les instantanés
jusqu’à l’installation de la réponse. Un refus de commande conserve le document,
la connexion et le brouillon du bandeau ; il ne déclenche pas la mise en quarantaine
d’une saisie valide. Un lot confirmé ne repasse pas par le relais brut et forme
une seule étape d’annulation native.

## Enregistrements ordonnés d’en-tête

Les collections et leurs champs proviennent des descripteurs du type courant,
sans table parallèle spécifique aux auteurs ou aux affiliations.

- `record.add` : `collection`, `id` explicite, `values` partiel, `after` facultatif.
  Sans `after`, ajout en fin. Les champs absents prennent les valeurs du type,
  y compris les fragments riches. Un identifiant existant, un champ inconnu ou
  une ancre absente est refusé.
- `record.move` : `collection`, `id`, `before` facultatif. Sans `before`, placement
  en fin. Seul l’ordre change ; les cartes, textes et fragments restent les mêmes.
  Un ordre déjà satisfait ne crée pas de révision.
- `record.remove` : `collection`, `id`. Toutes ses entrées d’ordre et sa carte sont
  retirées. Les associations restent explicites : pour supprimer une affiliation
  utilisée, le lot doit aussi modifier les `affiliation-ids` concernés avec
  `metadata.set`. Aucun détachement automatique ne masque une perte d’information.

La validation finale contrôle associations, unicité des langues et limites du
type. Un brouillon peut supprimer sa dernière localisation ; il reste conservable
mais ne devient pas exportable sans les informations exigées par le type.
Les fichiers bibliographiques suivent le même contrat d’enregistrements ; leur
contenu et leurs références continuent à être validés par la bibliothèque existante.

## Préparation et publication

`SharedSession::prepare_commands(guard, commands)` prépare un candidat isolé.
La garde associe identité du projet et révision. Chaque étape vise l’état privé
résultant des précédentes ; les chemins des étapes suivantes doivent en tenir
compte. Aucun réancrage implicite n’est effectué.

Les éditions textuelles modifient les objets partagés en place : paragraphes,
cellules, fragments riches, textes d’enregistrements et annotations conservent
leurs identités. Les exceptions des transformations structurelles sont décrites
ci-dessus. Les racines privées de la liaison Tiptap sont conservées. Le
remplacement d’un texte simple préserve son préfixe et son suffixe communs.

La validation structurelle, des métadonnées et des références porte sur l’état
final. Une référence temporairement invalide peut être réparée par une autre
étape du lot. Un refus ne publie aucune partie du candidat.

`publish` recontrôle la garde et produit une mise à jour différentielle et une
seule nouvelle révision. Les commandes déjà satisfaites ne créent pas d’édition
inutile. Une préparation périmée est refusée, même si elle avait été valide.

`availability` utilise la même préparation et retourne `enabled` avec, en cas
de refus, la même erreur structurée. Cette requête ne réserve aucune révision
et ne modifie pas l’état confirmé. Elle utilise actuellement une copie du
candidat ; elle n’est pas encore branchée aux rafraîchissements de l’interface.

Les erreurs comprennent `invalid_target` et son motif, `invalid_parameters`,
`command_unavailable`, ainsi que `batch_failed` avec l’indice de l’étape et sa
cause. Les gardes et erreurs de validation finale conservent leurs codes
communs. Aucune traduction ou présentation graphique n’entre dans le cœur.

## Vérifications et suite

Les tests Rust couvrent corps, cellules, annotations, champs simples et riches,
Unicode, styles, limites des zones, erreurs et disponibilité, gardes, refus
atomiques, validation finale et opérations sans changement.

Les recettes `npm run test:contract` font exécuter un lot Rust sur un document
créé par Yjs/Tiptap, puis appliquent sa mise à jour dans le véritable éditeur.
Elles vérifient aussi les blocs et les collections ordonnées, la stabilité après
les normalisations différées de l’éditeur, les références des objets partagés, les styles sur les badges,
les métadonnées et une seule étape d’annulation/rétablissement. Le transport de
diagnostic reste réservé aux tests ; ce n’est pas une nouvelle voie publique MCP.

Restent dans A.2 :

- Le report des sélections et styles privés de l’interface vers les
  [contextes de frappe](structured-editing-contexts.md), puis leur raccordement
  aux adaptateurs autonomes. Leur sélection et leurs styles sont communs dans le cœur.
  Les commandes de style à cible explicite continuent de refuser une plage vide ;
  les styles de frappe appartiennent à l’API de contexte.
- La fusion de paragraphes et les transformations supplémentaires. L’insertion,
  la suppression de lignes/colonnes et les en-têtes de tableaux sont désormais
  communs ; la fusion de cellules reste différée.
  Les retours riches passent par `block.split` ou des nœuds de retour explicites,
  pas par des sauts de ligne cachés dans `text.insert`.
- Les insertions/conversions/suppressions des autres widgets et objets en ligne,
  leurs paramètres, la recherche/remplacement et l’orthographe, les champs simples
  et collections d’en-tête, les ressources, options et changements de modèle.
  Ces éditions passent encore par le relais validé lorsqu’elles ne sont pas
  explicitement raccordées aux commandes typées ci-dessus.
- La conservation durable automatique et la réimportation des brouillons refusés.
  Le groupement de frappe, l’IME, l’historique natif par fenêtre et le parcours
  export/abandon sont raccordés dans Tauri.
- Le mode individuel allégé et la migration documentaire autonome de CLI/MCP,
  avec catalogue, disponibilité et requêtes communs.

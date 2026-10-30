# Commandes multizones et opérations structurelles — A.2

Ce lot prolonge le [socle Yrs](structured-contract-foundation.md) selon les
[décisions validées](structured-contract.md). Il ajoute une API Rust commune,
testée avec le véritable éditeur. Les tableaux, la mise en forme, les widgets et
les objets en ligne, l’en-tête, les ressources, les options et les modèles de Tauri
utilisent désormais cette API. Les adaptateurs CLI/MCP ont depuis été migrés :
voir le [bilan des adaptateurs structurés](structured-adapters.md).
Ce lot ne clôture pas A.2.

## Contrat exécuté

Les types publics sont dans `content::commands`. Le schéma JSON de `Command`
est produit par Schemars depuis les types réellement désérialisés. Les champs
inconnus et les offsets négatifs sont refusés. `specifications()` annonce le
périmètre et les identifiants exécutables ; `content::catalogue::describe`
annonce également le support `shared_core`, avec son périmètre d’exécution explicite.
Cette mention décrivait le périmètre de ce lot intermédiaire ; CLI et MCP
utilisent maintenant la même façade.

| Commande | Effet dans ce lot |
| --- | --- |
| `text.insert` | Remplacer une plage ou insérer du texte littéral au curseur, avec styles explicites |
| `text.delete` | Supprimer le contenu d’une plage dans une seule zone |
| `text.bold` | Appliquer ou retirer le gras d’une plage, avec `value: "on"` ou `"off"` |
| `text.italic` | Même fonctionnement pour l’italique |
| `text.link` | Appliquer un lien HTTP, HTTPS ou mailto à une plage |
| `text.remove_link` | Retirer ses liens en conservant les autres styles |
| `text.clear` | Retirer les styles d’une plage |
| `spelling.ignore` | Accepter le mot d’une plage vérifiée pour la langue courante, dans ce document seulement |
| `metadata.set` | Modifier un champ simple, un champ d’enregistrement identifié ou une option générale typée |
| `resource.add` | Admettre une image immuable avec le lot qui l’utilise, sans écriture de fichier |
| `document.set_signature` | Affecter une ressource image à la signature du courrier, ou retirer la signature avec `path: null` |
| `document.set_model` | Valider et installer le source Typst et son identifiant de provenance en conservant les branches documentaires |
| `block.insert` | Insérer un sous-arbre à une frontière de blocs |
| `block.replace` | Remplacer explicitement un sous-arbre |
| `block.splice` | Remplacer une suite de blocs d’un conteneur, puis réparer ses frontières une seule fois |
| `inline.replace` | Remplacer une plage en ligne par un badge de note ou de citation, avec marques explicites |
| `node.set_attrs` | Modifier en place les paramètres publics d’un widget ou badge, en conservant son contenu et son identité |
| `block.remove` | Supprimer un bloc ou un item, en respectant les frontières |
| `block.join` | Fusionner deux voisins dans le même conteneur ; conserver un retour simple entre paragraphes non vides |
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
vide n’est envoyée au cœur. Le couple sélection/styles est transmis au
[contexte privé commun](structured-editing-contexts.md#sélections-et-styles-raccordés-dans-tauri--29-octobre-2026),
sans mutation documentaire. Les brouillons de formulaires détachés restent locaux
jusqu’à leur validation.

Les commandes explicites suivent préparation et publication gardées. La frappe,
les règles textuelles et le collage restent immédiats via le relais validé :
ils ne sont pas artificiellement découpés en commandes différées de mise en forme.
Le court engagement final utilise la même file que le relais et l’historique,
verrouille toutes les zones montées de la fenêtre et retarde les instantanés
jusqu’à l’installation de la réponse. Un refus de commande conserve le document,
la connexion et le brouillon du bandeau ; il ne déclenche pas la mise en quarantaine
d’une saisie valide. Un lot confirmé ne repasse pas par le relais brut et forme
une seule étape d’annulation native.

## Widgets et objets en ligne raccordés dans l’application

`object-commands.ts` est l’adaptateur commun des insertions, conversions,
paramètres et suppressions. La corbeille et Ctrl/Cmd+Retour arrière utilisent
`removeObject`, tout comme la suppression protégée des badges depuis leur bandeau.
Les listes simples, listes numérotées et descriptions, citations en bloc, images,
P.-S./P.J., notes et citations bibliographiques utilisent le propriétaire Rust.
Les opérations spécialisées de tableau gardent leurs commandes déjà raccordées.

- `block.splice` reçoit `parent`, `index`, `delete_count` et `nodes`. Le planificateur
  de l’éditeur conserve les règles d’insertion validées (division au curseur,
  absence de ligne vide avant, ligne de reprise après) puis traduit uniquement
  la suite de blocs changée. Préfixe et suffixe inchangés conservent leurs objets
  Yjs/Yrs. Le cœur répare le conteneur après le remplacement complet, sans ligne
  supplémentaire issue d’un état intermédiaire vide. Les frontières de cellules,
  rôles de tableau et grammaires de conteneurs restent protégés.
- `inline.replace` reçoit une cible textuelle dans une seule zone riche et un
  nœud `footnote` ou `citation`. Les frontières UTF-16 sont contrôlées avant toute
  modification. Pour insérer un badge au milieu d’un Y.XmlText, seul son suffixe
  est reconstruit ; paragraphe, autres badges et éléments voisins sont conservés.
  Les marques du nœud sont explicites. `text.delete` supprime le badge sur sa plage
  d’une unité ; il n’efface pas le contenu des annotations voisines.
- `node.set_attrs` reçoit une cible d’arbre et les paramètres modifiés. Les noms,
  types simples, bornes et énumérations viennent du catalogue des capacités ;
  les paramètres de structure ou de contenu externe ne deviennent pas des
  attributs arbitraires. La validation finale contrôle les ressources et les
  citations bibliographiques. Ni `__marks`, ni les rôles de cellules, ni des clés
  privées ne peuvent être introduits par cette commande. L’image et sa légende,
  ou le badge et ses marques, conservent leurs identités.

Les bandeaux gardent leur brouillon pendant la préparation et en cas de refus ;
remplacer le bandeau annule sa préparation et ne ferme jamais le nouveau panneau.
une confirmation est une étape d’annulation, sans relais brut supplémentaire.
Le curseur est rétabli au moment d’installer la réponse, avant les modifications
distantes suivantes. Après validation d’un badge cliqué, il revient à sa droite :
le badge reste actif pour Ctrl/Cmd+Entrée et Retour arrière ouvre sa suppression.
Un appel depuis le texte conserve sa sélection ; modifier les paramètres ne doit
pas faire reculer le curseur sur le caractère précédent ni sortir d’une légende.
Les contrôles bibliographiques appartiennent désormais à
la même validation finale que les autres commandes, sans seconde édition locale
après une prévalidation indépendante.

La frappe, les retours, les règles « - espace » et « 1. espace », le collage et
les jointures d’items au clavier restent immédiats via le relais validé. Cette
exception est volontaire : leur migration éventuelle vers des plans locaux
communs ne doit pas introduire un aller-retour avant l’affichage de la saisie.
Les tests de collaboration attendent la confirmation du bandeau avant d’enchaîner
une seconde commande ; les tests de concurrence vérifient séparément son refus.

## Recherche/remplacement et corrections raccordés dans l’application

`text-edits.ts` traduit les remplacements de recherche et les suggestions
orthographiques par la même voie. Les opérations explicites utilisent
`text.insert` avec styles explicites, ou `text.delete` si le remplacement est
vide. Le texte est littéral, même s’il contient des caractères de balisage.
La recherche reste exacte, insensible à la casse par défaut et sans regex.

Un remplacement ne traverse pas de zone, de retour ou de badge. Les plages sont
vérifiées contre leur texte courant, puis ordonnées de la fin vers le début
pour conserver leurs offsets UTF-16 dans le candidat. « Tout remplacer » publie
un seul lot et forme une seule étape d’annulation ; un refus ne publie aucune
occurrence. La limite actuelle du moteur reste de 256 commandes par lot : au-delà,
la préparation échoue intégralement, sans fractionnement ni modification partielle.

`applyPlannedEdit` est commun aux widgets et aux remplacements : préparation,
publication gardée, rétablissement de la sélection et du focus. La projection
locale sert à prévoir la sélection après l’édition ; seul le delta confirmé du
propriétaire est installé. Cette projection répare les paragraphes vides selon
le profil de la zone avant de rétablir le curseur, notamment après suppression
du premier paragraphe. Les formulaires détachés gardent leur édition locale.
Un changement de document, de sélection, de requête ou de remplacement pendant
la préparation annule l’opération. La fermeture ou le remplacement d’un bandeau
ne permet pas à sa réponse tardive de fermer le suivant. Un refus conserve les
champs de recherche et le bandeau utilisables, sans mettre la fenêtre en quarantaine.

`spelling.ignore` reçoit `target`, `language` et `word`. Le cœur vérifie la plage,
le mot attendu et la langue courante prise en charge. Il ajoute une exception
normalisée par `spelling::word_key`, sans modifier le texte ni les autres
exceptions ; répéter cette action ne crée pas de révision. Cette édition est
annulable dans l’historique du participant. Le dictionnaire personnel reste une
préférence globale, extérieure aux commandes documentaires. L’analyse et les
suggestions continuent à utiliser Spellbook et les dictionnaires existants.

La recette du lot vérifie les remplacements multizones, les styles, Unicode,
les badges préservés et les résumés Article sans recréation de leurs fragments.
Elle couvre une annulation/rétablissement du lot, deux fenêtres synchronisées,
la conservation d’une saisie concurrente, le refus sans publication partielle,
le remplacement d’un bandeau en cours de préparation, et le curseur après
suppression d’un premier paragraphe devenu vide. Ces vérifications ne produisent
pas de captures d’écran.

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

## En-tête raccordé dans l’application

Les champs simples de l’en-tête utilisent `metadata.set`. Leur saisie est
immédiate dans le contrôle natif ; `metadata-commands.ts` conserve la valeur locale
tant que le propriétaire ne l’a pas confirmée. Les valeurs intermédiaires en
attente d’un même champ sont regroupées, sans temporisation de frappe ajoutée.
Le delta confirmé est installé dans Yjs sans second relais brut.

`metadata-input.ts` partage le suivi du champ, la conservation du focus et du
curseur, l’isolation de la composition IME et le traitement d’un refus. Une
préédition ne devient pas une commande. Une valeur refusée reste visible :
l’utilisateur peut la valider de nouveau explicitement ou l’annuler pour revenir
à la valeur confirmée. Elle n’écrase pas silencieusement une édition distante.
La sauvegarde et l’export attendent les éditions en cours ; ils sont refusés si
un champ attend encore une décision ou une validation IME. Sur un champ refusé,
Entrée permet de valider de nouveau et Échap de revenir à la valeur confirmée,
sans fermer tout l’en-tête. Les actions utilisent la navigation clavier commune.
La récupération automatique protège l’état confirmé sans attendre la décision
sur ce champ. L’export JSON d’un brouillon refusé ajoute `headerDrafts` (cibles
et valeurs en attente) au document Yjs ; ces valeurs ne sont pas introduites dans
l’archive confirmée. Leur conservation automatique durable et leur réimportation
restent à réaliser dans A.3.

Les formulaires privés d’auteurs, d’affiliations et de localisations préparent un
plan de commandes avant de publier un seul lot. L’ajout utilise `record.add` ;
les changements scalaires utilisent `metadata.set`, et les champs riches modifiés
une opération `block.splice` sur leur fragment existant. Les champs inchangés
ne sont pas remplacés. Un changement concurrent du même enregistrement conserve
le formulaire et signale le refus ; fermer le formulaire annule sa préparation.

Les boutons de déplacement produisent `record.move`, avec des ancres d’identifiant.
Une suppression avec retrait des associations produit les `metadata.set`
nécessaires suivis de `record.remove` dans un seul lot. Une seule annulation
restaure l’enregistrement et ses références. Importer, remplacer ou retirer un
fichier bibliographique utilise également ces commandes ; sa vérification
préalable lisible ne remplace pas la validation finale commune du candidat.

Les onglets conservent leurs boutons pendant une mise à jour documentaire ; une
confirmation asynchrone ne recrée pas le contrôle qui porte le focus clavier.
Les fonctions locales d’initialisation et de manipulation des brouillons ne sont
pas utilisées pour publier ces éditions d’en-tête ; les anciennes méthodes de
publication directe, de déplacement et de suppression de collections ont été
retirées. Sommaire et style bibliographique utilisent aussi `metadata.set`.
Les options de page/langue, le changement de modèle et les ressources rejoignent
également les commandes communes, selon le contrat ci-dessous.

Recette du lot : 186 tests web unitaires et trois contrôles des langues natives,
17 contrats Rust ↔ Yjs/Tiptap, tests du workspace Rust et Clippy. Les parcours
Chromium/WebKit couvrent les en-têtes, la concurrence, la conservation des
brouillons, l’IME, les changements de modèle et les commandes voisines. Les
tests de formulaires attendent la confirmation asynchrone et la fin du démarrage
de la fenêtre avant de lire l’état diagnostique.

## Ressources, options et modèles raccordés dans l’application

`metadata.set` accepte une cible `{"kind":"option","name":"paper"}` et une valeur
JSON contrôlée par les types Rust : `paper`, `landscape`, `language`,
`bullet_marker` et `spelling_ignored`. Les sélecteurs de l’onglet Général utilisent
le même gestionnaire de saisie locale, de confirmation, de refus et de décision
explicite que les champs d’en-tête. Chaque champ possède son périmètre clavier ;
les touches natives des listes déroulantes restent disponibles. Les actions de
revalidation/annulation s’atteignent par Tab et les flèches de leur groupe.
Un changement de langue crée seulement les enregistrements linguistiques manquants,
dans le même lot annulable ; il ne remplace ni les titres ni les résumés existants.

`document.set_model` reçoit `model_id` et `source`. Il modifie la base scalaire du
modèle sans réinitialiser l’identité du document, les collections, leurs textes
partagés ou les fragments riches. Le contrat Typst et les dépendances contrôlées
sont vérifiés hors ligne par compilation du candidat. Cette vérification s’applique
aussi à un changement de modèle issu d’une mise à jour Yjs brute ou de l’historique.
Les options conservées ne sont pas réinitialisées ; les éventuels réglages de page
imposés par un modèle fourni exact sont appliqués avec lui. L’interface des futurs
régimes libre/restreint/imposé reste dans B : ce lot ne la déclare pas achevée.
Le changement de modèle entre dans l’historique commun comme une seule opération.

`prepare_image` est une préparation pure, commune au profil global et au document :
lecture par l’adaptateur, décodage par le cœur et chemin déterministe. Les octets
circulent en base64, avec limite vérifiée avant décodage ; ils ne sont pas placés
dans Yjs. Cette préparation n’ajoute rien aux ressources du propriétaire et ne
diffuse aucun événement. Le profil reste global jusqu’à son incorporation explicite
ou à la création d’un courrier qui reçoit une copie autonome de sa signature.

L’insertion d’image publie `resource.add` avec le plan structurel ; le remplacement
la conserve dans le bandeau privé jusqu’à Valider et publie `resource.add` avec
`node.set_attrs`. La signature utilise `resource.add` puis
`document.set_signature` dans le même lot. Le candidat garde sa propre copie du
répertoire de ressources, qui n’est admise qu’après la dernière vérification de
la garde. Un chemin existant ne peut recevoir d’autres octets ; réadmettre la même
image est sans effet. Refus, garde périmée et annulation du bandeau ne laissent
aucune image importée dans la session confirmée.

La publication retourne les nouvelles ressources ; les transports les livrent
avant la mise à jour documentaire. Les événements sont filtrés par génération,
y compris pendant le rattachement d’une fenêtre. Le destinataire conserve seulement
le chemin et les données, sans recopier l’enveloppe de transport dans les fichiers.
Les ressources admises restent disponibles pendant la séance pour annuler/rétablir.
La sauvegarde et l’export continuent de filtrer celles requises par l’état courant
et son modèle ; les snapshots durables restent à définir dans A.3/A.4.

Recettes : import annulé sans ressource diffusée, signature du profil et du document,
refus concurrent de sélecteur avec revalidation au clavier, changement de modèle
avec conservation des fragments Article, une annulation par lot, sauvegarde et
réouverture, ressource utilisée seulement par le modèle et compilation de l’archive
hors ligne. Les contrats Rust ↔ Yjs couvrent également langue/modèle et signature.

Validation locale du lot : tests du workspace Rust (354 tests du cœur),
187 tests web et trois contrôles des langues natives, 19 contrats Rust ↔ Yjs,
68 parcours Chromium/WebKit, compilation web/native, Clippy et contrôles de format.
Aucune capture, vidéo ou trace n’est produite. Le contrôle de performance ignoré
reste un diagnostic séparé ; cette recette ne fixe pas de nouveau budget de latence.

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

- La disponibilité est désormais raccordée à l’interface. La façade commune
  et les adaptateurs CLI/MCP utilisent désormais les commandes, requêtes et
  contextes structurés ; voir le [bilan du lot](structured-adapters.md).
  Les commandes de style à cible explicite refusent toujours une plage vide ;
  les styles de frappe appartiennent à l’API de contexte.
- Les transformations supplémentaires. La [fusion de paragraphes](paragraph-join.md)
  est commune au moteur et à l’éditeur, y compris la restauration du curseur. L’insertion,
  la suppression de lignes/colonnes et les en-têtes de tableaux sont désormais
  communs ; la fusion de cellules reste différée.
  Les retours riches passent par `block.split` ou des nœuds de retour explicites,
  pas par des sauts de ligne cachés dans `text.insert`.
- La conservation durable automatique et la réimportation des brouillons refusés
  relèvent désormais d’A.3.
  Le groupement de frappe, l’IME, l’historique natif par fenêtre et le parcours
  export/abandon sont raccordés dans Tauri.
- Le mode individuel allégé, avec le même propriétaire documentaire.

# Commandes multizones et opérations structurelles — A.2

Ce lot prolonge le [socle Yrs](structured-contract-foundation.md) selon les
[décisions validées](structured-contract.md). Il ajoute une API Rust commune,
testée avec le véritable éditeur ; il ne raccorde pas encore les adaptateurs
de production à cette API et ne clôture pas A.2.

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

- Les modes de frappe des participants. Une commande de style sur une plage
  vide retourne actuellement `empty_selection`.
- Les opérations fines sur les lignes/colonnes des tableaux, la fusion de
  paragraphes et les transformations supplémentaires. Un tableau entier est
  déjà insérable/remplaçable ; les éditions textuelles de ses cellules sont communes.
  Les retours riches passent par `block.split` ou des nœuds de retour explicites,
  pas par des sauts de ligne cachés dans `text.insert`.
- Les ressources, options et changements de modèle, les autres commandes et requêtes.
- L’historique commun côté moteur. La recette montre qu’un lot importé dans
  Yjs peut former une seule étape ; elle ne remplace pas la migration des piles
  des participants ni le parcours d’acceptation des propositions.
- Le raccordement du relais de frappe, les accusés/refus et brouillons, le
  mode individuel à la demande, puis les adaptateurs CLI et MCP.

# Cibles multizones et premières commandes communes — A.2

Ce lot prolonge le [socle Yrs](structured-contract-foundation.md) selon les
[décisions validées](structured-contract.md). Il ajoute une API Rust commune,
testée avec le véritable éditeur ; il ne raccorde pas encore les adaptateurs
de production à cette API et ne clôture pas A.2.

## Contrat exécuté

Les types publics sont dans `content::commands`. Le schéma JSON de `Command`
est produit par Schemars depuis les types réellement désérialisés. Les champs
inconnus et les offsets négatifs sont refusés. `specifications()` annonce le
périmètre et les identifiants exécutables ; `content::catalogue::describe`
annonce également le support `shared_core`, avec sa limite aux plages explicites.
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

## Préparation et publication

`SharedSession::prepare_commands(guard, commands)` prépare un candidat isolé.
La garde associe identité du projet et révision. Chaque étape vise l’état privé
résultant des précédentes ; les chemins des étapes suivantes doivent en tenir
compte. Aucun réancrage implicite n’est effectué.

Les opérations modifient les objets partagés en place : paragraphes, cellules,
fragments riches, textes d’enregistrements et annotations conservent leurs
identités. Les racines privées de la liaison Tiptap sont conservées. Le
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
Elles vérifient les références des objets partagés, les styles sur les badges,
les métadonnées et une seule étape d’annulation/rétablissement. Le transport de
diagnostic reste réservé aux tests ; ce n’est pas une nouvelle voie publique MCP.

Restent dans A.2 :

- Les modes de frappe des participants. Une commande de style sur une plage
  vide retourne actuellement `empty_selection`.
- Les éditions structurelles de blocs, paragraphes et tableaux. Les retours à
  la ligne insérés dans une zone riche retournent `structural_edit_required`
  dans ce premier lot ; les textes bruts conservent leurs retours littéraux.
- L’ajout, la suppression et le déplacement des enregistrements et ressources,
  les options et changements de modèle, les autres commandes et requêtes.
- L’historique commun côté moteur. La recette montre qu’un lot importé dans
  Yjs peut former une seule étape ; elle ne remplace pas la migration des piles
  des participants ni le parcours d’acceptation des propositions.
- Le raccordement du relais de frappe, les accusés/refus et brouillons, le
  mode individuel à la demande, puis les adaptateurs CLI et MCP.

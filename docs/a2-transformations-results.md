# Transformations R1–R3 — A.2

Livraison du **30 octobre 2026**, après l’[inventaire](a2-transformations-inventory.md).
Choix confirmés : Suppr joint les items symétriquement à Retour arrière ; une
édition traversant des zones modifie leurs contenus sans supprimer leurs frontières.
Ces lots ne clôturent pas A.2 : restent le mode individuel allégé et la recette de clôture.

## R1 — Retours en ligne

`inline.replace` accepte désormais `softBreak` et `hardBreak`, en plus des notes
et citations. Exemple :

```json
{
  "id": "inline.replace",
  "params": {
    "target": { "zone": { "kind": "body", "path": [0] }, "from": 4, "to": 4 },
    "node": { "type": "softBreak" }
  }
}
```

Le retour simple est une espace dans Typst ; le retour forcé produit une rupture
de ligne. Une séparation de paragraphes reste `block.split`. Aucune interprétation
implicite de CR/LF n’est ajoutée à `text.insert`.

L’insertion conserve le paragraphe, son préfixe partagé et les autres éléments ;
seul le suffixe du texte coupé peut devoir être recréé pour accueillir le nœud.
Corps, cellules, légendes, attribution et résumés utilisent la même commande.
Les champs bruts et les titres sur une seule ligne refusent les retours. La
validation du titre vérifie également les remplacements complets de sous-arbres :
une autre commande ne doit pas contourner cette restriction.

## R2 — Items non imbriqués

| Commande | Contrat |
| --- | --- |
| `list.item.split` | Cible textuelle dans un paragraphe d’item ou une définition. Remplace la plage sélectionnée et crée un item avec le suffixe et les paragraphes suivants. Le nouvel item de description a un terme vide. |
| `list.item.join` | Deux items ordinaires voisins d’une même liste. Concatène le dernier paragraphe gauche et le premier droit, sans espace ou retour ajouté ; conserve les autres paragraphes. Un item droit entièrement vide est retiré. |
| `list.item.exit` | Retire un item entièrement vide et crée ou réutilise la ligne de reprise après la liste. Retire la liste entière seulement si cet item était le dernier. |

Les cibles et plages sont celles du contrat structuré ; le catalogue et ses
exemples proviennent du cœur, pour CLI et MCP également. Le MCP rattaché garde
sa limite au corps et ses propositions soumises à acceptation humaine.

Le clavier utilise un planificateur commun dans `list-transforms.ts`. Son plan
local est affiché immédiatement, puis confirmé par Rust, sans RPC bloquant avant
la saisie. Les recettes croisées exécutent les commandes équivalentes sur le même
état initial. L’item gauche, ses paragraphes conservés, son préfixe et les items
voisins gardent leurs identités. Les contenus déplacés entre parents peuvent être
recréés, comme dans la séparation de paragraphes.

- Entrée sépare l’item au curseur ; depuis le terme d’une description, elle
  rejoint d’abord sa définition. Le nouvel item de description commence par son terme.
- Retour arrière au début de l’item droit et Suppr à la fin du gauche ont le
  même résultat. Dans un item comportant plusieurs paragraphes, les frontières
  internes gardent les règles de `block.join`, avec retour simple entre paragraphes.
- Terme et définition restent séparés ; la jointure d’items de description est
  refusée. Le premier item revient au texte précédent sans déballer son widget.
- Un item vide ne contient aucun contenu en ligne. Une note, même avec un texte
  facultatif vide, est du contenu : elle ne doit pas disparaître lors d’une sortie.

Les listes imbriquées restent hors périmètre.

## R3 — Entrées natives et frontières

`SelectionEdits` est commun aux profils riches. Il distingue une sélection dans
un même flux de paragraphes d’une sélection traversant des zones sémantiques :
corps/attribution, terme/définition, items ou cellules, par exemple.

Pour une sélection traversant des zones :

- suppression et coupe retirent uniquement leurs contenus sélectionnés ;
- saisie et collage insèrent le remplacement une seule fois dans la première
  zone, en supprimant les autres portions sélectionnées ;
- les paragraphes du contenu collé deviennent des retours simples quand il faut
  l’adapter à une zone en ligne ; styles et badges compatibles sont conservés ;
- un widget entièrement compris dans la sélection peut être retiré ; un widget
  partiellement sélectionné conserve son enveloppe et ses champs ;
- la coupe remplit les formats HTML/texte du presse-papiers avant de supprimer.
  Sans accès aux données du presse-papiers, aucune suppression n’a lieu ;
- un déplacement interne applique cette suppression puis insère le contenu en
  ligne au point de dépôt, en une transaction. Sans point de dépôt éditable,
  il ne retire rien à la source.

La sélection rectangulaire a une adaptation de contenu par cellule : effacer
vide les cellules sans retirer la grille ; coller distribue les contenus dans le
rectangle, avec répétition ou découpage comme la politique de ProseMirror. Les
dimensions, identités et rôles d’en-tête des cellules et la légende sont conservés.
Les commandes équivalentes sont des `block.splice` dans les cellules, pas un
remplacement de tableau. Aucune fusion de cellules n’est ajoutée.

Le collage dans un titre riche adapte les séparations à des espaces dans son
unique paragraphe. Les champs d’en-tête restent des éditeurs distincts. La saisie
ordinaire et les sélections entre paragraphes d’un même flux gardent leurs
gestionnaires habituels ; Ctrl/Cmd+Entrée reste contextuel.

Les plans de suppression/remplacement ont un lot typé équivalent, vérifié contre
Rust. `dispatchImmediateEdit` associe le contexte précédent au delta immédiat ;
il est aussi partagé par les jointures de paragraphes et les opérations de liste.
L’historique reste celui du propriétaire, sans nouvelle pile d’annulation JavaScript.

La composition multizone prépare une seule zone d’insertion après le début du
tamponnage natif. Sa préédition n’est pas diffusée. Une composition annulée sans
insertion restaure la sélection et le contenu préparé avant la confirmation.
Une ressource absente ne déclenche pas d’import implicite : le propriétaire refuse
le delta, conserve le document confirmé et laisse le brouillon exportable.

## Vérifications

- Rust : retours, restrictions des titres, items, Unicode, marques/badges,
  gardes, refus atomiques et identités conservées ; catalogue et adaptateurs.
- Contrats Rust ↔ Yjs/Tiptap : retours dans plusieurs zones, séparation/jointure/
  sortie d’items, remplacement multizone et collage rectangulaire ; annulation.
- Navigateurs : vrais événements DOM de coupe, collage, saisie et déplacement,
  sélections rectangulaires, styles, rôles d’en-tête, titres, conservation des
  brouillons refusés, IME simulée et annulation entre fenêtres.

Ces recettes ne sont pas des captures et ne simulent pas un remplacement du
contenu via `setContent` pour tester le presse-papiers. Elles exercent les événements
`ClipboardEvent` et `DragEvent` et le traitement ProseMirror réel. La méthode
de saisie physique et le presse-papiers système des trois OS restent à vérifier
manuellement ; un événement IME simulé ne remplace pas ces essais.

Résultat local : **292 tests Rust**, **311 tests web**, **3 contrôles de locales**
et **29 recettes Rust ↔ Yjs/Tiptap** réussis. Clippy workspace/toutes cibles sans
avertissements, compilation web, formatage, frontières de dépendances et taille
des modules réussis. Le diagnostic de performance habituel reste ignoré.

La passe étendue a exercé **42 parcours Chromium/WebKit**. Quarante ont réussi
dans cette passe ; une préparation de fenêtre déconnectée pendant les rechargements
de développement et une recette qui appelait `flush` avant la fin asynchrone de
l’IME ont été reprises. La recette attend désormais explicitement la fin de
composition. Les **8 parcours ciblés rejoués** (jointures et IME sur les deux
moteurs) réussissent. Le parcours de collage multizone avec annulation partagée
rejoint également les tests rapides `@smoke`.

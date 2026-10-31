# Clôture A.2 — Courrier / Article

**31 octobre 2026 — A.2 clos.** Le même projet circule entre Tauri, CLI,
MCP rattaché, MCP autonome puis Tauri, sans conversion de contrat ni
reconstruction du document partagé. La recette complète a réussi sur macOS,
avec la véritable WebView WebKit et le frontend de production embarqué.

Le [rapport brut](../experiments/a2-closure/report-macos.json) vient du commit
`be18a8c`. Les fichiers de travail restent dans
`output/a2-closure/run-1793404149288/` (non versionné). Aucun jeton d’invitation
n’est copié dans le rapport versionné. Aucune capture, vidéo ou trace graphique.

## Corpus et parcours

| Projet | Contenu exercé | Résultat final |
| --- | --- | --- |
| Courrier | En-tête, gras, P.-S., pièces jointes mentionnées, image/légende/alternative, signature | 2 pages ; 2 images conservées |
| Article | Auteurs/affiliations, résumés multilingues riches, citations/bibliographie, sommaire, note, citation en bloc, tableau/légende, image | 3 pages ; 1 image conservée |

Le courrier part de l’exemple public `examples/cli/courrier.json` ; l’article
part d’une copie de `examples/article-working.typsto`. Les exemples originaux
restent intacts. Une ressource supplémentaire inutilisée est admise en mémoire,
puis vérifiée absente de l’archive et des exports.

Pour chaque type :

1. La CLI prépare le projet ; Tauri l’ouvre par le parcours de fichier natif.
   La saisie modifie l’en-tête et le corps. Annuler/rétablir conserve les données.
2. Un vrai client MCP STDIO rejoint l’application sous invitation. Une proposition
   est refusée par le bouton de l’interface, une autre devient périmée après une
   saisie humaine, puis un lot texte + note est accepté par le bouton de relecture.
   Refus et péremption ne publient rien. Le lot accepté conserve la sélection
   humaine et s’annule/se rétablit en une étape complète.
3. Le projet est sauvegardé puis exporté depuis Tauri. La CLI lit exactement
   l’état confirmé et produit le même export source. Un lot CLI refusé ne change
   pas les octets de l’archive ; `run` sans sauvegarde reste un aperçu. Un lot
   valide modifie le corps et l’en-tête ; l’article reçoit aussi une ligne de tableau.
4. Le MCP autonome ouvre ce fichier avec des racines explicites. Son contexte
   privé ne modifie ni état ni révision. Les lots invalides, références périmées,
   écritures CLI concurrentes et exports hors racines sont refusés. Il modifie le
   texte et une colonne de tableau dans l’article, vérifie annuler/rétablir,
   compile, rend une page PNG, sauvegarde, exporte puis ferme le projet.
5. Tauri rouvre ce dernier fichier. L’état documentaire complet est identique à
   celui sauvegardé par le MCP. L’ancre Yjs du premier bloc resté intact conserve
   son identité. Une nouvelle sauvegarde et un nouvel export réussissent.

Les quatre adaptateurs découvrent le même catalogue de **71 commandes/requêtes**
avec les mêmes signatures. Le MCP rattaché expose 22 outils et ne dispose pas de
sauvegarde, export ou acceptation. Les propositions d’en-tête y restent refusées :
la relecture actuelle ne présente que le corps, les cellules et les annotations.
Une connexion MCP n’active pas les services de présence humaine.

## Comparaisons et contrôles

La recette compare les états complets, les manifestes et les octets de tous les
fichiers exportés hors PDF : contenu, modèle, métadonnées, capacités, dépendances,
bibliographie et images. Cela représente **791 fichiers pour le courrier** et
**13 pour l’article**, à chacune des trois comparaisons d’exports. Les PDF sont
vérifiés comme tels ; leurs octets ne sont pas comparés à cause des dates de
création. Les compilations partagent le nombre et les dimensions des pages
(tolérance 0,01 pt pour le passage natif f32/f64). Les rendus CLI et MCP sont de
véritables PNG. Ce contrôle fonctionnel ne constitue pas une recette visuelle.

En complément, les deux exports `main.typ` ont été recompilés avec **Typst CLI
0.15.1**, sans accès réseau accordé, avec des caches de paquets vides et sans
recherche de polices système (`--features a11y-extras --ignore-system-fonts`).
Les dépendances locales exportées suffisent à ces deux documents.

Vérifications complémentaires :

- **299 tests Rust** réussis, doctest inclus ; un diagnostic de performance
  volontairement ignoré ;
- **321 tests web + 3 contrôles des langues natives** réussis ;
- **29 recettes de contrat Rust ↔ Yjs/Tiptap** réussies ;
- **60 parcours Chromium/WebKit ciblés** réussis : 58 à la première passe,
  puis les 2 contrôles de cellule repris après correction de leur assertion
  (le libellé du curseur distant ne fait pas partie du texte documentaire) ;
- build TypeScript/Vite, Clippy du workspace avec `recipe-driver` et de
  l’application sans cette feature, formats, frontière du cœur, taille des modules
  et **15 tests des gardes d’architecture/CI** réussis.

## Écarts corrigés pendant la recette

- L’exemple CLI de courrier utilisait encore des opérations de l’ancien contrat.
  Il utilise maintenant les cibles et blocs explicites ; un test exécute le fichier
  public lui-même jusqu’à sa préparation pour l’export.
- Une ouverture/fermeture pouvait conserver la présence activée du document
  précédent. Le changement de génération remet le mode individuel et sa version
  à zéro, sans perdre les fenêtres inscrites. Un refus de fermeture conserve le
  document et son mode actuels.
- L’exemple public Image annonçait `placement`, absent du contrat et de l’éditeur.
  Il a été corrigé ; les attributs de nœud non déclarés sont maintenant refusés
  dans la validation commune avant publication, y compris dans un lot générique.
- Les options d’en-tête du tableau étaient matérialisées comme attributs par
  l’éditeur, alors que leur valeur se trouve dans les rôles des cellules. Le
  contrat indique désormais `x-editor.storage: "structure"` : le panneau reste
  disponible, sans deuxième état sérialisé ni contournement par `node.set_attrs`.

Les contrôles de cohérence de sauvegarde restent stricts : aucune normalisation
spéciale dans le banc d’essai ne masque ces écarts.

## Rejouer

Depuis la racine, après installation des dépendances existantes :

```sh
npm run build:web
cargo build --locked --offline -p typstographe -p typstographe-cli --features typstographe/recipe-driver
npm run test:closure-native
```

La commande crée un répertoire neuf sous `output/a2-closure/`, lance les processus
natifs successivement et écrit `report.json` ainsi que les projets/exports.
Un affichage natif actif est nécessaire. Sur macOS, empêcher la veille pendant
la recette peut être utile. Pour conserver des binaires dédiés pendant d’autres
compilations, utiliser `TOG_SPIKE_BINARY` et `TOG_CLI_BINARY` avec leurs chemins
absolus. Un répertoire de sortie absolu et vide peut être fourni en argument.

La recompilation indépendante, facultative pour rejouer la recette automatisée,
utilise un Typst CLI installé séparément :

```sh
mkdir -p /chemin/vers/recette/empty-packages
typst compile --features a11y-extras --ignore-system-fonts \
  --package-path /chemin/vers/recette/empty-packages \
  --package-cache-path /chemin/vers/recette/empty-packages \
  /chemin/vers/recette/article/cli-final-export/main.typ \
  /chemin/vers/recette/article/standalone-export.pdf
```

## Limites conservées et suite

Les saisies sont déclenchées par les véritables contrôles de l’interface et les
commandes Tiptap, sans clavier physique. Cette passe ne revendique ni test IME
physique, ni menu de fichier système, ni exécution native sur Windows/Linux.
Les parcours navigateur couvrent Chromium/WebKit ; la CI reste le contrôle
multisystème distinct. Aucun nouvel essai avec un LLM externe n’a été réalisé.

La clôture porte sur A.2. **A.3** reste à préparer ensemble : conservation durable
des brouillons refusés et des champs en attente, récupération et conflits de
fichiers. Les snapshots (A.4), la collaboration réseau (G), les cellules
fusionnées et listes imbriquées (C) restent hors de ce lot.

## Ajustement des recettes après la CI de la PR nº 3

La [CI du 31 octobre](https://github.com/klutchnikoff/typstOgraphe/actions/runs/41147750004)
a validé Rust, les tests web, les contrats et les audits, puis relevé deux
scénarios obsolètes dans les deux moteurs navigateur : l’ajout d’une ligne était
encore cherché dans les paramètres du tableau ; le libellé du curseur distant
était compté comme du texte de citation. Le scénario utilise désormais la poignée
de ligne et son action clavier. L’assertion commune `expectEditorText` compare
exactement le texte rendu en retirant uniquement les widgets décoratifs
ProseMirror d’une copie du DOM, sans modifier le DOM réel ni retirer un nom ou
une portion arbitraire du texte documentaire. Elle est partagée par les recettes
multizones et d’en-têtes de tableau.

Validation locale de cet ajustement : **18 scénarios critiques**, **16 autres
parcours multizones** et **2 parcours d’en-têtes de tableau**, tous réussis dans
Chromium/WebKit ; TypeScript et formatage réussis. Les workflows de CI et le
comportement de l’application ne sont pas modifiés par ce correctif de recettes.

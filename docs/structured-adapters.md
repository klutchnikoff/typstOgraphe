# Adaptateurs structurés — A.2, 29 octobre 2026

## Une seule édition documentaire

`content::api` est la façade sans interface de `EditingSession` : catalogue,
requêtes, création/ouverture, conservation et export. La CLI et le serveur MCP
s’y raccordent ; leurs sources n’utilisent plus le moteur de projection plate,
`StoredDocument`, le parseur de source ou un interprète indépendant des widgets.
Les restrictions du catalogue de capacités par type sont contrôlées dans le cœur,
y compris pour un nœud générique ou une archive : un P.-S. ne peut pas contourner
le type Article en utilisant `block.insert`.

Le moteur historique reste dans le cœur avec ses tests et ses catalogues archivés,
mais il ne sert plus de chemin d’exécution à ces adaptateurs.

Tauri conserve son propriétaire vivant et ses contextes par fenêtre. Le MCP
rattaché prépare ses appels dans ce même propriétaire. Les sélecteurs sont résolus
sur le brouillon isolé, puis passent aux mêmes `commands::Command` typées. La
validation porte sur l’état final : aucun canal ne publie une étape intermédiaire.

Les commandes textuelles reçoivent une zone et une plage UTF-16 ; les commandes
structurelles reçoivent une racine et un chemin. Les enregistrements d’en-tête ont
des identifiants stables. Les chemins doivent être relus après une modification.
Une sélection privée n’est ni une édition ni une cible documentaire persistante.

## Découverte et différences d’adaptateurs

Les schémas des éditions proviennent de l’énumération exécutée. Les signatures des
requêtes et du cycle de vie sont regroupées dans la façade. Les validateurs des
paramètres sont mis en cache, hors chemin de frappe locale. Le serveur rattaché
amorce cette validation avant de prendre le verrou du document vivant. Les schémas MCP
ajoutent l’enveloppe de référence ; ils sont vérifiés aussi à l’exécution.

Les capacités sémantiques fournissent des exemples d’étapes : créer une liste ou
une citation en bloc ne demande pas un second interprète par adaptateur. Une
nouvelle capacité doit déclarer son nœud, sa validation et ses exemples avant de
pouvoir être annoncée comme exécutable. Les noms `search.find`, `edit.undo/redo`
et `render.selection_boxes` sont conservés ; leurs cibles sont désormais typées.
Les anciennes positions et références de projection ne sont pas simulées.

| Adaptateur | Édition et contexte | Conservation |
| --- | --- | --- |
| Tauri | Propriétaire commun, contexte privé par fenêtre, frappe confirmée asynchroniquement | Parcours graphique existant |
| CLI | Même propriétaire ; `run` explicite et scripts atomiques ; aperçu par défaut | Verrou avant lecture pour `--save`, copie indépendante pour `--output` |
| MCP autonome | Même façade, références document/révision strictes, contexte privé | Racines lecture/écriture explicites ; sauvegarde/export séparés |
| MCP rattaché | Lecture confirmée ; contexte agent privé ; propositions du corps, cellules et annotations | Acceptation humaine uniquement ; aucune permission de fichier |

En mode rattaché, la préparation ne publie rien. À l’acceptation, le propriétaire
rejoue le lot gardé et produit le delta Yrs confirmé. Le navigateur ne calcule plus
un autre delta à partir de l’aperçu. Le cache d’instantanés transmis au MCP est
supprimé, ainsi que sa copie et sa validation supplémentaire dans le parcours
d’aperçu ; ses lectures utilisent directement le propriétaire confirmé. L’historique de la fenêtre qui accepte reçoit
une seule contribution. Les cibles d’en-tête restent refusées, même si un sélecteur
textuel les trouve : l’actuelle relecture ne présente que le corps.

## Fichiers et ressources

Les archives structurées `.typsto` conservent l’état Yjs et les ressources
nécessaires à leur autonomie. Les ressources admises en mémoire restent
accessibles pour annuler ; seules celles utilisées sont conservées dans l’archive
et l’export. Les modèles sont des sources Typst intégrées au projet.

Une ouverture éditable réserve le verrou avant lecture et vérifie l’empreinte
après chargement. Une lecture seule ne prend pas le verrou d’un écrivain. Une
copie ne remplace jamais implicitement une cible. Un lot peut sauvegarder son
candidat en dernière action ; un échec conserve l’état, les contextes et
l’historique confirmés. L’export multi-fichiers reste une opération séparée.

Les chemins d’import implicites de l’ancien contrat disparaissent : `resource.add`
reçoit les octets validés (base64 sur le fil), `document.set_model` le source
Typst. Seuls les chemins documentaires d’ouverture, sauvegarde et export sont
résolus par les adaptateurs. Les préférences personnelles ne sont pas implicites.

## Recettes et limites

Les tests vérifient les archives structurées, les brouillons incomplets, les
références d’auteurs/affiliations, zones riches, cellules, Unicode et frontières,
styles, ressources/signatures, état modifié après admission d’une ressource,
orthographe selon la langue de chaque résumé, sélecteurs évolutifs, lots refusés, erreurs
localisées, verrous, modifications externes et export hors ligne.

Le client officiel rmcp vérifie le protocole, la découverte jusqu’à l’export d’un
courrier, la concurrence, les permissions et les ressources virtuelles. La
passerelle native et la CLI utilisent le même format d’invitation ; la reconnexion
explicite relit la nouvelle invitation sans déplacer les clients déjà lancés.
La recette Rust ↔ Yjs vérifie aussi l’acceptation d’un lot texte/tableau, les
identités conservées et une seule annulation/rétablissement. Aucun nouvel essai
avec un LLM externe, aucune capture dans ce lot.

A.2 reste ouvert : transformations supplémentaires (dont fusion de paragraphes),
mode individuel allégé et raccordement global de la disponibilité dans l’interface.
La conservation durable des brouillons refusés relève d’A.3. Étendre la relecture
MCP à l’en-tête/aux ressources et réintroduire des droits de fichiers rattachés
demanderont des parcours explicites ; les droits historiques egui ne sont pas
réactivés par cette migration.

Guides courants : [CLI](cli.md), [MCP](mcp.md), [invitation Tauri](mcp-local.md).

### Validation du lot

Recette locale macOS, sans captures :

- `cargo test --locked --offline --workspace` : **456 tests réussis**, aucun
  échec ; le diagnostic de performance natif `profile_preview_pipeline` reste
  explicitement ignoré par défaut, comme auparavant.
- `npm test` : **202 tests web et 3 tests de déclaration des langues** réussis.
- `npm run test:contract` : **20 recettes Rust ↔ Yjs/Tiptap** réussies.
- `npm run test:browser -- tests/browser/editing-owner.spec.ts tests/browser/editing-context.spec.ts` :
  **20 parcours Chromium/WebKit** réussis (contextes, focus, historique, refus et
  compositions simulées).
- Clippy workspace/toutes cibles avec `-D warnings`, rustfmt, Prettier,
  constructions Tauri/production web, frontière des dépendances du cœur et
  limites de taille des modules : réussis ; les **9 tests des garde-fous Python** passent.

La recette native `app/scripts/mcp-recipe.mjs` est aussi passée sans fenêtre :
passerelle MCP STDIO réelle, socket de bouclage et propriétaire Rust via le pilote
de recette, proposition sans publication, acceptation puis annulation/rétablissement.
Le script et son rapport sont conservés localement dans
`output/a2-structured-adapters/` (ignoré par Git). Aucun LLM externe n’est intervenu.

Les catalogues générés sont contrôlés contre la CLI. Les contrats antérieurs de
projection restent dans `docs/archive/`, avec leurs recettes. Ces vérifications
locales ne constituent pas une validation de la CI Windows/Linux distante.

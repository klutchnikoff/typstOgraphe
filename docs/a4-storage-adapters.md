# A.4.2 — CAS dans les adaptateurs et la récupération

Le stockage du [cœur A.4.1](a4-storage-core.md) est désormais le seul format de
production. Tauri et la session commune CLI/MCP conservent le propriétaire
`content::project::Project` ; `Archive` reste sa charge documentaire validée,
sans lecteur ni écrivain de l’ancien format 10.

## Identité, sauvegarde et copie

- Première sauvegarde d’un projet neuf : identité initiale conservée.
- Sauvegarde ordinaire : même UUID, index, versions et références internes.
- Enregistrer sous ou copie indépendante d’un projet déjà enregistré : nouvel
  UUID, mêmes identifiants et métadonnées de versions, sources et ressources.
- Un candidat de sauvegarde n’est publié dans la session qu’après réussite de
  l’écriture atomique et des contrôles existants. Les gardes de fichier, conflits
  de révision et garanties des lots restent en place.
- Un fork CLI/MCP conserve le propriétaire et son origine, sans acquérir le verrou
  du document source. La copie indépendante est matérialisée à sa sauvegarde.

`document.inspect` expose `project_id`. Le catalogue des commandes Versions et
leur interface ne sont pas ajoutés dans ce lot.

## Ressources épinglées

Aperçu, validation de modèle, politique de présentation, sauvegarde et export
utilisent l’inventaire du projet. Une simple édition ne reconstruit pas ses
packages depuis la version installée de l’application. Un changement explicite
vers un modèle fourni résout les dépendances de ce nouveau modèle.
Les tests utilisent un wrapper de dates absent du catalogue, puis vérifient sa
compilation et sa conservation jusque dans l’export hors ligne.

Les contrôles des anciennes archives plates ont été remplacés par ceux du CAS :
intégrité des objets, cohérence des projections et des sources, inventaire des
ressources bibliographiques manquantes, modifiées ou non déclarées. Aucun lecteur
de compatibilité n’est conservé.

## Récupération

Le checkpoint privé `typstographe-web-editor-v2` porte un contexte `project`
obligatoire : archive CAS compressée/base64, origine enregistrée ou non. Il est
autonome après disparition du fichier source. Son historique est mis en cache
par propriétaire/modèle ; les frappes mettent seulement à jour les projections
confirmées et le checkpoint Yjs. La reprise superpose ce dernier état au CAS,
avec ses images courantes, sans changer les versions anciennes.

L’enveloppe JSON est bornée à 1 Gio ; la taille est contrôlée avant lecture.
Les budgets de l’archive, de chaque état et des saisies privées restent distincts.
Formulaires ouverts, champs privés et modifications refusées restent dans leur
enveloppe A.3 ; ils ne sont ni compilés ni ajoutés aux versions confirmées.
La reprise n’ouvre pas implicitement le fichier d’origine sous verrou.

## Exemple

`examples/article-working.typsto` est régénéré depuis le contenu Git antérieur
au lot. La copie locale modifiée par l’utilisateur a été conservée dans
`output/a4-2/`, en version d’origine et en version CAS, avec une trace locale
de provenance. Ces sauvegardes personnelles ne sont pas versionnées.

## Validation

Contrôles locaux macOS, avec dépendances verrouillées et hors ligne :

- `CARGO_INCREMENTAL=0 cargo test --locked --offline --workspace --features typstographe/recipe-driver -j 1` :
  343 tests réussis, un diagnostic de performance explicitement ignoré. Cette
  suite inclut les processus CLI/MCP et leurs sockets locales, les sauvegardes
  atomiques, les conflits, l’export, les formulaires et la récupération A.3.
- Test ciblé de sauvegarde portable, enrichi ensuite : conservation d’une image
  d’ancienne version et d’une référence interne de relecture, sans fichier source.
- `cargo clippy --locked --offline --workspace --all-targets --features typstographe/recipe-driver -j 1 -- -D warnings` : réussi.
- Contrat éditeur web/Yjs/Rust avec l’artefact `recipe-driver` : 29 tests réussis.
- `cargo fmt --all -- --check`, `git diff --check` et
  `python3 scripts/check_file_length.py` : réussis.
- Vérification binaire de l’exemple : document JSON, métadonnées, modèle et Yjs
  identiques à la version Git de départ ; empreinte de la sauvegarde personnelle
  d’origine vérifiée.

La première tentative de contrat utilisait un binaire de lancement ordinaire
sans `recipe-driver`. Le contrôle réussi utilise explicitement l’artefact de
recette. Ces résultats sont locaux ; ils ne valent pas validation CI Windows/Linux.
Aucune capture graphique ni nouvelle mesure de performance n’a été produite.

## Suite

A.4 reste en cours : commandes et interface Versions, comparaison commune,
restauration dans une session active, puis raccordements aux parcours de relecture
et aux propositions MCP selon leurs contrats respectifs.

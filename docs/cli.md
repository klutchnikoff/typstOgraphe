# CLI typstOgraphe

Document généré par `cargo run --locked -p typstographe-cli -- commands list --markdown`.

La CLI permet de créer des documents, consulter le catalogue, inspecter, vérifier et exporter un fichier `.typsto`, sans fenêtre ni préférences personnelles implicites.

```sh
typstographe-cli commands list --json
typstographe-cli commands describe metadata.set --json
typstographe-cli query document_types.list --json
typstographe-cli new --type courrier --output courrier.typsto
typstographe-cli run text.insert courrier.typsto --params '{"text":"Bonjour Élodie."}' --save
typstographe-cli run metadata.set courrier.typsto --params '{"field":"letter.subject","value":"Demande de renseignements"}' --save
typstographe-cli run text.bold courrier.typsto --select 'Élodie' --output copie.typsto
typstographe-cli query document.text copie.typsto --json
typstographe-cli inspect courrier.typsto --json
typstographe-cli check courrier.typsto --for-export
typstographe-cli check courrier.typsto --spelling
typstographe-cli export courrier.typsto --format pdf --output export/
typstographe-cli render courrier.typsto --page 1 --png courrier.png
```

Les lectures utilisent la dernière version enregistrée, même si le document est ouvert dans l’application. L’export crée un répertoire ou utilise un répertoire vide, avec `--pages` pour ajouter des images PNG.

`--lang fr|en` choisit la langue d’interface ; sinon langue système, repli anglais. La langue du document est indépendante. `--json` produit un objet `{"version":1,"ok":true,"outcome":…}` ou `{"version":1,"ok":false,"error":{"code","message","params"}}`. Aucun autre texte sur stdout en mode JSON.

Codes de sortie : 0 succès, 1 contrôle négatif ou opération refusée, 2 arguments invalides, 3 entrée/sortie. Un contrôle négatif a `ok: true` (exécution réussie), `outcome.passed: false` et le code de sortie 1. L’orthographe est facultative ; avec `--spelling`, un dictionnaire indisponible ou une suspicion donne un contrôle négatif.

`new` choisit le modèle automatiquement si le type en propose exactement un ; sinon fournir `--model`. Ses paramètres JSON (`initial_values`, `signature`) se donnent avec `--params` ou `--params-file` (`-` pour stdin). Un objet JSON UTF-8 est requis, limité à 4 Mio. Les fichiers et ressources indiqués dans le JSON sont relatifs au répertoire courant, y compris quand le JSON vient d’un fichier.

`query <id> [fichier.typsto]` exécute une requête du registre. Le fichier est requis pour les lectures documentaires ; la découverte globale et les suggestions sur un mot explicite n’en ont pas besoin. Les réponses brutes de requête sont affichées en JSON lisible, ou dans l’enveloppe stable avec `--json`.

`run <id> fichier.typsto` applique une commande documentaire sur une copie en mémoire et indique explicitement `saved: false`. Le résultat JSON contient le source et les métadonnées résultants pour examiner l’effet. `--save` prend le verrou avant la lecture et enregistre atomiquement dans le fichier source. `--output copie.typsto` crée une nouvelle archive sans modifier la source, même si elle est ouverte dans l’application. Les options sont exclusives et aucune destination existante n’est écrasée. Les commandes de cycle de vie passent par `new` et `export` ; lots, historique et récupération sont indisponibles dans ces appels isolés.

Les paramètres de `query` et `run` utilisent aussi `--params` ou `--params-file`. Une seule source est admise. Les identités de session ne sont pas persistantes : les gardes et références de rendu sont remplies dans le processus courant, les références copiées depuis une exécution précédente sont refusées. Les requêtes de géométrie compilent leur rendu localement, sans produire de fichiers.

Pour cibler le document, `--select TEXTE` recherche un texte exact sensible à la casse. Plusieurs occurrences exigent `--occurrence N`, à partir de 1. `--block N` cible un bloc dans l’ordre de `document.structure`, à partir de 1 ; c’est la cible des commandes d’images. Ces deux sélecteurs sont exclusifs, remplacent la sélection courante et ne se combinent pas avec une cible JSON explicite. Les corrections orthographiques et le remplacement d’une occurrence sont également résolus depuis `--select`. Les cibles symboliques du moteur, comme `{"target":{"kind":"all"}}`, restent utilisables en JSON sans sélecteur. Sans sélecteur, le moteur commence au début du document ; les commandes de style sur un curseur vide n’ont pas d’effet durable sur le texte.

## Scripts JSON atomiques

`script job.json` applique les étapes sur un brouillon privé, puis effectue un seul enregistrement final. `script -` lit le JSON sur stdin. Le format est strict, versionné et limité à 4 Mio et 256 étapes :

```json
{
  "version": 1,
  "document": { "kind": "new", "type": "courrier" },
  "steps": [
    { "id": "text.insert", "params": { "text": "Bonjour Élodie." } },
    { "id": "text.bold", "target": { "select": "Élodie" } },
    { "id": "letter.insert_postscript" },
    { "id": "text.insert", "params": { "text": "Une précision utile." } }
  ],
  "save": { "kind": "as", "path": "courrier.typsto" }
}
```

`document.kind` vaut `open` avec un `path`, ou `new` avec un `type`, un `model` facultatif et les `params` de création (`initial_values`, `signature`). `save` absent ou null signifie aperçu sans sauvegarde ; `{"kind":"current"}` enregistre le document ouvert sous verrou ; `{"kind":"as","path":"copie.typsto"}` crée un nouveau fichier sans écrasement. Un nouveau document exige `as` pour être enregistré.

Les chemins du document, de la destination, des modèles et des images/signatures sont relatifs au répertoire du script canonique. Sur stdin, ils sont relatifs au répertoire courant. Aucun changement du répertoire de travail global n’est effectué. Les répertoires de destination doivent déjà exister.

Les cibles facultatives d’étape sont `{"select":"texte","occurrence":2}` ou `{"block":1}` dans le champ `target`, avec les mêmes règles que `run`. Elles sont résolues juste avant l’étape : une étape peut ainsi modifier du texte ou une image créés plus tôt dans le lot. Sans cible, la sélection laissée par l’étape précédente est utilisée. Les gardes sont calculées sur le brouillon courant ; les références de session externes restent refusées.

Seules les commandes marquées `batch_step` dans le catalogue sont admises. Sauvegarde, export, ouverture, fermeture, création d’un deuxième document, historique, récupération, requêtes et lots imbriqués sont refusés dans les étapes, avant le chargement du document. Les erreurs d’étape donnent `error.params.step` (indice à partir de 0), `command` et l’erreur d’origine ; le message traduit numérote les étapes à partir de 1. Une erreur, même tardive ou pendant la sauvegarde finale, ne publie pas le brouillon et conserve le fichier précédent. Les petits compagnons de verrou peuvent subsister sans verrou actif.

L’exemple `examples/cli/courrier.json` crée un courrier complet avec P.-S. et P.J. Créer le répertoire `output` au préalable, puis exécuter :

```sh
typstographe-cli script examples/cli/courrier.json --json
typstographe-cli check output/courrier-script.typsto --for-export
typstographe-cli export output/courrier-script.typsto --output output/courrier-script-export
```

L’export est une opération distincte après la réussite du script. MCP utilise aussi le moteur commun ; voir les modes [autonome](mcp.md) et
[rattaché en lecture](mcp-local.md).

## Rendu d’une page en PNG

`render fichier.typsto --page N --png page.png` ouvre un instantané enregistré en lecture seule, compile avec le moteur commun et rasterise uniquement la page demandée. Les deux options sont obligatoires ; les pages sont numérotées à partir de 1. Une page hors document est refusée sans créer de fichier. Un brouillon dont les champs ne sont pas encore tous renseignés peut être rendu si sa compilation réussit.

Le PNG utilise la même résolution Typst que `export --pages` (2 pixels par point, soit 144 ppp), sans limite liée aux textures d’affichage egui. Aucun PDF ni répertoire d’export n’est créé. La lecture reste possible avec un écrivain actif, sans inclure ses modifications non enregistrées. Le fichier `.typsto` n’est pas modifié.

Le dossier de destination doit exister. Le PNG est préparé dans un temporaire voisin, puis publié sans remplacement : fichier existant, dossier ou lien symbolique sont refusés, même si la cible apparaît pendant la préparation. Un échec de compilation ou d’écriture ne laisse pas de PNG partiel. Les chemins relatifs sont interprétés depuis le répertoire courant.

Avec `--json`, le résultat contient `path`, `page` et `pages` (numéros/nombre de pages), ainsi que `size_points` (largeur et hauteur en points Typst). Les octets PNG sont dans le fichier, jamais dans stdout. Les erreurs conservent les codes de sortie communs : 1 pour une page inexistante ou une destination occupée, 2 pour l’usage des options, 3 pour une erreur d’entrée/sortie.

## Catalogue du moteur

`typstographe-cli mcp` lance le serveur MCP sur stdio : découverte, éditions
gardées, lots et aperçu PNG. `--read-root` et `--write-root` (répétables) autorisent
les fichiers sous des répertoires existants ; sans racine, aucun accès documentaire
au disque. L’ouverture éditable réserve le verrou commun à l’application ;
sauvegarde et export sont explicites. `--read-only` autorise les requêtes,
l’ouverture en lecture seule et la fermeture. Les capacités sont documentées par
`capabilities.describe` ; les cinq ressources virtuelles incluent le document courant,
le catalogue documenté et les fiches de modèles. Voir [docs/mcp.md](mcp.md).

Le catalogue décrit les contrats du moteur, y compris les opérations qui ne disposent pas encore d’un déclencheur CLI. L’ouverture accepte l’option additive `read_only` (false par défaut).

| Identifiant | Famille | Description |
| --- | --- | --- |
| `document.batch` | batch | Apply bounded editing steps on a private session as one undoable transaction, with an optional single final atomic save. |
| `recovery.restore` | file | Restore a configured recovery reference as an unsaved copy, protecting current work. |
| `recovery.delete` | file | Explicitly delete one configured recovery copy without modifying document history. |
| `document.new` | file | Create an empty document from a supplied type/model and explicit initial values, protecting unsaved work. |
| `document.open` | file | Open an autonomous archive from an explicit path, protecting unsaved work. |
| `document.close` | file | Close the current document after an explicit decision about unsaved work. |
| `document.save` | file | Atomically save the guarded document to an explicit or current path without changing editing history. |
| `document.save_as` | file | Atomically save the guarded document to a required path, preserving the previous file. |
| `document.export` | file | Export PDF, autonomous Typst sources or both into a new or empty directory, with optional page PNGs. |
| `document.set_model` | edit | Validate and install Typst model source or a UTF-8 file with controlled dependencies in one guarded transaction. |
| `courrier.model.personal` | letter | Apply the supplied personal-letter model using the shared guarded model operation. |
| `metadata.set` | edit | Set an active document-type field, with a guarded value and optional typing history grouping. |
| `document.set_option` | edit | Set a typed document option while enforcing the model page and marker policy. |
| `spelling.next` | selection | Select the next spelling issue in the current document, wrapping around. |
| `spelling.previous` | selection | Select the previous spelling issue in the current document, wrapping around. |
| `spelling.correct` | edit | Correct a revision-guarded spelling issue while preserving inline styles. |
| `spelling.ignore` | edit | Accept a validated word for the document language in this document only. |
| `spelling.reset` | edit | Clear spelling exceptions for all languages of the guarded document. |
| `edit.find_next` | selection | Select the next literal match, wrapping around the editable document. |
| `edit.find_previous` | selection | Select the previous literal match, wrapping around the editable document. |
| `edit.replace_current` | edit | Replace a revision-guarded occurrence with expected text, then select the next match. |
| `edit.replace_all` | edit | Replace all literal matches from one guarded document revision in one transaction. |
| `text.insert` | edit | Insert text or replace a selection, with literal input by default and optional typing rules. |
| `text.delete` | edit | Delete text within validated structural boundaries. |
| `text.select` | selection | Select text or move to a body, annotation, description term or definition. |
| `text.bold` | edit | Set or toggle bold text or the typing style. |
| `text.italic` | edit | Set or toggle italic text or the typing style. |
| `text.clear` | edit | Remove inline formatting from text or the typing style. |
| `text.link` | edit | Apply an explicit URL to a text selection. |
| `text.remove_link` | edit | Remove links from a text selection. |
| `block.paragraph` | edit | Convert selected paragraphs to ordinary text. |
| `block.heading_1` | edit | Set level-one headings. |
| `block.heading_2` | edit | Set level-two headings. |
| `block.heading_3` | edit | Set level-three headings. |
| `block.bullet` | edit | Set an unordered list using the document marker policy. |
| `block.numbered` | edit | Set an ordered list. |
| `block.description` | edit | Set description items with term and definition zones. |
| `block.quote` | edit | Set a block quotation with an optional attribution. |
| `insert.image` | image | Insert a validated image with optional literal caption and alternative text. |
| `image.replace` | image | Replace an image resource while preserving its caption and alternative text. |
| `image.remove` | image | Remove an image block and its caption in one transaction. |
| `image.set_alt` | image | Set alternative text on a revision-guarded image block. |
| `document.set_signature` | letter | Set or remove the letter signature using explicit image data or a file. |
| `block.split` | edit | Split a quote, list item or letter note, or exit an empty item or optional annotation. |
| `block.exit` | edit | Create a paragraph after an annotated structure or letter note without replacing its content. |
| `letter.insert_postscript` | letter | Create or select the letter postscript. |
| `letter.insert_attachment` | letter | Insert an attachment mention into a letter. |
| `edit.undo` | history | Undo the previous document transaction. |
| `edit.redo` | history | Redo the next document transaction. |
| `document.validate` | query | Validate a guarded draft, model, resources and options; report errors separately from fields missing for export, without editing or installing a render. |
| `render.compile` | query | Compile a guarded document synchronously into a cached in-memory render reference and diagnostics, without rasterization, export or document edits. |
| `document_types.list` | query | List built-in types, supplied models, creation availability and documentary command capabilities without requiring an open document. |
| `document.source` | query | Read the committed documentary Typst source with its revision guard, without executing code. |
| `document.assets` | query | Inventory document/model resources retained by the save/export policy and packaged dependencies, without returning file contents or traversing undo history. |
| `render.locate` | query | Locate a logical text target in an identified render or return the guarded text target at a page point, without moving selection. |
| `render.selection_boxes` | query | Read page quadrilaterals for a guarded text selection on an identified render, in Typst points. |
| `document.statistics` | query | Read Unicode word and grapheme counts for the document and an optional text target, excluding structural markers and private fields. |
| `recovery.list` | query | List opaque references from the configured recovery service without opening documents. |
| `models.list` | query | List supplied models for the active document type without changing the document. |
| `models.presentation` | query | Read the current model page constraints and permitted bullet markers. |
| `spelling.issues` | query | Read revision-guarded spelling issues and dictionary availability using the configured personal words. |
| `spelling.suggestions` | query | Suggest corrections for an explicit word and language or a validated spelling issue, without changing selection. |
| `document.inspect` | query | Read session state, document type, documentary command capabilities, guarded selection and typing style; document context is null when closed. |
| `document.text` | query | Read editable text without Typst markup. |
| `document.structure` | query | Read blocks, text zones, roles and their opaque identities. |
| `metadata.get` | query | Read the document metadata. |
| `commands.list` | query | Read command/query schemas and context availability with reasons, distinguishing calls that still require parameters; no execution or file access. |
| `search.find` | query | Find literal Unicode text without modifying the selection. |

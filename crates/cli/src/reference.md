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

L’export est une opération distincte après la réussite du script. Les images de pages dédiées restent un lot suivant ; MCP est prévu en phase 5.

## Catalogue du moteur

Le catalogue décrit les contrats du moteur, y compris les opérations qui ne disposent pas encore d’un déclencheur CLI. L’ouverture accepte l’option additive `read_only` (false par défaut).

| Identifiant | Famille | Description |
| --- | --- | --- |

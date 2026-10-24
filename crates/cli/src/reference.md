# CLI typstOgraphe

Document généré par `cargo run --locked -p typstographe-cli -- commands list --markdown`.

La CLI permet de créer des documents, consulter le catalogue, inspecter, vérifier et exporter un fichier `.typsto`, sans fenêtre ni préférences personnelles implicites.

```sh
typstographe-cli commands list --json
typstographe-cli commands describe metadata.set --json
typstographe-cli new --type courrier --output courrier.typsto
typstographe-cli inspect courrier.typsto --json
typstographe-cli check courrier.typsto --for-export
typstographe-cli check courrier.typsto --spelling
typstographe-cli export courrier.typsto --format pdf --output export/
```

Les lectures utilisent la dernière version enregistrée, même si le document est ouvert dans l’application. L’export crée un répertoire ou utilise un répertoire vide, avec `--pages` pour ajouter des images PNG.

`--lang fr|en` choisit la langue d’interface ; sinon langue système, repli anglais. La langue du document est indépendante. `--json` produit un objet `{"version":1,"ok":true,"outcome":…}` ou `{"version":1,"ok":false,"error":{"code","message","params"}}`. Aucun autre texte sur stdout en mode JSON.

Codes de sortie : 0 succès, 1 contrôle négatif ou opération refusée, 2 arguments invalides, 3 entrée/sortie. Un contrôle négatif a `ok: true` (exécution réussie), `outcome.passed: false` et le code de sortie 1. L’orthographe est facultative ; avec `--spelling`, un dictionnaire indisponible ou une suspicion donne un contrôle négatif.

`new` choisit le modèle automatiquement si le type en propose exactement un ; sinon fournir `--model`. Ses paramètres JSON (`initial_values`, `signature`) se donnent avec `--params` ou `--params-file` (`-` pour stdin). Un objet JSON UTF-8 est requis, limité à 4 Mio. Les fichiers et ressources indiqués dans le JSON sont relatifs au répertoire courant, y compris quand le JSON vient d’un fichier.

`query`, `run`, scripts atomiques et images de pages dédiées : lots suivants. Les identités de session ne sont pas des références persistantes entre deux exécutions. MCP est prévu en phase 5.

## Catalogue du moteur

Le catalogue décrit les contrats du moteur, y compris les opérations qui ne disposent pas encore d’un déclencheur CLI. L’ouverture accepte l’option additive `read_only` (false par défaut).

| Identifiant | Famille | Description |
| --- | --- | --- |

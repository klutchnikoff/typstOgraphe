# Contrat général des interfaces

Le cœur actuel utilise `content::editing::EditingSession` et Yrs. La façade
`content::api::Session` fournit aux adaptateurs sans fenêtre les commandes,
requêtes, fichiers et exports structurés. Tauri possède le même propriétaire
avec un contexte privé par fenêtre ; le MCP rattaché prépare des propositions
soumises à l'acceptation humaine.

## Données et cibles

- Les commandes viennent de `content::commands::Command` ; leurs schémas et le
  catalogue sont produits depuis les types exécutés. Aucun catalogue plat parallèle.
- Une plage textuelle désigne une zone et des offsets UTF-16, jamais une position
  globale propre à ProseMirror. Les blocs et cellules portent une racine et un chemin.
- Les enregistrements d'en-tête ont des identifiants stables. Les chemins doivent
  être relus après une modification structurelle.
- Les gardes associent document et révision. Une opération différée périmée est
  refusée ; la sélection d'un participant ne permet pas de réancrer un lot agent.
- Sélection, styles de frappe et historique sont privés au contexte d'édition.
  Les modifications sont publiées atomiquement, après validation de l'état final.

## Exemple sans interface

```rust
use serde_json::json;
use typstographe_core::content::api::Session;

let mut session = Session::new();
session.execute("document.new", json!({"type":"courrier"})).unwrap();
let target = json!({"zone":{"kind":"body","path":[0]},"from":0,"to":0});
session.execute("text.insert", json!({"target":target,"text":"Bonjour Élodie 😀"})).unwrap();
assert!(session.query("document.text", json!({})).unwrap()["text"].as_str().unwrap().contains("Bonjour Élodie 😀"));
assert_eq!(session.query("context.get", json!({})).unwrap()["undo_steps"], 1);
session.execute("edit.undo", json!({})).unwrap();
assert!(!session.query("document.text", json!({})).unwrap()["text"].as_str().unwrap().contains("Bonjour"));
```

## Frontières et garanties

`protocol` contient les erreurs d'adaptateurs, les formats d'export et les
limites communes ; `content::contract::Error` décrit les refus documentaires.
Le cœur ne dépend ni de Fluent, ni d'une interface graphique, ni du transport MCP.
Les adaptateurs choisissent les messages et gèrent dialogues, focus et défilement.

Les archives `.typsto` conservent le document structuré, son état Yjs, ses
métadonnées, le modèle et ses dépendances locales. Les ressources utilisées sont
retenues à partir de l'arbre structuré, sans reparsage d'un tampon Typst plat.
Les écritures sont atomiques et passent par `DocumentLease`. Une ouverture
éditable prend le verrou avant lecture ; une modification externe est détectée.
Les brouillons incomplets restent autorisés ; les champs obligatoires à l'export
sont vérifiés séparément. L'export multi-fichiers ne promet pas de transaction.
L'ancien format egui n'est ni lu ni converti.

La géométrie de l'aperçu est associée à sa révision ; une navigation périmée ne
s'applique pas à un autre état. L'interface conserve son ordonnanceur de rendu
et le moteur Typst en mémoire. Les anciens workers egui sont retirés.

Voir le [contrat structuré](structured-contract.md), les
[commandes multizones](structured-commands.md), les
[contextes](structured-editing-contexts.md) et les
[adaptateurs et recettes](structured-adapters.md). Le contrat egui reste
[archivé](archive/interface-contract-egui.md), sans exemple compilé dans le cœur.

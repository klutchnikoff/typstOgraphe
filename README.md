# typstOgraphe

[![CI](https://github.com/klutchnikoff/typstOgraphe/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/klutchnikoff/typstOgraphe/actions/workflows/ci.yml)
[![Couverture des lignes — Linux](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Fklutchnikoff%2FtypstOgraphe%2Fcoverage%2Fcoverage.json)](https://github.com/klutchnikoff/typstOgraphe/actions/workflows/coverage.yml)
[![Rust](https://img.shields.io/badge/Rust-1.92%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2-blue?logo=tauri)](https://tauri.app/)
[![Typst](https://img.shields.io/badge/Typst-0.15.1-239DAD?logo=typst)](https://typst.app/)

typstOgraphe permet de rédiger des documents soignés **sans écrire de code** :
un éditeur visuel à gauche, le rendu Typst en temps réel à droite.
Les modèles prennent en charge la présentation et les conventions de l’organisation.

L’application vise **macOS, Windows et Linux**, avec des fichiers locaux autonomes.
Le premier modèle utilisable est un **courrier personnel français**.

**En développement préliminaire.** La nouvelle interface utilise Tauri,
Tiptap/ProseMirror et Yjs, avec un cœur Rust et Typst embarqué.

## Lancer la nouvelle interface

Avec Rust, Node.js et les [prérequis Tauri](https://v2.tauri.app/start/prerequisites/) :

```sh
cd experiments/web-editor
npm ci
npm run tauri dev
```

La nouvelle interface est intégrée à `main` ; sa sortie du dossier expérimental
fait partie du premier chantier de développement.
`cargo run --locked` lance encore l’interface egui de référence.
La [CLI documentaire](docs/cli.md) utilise encore son contrat historique.

Voir la [migration de l’interface](docs/tauri-migration.md) et la
[feuille de route](ROADMAP.md).

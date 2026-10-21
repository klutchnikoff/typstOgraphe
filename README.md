# typstOgraphe

[![CI](https://github.com/klutchnikoff/typstOgraphe/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/klutchnikoff/typstOgraphe/actions/workflows/ci.yml)
[![Couverture des lignes — Linux](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Fklutchnikoff%2FtypstOgraphe%2Fcoverage%2Fcoverage.json)](https://github.com/klutchnikoff/typstOgraphe/actions/workflows/coverage.yml)
[![Rust 1.92+](https://img.shields.io/badge/Rust-1.92%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![egui 0.33.3](https://img.shields.io/badge/egui-0.33.3-blue)](https://docs.rs/egui/0.33.3/egui/)
[![Typst 0.15.1](https://img.shields.io/badge/Typst-0.15.1-239DAD?logo=typst)](https://typst.app/)

typstOgraphe permet de rédiger des documents soignés **sans écrire de code** :
un éditeur visuel à gauche, le rendu Typst en temps réel à droite.
Les modèles prennent en charge la présentation et les conventions de l’organisation.

L’application vise **macOS, Windows et Linux**, avec un usage individuel,
des fichiers locaux et un document autonome regroupant contenu, métadonnées,
modèle et images. Les premiers types sont le **courrier** et le **compte-rendu de réunion**.

**En développement préliminaire.** Le projet est écrit en Rust, avec egui/eframe
et le moteur Typst embarqué.

## Lancer depuis les sources

Avec Rust **1.92 ou supérieur** :

```sh
cargo run --locked
```

Les prochaines étapes sont présentées dans la [feuille de route](ROADMAP.md).

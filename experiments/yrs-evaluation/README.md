# Évaluation de Yrs pour typstOgraphe

Évaluation du 27 octobre 2026, avant décision sur son adoption dans le chantier
A.2. Ce répertoire est une expérience autonome : Yrs n'est ajouté ni au cœur,
ni à l'application, ni à leurs fichiers de dépendances.

## Conclusion

**Yrs est un candidat convaincant pour permettre au Rust de lire et modifier
l'état partagé de notre éditeur Yjs.** Les 19 scénarios passent avec les versions
actuelles du projet, y compris un article réel, son en-tête structuré, une
sauvegarde `.typsto`, sa réouverture dans l'éditeur et sa compilation Typst hors
ligne. Aucun obstacle bloquant n'a été observé dans ce périmètre.

Cela rend crédible une CLI et un MCP capables d'éditer le même document que
l'interface, en conservant les identités collaboratives. L'adoption reste à
valider. Le travail principal serait notre adaptation entre les structures Yjs,
le contrat documentaire et les commandes Rust : Yrs ne fournit pas un moteur
ProseMirror ni nos règles documentaires.

## Configuration et document utilisés

| Élément | Valeur testée |
| --- | --- |
| Yrs | 0.28.0, licence MIT, option `small-client` |
| Yjs | 13.6.33 |
| Liaison ProseMirror/Yjs | `@tiptap/y-tiptap` 3.0.10 |
| Échanges | Mises à jour binaires V1, états complets et différentiels |
| Positions de texte | `OffsetKind::Utf16`, explicitement |
| Rétention | `skip_gc: true`, sauf le contrôle comparatif du scénario 19 |
| Machine | macOS, arm64, Apple M1 Max |
| Compilation de la sonde | Rust 1.98.1, profil release |
| Exécution des tests | Node 26.9.0, Vitest/jsdom et composants réels de l'application |

Le point de départ est la copie locale de `examples/article-working.typsto`,
lue sans modification : 22 072 octets, dont 21 130 octets de `state.yjs`.
Son empreinte SHA-256 au moment de l'essai est
`0ec624e8e388158336d4604d1fd7554521dc18cd8c673ef7207c9847bb8c2fcc`.
Cet exemple contient notamment les auteurs, affiliations, résumés, citations,
sources bibliographiques et un tableau. Il peut avoir évolué depuis le commit
qui l'a créé ; le lanceur relève l'empreinte de l'exemplaire effectivement testé.

L'état réencodé par Yjs mesure 21 050 octets. L'équivalence est vérifiée par
les structures partagées et les vecteurs d'état, pas par l'identité binaire des
encodages. Les attributs implicites ProseMirror sont normalisés avec le schéma
réel avant comparaison avec `document.json`.

Deux réglages méritent une attention particulière :

- Yrs utilise les positions en octets par défaut. Notre contrôle négatif montre
  un déplacement incorrect après un emoji si l'on oublie `Utf16`.
- `small-client` conserve les identifiants de client adaptés à Yjs 13. Les
  identifiants fixes de la sonde rendent certains essais reproductibles ; une
  intégration utilisera des identités propres à chaque acteur.

Les racines partagées comprennent le corps XML `default`, `settings` et la
racine privée de migration de positions de Tiptap. Les réglages contiennent des
`Y.Map`, `Y.Array`, `Y.Text`, des fragments XML riches et des valeurs JSON
atomiques. La sonde conserve aussi la racine privée.

## Résultats fonctionnels

Les résultats détaillés sont conservés dans [results.json](results.json).
Un scénario de limite réussi signifie que la limite attendue est effectivement
reproduite, pas que Yrs la résout.

| N° | Vérification | Résultat |
| --- | --- | --- |
| 01 | Aller-retour de l'archive réelle : types, contenu, identités, vecteurs d'état et ancrage | Conforme |
| 02 | Accents, japonais, emoji et caractères combinants ; contrôle négatif avec les positions en octets | Conforme avec UTF-16 |
| 03 | Lecture et édition Rust du gras, de l'italique et des liens | Conforme |
| 04 | Auteur en `Y.Text`, titres et résumés riches, ordre des auteurs, sommaire et bibliographie | Conforme |
| 05 | Attributs JSON des citations, notes, images/légendes et descriptions | Conforme |
| 06 | Insertion Rust d'une ligne de tableau en concurrence avec l'édition JS d'une cellule existante | Convergence et identité de cellule conservée |
| 07 | Insertions concurrentes au même endroit et modifications de champs indépendants | Convergence |
| 08 | Deux modifications concurrentes d'un tableau JSON atomique d'associations | Une valeur gagne, sans union des membres |
| 09 | Différentiels reçus en ordre inversé, dépendances en attente et livraison répétée | Convergence et idempotence |
| 10 | Position relative Yjs, lecture/réencodage par `StickyIndex` Rust, insertion et suppression voisines | Ancrage conservé |
| 11 | Annuler/rétablir humain dans le véritable éditeur Tiptap, avec une édition Rust distante | Édition distante conservée |
| 12 | Suppression d'un bloc concurrente avec une édition de son texte | Convergence, mais le texte du bloc supprimé est masqué |
| 13 | Projection Rust de l'article édité, validation par le cœur et export Typst | Conforme |
| 14 | Citation vers une référence absente | Acceptée par le CRDT, refusée par la validation du cœur |
| 15 | Performances release sur l'article et deux documents synthétiques | Mesurées ci-dessous |
| 16 | Annuler/rétablir dans Rust, puis lecture par Yjs | Conforme |
| 17 | Enregistrement avec la vraie API d'archive, réouverture, compilation hors ligne et chargement dans le vrai éditeur | Conforme ; 3 pages compilées |
| 18 | 32 cycles de fusions successives avec Unicode et styles concurrents | Convergence |
| 19 | 500 remplacements avec et sans récupération des contenus supprimés | Même texte visible, rétention très différente |

La compilation et la sauvegarde passent par le cœur existant. Les essais de
l'éditeur utilisent `createRichZone` et ses extensions de production dans jsdom,
sans capture ni simulation de présentation. Ce contrôle ne remplace pas un test
de saisie ou de focus dans la fenêtre native.

Contrôle supplémentaire : le répertoire Typst exporté après l'édition Rust a
également été compilé avec la CLI Typst locale 0.15.1, sans erreur.

## Latence et mémoire

Vingt échantillons après quatre tours de chauffe, binaire Rust release. Chaque
requête crée un document, charge l'état complet, insère un court texte et encode
l'état complet ainsi que le différentiel. Le processus reste ouvert entre les
requêtes. La colonne native inclut la gestion de cette requête côté Rust, mais
exclut la lecture/écriture JSON ; la colonne aller-retour inclut JSON, base64 et
stdio depuis JavaScript.

**Ces mesures n'incluent ni la projection documentaire, ni la validation, ni
la compilation Typst, ni le rendu, ni la sauvegarde.** Ce ne sont donc pas les
temps de mise à jour de l'aperçu.

| Document | État Yjs entrant | Natif, médiane | Natif, p95 | Aller-retour, médiane |
| --- | ---: | ---: | ---: | ---: |
| Article réel | 21 050 octets | 0,12 ms | 0,15 ms | 0,22 ms |
| 250 paragraphes synthétiques | 311 844 octets | 1,09 ms | 1,17 ms | 2,03 ms |
| 1 000 paragraphes synthétiques | 1 647 856 octets | 5,53 ms | 5,70 ms | 10,40 ms |

L'édition seule prend environ 1 à 4 µs dans ces essais très simples. Le
différentiel produit mesure 20 à 29 octets, hors enveloppe de transport. Cela
plaide pour des sessions persistantes et des échanges différentiels. Cela ne
mesure pas le coût futur d'une commande complète.

Pour la mémoire, sept processus neufs par document, caches du système déjà
chauds ; pic RSS mesuré par `/usr/bin/time`. Ce chiffre comprend le petit
exécutable Rust, le chargement, JSON/base64 et les tampons de sortie. Il exclut
JavaScript, Tauri, Typst et les fichiers du projet.

| Document | Démarrage + requête, médiane | Pic RSS maximal observé |
| --- | ---: | ---: |
| Article réel | 5,60 ms | 2,78 Mio |
| 250 paragraphes | 8,61 ms | 5,03 Mio |
| 1 000 paragraphes | 23,40 ms | 16,70 Mio |

Le binaire de sonde dépouillé mesure 1 294 704 octets, soit 1,23 Mio. Il ne
permet pas de déduire directement la taille supplémentaire de l'application.

Les chiffres sont un relevé local, pas un budget garanti. Les versions exactes,
tailles et mesures brutes sont dans `results.json` ; les tableaux sont arrondis.
Les documents synthétiques testent surtout le volume de texte, pas la
complexité d'un long article avec de nombreux tableaux et références.

## Ce qui resterait à construire si nous retenons Yrs

1. **L'adaptation documentaire.** Le format binaire est compatible, mais Yrs ne
   connaît ni notre schéma, ni les valeurs par défaut ProseMirror, ni les marques
   Tiptap ou les règles d'édition des widgets. La sonde réalise une projection
   expérimentale Rust du corps et des métadonnées à partir du catalogue actuel.
   Elle démontre la faisabilité ; elle ne constitue pas un adaptateur complet
   prêt à être réutilisé en production. Les modifications devront être ciblées
   pour préserver les identités, plutôt que reconstruire tout le document.
2. **La validation et les lots atomiques.** Il faut valider le document obtenu
   avant de publier les changements, avec les mêmes règles pour les interfaces,
   la CLI et MCP. Une transaction CRDT ne suffit pas à garantir notre contrat
   ni à organiser le refus d'une proposition invalide.
3. **La politique de concurrence.** Une suppression concurrente peut masquer
   une contribution ; deux tableaux JSON atomiques ne fusionnent pas leurs
   membres. Il faut décider les gardes et la restitution à l'utilisateur.
4. **L'historique et la rétention.** Les annulations locales fonctionnent avec
   des origines distinctes. Leur pile n'est pas incluse automatiquement dans
   l'état sauvegardé : le projet se rouvre avec une pile vide. Dans le contrôle
   de 500 remplacements, l'état conservant les contenus supprimés mesure
   30 147 octets, contre 147 avec récupération. Ce ne sont pas des tailles de
   pile d'annulation ; c'est le coût de rétention dans cet exemple. Une politique
   bornée devra articuler annulation, snapshots et nettoyage.
5. **Le durcissement.** Corpus de tous les types et profils riches, requêtes
   invalides, interruptions, migrations de schéma, stress prolongé, Windows et
   Linux. Le compilateur minimal du projet n'a pas été testé ici. Aucun serveur
   collaboratif ou protocole réseau complet n'a été évalué.

La recommandation issue de l'essai est de retenir Yrs comme candidat au stockage
et à l'édition collaborative côté Rust, en conservant Yjs dans l'interface et
un contrat public propre à typstOgraphe. Ce choix permettrait un fonctionnement
sans interface pour la CLI/MCP ; il ne supprimerait pas notre couche métier.
La décision d'adoption et les autres choix du chantier A.2 restent à discuter.

## Reproduire

Prérequis : dépendances JavaScript du dépôt installées, Python 3, Rust et
dépendances natives permettant de compiler l'application. Premier téléchargement
des dépendances propres à l'expérience :

```sh
cargo fetch --locked --manifest-path experiments/yrs-evaluation/Cargo.toml
```

Puis, depuis la racine :

```sh
node experiments/yrs-evaluation/run.mjs
```

Un autre exemplaire du même article peut être passé en argument. Les scénarios
spécifiques attendent la structure et les collections de l'article d'exemple ;
ce lanceur n'est pas un validateur générique de tous les projets. Si l'exemple
ne contient pas encore de `state.yjs`, l'amorçage utilise les adaptateurs réels
de l'application à partir de ses données sauvegardées.

Le lanceur compile la sonde séparément, puis l'application et un petit exemple
Rust temporaire qui appelle l'API d'archive existante. Il refuse d'écraser un
exemple existant et retire ce fichier source dans un `finally`. Il lance ensuite
Vitest et les mesures de processus, puis retire les exécutables temporaires de
cet exemple. Il ne modifie pas le projet d'entrée.

`.generated/` et `target/` restent ignorés. Les mesures de mémoire reposent sur
`/usr/bin/time -l` sous macOS ou `-v` sous Linux ; macOS peut nécessiter une
exécution hors sandbox. La recette complète a été exécutée uniquement sur
macOS. `results.json` est remplacé à chaque exécution pour conserver le relevé.

Contrôle statique de la sonde :

```sh
cargo fmt --check --manifest-path experiments/yrs-evaluation/Cargo.toml
cargo clippy --release --locked --offline --manifest-path experiments/yrs-evaluation/Cargo.toml -- -D warnings
```

## Références

- [Projet Y CRDT, objectifs de compatibilité avec Yjs](https://github.com/y-crdt/y-crdt).
- [Documentation de Yrs](https://docs.rs/yrs/latest/yrs/), notamment `Options`,
  `OffsetKind`, `StickyIndex`, `UndoManager` et la fonctionnalité `small-client`.
- [Source exacte de Yrs 0.28.0](https://docs.rs/crate/yrs/0.28.0/source/),
  également consultée dans le registre Cargo local pour vérifier les réglages.

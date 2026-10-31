# Évaluation du stockage des snapshots — A.4

Expérience isolée, sans changement du format `.typsto` de production et sans
ajout de Git aux dépendances de l’application. Les choix fonctionnels validés
sont dans les deux feuilles de route. Les résultats et la recommandation sont
dans [le bilan](../../docs/a4-snapshot-storage-evaluation.md).

## Deux candidats réellement exécutés

- **Git embarqué** : `git2` 0.21.0 avec libgit2 embarquée, sans SSH ni HTTPS.
  Les fichiers racines d’un état (`document.json`, `state.yjs`, sources Typst,
  métadonnées et manifeste) sont des blobs/arbre/commit Git. Chaque snapshot a
  un commit indépendant, sans parent ; un index applicatif porte leur ordre et
  leurs métadonnées. Cela permet d’effacer une version sans la conserver comme
  ancêtre d’un commit ultérieur. Les commits encore référencés sont empaquetés
  en un pack, avec compression différentielle Git, à un seul thread.
- **États à objets dédupliqués (CAS)** : chaque fichier est identifié par son
  SHA-256 ; un index de snapshot référence ses fichiers. Les octets identiques
  sont stockés une seule fois. Chaque objet est compressé indépendamment dans
  le ZIP ; aucune compression différentielle entre objets.

Dans les **deux** solutions, images, bibliographie et packages sont dans le même
stockage d’objets SHA-256 partagé, hors de Git. Ils restent inclus dans le fichier
autonome. Aucun nettoyage ne se fonde uniquement sur le dernier état.
Les tables chemin → empreinte des ressources sont elles-mêmes des objets
dédupliqués ; les versions ne recopient pas la liste des centaines de fichiers
d’un package dans chaque ligne de l’index.

Le pack Git est conservé sans recompression dans le ZIP ; les autres entrées
utilisent Deflate. Les deux moteurs publient l’archive avec `atomic-write-file`.
Le moteur Git utilise une base d’objets temporaire isolée ; aucune commande Git
n’est lancée pendant les mesures. Git n’est utilisé par le lanceur que pour
identifier le commit source et lire l’exemple versionné, sans toucher au dépôt.

L’enveloppe `history.store` est expérimentale : **ce n’est pas le futur schéma
public du `.typsto`**, ni une implémentation des commandes de snapshots.

## Corpus et opérations

Le lanceur Node génère les états via la CLI de production reconstruite avec
`--locked --offline`. Les deux moteurs lisent exactement les mêmes archives :

1. Courrier personnel issu du script versionné `examples/cli/courrier.json`.
2. Article issu de la version **committée** de `examples/article-working.typsto`,
   même si ce fichier a des modifications locales.
3. Le même article enrichi de 250 paragraphes, pour sonder l’effet du volume.

Chaque série compte 100 états Yjs successifs : remplacement d’un paragraphe avec
accents et japonais, image commune à tous les états, insertion d’une deuxième
image à la version 10, remplacement à la version 20, suppression à la version 30,
et modification du modèle aux versions 40 et 70. Les trois images PNG de
512 × 512 sont des fixtures déterministes à bruit pseudo-aléatoire, peu
compressibles ; elles ne représentent pas la diversité des images réelles.
Les textes de l’article long se répètent : c’est un cas de volume, pas un corpus
représentatif de mémoires scientifiques.

Pour chaque moteur et chaque série, mesures à 20 puis 100 versions :

- création et publication du premier snapshot ;
- ajout du dernier snapshot à un historique existant et publication complète ;
- ouverture de l’archive et restitution du plus ancien état ;
- restitution d’un état médian dans un stockage déjà ouvert ;
- suppression des états intermédiaires, conservation du premier et du dernier,
  nettoyage et publication de l’archive ;
- taille finale et taille après cette suppression.

L’ouverture lit l’index et, pour Git, importe le pack dans la base temporaire.
Pour les deux moteurs, les objets partagés sont décompressés seulement à la
consultation de l’état sélectionné. CAS peut également lire ses fichiers
documentaires indépendamment des autres versions. Ce parcours sélectif évite
de charger toutes les ressources historiques pour consulter une seule version.

Un tour de chauffe puis sept échantillons ; médiane, p95 et mesures brutes.
Avec sept valeurs, le p95 est le maximum observé, pas une estimation robuste
d’une distribution de production. Caches du système déjà chauds, processus
persistant, Rust release. Les créations de bases Git temporaires et l’import du
pack sont inclus dans l’ouverture. La génération du corpus, la validation
documentaire, la compilation Typst et l’interface sont exclues des temps.
Les mesures ne sont donc **pas** une latence d’aperçu ni une mesure de la frappe.

## Garanties vérifiées

- Restitution octet pour octet de chaque état des historiques de 20 et 100
  versions ; conservation de l’index, des noms, des dates et des tags.
- Après suppression, premier et dernier états intacts, versions supprimées
  inaccessibles et aucun objet binaire orphelin dans l’archive republiée.
- Échec injecté après écriture du candidat mais avant publication : ancien
  fichier inchangé et toujours ouvrable.
- Refus d’une archive tronquée, d’une ressource absente ou altérée ; refus d’un
  pack Git altéré, même réemballé dans un ZIP valide.
- Trente projets reconstruits (versions 0, 10, 20, 30, 99 × trois séries × deux
  moteurs), vérifiés par le cœur actuel, exportés en Typst et compilés hors ligne.

Ce sont des contrôles ciblés du stockage : ils ne simulent pas une coupure de
courant ni la corruption de tous les supports. Ils ne qualifient ni la
restauration collaborative ni les protections futures de l’interface Versions.
Le lecteur ZIP de la sonde ne remplace pas le lecteur de production durci.

## Reproduire

Depuis la racine, avec Rust, Node et les dépendances du projet disponibles :

```sh
# Une fois si les dépendances propres à la sonde ne sont pas en cache.
cargo fetch --locked --manifest-path experiments/snapshot-storage/Cargo.toml

# Choisir un répertoire de sortie neuf pour une nouvelle mesure indépendante.
node experiments/snapshot-storage/run.mjs --output output/a4-storage-evaluation

cargo test --release --locked --offline --manifest-path experiments/snapshot-storage/Cargo.toml
cargo clippy --release --locked --offline --manifest-path experiments/snapshot-storage/Cargo.toml --all-targets -- -D warnings
```

Le lanceur reprend des fixtures et exports déjà présents dans son répertoire de sortie ;
ne pas réutiliser ce répertoire après une modification de la recette du corpus.
Les relevés successifs de ce lot portent sur les mêmes fichiers sources ; leurs
exports existants sont réutilisés, les contrôles du cœur et compilations rejoués.
`--fixtures-only` s’arrête après la génération. Les archives et exports volumineux
restent dans `output/`, ignoré par Git ; seuls les rapports JSON légers sont
versionnés dans `results/`. Aucune capture d’écran n’est produite.
`macos-eager-reference.json` conserve le premier relevé diagnostique, qui chargeait
toutes les entrées à l’ouverture et répétait les références de ressources dans
l’index. `macos-selective-reference.json` isole l’étape de lecture sélective.
`macos.json`, avec lecture sélective et manifestes dédupliqués, sert à la
recommandation ; les mesures de ces méthodes ne doivent pas être mélangées.

Les mesures du moteur seul peuvent être relancées sur un corpus existant :

```sh
cargo run --release --locked --offline --manifest-path experiments/snapshot-storage/Cargo.toml -- output/a4-storage-evaluation
```

Le code utilise les API publiques de
[PackBuilder](https://docs.rs/git2/0.21.0/git2/struct.PackBuilder.html) et
[OdbPackwriter](https://docs.rs/git2/0.21.0/git2/struct.OdbPackwriter.html).
Les fonctionnalités et licences sont décrites par
[git2-rs](https://github.com/rust-lang/git2-rs) et
[libgit2](https://github.com/libgit2/libgit2/blob/main/COPYING).

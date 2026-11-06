# Déclarations de types : contrat, chargeur et adaptateurs

Contrat initial du 3 novembre 2026, dans
`typstographe_core::document_types::definition`. Cette API lit et valide une
déclaration ; les types intégrés l’utilisent pour leurs champs, collections,
vues, capacités, validations et liaisons Typst. Elle **n’installe pas un type
externe** : les identités intégrées et codecs de données restent compilés.

La [structure commune `authorship`](../crates/core/structures/authorship.yaml)
fournit les auteurs et affiliations à l’Article. Le
[premier exemple auteurs/affiliations](proposals/b1/article-authorship.yaml)
reste une fixture autonome du chargeur. Les contrats actifs sont
[Courrier](../crates/core/types/courrier/type.yaml),
[Article](../crates/core/types/article/type.yaml) et
[Compte-rendu](../crates/core/types/compte-rendu/type.yaml). Ce dernier reste
non créable sans modèle fourni ; sa déclaration ne change pas sa disponibilité.

## Autorité et API

- `Definition` est le contrat Serde/Schemars typé. Les genres de valeurs, variantes
  et paramètres reconnus sont définis en Rust, sans expression exécutable en YAML.
- `load_yaml(&str)` renvoie un `LoadedDefinition` uniquement après décodage et
  validation sémantique. `Definition::validate(self)` sert au même contrôle pour
  une déclaration construite autrement ; le résultat est immuable.
- `manifest_schema()` produit le JSON Schema du manifeste depuis les types Rust.
  Le [schéma versionné](schemas/type-definition.schema.json) est généré, avec un
  test de concordance ; il n’est pas maintenu à la main.
- `LoadedDefinition::values_schema()` décrit la structure des valeurs à partir
  de la définition chargée. Il distingue les obligations d’export des contraintes
  de structure. Ce schéma n’applique pas les relations métier ni les éditions.
- `defaults()` crée les valeurs de champs racine et les collections vides ;
  `record_defaults(collection)` fournit les valeurs d’un nouvel enregistrement,
  sans inventer son identifiant. L’opération d’édition fournit l’identité stable.
- `validate_values()` contrôle les valeurs documentaires, leurs genres,
  identifiants, relations, unicités et limites. `export_diagnostics()` applique
  ensuite les obligations d’export, dont le titre dans la langue principale.

## Types intégrés et adaptateurs

Les déclarations sont embarquées avec `include_str!`, validées puis mises en
cache une fois avec leurs descripteurs. Modifier un YAML intégré demande une
reconstruction ; aucune lecture disque ou relecture du schéma à chaque frappe.

Le module `document_types::runtime` dérive les mêmes champs, collections,
défauts, onglets et sections pour le moteur et les adaptateurs. Les anciennes
listes `FIELDS`/`GROUPS`, `TextField`, `ModelContract`, le descripteur Article
manuel et le drapeau `letter_only` sont retirés. Le type déclare les capacités
autorisées ; le catalogue global conserve leurs implémentations.

- **Tauri** localise ces descripteurs, réutilise les composants communs et ouvre
  la signature dans l’onglet déclaré, sans déduire son emplacement du nom du type.
- **CLI** publie le contrat dans `document_types.describe`, via le moteur commun.
- **MCP** réutilise ce même détail, en conservant l’index compact et la découverte
  progressive des capacités ; aucun catalogue parallèle ni nouvelle permission.
- **Typst** reçoit les paramètres dans l’ordre des liaisons déclarées ; le
  sérialiseur commun conserve chaînes littérales, booléens, listes, dictionnaires,
  zones riches et chemins bibliographiques. Le modèle garde la mise en page.

Les structures Rust Article/Courrier/Compte-rendu restent les codecs des valeurs
et des archives. À l’initialisation, le registre vérifie que tous leurs champs
correspondent aux valeurs produites par la déclaration, sans omission silencieuse.
Ajouter un type installable ou supprimer ces codecs est un travail distinct ;
ce lot ne crée pas un chargeur de documents à schéma arbitraire.

La liste des capacités référence le catalogue exécutable commun. Une capacité
inconnue ou simplement prévue est refusée. Les permissions d’un adaptateur et
la disponibilité au curseur restent des contrôles distincts.

## Structures d’en-tête réutilisables

Un type sélectionne une structure enregistrée avec un raccourci :

```yaml
structures: [authorship]
```

`authorship` possède les collections `authors` et `affiliations`, leurs champs,
identités, relation par identifiants, obligations d’export, vues cartes/modales,
onglets et liaisons Typst. La suppression d’une affiliation encore référencée
reste refusée. Les codecs Rust `Author` et `Affiliation` se trouvent dans
`document_types::authorship`, indépendamment du type Article. Les composants
communs de l’interface restent responsables de leur présentation.

Le chargeur développe les structures embarquées **avant la validation du type**.
La définition résolue est le seul contrat consommé par le moteur, Tauri, CLI,
MCP et le sérialiseur Typst. Le raccourci et les instructions d’ordre sont
consommés à cette étape, sans ajouter de nouvelle forme aux valeurs documentaires.
Un test compare la définition Article résolue à son état antérieur complet,
ordre compris ; les sources Typst restent également inchangées.

Sans ordre explicite, les membres des structures apparaissent d’abord, dans
l’ordre des structures sélectionnées, puis viennent les membres locaux du type.
Celui-ci peut préciser l’ordre final sans modifier le contenu partagé :

```yaml
structures: [authorship]
order:
  collections: [references, authors, affiliations, localizations]
  tabs: [titles, abstracts, affiliations, authors, bibliography]
```

Chaque liste non vide doit être une permutation complète des identifiants
résolus. `order.bindings`, facultatif, utilise les identifiants des champs ou
collections sources des liaisons. Un ordre incomplet, un membre inconnu ou un
doublon est refusé : ces listes ne permettent ni filtre ni redéfinition.

La première version réutilise les structures à l’identique. Les noms inconnus,
sélections répétées, collisions avec une déclaration locale et paramètres de
personnalisation sont refusés. Les fragments enregistrés utilisent le même
décodeur YAML strict ; ils ne peuvent importer d’autres fragments ni changer
l’identité, les capacités ou les politiques de modèles du type. Le catalogue
reste compilé et n’autorise aucun chemin de fichier fourni par un manifeste.

## Vocabulaire du premier chargeur

| Déclaration | Sémantique et défaut |
| --- | --- |
| `text` | Chaîne littérale, vide par défaut ; `multiline: false` par défaut. Aucune source Typst injectée par la valeur. |
| `resource-reference` | Référence interne d’image facultative, `null` par défaut ; contrôle commun et copie de profil si déclarée. |
| `date` | Absence `null`, date calendaire complète ou texte personnalisé explicite ; défaut absent, transmission Typst native. |
| `boolean` | Booléen, `false` par défaut. |
| `choice` | Liste fermée de chaînes distinctes et défaut explicite appartenant à cette liste. |
| `language` | Langue documentaire du vocabulaire commun, français par défaut. Distincte de la langue d’interface. |
| `tokens` | Liste de chaînes, vide par défaut ; mots-clés notamment. |
| `rich-text` | Contenu éditorial structuré, vide par défaut ; profil `inline-text` ou `rich-text` commun. |
| `record-references` | Liste d’identifiants vers une collection existante ; `unique: true`, `on-target-remove: restrict`, défaut vide. |
| `ordered-records` | Collection à identités stables et ordre distinct des libellés ; champs décrits par les genres ci-dessus. |
| `bibliographic-records` | Notices natives, clés et provenance validées par la crate commune ; éditions par `bibliography.apply`. |
| `bibliographic-sources` | Sources originales immuables, identités et digests vérifiés ; aucune composition directe par le modèle. |

Les identifiants de déclarations sont des noms ASCII commençant par une lettre
minuscule, suivie de lettres minuscules, chiffres, tirets, soulignés ou points,
avec un maximum de 128 octets. Les paramètres et la fonction Typst doivent en
plus être de simples identifiants Typst, sans point ni mot-clé.
L’identifiant `id` d’un enregistrement est réservé à son identité ; il n’est
pas redéclaré comme champ éditable.

Le genre [`date`](structured-dates.md) conserve explicitement l’absence, une
valeur calendaire complète ou un texte personnalisé. Il possède un contrôle
commun avec modes, Aujourd’hui fixe et corbeille ; aucune inférence depuis le
texte et aucune lecture de l’horloge dans le cœur. Typst reçoit `none`, un
`datetime` ou une chaîne échappée ; `format-date` conserve Datify derrière son
interface stable. Le contrôle `outline` expose le booléen commun dans Général.
Le genre [`resource-reference`](resource-references.md) déclare les images
facultatives et leur éventuelle source de profil. La signature du Courrier
utilise cette brique, ses commandes et son contrôle commun ; l’ancienne structure
`signature` est supprimée. Les [régimes de modèles](model-options.md) sont implémentés.

### Collections, liaisons et vues

Chaque champ racine ou collection possède une liaison unique vers un paramètre
Typst, distinct des autres paramètres. `metadata-key`, facultatif, désigne le
nom dans le codec des données lorsqu’il diffère du paramètre Typst (par exemple
`citation_style` vers `citation-style`). Ces trois identités ne sont pas des
copies de la valeur. Le contrôle de modèle du moteur compile la fonction avec
les paramètres générés et refuse les signatures incompatibles.

L’en-tête permet trois vues :

- `form` : sections ordonnées référençant les champs racine ;
- `record-cards` : cartes d’une collection avec champ de résumé et édition
  `dialog`. Une liste `fields` facultative limite les champs édités par cette vue ;
  absente ou vide, elle sélectionne tous les champs.
- `bibliography` : vue enregistrée reliée aux `bibliographic-records` et au champ
  de choix du style. Les sources immuables sont des `bibliographic-sources`
  séparées ; un seul Hayagriva dérivé est transmis au modèle.

Plusieurs vues peuvent éditer des sous-ensembles de la même collection, comme
Titres et Résumés. Les références des vues sont vérifiées et chaque champ doit
être exposé par au moins une vue. Il n’y a ni duplication de données, ni CSS,
ni raccourci propre au type dans cette définition.

`unique-by` décrit les champs scalaires d’une clé composée unique par collection.
Le chargeur vérifie que ces champs existent et sont scalaires ; la validation
documentaire vérifie ensuite cette unicité. `language-key` impose l’unicité de
la langue et permet d’initialiser la langue principale ou d’ajouter une langue
sans supprimer les textes déjà rédigés. `primary-language` dans
`export-requirements` exige un champ textuel dans la langue du document seulement.
Une relation `restrict` signifie de même que l’état final d’un lot ne peut pas
laisser de référence pendante ; elle ne commande pas une suppression en cascade.

## YAML et diagnostics

Le chargeur utilise [yaml_serde 0.10.7](https://docs.rs/yaml_serde/0.10.7/yaml_serde/),
publié par l’organisation YAML, avec son parseur Rust `libyaml-rs` 0.3.0.
Versions et sommes de contrôle figurent dans `Cargo.lock` ; yaml_serde est sous
MIT ou Apache-2.0, libyaml-rs sous MIT. Aucune dépendance nouvelle à Fluent ou à
une interface graphique.

Conventions :

- un seul document YAML, au maximum 256 Kio de source UTF-8 ;
- commentaires et ancres/alias acceptés ; limites de récursion et de répétition
  du parseur conservées ;
- doublons de clés refusés à toute profondeur, avant le décodage typé ;
- clés de mapping textuelles, pas de clés de fusion `<<`, de tags personnalisés
  ni de nombres non finis ;
- champs inconnus, variantes non prises en charge et défauts du mauvais genre
  refusés, sans coercition applicative ni exécution de contenu ;
- aucun accès disque, import externe, réseau ou enregistrement dans un catalogue
  lors de `load_yaml` : l’appelant fournit le texte à lire ; les structures
  enregistrées sont embarquées et mises en cache.

Le passage par la valeur YAML du parseur vérifie les doublons et le sous-ensemble
commun avec JSON ; le décodage typé repart du source pour conserver les positions
fournies par la bibliothèque. Ce contrôle n’utilise pas de parseur maison.

Les erreurs sont sérialisables et typées : `source_too_large`, `decode`,
`unsupported_yaml`, `invalid_definition`. Les erreurs de décodage conservent
ligne/colonne lorsqu’elles sont fournies ; les diagnostics sémantiques exposent
un code et un chemin JSON Pointer. Le cœur ne traduit pas ces messages : les
adaptateurs pourront les présenter avec Fluent, comme les autres erreurs typées.

Les limites du contrat initial sont 64 collections, 128 champs par liste,
100 enregistrements par collection, 64 onglets et 64 sections par formulaire.
Les défauts textuels sont limités à 16 000 octets, les listes de mots-clés à
100 éléments de 1 000 octets chacun. Les contrôles byte/relations exposés dans
les schémas de valeurs avec `x-*` sont des annotations : JSON Schema ne les
transforme pas en règles métier exécutables.

## Vérification et génération

```sh
cargo test --locked --offline -p typstographe-core document_types
cargo run --locked --offline -p typstographe-core --example type_manifest_schema > docs/schemas/type-definition.schema.json
```

Les tests contrôlent le fragment auteurs/affiliations, les défauts de brouillons,
les références de vues et de liaisons, les politiques de relations, les défauts
riches et leurs profils, les cas YAML ambigus, le schéma généré et la fermeture
de ses références récursives. Le schéma des valeurs peut être utilisé comme
contrôle de structure ; il ne suffit pas à autoriser la publication d’une édition.

La recette vérifie les codecs des trois types, les diagnostics d’export et la
conservation à l’octet près des sources de démonstration et de brouillon dans
`fixtures/document-types/source-contract.json`. L’exemple local de travail est
également ouvert en lecture seule pendant le contrôle ; il n’est ni régénéré ni
modifié. Les parcours Chromium/WebKit couvrent les en-têtes, formulaires,
relations, conflits, citations, sauvegardes et exports, sans captures.

Les changements des YAML intégrés déclenchent les contrôles CI et les recettes
ciblées des types/en-têtes/bibliographies, y compris ceux de
`crates/core/structures/`. Le schéma publié et la fixture YAML
testée sous docs/ sont des entrées de vérification, pas de simples documents à
exclure des contrôles. La qualification CI des trois systèmes et de WebAssembly
reste distincte de ces résultats locaux.

Résultats locaux du raccordement : 429 tests Rust du workspace réussis, complétés
par le test de parité LF/CRLF (un diagnostic de performance reste ignoré),
418 tests web et six tests Node réussis, 40 parcours
Chromium/WebKit réussis, 17 contrôles Python de politique CI/architecture réussis.
Clippy, formatages, limite de taille et frontière des dépendances du cœur passent.
Le watcher de fichiers et les tests de sockets MCP nécessitent les notifications
et sockets natifs macOS ; leurs essais finaux sont exécutés hors du bac à sable,
avec les répertoires temporaires propres aux recettes.

Recette de mutualisation `authorship` : 436 tests Rust du workspace réussis
(un diagnostic de performance ignoré), puis 29 tests du chargeur rejoués après
l’ajout des assertions d’ordre des liaisons et d’obligation d’export. Les
12 parcours auteurs/affiliations dans Chromium et WebKit et les 17 contrôles
Python passent, ainsi que Clippy du workspace, le formatage et les gardes
d’architecture. Le fichier local `examples/article-working.typsto` s’ouvre avec
le nouveau moteur et sa somme de contrôle est inchangée. Aucun test web unitaire
supplémentaire n’est ajouté : les composants de l’interface sont conservés.

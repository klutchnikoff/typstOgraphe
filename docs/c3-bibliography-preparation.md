# C.3 — Préparation commune des notices et des imports

Bilan historique du premier lot du 6 novembre 2026, après la [fondation bibliographique](c3-bibliography-foundation.md).
Les trois requêtes suivantes utilisent le même contrat dans le cœur, la CLI et
MCP, en session autonome ou rattachée à l'application. Elles nécessitent un
projet Article ouvert et ne publient aucune édition.

| Requête | Paramètres | Résultat |
| --- | --- | --- |
| `bibliography.profiles` | Aucun | Profils calculés, champs et structures de création communs |
| `bibliography.prepare_record` | `id`, `profile`, `entry` natif facultatif, `key` facultative | Notice proposée, diagnostics de complétude, empreinte |
| `bibliography.prepare_import` | `id` de source, `name` d'affichage, `format` (`hayagriva` ou `biblatex`), `contents` UTF-8 | Notices proposées, diagnostics de conversion, résumé de source et empreinte |

Les paramètres et résultats possèdent des schémas JSON accessibles par la
description des opérations ; `result_schema_scope: query_outcome` désigne le
résultat métier, avant l'enveloppe du transport. MCP annonce les requêtes en
lecture seule et fournit des exemples. Le catalogue statique et la référence CLI
sont régénérés depuis le contrat exécuté.

## Utilisation et garanties

Exemple CLI, depuis la racine après compilation du binaire CLI :

```sh
target/debug/typstographe-cli --json query bibliography.prepare_record article.typsto \
  --params '{"id":"proposed-book","profile":"book","entry":{"title":"Un ouvrage","author":"Martin, Camille"}}'
```

Sans `key`, la préparation suggère un nom disponible dans la bibliothèque
actuelle. Une clé explicitement occupée est refusée. `entry` décrit des champs
natifs superposés à la structure initiale du profil ; cette requête prépare une
création, pas une modification partielle d'une notice existante.

Les deux préparations renvoient `active: false`. Elles ne changent ni les
références actives, ni le contexte d'édition, la révision, l'historique ou l'état
modifié du document. Elles n'enregistrent pas le projet. La CLI peut les exécuter
en lecture sous le verrou d'un autre rédacteur. La source est fournie en texte,
jamais lue depuis un chemin communiqué par l'agent. Le résultat d'import contient
le nom, le format et le digest de la source, sans répéter son contenu original.

Les identités des notices préparées à l'import sont déterministes pour le même
identifiant de source, les mêmes octets et la même clé. Ce sont des identités de
proposition, **pas des cibles de citation actives ni des preuves d'identité
bibliographique**. L'empreinte couvre la préparation, dont ses diagnostics et
l'original ; elle n'autorise aucune écriture et ne représente pas la révision du
projet. La future publication vérifiera séparément l'état documentaire et les
décisions de rapprochement.

## Conservation des données natives

Les valeurs composées et les parents restent natifs. Les tests comparent leur
sens après modification d'un autre champ, sérialisation JSON et réouverture du
Hayagriva projeté par le lecteur Hayagriva lui-même.

Une correction protège notamment la casse littérale d'un titre : le lecteur
Hayagriva 0.10.1 peut ignorer `verbatim` lorsqu'il est voisin de `value` dans la
forme abrégée d'un texte formaté. La crate normalise ce drapeau vers le texte
interne avant décodage, en conservant le titre court. Le sérialiseur Hayagriva
continue de produire le YAML ; l'original importé reste identique octet par octet.

Un même contrôle repère les champs inconnus à la racine, dans les parents et dans
les structures prises en charge de texte formaté, personnes et éditeur. Le
décodage strict d'une notice les refuse avec un chemin ; l'import les signale
comme conservés seulement dans l'original. Ce contrôle ne prétend pas qualifier
toutes les pertes possibles de conversion BibLaTeX ou tous les composites natifs.

Les erreurs conservent leurs codes, clés et chemins dans les réponses machine.
Leur présentation humaine passe par Fluent en français et anglais ; une langue
non disponible utilise le repli anglais de cette présentation.

## Vérification et suite

Les tests couvrent les schémas, les genres, les collisions de clés, les sources
invalides ou trop grandes, les champs inconnus, la conservation des valeurs
composées, la répétabilité et l'absence de mutation. Les parcours CLI et MCP
vérifient les mêmes résultats et l'intégrité du fichier ou de la session. Les
contrôles du workspace et de la CI sont exécutés avec ce lot. Aucun test graphique,
mesure de couverture ou compilation WASM n'est annoncé : l'interface ne change
pas et la cible standard WASM n'est pas installée localement.

Résultats locaux : **44 tests bibliographiques, 284 tests du cœur, 44 tests CLI,
14 tests MCP, 13 tests i18n et un doctest du cœur**, soit **400 réussites**.
Le dernier test de champs natifs imbriqués est vérifié par une passe ciblée de
la crate après la passe des cinq packages. Les **20 tests Python** des contrôles
de CI passent, ainsi que Clippy du workspace, toutes cibles, avec `-D warnings`,
le format Rust, la limite de lignes de production et la frontière des dépendances
du cœur. Les tests de transport local sont exécutés hors sandbox. Aucun paquet
du registre n'a changé de version.

Restent à implémenter : l'état structuré des notices et les commandes de
publication annulables, les citations par `RecordId`, le Hayagriva actif unique
dans le CAS et l'export, puis le [gestionnaire validé](c3-bibliography-interface.md).
Le format CAS et les cibles actuelles des citations sont inchangés dans ce lot.
Les fichiers `.typsto` locaux sont conservés intacts ; aucun exemple n'est régénéré.

Le [lot documentaire suivant](c3-bibliography-document-state.md) ajoute la publication
annulable, les citations par identité et le CAS 15 ; ses états et commandes
remplacent les limites historiques décrites ci-dessus.

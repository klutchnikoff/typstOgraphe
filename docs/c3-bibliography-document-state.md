# C.3 — Notices documentaires, citations par identité et CAS 16

Lot du 6 novembre 2026, après les [préparations communes](c3-bibliography-preparation.md).
Le même état et les mêmes éditions sont utilisés par Tauri, CLI et MCP. Le
[gestionnaire complet](c3-bibliography-manager.md) est raccordé. La
[classification native](c3-native-bibliography-profiles.md) remplace le genre indépendant.

## État et publication

L’Article porte deux collections enregistrées, sans collection de fichiers actifs :

- `references` (`bibliographic-records`) : notices natives complètes, identités,
  parents inactifs conservés, clés et provenance ;
- `reference_sources` (`bibliographic-sources`) : originaux UTF-8 immuables,
  nom d’origine, format et digest vérifié à la désérialisation.

Yjs/Yrs conserve une map par notice/source et un ordre distinct. Les éditions
ne remplacent pas la collection entière ni les branches de texte ou d’auteurs.
Les champs natifs restent des valeurs complètes : parents, personnes, chaînes
formatées et identifiants non exposés par le futur formulaire sont conservés.
Un changement de type prépare une structure native ; les conteneurs devenus
inactifs restent conservés avec la notice, hors de la projection CSL active.

Les opérations génériques `record.add/remove/move` et `metadata.set` ne peuvent
pas contourner les éditions bibliographiques de ces collections.

| Opération commune | Effet |
| --- | --- |
| `bibliography.inspect` | Notices actives, empreinte de bibliothèque et résumés des sources |
| `bibliography.record` | Notice par `id`, empreinte et diagnostics de complétude |
| `bibliography.prepare_record` | Préparation d’une création, sans publication |
| `bibliography.prepare_import` | Conversion et diagnostics, empreinte du rapport, plan et empreinte de bibliothèque |
| `bibliography.apply` | Une édition gardée et annulable, sans sauvegarde immédiate |

`bibliography.apply` exige `expected`, l’empreinte courante de bibliothèque.
Son `change.kind` vaut `create`, `edit`, `replace`, `rename`, `remove` ou `import`.
`edit`, `replace`, `rename` et `remove` exigent aussi l’empreinte de la notice ; `edit` porte
les patches de champs natifs et éventuellement un profil demandé, qui modifie
la classification native. `replace` publie la notice complète préparée par le
formulaire en préservant son identité et sa provenance. La création manuelle
ne peut pas attribuer une provenance d’import : elle vient du plan d’import.

Un import fournit la source explicite, `preparation` (empreinte du rapport relu)
et les décisions ciblées `same`, `distinct` ou `skip`. Le moteur reconstruit son
plan à partir des entrées validées ; aucun plan désérialisé n’est traité comme
une instruction de mutation fiable. Base périmée, rapport différent, conflit
non résolu, clé occupée ou modification d’une source existante : le lot est refusé
sans publication partielle. Le rapprochement garde l’identité du destinataire
existant ; les identités candidates du rapport ne deviennent donc pas toutes
des cibles actives. Relire `inspect` ou `search` après publication.

La source d’une notice retirée peut quitter l’état courant quand plus aucune
notice ne la cite comme provenance. Annulation et versions conservent leurs
propres états. Une répétition du même import ne duplique ni notice ni provenance
et ne crée pas d’étape d’annulation lorsqu’elle ne change rien.

## Citations et composition

Les éléments de citation portent `record_id` et un supplément littéral facultatif,
plus `form` au niveau du groupe. Le générateur Rust résout la clé effective depuis
la bibliothèque pour produire le source Typst et sa carte de synchronisation.
Modifier une clé ne réécrit donc pas les badges. Les appels de rendu, exports,
aperçus et validation des propositions MCP utilisent tous ce contexte documentaire.

Une notice encore citée ne peut pas être supprimée de l’état final. Retirer ses
citations et la notice dans le même lot est possible. Ajouter une notice et son
badge dans un même lot donne une seule édition annulable, sans notice orpheline
si le lot est refusé. Les libellés temporaires ne montrent pas l’identité opaque
pendant le chargement de l’index.

## Ressources autonomes

Le conteneur passe à **CAS 16**, générateur de source **5**, sans lecteur des
anciens formats de développement. Chaque état vérifie la cohérence entre
métadonnées, checkpoint partagé, sources dérivées et ressources bibliographiques.

```text
bibliography/references.yaml            # seul Hayagriva actif, dérivé des notices
bibliography/retained.json              # conteneurs inactifs, hors du rendu
bibliography/sources/<source-id>.bib    # original BibLaTeX exact
bibliography/sources/<source-id>.yaml   # original Hayagriva exact
```

Les identités, conteneurs inactifs et liens de provenance restent dans les métadonnées,
jamais dans de faux champs Hayagriva. Le modèle reçoit seulement le chemin du
Hayagriva consolidé. L’export Typst garde les originaux à part et fonctionne
sans bibliothèque personnelle ou fichier externe. Récupération, versions,
comparateur et propositions utilisent les mêmes états CAS.

Les exemples versionnés et la fixture native ont été régénérés depuis leur
contenu Git (sans snapshots dans ces entrées). Les fichiers locaux modifiés
`examples/article-working.typsto` et `examples/article-mathematics.typsto` sont
restés intacts, avec leur historique local ; ils ne sont pas convertis
silencieusement. Le nouvel exemple ouvrable est
[`examples/article-bibliography-profiles.typsto`](../examples/article-bibliography-profiles.typsto).

## Adaptateurs et vérification

Le panneau d’en-tête existant utilise maintenant les notices : import après
revue, refus visible si le plan demande un arbitrage, liste de clés et suppression
protégée. Le sélecteur de citations, ses raccourcis, les badges et les aperçus
utilisent les identités actives. Le [lot suivant du gestionnaire](c3-bibliography-manager.md)
raccorde les formulaires, arbitrages interactifs et brouillons, avec les
composants et la liste compacte validés.

Le compactage des schémas est mutualisé dans le cœur, puis réutilisé par MCP.
Le catalogue n’embarque plus les définitions inutilisées dans chaque opération ;
les tests conservent les références récursives et les mêmes entrées acceptées
ou refusées. La recette navigateur conserve aussi les erreurs structurées du
transport, au lieu de les transformer en « [object Object] ».

Vérifications locales :

- 409 réussites Rust dans les cinq packages communs ; 131 tests natifs Tauri
  réussis, un diagnostic préexistant ignoré ;
- 483 tests web, 10 tests des scripts web natifs, 31 tests de contrat Yjs/Yrs et
  10 parcours navigateur réussis sur Chromium et WebKit ;
- 20 tests des scripts Python ; Clippy du workspace, toutes cibles avec
  `recipe-driver` et `-D warnings`, format Rust/Prettier, limite des lignes de
  production et frontière des dépendances du cœur vérifiés.

Les tests couvrent création avec citation atomique, renommage sans perte de cible,
conflits et rapports périmés, originaux immuables, champs composés conservés,
réimport, suppression protégée, annulation/rétablissement, CAS, reprise et export.
La recette distingue aussi avertissement de complétude et perte de conversion :
une notice incomplète reste importable après revue. Les tests MCP couvrent une
publication native jusqu’à la compilation. Les tests
Yjs/Yrs couvrent également l’acceptation d’une proposition dans l’Article.
Aucune capture, mesure de couverture ou mesure de latence n’est produite.
La cible standard WASM n’est pas installée localement ; sa compilation n’est
pas annoncée vérifiée. Aucun paquet du registre n’a changé de version.

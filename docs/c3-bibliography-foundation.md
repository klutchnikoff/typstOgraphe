# C.3 — Premier lot : crate bibliographique commune

Bilan du premier lot, complété par la [classification native](c3-native-bibliography-profiles.md).

Implémenté le **6 novembre 2026** sur `feat/c3-bibliography-core`.
Le [contrat cible](c3-bibliography-contract.md) et la
[présentation validée](c3-bibliography-interface.md) restent les références.
Ce lot livre le moteur pur et un premier raccordement ; C.3 n'est pas clôturé.

## Livré

`crates/bibliography` fournit `typstographe-bibliography`, avec la dépendance
`core → bibliography → hayagriva`. Les notices conservent le modèle natif,
leurs identités distinctes des clés, leur genre de formulaire et leur provenance.
Les identités sont fournies par l'appelant, sans dépendance à l'aléatoire système.
Les modèles Typst et la présentation de l'éditeur restent inchangés.

- Imports BibLaTeX/Hayagriva limités avant lecture, originaux UTF-8 conservés
  intacts, digest des octets d'origine, identités/clés vérifiées, diagnostics
  typés. Les notices incomplètes restent valides.
- Formulaires décrits par genre, y compris prépublication et logiciel ; ces
  descripteurs ne remplacent pas les structures natives importées. Les valeurs
  composées (titre formaté, personnes, éditeur, URL, identifiants) restent typées.
- Recherche fuzzy Unicode titre/auteurs, résumés communs et suggestion de clé
  disponible. La requête est normalisée une seule fois, hors de la boucle des
  notices. Les noms proposés ne sont jamais une preuve d'identité.
- Édition en mémoire de champs natifs avec empreinte attendue, conservation des
  champs non touchés et renommage de clé sans changement d'identité.
- Indices DOI/arXiv, auteur/année/début du titre et auteur/titre ; réfutation et
  vérification du groupe entier, sans fusion par transitivité ni score fuzzy.
- Plans immuables : nouvelles notices, inchangées, enrichissements ou conflits ;
  décisions ciblées même œuvre/distincte/exclure. L'enrichissement comble les
  lacunes et signale les valeurs différentes sans les remplacer.
- Application atomique **en mémoire**, refus d'une base périmée ou d'un conflit
  restant. Exclure un candidat réévalue les autres ; cycles de décisions et
  cibles non candidates refusés. Ordre des candidats sans effet sur la composition.
- Projection Hayagriva déterministe ; aucune écriture sur disque dans la crate.

Le cœur délègue déjà lecture, synthèse des notices et score de recherche à la
crate. Le cache de fichiers et les règles documentaires restent dans le cœur.
Ses dépendances directes à Hayagriva et `strsim` sont remplacées par la crate.
Les opérations existantes CLI/MCP bénéficient de ce raccordement, sans nouvelle
commande ni changement silencieux de cible.

## Vérification et limites

Les **24 scénarios d'identité du corpus** sont devenus des tests Rust nommés,
avec une assertion pour chaque attente déclarée ; une attente inconnue fait
échouer le test. Les scénarios ne lancent pas Python et ne lisent pas Citekeep.
Sa référence et sa licence MIT restent dans les fixtures.

Les tests complémentaires vérifient originaux et provenance, champs/parents/rôles
non exposés, texte formaté, champs invalides, recherche Unicode, doublons
d'identités/clés, garde de révision, absence d'application partielle, décisions
cycliques, permutations et répétition d'un import avec de nouvelles identités.
Les tests du cœur conservent la compilation des citations, les trois styles,
l'export autonome et les géométries de synchronisation. La CI sélectionne la
recette bibliographique si le moteur partagé ou son raccordement change.

Vérifications locales du lot :

- **40 tests** de la crate, dont les 24 scénarios du corpus.
- **278 tests** du cœur, **41 tests** CLI, **13 tests** MCP et un doctest du
  cœur : **373 réussites** pour la commande des quatre packages.
- **20 tests** des contrôles Python ; format Rust, contrôle des dépendances et
  limite de 800 lignes de production vérifiés.
- Clippy du workspace entier, toutes cibles, avec `-D warnings`, réussi.

Les tests CLI/MCP utilisant des sockets locaux ont nécessité une relance hors
de la sandbox après un refus `Operation not permitted` ; cette relance a passé
l'ensemble ci-dessus. Aucun test graphique ni mesure de couverture n'a été
relancé pour ce lot sans modification de l'interface. La cible standard WASM
n'est pas installée dans cette toolchain locale ; aucune compilation de cette
cible n'est annoncée vérifiée ici. Aucun paquet du registre n'a changé de version.

La conversion BibLaTeX n'est **pas certifiée sans perte** : intervalles de dates,
macros/champs particuliers, version de logiciel et intention de genre sont
signalés pour revue. `@software` reste natif `misc` après conversion ; le genre
applicatif est conservé séparément. Aucun `repository` n'est inféré logiciel à
partir du seul type. Les correspondances éditables et leur rendu devront être
qualifiés au raccordement des formulaires. Les matrices de cas futurs du corpus
restent distinctes des 24 scénarios exécutés.

Les descripteurs décrivent les structures de création ; les notices importées
peuvent placer certains champs dans d'autres parents ou les hériter. Le futur
codec de formulaire doit lire leur structure réelle et préserver les métadonnées
des valeurs composées, sans reconstruire une notice à partir des champs visibles.

## Prochains raccordements

1. État structuré documentaire, commandes et requêtes communes ; citations
   ciblées par `RecordId`, générations de clés et gardes intégrées à l'historique.
2. Un seul Hayagriva actif dans le CAS et l'export, originaux de provenance,
   cohérence à la lecture, récupération/snapshots ; décider la nouvelle révision
   du format et régénérer uniquement les exemples versionnés.
3. Gestionnaire validé avec liste compacte à deux lignes, formulaires et imports,
   brouillons récupérables, suppression protégée et Créer et citer atomique.
4. Recette GUI/CLI/MCP et clôture C.3. Le stockage personnel et les échanges
   projet/personnel restent C.4, en réutilisant la même crate.

Les `.typsto` locaux n'ont pas été convertis, régénérés ou ajoutés aux commits.
Aucune capture n'a été produite.

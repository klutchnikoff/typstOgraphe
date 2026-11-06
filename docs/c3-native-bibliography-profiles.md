# C.3 — Profils déduits de Hayagriva

Décision du 6 novembre 2026, après la première livraison du gestionnaire.

## Une seule classification

La classification enregistrée est la structure native Hayagriva : type de
l’entrée et relations bibliographiques actives. Supprimer le `genre` indépendant
des notices. Les profils organisent les formulaires et leurs recommandations ;
ils sont déduits de cette structure, jamais une deuxième classification à sauver.
Un article de revue est notamment `article` avec un parent `periodical`, une
communication publiée `article` avec un parent `proceedings`.

Le choix d’un profil lors d’une création ou d’un changement de type prépare une
structure native correspondante. Un profil déjà adapté à la structure importée
ne la normalise pas arbitrairement : conserver notamment les variantes natives
et les parents multiples. Les données natives non exposées restent conservées.

## Changement de type et conservation

Changer de type dans le formulaire reste privé jusqu’à Valider. Conserver les
champs communs, les valeurs masquées, l’identité de notice, la clé et la provenance.
Une saisie native invalide refuse la transition et conserve les contrôles, sans
publier ni effacer leur contenu. La complétude n’est pas une obligation.

Conserver avec la notice les anciens conteneurs devenus inactifs, sans les
transmettre comme publications actives au moteur CSL. Une revue ne devient pas
implicitement un titre d’actes. Restaurer le conteneur correspondant au retour à
son type, avec ses données et ses propres champs composés. Une nouvelle relation
sans conteneur correspondant commence vide. Les valeurs héritées communes restent
disponibles quand leur conteneur quitte la projection active.

Ces données conservées sont du contenu natif, pas une seconde classification ni
une seconde bibliothèque active. Elles participent aux empreintes, à l’état
partagé, à l’annulation, aux versions et à la récupération. Les exports Typst
conservent ces données séparément de la bibliothèque utilisée par le rendu.
Le style CSL et Typst restent responsables de la composition des références ;
masquer un contrôle dans l’interface ne suffit pas à neutraliser un parent actif.

## Contrat commun et qualification

Le codec et les changements de classification appartiennent à la crate commune.
Les requêtes exposent les profils calculés et les préparations ; les adaptateurs
utilisent les mêmes opérations gardées pour publier la notice native complète ou
éditer ses champs/classification explicitement. Une validation fait une seule
édition annulable, sans sauvegarde implicite ni changement de cible de citation.

Tester les transitions aller-retour, les parents multiples et inactifs, les champs
composés, la provenance, les modifications non validées, les refus concurrents,
les garde-fous de schéma, la récupération, le CAS et le rendu CSL/Typst effectif.
Le format documentaire peut évoluer dans cette phase ; préserver les fichiers
locaux et fournir un nouvel exemple plutôt que les convertir implicitement.

## Livraison et qualification

Implémenté sur `feat/c3-bibliography-core`. `Record` ne contient plus de genre
indépendant ; `Profile::infer` lit la structure native. Les contrôles et les
recommandations utilisent les chemins réels des champs, y compris volume/numéro
hérités d’une publication. Les variantes importées ne sont pas normalisées lors
d’une simple consultation. Un enrichissement ne copie pas un parent incompatible
avec le type existant dans son graphe actif ; il en conserve les données à part.

`bibliography.profiles` décrit les profils disponibles. `bibliography.form` et
`bibliography.prepare_form` retournent le profil calculé ; le second peut préparer
un changement natif avec `profile`. Le formulaire conserve une notice de travail
privée et valide toutes ses transitions en un `bibliography.apply` de type
`replace`, gardé par l’empreinte initiale. Cette opération exige la même identité
et la même provenance. Les scripts peuvent aussi demander un changement natif
par `edit.profile`. Un `replace` porte une notice complète préparée/lue : ne pas
construire un objet partiel qui omettrait les données natives conservées.

Le CAS 16 contrôle l’état complet, la projection active et les données conservées.
Aucun lecteur des anciens formats n’est ajouté. La fixture de tests a été
régénérée depuis son contenu Git ; les recettes natives utilisent désormais
cette fixture plutôt qu’un document d’exemple potentiellement modifié localement.
[`article-bibliography-profiles.typsto`](../examples/article-bibliography-profiles.typsto)
est le nouvel exemple, reproductible par sa
[recette CLI](../examples/cli/article-bibliography.json). Les exemples locaux
`article-working.typsto` et `article-mathematics.typsto` restent intacts.

Vérifications locales :

- 56 tests dans la crate bibliographique, dont les transitions, les variantes,
  les champs hérités et un véritable traitement CSL qui distingue revue/livre/actes ;
- suites communes Rust, dont les 295 tests du cœur, et 131 tests natifs Tauri
  réussis (un diagnostic préexistant ignoré) ; préparation privée, gardes,
  annulation/rétablissement, CAS, export et provenance vérifiés ;
- 483 tests web, 10 tests des scripts natifs web et 31 tests de contrat Yjs/Yrs ;
- parcours bibliographiques Chromium/WebKit : changements de type, champs
  masqués, publications distinctes, édition atomique, citations stables,
  concurrence, refus d’une date invalide et reprise d’un formulaire transformé ;
- Clippy du workspace, toutes cibles avec `recipe-driver`, formats Rust/Prettier,
  limite des lignes de production et frontière des dépendances du cœur.

Aucune capture ni mesure de couverture ou de latence n’est produite.

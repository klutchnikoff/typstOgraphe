# C.3 — Gestionnaire bibliographique du projet

Implémentation du **6 novembre 2026**, suivant la
[présentation validée](c3-bibliography-interface.md). L’onglet Bibliographie
conserve le choix du style du modèle et ouvre « Gérer les références ».

## Parcours

La grande modale reprend le fond atténué/flouté commun. Sa colonne gauche
présente une recherche fuzzy sur titres/auteurs, le nombre de résultats,
l’import et le bouton +. Chaque notice occupe deux lignes, avec ellipse sur le
titre ; la liste et le formulaire défilent indépendamment. Les flèches gardent
le focus dans la recherche ; Entrée rejoint le formulaire sélectionné.

À droite, les champs sont décrits par les genres de la crate bibliographique :
notice, publication, puis identifiants/URL/clé dans une section dépliable.
Les personnes utilisent la poignée commune, à la souris ou au clavier
(Espace, flèches, Entrée/Échap). La complétude reste un conseil, jamais une
condition de publication. Les profils sont déduits du type et des parents Hayagriva. Le sélecteur
« Type de notice » est modifiable à la création et à l’édition : il prépare une
classification native sans publier la saisie, conserve les champs masqués et
restaure les publications parentes correspondantes. Une revue n’est jamais
réutilisée comme titre d’actes. Voir le [contrat natif](c3-native-bibliography-profiles.md).

Valider produit une édition annulable, sans enregistrer le fichier. Une fermeture
ou un changement de notice avec une saisie modifiée propose Abandonner,
Continuer l’édition, Valider et poursuivre. Ctrl/Cmd+Entrée valide ; la
préédition IME bloque cette décision. Les opérations asynchrones désactivent
leurs commandes et les champs concernés pendant la préparation/publication.

Les formulaires et les décisions d’import utilisent la récupération privée
commune. La reprise réaffiche la saisie, sans la publier ni consommer sa copie
avant validation ou abandon explicite. Une modification concurrente de la même
notice refuse l’écrasement et conserve le formulaire.

Depuis le bandeau de citation, « Créer une notice… » réutilise cette modale.
« Créer et citer » publie un lot atomique : notice puis objet citation, avec les
références, suppléments et forme déjà choisis conservés. Une annulation retire
les deux effets. Si la cible documentaire a changé, la saisie reste privée.
La suppression d’une notice citée reste refusée ; « Voir les citations » rejoint
sa première occurrence, y compris dans un widget du corps.

## Contrat et conservation

Le codec de formulaire appartient à `typstographe-bibliography`, sans dépendance
à l’interface. Il suit les véritables parents natifs et ne transforme que les
contrôles modifiés en patches de champs. Il conserve les titres courts, les
segments formatés non modifiés, les préfixes/alias des personnes, les métadonnées
de l’éditeur, la date de consultation, les identifiants et données non exposés.
La réorganisation des personnes transporte leurs métadonnées avec leur identité
privée de ligne. Une publication manquante peut être complétée sans remplacer
les parents ou champs déjà présents.

Les requêtes communes, également découvertes par CLI/MCP, sont :

| Requête | Résultat privé, sans publication |
| --- | --- |
| `bibliography.profiles` | Profils de formulaire calculables et champs natifs |
| `bibliography.form` | Contrôles simples d’une notice native figée |
| `bibliography.prepare_form` | Notice proposée, patches et diagnostics ; clé disponible suggérée pour une nouvelle notice |
| `bibliography.browse` | Résultats compacts, sans la limite de 80 suggestions du sélecteur de citations |
| `bibliography.usage` | Cibles typées des citations dans le corps, widgets compris |
| `bibliography.review_import` | Plan recalculé avec décisions explicites et empreinte de bibliothèque figée |

La publication reste exclusivement `bibliography.apply`, éventuellement en lot
avec `inline.replace`. Une édition peut modifier les champs, la classification native et la clé sous
la même garde de notice. Le formulaire publie son résultat natif complet via
`replace`, qui conserve identité et provenance. Le profil ne constitue aucun
champ de classification enregistré ; le code Typst et la présentation CSL ne
sont pas réinventés dans les contrôles web.

L’import BibTeX/Hayagriva affiche candidats, champs complétés, valeurs conservées,
avertissements de conversion et original en lecture seule. Les choix sont
rapprochement proposé, référence distincte avec clé, même référence ou exclusion.
Une décision manquante/invalide, un plan périmé ou un conflit non résolu empêche
l’import ; aucun rapprochement n’est automatiquement rebasé. Les originaux
restent dans le CAS suivant le contrat du lot documentaire.

## Qualification

Les tests du codec vérifient la conservation des champs composés, parents réels,
valeurs non exposées et opérations sans effet. Les tests du cœur vérifient la
préparation privée, l’édition/renommage atomique, les gardes périmées, les cibles
imbriquées et les décisions d’import.

Les parcours Chromium et WebKit couvrent import multi-format et arbitrage,
recherche/badges/rendu/export natif, suppression protégée, création/annulation,
réorganisation au clavier, isolation IME, concurrence, reprise après fermeture,
création avec citation groupée et liste de 92 notices avec titres longs,
recherche Unicode et défilements indépendants. Les assertions de géométrie ne
remplacent pas une recette visuelle humaine ; aucune capture n’est créée.

Bilan du premier raccordement (voir aussi la [qualification des profils natifs](c3-native-bibliography-profiles.md#livraison-et-qualification)) : 416 réussites dans les packages Rust communs, 131 tests
natifs Tauri réussis (un diagnostic préexistant ignoré), 483 tests web et 10 tests
des scripts natifs web, 31 tests de contrat Yjs/Yrs, 24 parcours Chromium/WebKit
et 20 tests Python. Clippy du workspace avec toutes les cibles et
`recipe-driver`, formats Rust/Prettier, limite des lignes de production et
frontière de dépendances du cœur sont vérifiés. Le build web de production
réussit ; son avertissement préexistant sur la taille des chunks reste présent.
Aucune mesure de couverture ou de latence n’est annoncée pour ce lot.

C.4 ajoutera le stockage personnel et les échanges explicites, en réutilisant
ce gestionnaire et cette crate. Pas de résolution DOI en ligne, d’import PDF ou
de réécriture des notices depuis un service distant dans ce lot.

# Couverture après retrait du moteur egui

Analyse du 29 octobre 2026, comparant `0f79046` à `d85319b`. Les deux rapports
proviennent de la CI Linux avec le même périmètre : workspace Rust, code des tests
exclu, code de production conservé dans la mesure. Les tests navigateur et les
recettes exécutées dans des processus séparés ne sont pas une mesure de couverture
Rust des gestionnaires graphiques.

## Résultats mesurés

| Périmètre | Avant | Après le premier retrait |
| --- | ---: | ---: |
| Workspace | 84,05 % — 15 858 / 18 867 lignes | 77,92 % — 9 265 / 11 891 lignes |
| Cœur | 91,43 % | 88,04 % |
| CLI | 90,99 % | 90,99 % |
| MCP | 82,60 % | 82,60 % |
| Internationalisation | 73,72 % | 82,86 % |
| Adaptateur Tauri | 50,89 % | 50,67 % |

Les fichiers entièrement retirés représentaient **6 592 lignes mesurées**, dont
**6 143 couvertes**, soit 93,19 %. Retirer un moteur très couvert augmente donc
le poids relatif des parties moins couvertes dans le total. Le reste de l’écart
de dénominateur vient des fonctions supprimées dans des fichiers conservés et
des extractions vers de nouveaux modules.

Ce n’est toutefois pas une explication suffisante à chaque perte de couverture.
L’examen des lignes conservées a révélé des méthodes publiques sans appelant :
ancien import/insertion d’images, mutation directe des métadonnées, signatures
d’affichage, réparation des langues, fusion de dépendances et ancien surlignage
par plage continue. Les tests supprimés du moteur egui étaient leurs derniers
utilisateurs. Elles auraient dû disparaître avec lui.

Les rapports et leur comparaison sont disponibles localement dans
`output/coverage-flat-retirement-2026-10-29/` (ignoré par Git). Le rapport distant
complet est attaché à [la mesure de cette révision](https://github.com/klutchnikoff/typstOgraphe/actions/runs/40855800057).

## Suite du nettoyage et garanties

La seconde passe retire ces méthodes et les accesseurs mutables des descriptions
de champs. Les commandes structurées restent le chemin d’édition documentaire.
La création d’un résumé dans la nouvelle langue est déjà vérifiée via les
commandes communes, avec conservation des données et de l’historique.

Deux tests sur des comportements actuels complètent les recettes conservées :

- compilation réelle avec choix de puces figé, liste autorisée, choix personnalisé
  ou automatique ; un choix refusé n’empêche pas la compilation valide suivante ;
- export PDF seul, Typst seul ou combiné avec image ; présence des fichiers
  attendus, refus d’une destination occupée, d’un fichier et, sur Unix, d’un lien
  symbolique, avec conservation des données préexistantes.

Les parties natives les moins couvertes comprennent les commandes Tauri de
`main.rs`, le pilote de recettes, les dialogues de fichiers et les diagnostics.
La persistance et l’acceptation MCP ont leurs tests indépendants des fenêtres.
Les compléments utiles restent notamment la navigation géométrique sur les
objets graphiques et certains parcours de dialogue natif. Le périmètre de la
mesure reste inchangé ; le prochain taux doit être lu après ces changements.

## Échec Tauri : sélection entre paragraphes

Les deux dernières CI échouaient sur le même parcours Chromium/Linux ; les
253 autres parcours passaient. Le parcours concerné traverse deux paragraphes
avec Maj + Flèche droite puis remplace la sélection.

Le décalage a été reproduit entre sélection DOM et état ProseMirror :
`selectionchange` peut encore attendre sa publication quand le prochain
`keydown` arrive. Le franchissement du symbole ¶ utilisait alors une ancienne
position et pouvait réinitialiser une sélection déjà étendue.

Le gestionnaire commun lit maintenant la sélection DOM courante lorsque
l’éditeur a le focus, via les positions publiques de ProseMirror. Il conserve
l’ancre et ne prend en charge que le franchissement de paragraphes ; les
raccourcis par mot, les déplacements internes et l’IME gardent leur traitement.
Deux tests déterministes reproduisent le retard dans le corps et une zone riche,
y compris dans le sens inverse. Ils échouaient avant le correctif. Un essai
Chromium avec processeur ralenti six fois traverse également les paragraphes
sans réinitialisation.

Validation locale après cette passe : **259 tests Rust**, **210 tests web et
3 tests de langues natives**, **20 recettes Rust ↔ Yjs**, **6 parcours
Chromium/WebKit** sur la sélection et les contextes privés. Clippy workspace/toutes
cibles avec `-D warnings`, constructions web et native, rustfmt, Prettier, limites
de taille, frontière des dépendances et les 9 tests des gardes passent. La
validation Linux du correctif et la nouvelle mesure dépendent de la CI suivante.

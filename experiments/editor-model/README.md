> Bilan historique : le code du diagnostic egui a été retiré le 27 octobre 2026.
> Les anciennes commandes ci-dessous documentent les essais passés.

# Prototype de décision du modèle d’édition — phase 1.6

Ce diagnostic réutilise le vrai `Projection` et son adaptateur `egui::TextBuffer`.
Il est exclu des builds par défaut par la fonctionnalité Cargo `editor-model-spike`.
Il n’ajoute ni tableaux, ni commandes, ni format de stockage à l’application.
Aucune dépendance n’a été ajoutée.

```sh
# Ouvrir le diagnostic natif.
cargo run --locked --features editor-model-spike --example editor-model-spike

# Six essais ciblés, y compris les événements natifs egui de Tab et d’IME.
cargo test --locked --features editor-model-spike --example editor-model-spike model_spike

# Photographier le diagnostic puis fermer sa fenêtre.
cargo run --locked --features editor-model-spike --example editor-model-spike -- output/table-spike.png
```

## Ce qui est essayé

Un tableau à deux colonnes occupe un seul tampon : tabulations entre cellules,
retours entre lignes. Le rendu conserve une seule sélection native et une navigation
par Tab / Maj+Tab. Les boutons insèrent ou retirent une ligne ou une colonne.
L’historique du diagnostic restaure la projection, le nombre de colonnes et la
sélection ; il ne remplace pas `Session` dans l’application.

Les essais automatisés vérifient les indices Unicode, la navigation sans étape
d’annulation, les changements de structure et leur historique, le remplacement
d’une sélection traversant deux cellules, le texte accentué, puis une préédition
et une validation IME japonaises. Le source de tableau généré est compilé par
le moteur Typst existant et le modèle personnel ; le projecteur de production
le refuse encore, puisque cette capacité n’y est pas implémentée.

## Résultats et limites

- Le tampon et la sélection egui peuvent servir à écrire et à naviguer dans une
  grille simple. L’IME injectée via les événements egui fonctionne dans une cellule.
- Un remplacement natif traversant une tabulation retire la frontière entre cellules.
  Le diagnostic le signale et permet d’annuler ; il n’interdit pas la mutation pour
  masquer cette limite. Un futur adaptateur devrait valider l’édition complète.
- La normalisation des paragraphes limite les suites de retours à deux caractères.
  Trois séparateurs de lignes vides d’un tableau à une colonne ne sont donc pas
  conservés. Les règles du corps ne peuvent pas être appliquées telles quelles aux cellules.
- Un retour à l’intérieur d’une cellule n’est pas distingué d’une nouvelle ligne du
  tableau. Il faut une représentation explicite de cette structure.
- Le diagnostic ne traite ni cellules fusionnées, ni mise en forme riche dans les
  cellules, ni largeur variable ou renvoi à la ligne. Les modifications de dimensions
  reconstruisent son texte simple ; elles ne prouvent pas la conservation de styles.
- Les événements simulés ne valident pas le chemin clavier → système → winit.
  L’utilisateur a confirmé les accents et touches mortes sur macOS. La préédition,
  sa validation et son annulation avec une véritable méthode IME du système restent
  à essayer lors des vérifications multilingues ; ce chemin n’est pas présenté comme testé.

## Orientation validée le 22 octobre 2026

Conserver Typst comme source de référence et comme format de stockage, mais décrire
explicitement les blocs et cellules **en mémoire**. La projection textuelle actuelle
reste réutilisable pour les zones de texte ; egui reste responsable de la saisie native.
Les cibles des commandes devront pouvoir désigner une zone et une position Unicode dans celle-ci,
plutôt qu’un indice global comme unique représentation possible.

Un tampon unique assorti de frontières protégées et d’une validation des mutations
a été examiné comme alternative. Les observations ci-dessus ne prouvent pas qu’il soit
impossible ; elles montrent les règles supplémentaires qu’il devrait porter.
Le choix des blocs/cellules explicites a été validé par l’utilisateur après les essais.
Les identifiants de zones et les règles de sélection entre zones restent à préciser
dans l’inventaire de phase 2.1 avant de figer les signatures.

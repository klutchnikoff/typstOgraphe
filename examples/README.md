# Exemples

## Bibliographie et profils natifs — format courant

[article-bibliography-profiles.typsto](article-bibliography-profiles.typsto)
est un article au format **CAS 16**, avec citations et bibliothèque native.
Il reprend le contenu de l’exemple bibliographique versionné ; un livre conserve
également une revue inactive de démonstration. Dans Bibliographie → Gérer les
références, changer son type en Article de revue retrouve cette publication ;
les autres champs restent conservés. Une validation constitue une seule édition
annulable, sans sauvegarde automatique.

La [recette CLI](cli/article-bibliography.json) le reconstruit vers une destination
libre. Le fichier actif `bibliography/references.yaml` sert au rendu ;
`bibliography/retained.json` conserve séparément les publications inactives dans
le CAS et l’export Typst. Aucun document local existant n’est converti.

Les exemples ci-dessous restent des références de lots antérieurs ; leurs anciens
formats ne sont pas annoncés compatibles avec CAS 16.

[article-working.typsto](article-working.typsto) est un article de démonstration
au format CAS 14, avec le générateur Typst 4 (guillemets et apostrophes typographiques, manifeste du modèle embarqué),
régénéré depuis le contenu versionné : auteurs et affiliations, résumés bilingues, sommaire, titres, citation
en bloc, note de bas de page, liste, tableau et citations bibliographiques.
Les ancres de blocs C.1 et l’état Yjs archivé sont concordants : l’ouverture ne
nécessite aucune attribution d’identité susceptible de modifier l’instantané.

Sa bibliothèque contient deux fichiers internes (BibLaTeX et Hayagriva), avec
trois références fictives citées et une notice non citée. Les quatre citations
illustrent les groupes de références, les compléments de pages ou de chapitre
et la forme narrative. Le modèle compose la bibliographie finale automatiquement.

Ouvrir le fichier dans typstOgraphe ; aucun fichier bibliographique externe
n’est nécessaire. Les scripts JSON de la CLI sont dans [cli/](cli/).

Le courrier utilise les cibles et nœuds du contrat structuré courant : paragraphes
explicites, métadonnées, gras, P.-S. et mention de pièce jointe. Depuis la racine :

```sh
mkdir -p output
cargo run --locked --offline -p typstographe-cli -- script examples/cli/courrier.json
```

Le résultat est `output/courrier-script.typsto`. Pour rejouer l’exemple, choisir
une destination libre dans son action `save` ; le script ne remplace pas un fichier existant.

## Renvois

[article-cross-references.typsto](article-cross-references.typsto) utilise le modèle
courant et montre les renvois vers un titre et un tableau : libellé complet, page
ou les deux. Les cibles ont des identités stables ; modifier leur intitulé ne
casse pas les renvois. L’exemple et ses dépendances sont autonomes hors ligne.
La [recette CLI](cli/article-cross-references.json) le reconstruit vers une
destination libre ; modifier son chemin de sortie pour conserver l’exemple fourni.

## Mathématiques

[article-mathematics.typsto](article-mathematics.typsto) montre les formules Typst
et LaTeX en ligne, deux équations numérotées et leurs renvois.
[La recette CLI](cli/article-mathematics.json) reconstruit cet exemple vers une
destination libre. Les exports PDF et Typst restent autonomes hors ligne.

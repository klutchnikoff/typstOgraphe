# Exemples

[article-working.typsto](article-working.typsto) est un article de démonstration
au format CAS 11, régénéré depuis le contenu versionné avant A.4.2 : auteurs et affiliations, résumés bilingues, sommaire, titres, citation
en bloc, note de bas de page, liste, tableau et citations bibliographiques.

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

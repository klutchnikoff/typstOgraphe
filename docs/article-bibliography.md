# Article : citations, bibliographie et sommaire

Décisions validées le 27 octobre 2026.

- Bibliothèque locale au projet : copies internes de fichiers BibLaTeX `.bib` et
  Hayagriva `.yaml`/`.yml`. Le texte original reste la source de vérité ; l’index
  titre/auteurs/année est dérivé avec Hayagriva, déjà utilisé par Typst.
- Onglet Bibliographie : ajouter, remplacer, retirer un fichier ; choisir le
  style seulement si le modèle le permet. Aucune synchronisation externe.
- Recherche fuzzy sur titres et auteurs pour insérer une citation, avec focus
  conservé dans le champ, flèches pour les résultats et Entrée pour choisir.
- Badge auteur–année indépendant de la présentation finale ; ouverture par clic
  et commande contextuelle commune. Bandeau partagé, y compris dans les widgets.
  Une citation peut réunir plusieurs références, avec un complément de pages ou
  de chapitre pour chacune ; formes normale et narrative.
- Typst compose les citations et ajoute automatiquement les seules références
  citées en fin d’article. Le modèle de travail propose Chicago auteur–date,
  APA et IEEE. Les styles en notes viendront après leur coordination avec les notes.
- Général : « Afficher le sommaire », désactivé par défaut ; le modèle le compose
  après les résumés et avant le corps. Pas de réglage de profondeur initial.
- Les modèles déclarent les options libres/restrictives. Une option imposée est
  masquée ; le modèle applique sa valeur sans effacer le choix documentaire.
- Contrats et validations Rust communs aux adaptateurs ; annotations structurées
  ciblées sans positions de texte plat. Une bibliothèque est partagée par fichier
  dans Yjs ; les textes originaux ne sont pas transformés en notices maison.

## Règles de publication

Les identités de fichiers sont stables, indépendantes des noms. Les clés de
citation doivent être uniques dans la bibliothèque du projet : une collision
est signalée avec les fichiers concernés, sans choisir une notice arbitrairement.
Le remplacement ou le retrait d’un fichier est refusé s’il rend une citation
existante introuvable. Les éditions non publiées restent locales et les éditions
concurrentes du fichier remplacé sont détectées avant validation.

Les archives et exports contiennent des fichiers `bibliography/<id>.<extension>`.
Leur contenu est identique aux instantanés de fichiers partagés avec le document ;
la lecture vérifie cette cohérence. Aucun accès aux fichiers originaux, ni aucun
réseau, n’est requis pour compiler le projet.

## Utilisation et contrat des adaptateurs

Dans **Bibliographie**, ajouter les fichiers `.bib`, `.yaml` ou `.yml` exportés
par son gestionnaire de références. Les boutons de chaque ligne remplacent ou
retirent le fichier ; les opérations sont annulables dans l’historique commun.
Les fichiers sont limités à 2 Mio chacun, 32 fichiers et 8 Mio par bibliothèque.

La commande **Citer une référence**, dans la barre d’outils ou la palette,
ouvre un brouillon de citation. Rechercher, choisir avec les flèches et Entrée,
ajouter d’autres références si nécessaire, puis **Valider**. Les compléments
sont du texte libre littéral. **Échap** abandonne le brouillon sans insérer de
badge. Un clic sur un badge, ou Ctrl/Cmd+Entrée juste après lui, rouvre ce même
bandeau. Retour arrière et Suppr demandent confirmation avant sa suppression.

Le MCP rattaché propose `bibliography.search` en lecture seule : la réponse
contient la révision, les titres, auteurs, années, clés et identités des fichiers.
`insert.citation` et `citation.set` appartiennent au catalogue structuré commun :

```json
{
  "id": "insert.citation",
  "params": {
    "items": [{"file_id": "books", "key": "bourdieu1980", "supplement": "p. 23–25"}],
    "form": "normal"
  }
}
```

`file_id` et `key` doivent venir de la bibliothèque actuelle ; aucune notice
n’est inventée ou créée par cette commande. `citation.set` reçoit le même
contenu et une cible `target.path` désignant le badge. L’agent soumet toujours
un lot soumis à validation humaine. Le MCP autonome et la CLI de l’ancien
tampon textuel découvrent les métadonnées bibliographiques, mais ces nouvelles
opérations structurées ne sont disponibles que dans le MCP rattaché.

La bibliothèque Typst commune expose `citation(body, form: "normal")` et
`reference-list(sources, style: "chicago-author-date")` dans
`capabilities/references.typ`. Le corps réunit des éléments natifs `cite`, générés
avec une clé Typst `label` et un complément de contenu sûr. Les éléments sont
construits dans le source du corps pour préserver leurs positions dans l’aperçu ;
le wrapper applique la forme sans les reconstruire. Ces fonctions utilisent les
primitives natives de Typst, sans paquet Universe supplémentaire. Le fichier est inventorié dans
`templates/dependencies.json` avec les autres capacités.

Les styles de citations en notes, l’édition de notices dans l’application,
Zotero et la synchronisation externe restent hors de ce lot.

## Recette reproductible

`fixtures/article-bibliography.typ` utilise un fichier de chaque format, une
citation groupée avec pages, une citation narrative et le sommaire. Une notice
non citée vérifie que la bibliographie ne contient que les travaux mentionnés.

```sh
mkdir -p output/pdf
typst compile --root . --ignore-system-fonts fixtures/article-bibliography.typ output/pdf/article-bibliography.pdf
```

Ajouter `--input style=apa` ou `--input style=ieee` permet de comparer les autres
styles avec les mêmes données. Les tests Rust couvrent la compilation, la carte
source/aperçu, les archives autonomes et leurs contrôles d’intégrité, l’export
graphique et les propositions MCP. Les parcours Chromium/WebKit couvrent la
recherche au clavier, les groupes, les bandeaux dans les widgets, les annulations,
les remplacements, deux fenêtres et la persistance. Aucune capture n’est produite.

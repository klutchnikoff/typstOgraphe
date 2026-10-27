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

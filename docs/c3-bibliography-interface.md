# C.3 — Gestionnaire bibliographique : présentation validée

Décisions du **6 novembre 2026**, après exploration de la maquette. Cette page
décrit l’interface retenue pour le [moteur commun](c3-bibliography-contract.md).
Son [raccordement et ses limites](c3-bibliography-manager.md) sont documentés
séparément.

- Grande modale centrée, environ 90 % de la fenêtre, arrière-plan atténué et
  légèrement flouté comme les dialogues existants.
- Colonne gauche d'environ un tiers : recherche fuzzy titre/auteurs, nombre de
  résultats, bouton + et liste **compacte à deux lignes par notice**. Première
  ligne : auteur principal, « et al. » si nécessaire, année ; deuxième : titre
  sur une ligne avec ellipse. La liste défile indépendamment et sa recherche
  reste accessible. Genre, nombre de citations et détails restent à droite.
- Un indicateur discret signale les notices incomplètes, sans ajouter de ligne
  ni rendre leur validation impossible. Les intitulés complets restent
  accessibles ; le clic ouvre toutes les informations dans la colonne droite.
- À droite : formulaire par genre, titre/auteurs puis publication ; identifiants,
  URL et clé dans une section dépliable. Modifier un champ préserve les données
  natives non exposées. Réutiliser les contrôles et collections communs ; les
  personnes bibliographiques ne sont pas la structure `authorship` de l'article.
- Actions groupées à droite, destructif à gauche du groupe et action principale
  tout à droite : Supprimer, Annuler, Valider. La suppression d'une notice citée
  est protégée et permet de retrouver les citations.
- Changer de notice ou fermer avec des modifications propose Abandonner,
  Continuer l'édition, Valider et poursuivre ; le brouillon reste récupérable.
  Une modification concurrente de la même notice conserve la saisie et bloque
  l'écrasement. Valider est une édition annulable, distincte de la sauvegarde.
- Depuis une insertion de citation, le même formulaire propose Créer et citer,
  en un lot atomique, sans nouvelle modale spécialisée.
- Importer ouvre une préparation dans le gestionnaire : nouvelles notices,
  enrichissements, pertes de conversion et rapprochements à décider. Aucun lot
  n'est publié avant résolution ou exclusion explicite des conflits.
- Réutiliser `formDialog`, `contextPanel`, les champs et la navigation clavier
  commune. Les flèches parcourent les résultats sans faire sortir le focus de la
  recherche ; Entrée rejoint le formulaire. Tab parcourt les contrôles ; Entrée
  garde son rôle dans les champs multilignes, Ctrl/Cmd+Entrée valide. Échap
  ferme le niveau actif en respectant la garde de brouillon. La préédition IME
  ne déclenche aucune validation.
- Pas de sélecteur de bibliothèque à option unique : C.4 ajoutera le choix
  Projet/Personnelle et réutilisera le même gestionnaire.

Recettes de qualification : nombreuses notices et titres longs, recherche Unicode,
clair/sombre, focus et défilements indépendants, import au clavier, fermeture
d'un brouillon et reprise après incident. Les contrôles de la maquette ne
remplacent pas ces recettes dans l'application ; aucune capture n'est demandée.

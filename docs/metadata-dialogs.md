# Édition des auteurs et affiliations

Orientation retenue le 28 octobre 2026. La présentation fait l’objet d’une
maquette interactive avant le raccordement aux données du document.

## Présentation

- Conserver des lignes compactes dans les onglets Auteurs et Affiliations,
  avec l’accès à la modification et le bouton `+` d’ajout.
- Ouvrir le même dialogue réutilisable pour la création et la modification,
  centré dans la fenêtre de l’application.
- Atténuer l’arrière-plan et le flouter légèrement : proposition initiale de
  3 px, sans animation. Le formulaire reste opaque et net. Si le flou n’est
  pas disponible, l’atténuation suffit.
- Afficher les champs décrits par le contrat du type. Le dialogue fournit
  le titre, la disposition générale, les actions, le focus et le clavier ;
  les formulaires ne doivent pas être dupliqués par type de document.
- Conserver les résumés riches dans leur onglet, sans généraliser ce dialogue
  à tous les champs éditoriaux.

## Saisie et validation

- Créer un brouillon isolé pour un ajout **et** pour une modification.
  La frappe dans un dialogue ne modifie pas immédiatement le projet partagé.
- Placer le focus dans le premier champ. Garder la navigation Tab et
  Maj+Tab dans le dialogue ; rendre l’arrière-plan inactif.
- Échap et Annuler abandonnent le brouillon et rendent le focus au déclencheur.
  Le clic sur l’arrière-plan ne valide ni n’abandonne la saisie.
- Placer Annuler puis Valider à droite, Valider à l’extrême droite.
  La suppression reste une action séparée dans la liste.
- Entrée valide depuis un champ simple ; elle conserve son sens de retour à
  la ligne dans un champ multiligne. Ctrl/Cmd+Entrée permet de valider
  explicitement. Aucune validation pendant la préédition IME.
- Valider publie une seule opération atomique et annulable. Réutiliser la
  voie commune du moteur et sa protection de concurrence : une modification
  ou suppression distante ne doit jamais être écrasée silencieusement.
  Le raccordement précis aux gardes du contrat structuré reste à réaliser.

## Couleurs sémantiques de l’éditeur

La palette commune se trouve dans `app/src/editor-palette.css`. Le gras
utilise le rouille et l’italique le vert ; les notes utilisent le bleu et
les citations bibliographiques le violet. Ces couleurs ne changent pas le
rendu du modèle Typst.

Les badges sont produits par `inlineObjectView`, qui renseigne `data-kind`.
Un nouveau type peut déclarer `--badge-ink`, `--badge-surface`,
`--badge-border` et `--badge-active-text`. Sans palette dédiée, il hérite
de l’accent de l’interface. Le focus, la navigation et la mise en évidence
du badge actif restent communs.

Les tests navigateur vérifient un contraste de texte d’au moins 4,5:1 pour
le gras, l’italique et les badges, ainsi que la cohérence entre apparence
système et apparence explicite, en clair et en sombre.

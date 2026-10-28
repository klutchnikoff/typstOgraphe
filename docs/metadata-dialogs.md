# Édition des auteurs et affiliations

Présentation validée et intégrée le 28 octobre 2026, après une maquette
interactive pour les auteurs et affiliations.

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
  Le panneau déclare `editing: "dialog"` dans son descripteur. Les autres
  panneaux conservent leur présentation, notamment les titres et résumés.
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
- Valider publie une seule transaction atomique et annulable via
  `prepareRecordEdit`, commun aux ajouts et modifications. Le contrôle porte
  sur l’identité du document, la collection et l’objet édité (identité et
  contenu de départ). Une modification ou suppression de cet objet bloque
  la publication et conserve le brouillon ; les changements indépendants
  ne sont pas écrasés. Le futur adaptateur Rust du contrat unifié reprendra
  cette responsabilité lors du remplacement du relais web actuel.
- Une référence supprimée reste visible, même après décochage, jusqu’à la
  fermeture du formulaire. La validation exige de retirer explicitement
  son association ; aucun retrait silencieux et aucune perte de focus.

## Composants et vérifications

`formDialog` encapsule le dialogue natif HTML et réutilise `contextPanel` pour
le formulaire et ses actions. Les portées clavier communes gèrent la boucle
Tab, les flèches entre actions, Échap et Ctrl/Cmd+Entrée. Le dialogue natif
rend l’arrière-plan inactif ; la restitution du focus suit son événement
`close`, notamment pour WebKit. Les raccourcis documentaires globaux restent
inactifs pendant un dialogue.

`metadataCollections` réutilise les mêmes contrôles de champs pour les lignes
éditoriales et les dialogues. Les descripteurs définissent l’ordre des champs
et les informations de synthèse d’une ligne. Les champs de texte secondaires
partagent une ligne si la largeur le permet ; titre, texte multiligne et
associations occupent toute la largeur.

Les tests couvrent les brouillons privés, la validation atomique et son
annulation, les changements concurrents, la suppression d’une référence,
les retours multilignes, l’IME, le focus et la géométrie des dialogues en clair
et sombre sur Chromium et WebKit. Aucune capture d’écran n’est produite.

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

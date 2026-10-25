# Interface — inventaire, constats et proposition de design

Document de travail du 25 octobre 2026, établi par lecture du code de `crates/app`
et des captures de `output/`. **Rien n'est implémenté.** Il prépare un chantier
« polissage de l'interface » à engager après le lot 6 (voir « Décisions à prendre »).
Les maquettes interactives associées : [système d'interface](mockups/interface-system.html)
(tous les écrans, clair et sombre) et [bandeaux contextuels](mockups/interface-polish.html)
(variante B et pointe orthographique, déjà validées).

## 1. Ce qui est déjà décidé

- **Bandeaux (variante B)** : le bandeau d'un bloc est l'en-tête déplié de ce bloc
  (un seul cadre, bordure d'accent, fond d'accent doux). Pas de pointe pour les paramètres
  d'un bloc.
- **Orthographe (et grammaire plus tard)** : carte alignée sur la colonne, avec une pointe
  de 12 px vers le mot ; étiquette du type d'anomalie.
- **Orthographe dans un bloc** : le bandeau s'insère entre deux lignes, aux bords du
  cadre du bloc, et suit sa largeur.

## 2. Inventaire des éléments d'interface

| Zone | Source | Aujourd'hui | Constats |
| --- | --- | --- | --- |
| Menus | `menus.rs`, `app/mod.rs` | natifs sous macOS (`muda`), barre egui ailleurs ; construits du registre | bon : à conserver. Rien à styler sous macOS |
| Barre d'outils | `toolbar.rs`, `icons.rs` | marque « typstOgraphe » (17 px) + icônes Lucide 18 px, boutons 32 px, menu titres et listes, « P.-S. » et « P.J. » en texte | marque redondante avec le titre de fenêtre ; deux boutons textuels isolés ; annuler/rétablir absents de la barre ; sélection grise volontairement « neutre » (`neutral_selection`) |
| Panneau d'informations | `app/information.rs`, `information_fields.rs` | titre + petits boutons de rubrique, `Grid` 2 colonnes, `ComboBox`, zone défilante ≤ 45 % de la hauteur | trois « styles d'onglets » dans l'application (ici, préférences, relecture) ; largeur des libellés variable |
| Recherche | `app/search.rs` | chevron de remplacement, champ, compteur 90 px, chevrons, « Aa », « × » en texte, ligne de remplacement | « × » texte alors que `Icon::Close` existe ; pas d'icône de recherche ; compteur de même poids que les libellés |
| Éditeur | `app/editor_view.rs`, `editor/*` | `TextEdit` maître, marges proportionnelles à la police, widgets peints à la main | cœur de l'ergonomie : ne pas y toucher hors habillage |
| Widgets de bloc | `editor/widgets.rs`, `decorations.rs`, `app/bands/paint.rs` | cadre `faint_bg`, rayon 5, trait 0,75, en-tête fixe de 30 px, pied de 9 px ; engrenage et corbeille | rayon et traits non liés aux jetons ; étiquette en `weak small` |
| Bandeaux | `app/bands.rs`, `bands/paint.rs` | peints sur toute la largeur de la zone visible (`ui.clip_rect()`), fond `selection.bg_fill × 0,16` | **cause du défaut signalé** : un bord différent de celui des blocs |
| Aperçu | `app/preview.rs` | en-tête « Aperçu Typst · n page · x ms », pages en `Image` sur le fond du panneau | durée en millisecondes visible en permanence ; feuille sans relief sur fond identique |
| Séparateur de colonnes | `editor_view.rs` | 8 px, glissable, double-clic = 50 % | bon ; ajouter un état de survol visible |
| Barre d'état | `app/chrome.rs` | statistiques, séparateurs verticaux, orthographe, « Agent : … » | tout au même niveau visuel |
| Accueil | `app/dialogs.rs` (`welcome_ui`) | titre + 3 boutons de 240×32 centrés à 30 % | aucun bouton principal ; pas de récents |
| Nouveau document | `dialogs.rs` | `Modal`, `radio_value`, séparateur, deux boutons | boutons sans hiérarchie |
| Préférences | `app/preferences.rs` | `Modal` 480 px, onglets `selectable_value`, `Grid`, `ComboBox`, `Slider`, `CollapsingHeader` | bouton « Fermer » sans priorité ; choix à 3 valeurs en liste déroulante |
| Assistance | `app/assistance.rs` | `Modal` 480 px, étiquettes, cases, boutons empilés | actions de natures différentes au même niveau (copier, révoquer, autoriser) |
| Export | `dialogs.rs` | `Window` 470 px avec barre de titre et « × » | autre enveloppe que les modales ; champ de dossier tapé à la main, bouton « Exporter » sans mise en avant |
| Palette | `dialogs.rs` | `Window` ancrée en haut, boutons sans cadre | pas de groupes, pas d'icônes, raccourcis en texte brut |
| Enregistrer ? / erreur / lecture seule | `app/files.rs` | `Window` centrées, boutons « Enregistrer / Ne pas enregistrer / Annuler » alignés à gauche | action destructive non distinguée ; erreurs en `Color32::LIGHT_RED` |
| Récupération | `app/recovery.rs` | `Window` 560 px, `ui.separator()` entre entrées | erreur en `LIGHT_RED` fixe |
| Relecture MCP | `app/review.rs`, `review/*` | page entière : en-tête, onglets, barre de décision, deux colonnes | décision loin de l'en-tête ; couleurs de différences propres au module |
| Notification de proposition | `review.rs` (`proposal_notification`) | ligne de texte + bouton, séparateur | pas de mise en valeur |
| Retours (`dialogs.message`) | `assistance.rs` | texte de confirmation | à unifier en notification temporaire |

## 3. Constats transversaux

1. **Aucune couche de thème.** Les couleurs sont dispersées : `ACCENT` (40, 92, 119) dans
   `app/mod.rs`, la sélection dans `setup.rs`, `neutral_selection` dans `toolbar.rs`,
   `Color32::LIGHT_RED`, `from_rgb(183, 69, 57)` (`information.rs`) et `error_fg_color`
   pour le même sens « erreur », `PREVIEW_SELECTION`. Les modes clair et sombre
   reposent sur les valeurs par défaut d'egui.
2. **Deux enveloppes de dialogue** (`Modal` et `Window` à barre de titre), cinq largeurs
   (320, 430, 470, 480, 560) et aucune règle de pied de page.
3. **Aucune hiérarchie d'action.** Il n'y a jamais de bouton principal ; les actions
   destructives (« Ne pas enregistrer », « Supprimer ») ressemblent aux autres.
4. **Trois styles de contrôle segmenté** (petits boutons sélectionnables, `selectable_value`,
   onglets de relecture) et des listes déroulantes pour 2 ou 3 choix.
5. **Police d'interface par défaut d'egui**, fine et peu contrastée aux petites tailles
   (visible sur les captures) ; un seul poids.
6. **Bordures de champ très discrètes** (fond presque noir en sombre) : contraste de
   contrôle insuffisant (< 3:1) et pas d'anneau de focus homogène.
7. **Une seule icône manquante par rapport aux usages** : recherche, annuler, rétablir,
   fichier, agent, validation. Le sous-ensemble Lucide embarqué en compte 17.
8. **Informations techniques exposées** : durée de rendu en ms dans l'en-tête de l'aperçu.

Point positif : les surfaces sont déjà **adaptées aux gestes natifs** (menus, IME, focus,
raccourcis) ; le polissage doit rester dans la peinture et la composition.

## 4. Proposition

Détail visuel et clair/sombre : [maquette « Système d'interface »](mockups/interface-system.html).

### 4.1 Jetons

Un module `theme` unique (côté application seulement), avec :

| Jeton | Clair | Sombre | Rôle |
| --- | --- | --- | --- |
| `bg` / `panel` / `field` | #f4f4f1 / #ffffff / #ffffff | #1b1c1e / #232427 / #2b2d31 | fond, surfaces, champs |
| `text` / `muted` | #1f2328 / #5f656c | #e6e7e9 / #9aa0a8 | texte, texte secondaire |
| `border` / `border-strong` | #d9d9d2 / #80858c | #35373c / #7d838b | filets décoratifs / contrôles |
| `accent` / `accent-soft` | #285c77 / #e8f0f4 | #8cc3e0 / #26343d | action, sélection douce |
| `danger` / `ok` | #a53a32 / #2f6b4a | #e58b82 / #7fc59d | erreur, réussite (avec fonds doux) |

L'accent reprend la teinte actuelle (40, 92, 119) et `border-strong` fait passer les contrôles
au contraste de 3:1. Mesures (WCAG 2.x) : texte sur fond 14,3 (clair) et 13,8 (sombre) ;
secondaire 5,4 et 6,5 ; accent sur panneau 7,3 et 8,1 ; texte sur bouton principal 7,3 et 8,5 ;
bordure de champ 3,7 et 3,6. Les filets (≈ 1,5:1) restent décoratifs.

Échelles : espacements 4 · 8 · 12 · 16 · 24 ; rayons 6 (contrôles), 8 (cartes, bandeaux,
widgets), 12 (dialogues) ; tailles de texte 11 · 12,5 · 13,5 · 15 · 20.

### 4.2 Composants

- **Boutons** : principal (plein), secondaire (contour fort), discret, destructif (contour
  rouge), icône. Un seul principal par surface.
- **Champs** : bordure forte, anneau de focus de 3 px en `accent-soft`.
- **Contrôle segmenté** pour 2 à 4 choix exclusifs (rubriques, orientation, apparence,
  format d'export, bandeau de liste) ; liste déroulante réservée aux choix longs (papier,
  langue, police).
- **Dialogue** : enveloppe modale unique (titre, description, corps, pied à droite),
  largeurs 420 / 480 / 540. Échap annule.
- **Notification** : bandeau temporaire pour les retours (enregistré, exporté) ;
  bandeau d'accent pour la proposition de l'agent ; erreurs en ligne avec icône.
- **Touche de raccourci** (`kbd`) dans la palette, les info-bulles et les menus egui.

### 4.3 Écrans

- **Fenêtre** : marque retirée de la barre d'outils ; annuler/rétablir ajoutés (à droite,
  atténués) ; « P.-S. » et « P.J. » regroupés en un menu à une icône ; rubriques d'informations en contrôle
  segmenté ; recherche avec icône ; barre d'état à pastilles (orthographe, agent) et
  indicateur d'enregistrement.
- **Aperçu** : fond « table » plus sombre, feuille avec ombre, en-tête « Aperçu · 1 page »
  et indicateur d'état ; la durée passe dans l'info-bulle.
- **Widgets** : mêmes cadres mais liés aux jetons ; icône du type dans l'en-tête.
- **Dialogues** : tous sur la nouvelle enveloppe (nouveau document en cartes de choix, export
  avec choix de dossier et format segmenté, « Enregistrer ? » avec action destructive à
  gauche, récupération en cartes, assistance avec autorisations en interrupteurs).
- **Palette** : groupes, icônes, touches de raccourci.
- **Relecture** : décision (Refuser / Accepter) dans l'en-tête ; deux colonnes en cartes ;
  différences liées aux jetons, lisibles sans la couleur seule (biffé et souligné).
- **Accueil** : un bouton principal, des récents (si une liste de fichiers récents est
  ajoutée — c'est un changement de fonction, hors périmètre).

## 5. Faisabilité dans egui 0.33.3

- **Style global** : `Style`, `Visuals` (fonds, traits, rayons, ombres, sélection) et
  `Spacing` couvrent les fonds, bordures, rayons et ombres. Les états des boutons
  (`widgets.inactive/hovered/active`) se règlent globalement.
- **Boutons à variantes** : fonctions utilitaires dans un petit module `ui_kit`
  (bouton principal/discret/destructif, champ, contrôle segmenté, `kbd`, enveloppe de dialogue),
  construites sur `Button`, `TextEdit` et `Frame`. Les `WidgetInfo` actuels sont conservés
  pour l'accessibilité (AccessKit).
- **Police** : egui ne gère pas les polices variables. Deux fichiers embarqués (normal et
  semi-gras, quelques centaines de Kio chacun) donneraient les poids nécessaires ;
  l'alternative est de conserver la police actuelle avec une taille et un contraste mieux
  réglés. À décider (voir plus bas).
- **Bandeaux B et pointe** : le placement actuel (`reserve` dans `editor/widgets.rs`) réserve
  déjà de la hauteur au-dessus d'une ligne et `paint_band` peint à la largeur de la zone
  visible. Il faudra passer la plage horizontale du bloc (`widget_rect`) à `BandInset`, et
  placer l'insertion entre l'en-tête et la première ligne du bloc ; pour l'orthographe,
  utiliser le rectangle du mot (`pos_from_cursor`) pour la pointe et choisir la ligne du mot
  comme ancre. Le dessin de la pointe est un `Shape::convex_polygon`. À vérifier au moment
  du codage : l'ancrage entre deux lignes d'un bloc (zones de texte secondaires comme les
  légendes) et la disparition de la pointe quand le mot sort de la zone visible.
- **Ce qui ne change pas** : `TextEdit` maître, IME, focus, raccourcis, gardes du moteur,
  contrat des commandes. Le polissage n'ajoute aucune dépendance.

## 5 bis. Barre latérale droite (mécanisme seulement)

Décidé le 25 octobre 2026 : poser le mécanisme sans rien y brancher. Barre étroite à
droite, icônes carrées de 30 px ; un tiroir recouvre l'aperçu (jamais l'éditeur) sur
2/3 au plus de sa colonne. Le composant est inactif et invisible tant qu'aucun panneau
n'est déclaré ; les panneaux réels (plan du document, assistance, récents) relèvent du
chantier G de `ROADMAP.md`. Précautions : aucune largeur codée en dur supposant deux
colonnes ; panneau d'informations et accueil restylés avec des composants déplaçables ;
jeton de fond de panneau latéral dès L.1. Points d'interface : zone d'interface
enregistrée, Échap et IME, focus de l'éditeur conservé, calques au-dessus. Voir l'onglet
« Barre latérale » de la [maquette](mockups/interface-system.html). Un tiroir recouvrant
masque le rendu. **Décidé : pas d'épinglage**, et seulement des interactions simples qui
n'exigent pas de garder le panneau ouvert. Conséquence à confirmer à l'implémentation :
fermeture facile (Échap, clic hors du tiroir, éventuellement après l'action) et aucun
état de panneau persistant d'une session à l'autre.

## 6. Mise en œuvre proposée

Règles du dépôt à respecter : un commit par déplacement, comportement volontaire dans
des commits séparés, captures de référence régénérées (D19, D20 et l'assouplissement du
21 octobre).

1. **Thème** : module `theme`, jetons, `configure_context`, remplacement des `Color32`
   dispersés (aucune apparence nouvelle encore). Test : les captures ne changent pas là
   où les valeurs sont identiques.
2. **`ui_kit`** : boutons, champs, contrôle segmenté, enveloppe de dialogue, `kbd`,
   notification, mécanisme de barre latérale droite (sans contenu). Tests unitaires de présentation et d'accessibilité.
3. **Migration par risque croissant** : dialogues (nouveau document, export, enregistrer,
   récupération, assistance), préférences, palette, accueil, recherche, informations,
   barre d'outils, barre d'état, aperçu, relecture.
4. **Bandeaux B et pointe** (variante B, orthographe dans un bloc), avec les tests de 6.4a
   adaptés (focus, IME, redimensionnement, thèmes).
5. **Recette** : captures natives clair/sombre de tous les écrans, comparaison avant/après,
   essais IME, contrôle de contraste automatisé sur les jetons.

Chaque étape reste indépendante et peut être livrée seule.

## 7. Décisions à prendre

1. **Police d'interface** : embarquer une sans-empattement (deux graisses) ou conserver
   celle d'egui avec de meilleurs réglages ? Recommandation : embarquer, si une licence
   libre (OFL) convient.
2. **Marque** : la retirer de la barre d'outils (titre de fenêtre suffisant) ?
3. **P.-S. et P.J.** : *décidé le 25 octobre 2026* — un seul bouton, qui ouvre un menu
   « fin de lettre » (post-scriptum, pièces jointes), sur le modèle du menu des listes.
   Les deux commandes, leurs identifiants et leurs raccourcis ne changent pas. Le bouton
   porte un trombone avec chevron ; chaque entrée du menu a sa propre icône. Le bouton
   disparaît quand le type de document n'offre aucune des deux commandes. Icône du
   bouton à confirmer sur le rendu (trombone, stylo ou texte, voir la maquette).
4. **Annuler / rétablir** dans la barre d'outils : à ajouter ? (déjà dans les menus et les
   raccourcis)
5. **Durée de rendu** : la retirer de l'en-tête ?
6. **Accent** : conserver la teinte actuelle, ou en proposer une autre ?
7. **Moment** : engager ce chantier juste après 6.6, avant la phase 7 ? Il s'inscrirait
   comme chantier distinct, avec sa propre décision (D40) dans la feuille de route.

**Mise à jour du 25 octobre 2026 (L.0) :** choix validés — police embarquée, marque retirée,
annuler/rétablir à droite de la barre, durée de rendu dans l'info-bulle ; accent actuel
et trombone conservés par défaut. L.1 livrée : voir `crates/app/src/theme.rs`.

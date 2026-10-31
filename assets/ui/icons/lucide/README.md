Base : icônes SVG officielles de [Lucide 0.468.0](https://github.com/lucide-icons/lucide/tree/0.468.0/icons), embarquées sans modification.

Les licences ISC et MIT (pour les éléments issus de Feather) sont conservées
dans `LICENSE` et `LICENSE-FEATHER`. L’application Tauri charge les SVG via
`app/src/icons.ts` et utilise `currentColor`. La référence egui remplace cette
couleur par du blanc pour créer un masque teinté à la densité de pixels de l’écran.

Les icônes `settings-2`, `trash-2` et `x` ont été ajoutées pour les widgets et
bandeaux depuis la distribution locale officielle Lucide 1.8.0 (licence ISC).
Leurs éléments vectoriels sont conservés ; ils utilisent le même masque monochrome.

Les icônes `paperclip`, `pen-line`, `undo-2`, `redo-2`, `search`, `chevron-up` et
`chevron-right` (barre d’outils, menu « fin de lettre », recherche) ont été ajoutées le
25 octobre 2026 depuis les fichiers officiels de Lucide 0.468.0
(`https://raw.githubusercontent.com/lucide-icons/lucide/0.468.0/icons/<nom>.svg`),
sans modification. Même masque monochrome que les autres.

Les icônes `check` et `circle-minus` (choix retenu et non retenu du comparateur)
proviennent également des SVG officiels de Lucide 0.468.0, sans modification,
aux mêmes adresses versionnées `icons/check.svg` et `icons/circle-minus.svg`.

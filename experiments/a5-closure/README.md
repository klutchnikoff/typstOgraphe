# Qualification locale du socle — A.5

Les JSON conservent les résultats mesurés, sans invitations MCP, préférences,
brouillons privés ou captures d’écran.

- `report-macos.json` : recette native Courrier/Article réussie au commit
  `3329f78`, depuis un worktree propre. Tauri, CLI, MCP rattaché/autonome,
  version nommée, reprise par la modale, annulation/rétablissement, réouverture,
  ressources et exports comparés. Les répertoires de sortie cités sont temporaires.
- `report-macos-receipts.json` : même recette native réussie au commit `1fd3c9d`,
  depuis un checkout propre, après correction des courses de la recette entre
  événements documentaires et réponses IPC. Aucun délai artificiel dans cette
  passe finale ; voir le suivi du bilan A.5 pour la reproduction avec retard.
- `versions-preservation-macos.json` : après les éditions CLI et MCP puis la
  réouverture native, la version conservée retrouve exactement l’état canonique
  natif au moment de sa création, pour les deux types.
- `normal-launch-guards.json` : le binaire normal refuse STDIO, diagnostics et
  session de recette avec le code 2, sans création du répertoire demandé.
- `normal-startup-macos.json` : le processus natif normal a démarré sans projet,
  est resté actif pendant le contrôle puis a été arrêté par le runner. Il ne
  s’agit pas d’une vérification visuelle ni d’un test matériel de clavier.

Les binaires de recette sont ceux de `npm run build:recipe`, copiés dans
`target/recipes/debug/` et vérifiés avant publication atomique. La compilation
normale conserve son frontend `app/dist/` ; les diagnostics viennent de
`app/dist-recipe/` et de `recipe-driver`. Le cache Cargo a été réutilisé pour
éviter une reconstruction des dépendances : le checkout était propre, pas le cache.

Depuis la racine :

```sh
npm ci
npm run build:web
npm run check:distribution
npm run build:recipe -- --offline
npm run test:closure-native -- /chemin/vers/un/dossier-nouveau
```

Une session graphique est nécessaire. Les contrôles multisystème sont ceux du
workflow **Validation complète**, pas une exécution Windows/Linux sur ce poste.
Voir le [bilan A.5](../../docs/a5-closure-results.md) pour les commandes, incidents
de recette corrigés et limites de qualification.

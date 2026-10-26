# Prototype isolé d'éditeur web — décisions du 26 octobre 2026

Choix validés avec l'utilisateur, sur la branche `codex/spike-tauri-editor`.
L'objectif est de comparer l'édition structurée web à l'adaptateur egui avant
de décider d'une éventuelle migration. L'application existante reste inchangée.

## Organisation et périmètre

- Diagnostic autonome dans `experiments/web-editor/`, dépendances et verrous propres,
  hors des builds par défaut ; dépendance locale vers `typstographe-core`.
- Tauri 2, Tiptap 3 / ProseMirror, TypeScript et Vite, sans framework d'interface
  supplémentaire. Yjs pour l'état partagé et l'historique collaboratif.
- Deux colonnes : contenu structuré et véritable aperçu Typst, avec les principes
  visuels du chantier L. Toutes les ressources nécessaires sont locales.
- Paragraphes, titres, gras, italique, liens, liste simple, image avec légende et
  texte alternatif, tableau sans fusion et note de bas de page.
- Les métadonnées restent fixes dans ce diagnostic. Aucun raccordement ACP,
  serveur distant, compte utilisateur ou migration générale n'est inclus.

## État documentaire et validation

Yjs fait autorité pour l'état d'édition partagé pendant la session. Rust reçoit
des instantanés structurés, valide le sous-ensemble connu et génère le source
Typst compilé par le cœur existant. Le Typst est la représentation documentaire
validée ; les éditeurs ne sont pas reconstruits depuis le source à chaque frappe.

Un contenu refusé reste visible et corrigeable. Il ne remplace ni le dernier
rendu validé ni un enregistrement valide. Les réponses de compilation périmées
ne remplacent jamais un aperçu plus récent. Une modification initiée côté Rust
doit passer par le même état partagé, sans circuit parallèle de mutation.

Le diagnostic possède son adaptateur de schéma expérimental. Il réutilise le
monde Typst, les polices, les modèles, ressources et exports du cœur. Les tableaux
et notes ne sont pas pour autant ajoutés au catalogue de commandes de production :
leur compilation ne prouve pas leur prise en charge par le projecteur actuel.

## Historique et collaboration

- Une seule voie d'annulation du contenu : celle de la liaison Tiptap/Yjs.
  Chaque participant annule ses opérations, pas celles du voisin.
- Deux fenêtres locales, deux `Y.Doc` indépendants, échange de mises à jour
  sérialisées par un relais Tauri. Présence et curseurs sont éphémères.
- Essais d'édition simultanée, déconnexion/reconnexion, absence de doublons à
  l'initialisation et cohérence des ressources partagées.
- Le Typst seul ne conserve pas les identités de collaboration. La sauvegarde
  expérimentale comprend source validé, ressources, état Yjs et version de schéma.
  Le format `.typsto` de production n'est pas redéfini par cette expérience.

## Critères de bilan

1. Saisie, accents et IME, sélection entre blocs, copier-coller, clavier et focus.
2. Tableau, note et annotation éditables ; annulation locale après édition distante.
3. Fidélité des instantanés, rendu réel, sauvegarde et reprise sans perte.
4. Convergence après interruption et remise en relation de deux clients.
5. Modifications venant de Rust intégrées au même historique et à la synchronisation.
6. Fluidité et coût mémoire sur un document plus long ; limites constatées consignées.

Les essais natifs macOS, les tests automatisés et les vérifications de compilation
ne seront pas présentés comme des essais manuels Windows/Linux. Le bilan doit
identifier ce qui est réutilisable, ce qui reste expérimental et les coûts d'une
migration. Plusieurs commits, puis une seule poussée après vérification.

## Sources techniques

- [Tauri : modèle de processus](https://v2.tauri.app/concept/process-model/)
- [Tiptap sans framework](https://tiptap.dev/docs/editor/getting-started/install/vanilla-javascript)
- [Liaison Yjs / ProseMirror](https://docs.yjs.dev/ecosystem/editor-bindings/prosemirror)
- [Collaboration Tiptap](https://tiptap.dev/docs/editor/extensions/functionality/collaboration)

## État de réalisation

Prototype et recettes automatisées livrés sur cette branche. Voir le
[bilan, les mesures et les limites](web-editor-spike-results.md), puis le
[guide de lancement](../experiments/web-editor/README.md).
L'essai ergonomique par l'utilisateur, notamment avec son clavier et son IME,
reste nécessaire avant toute décision de migration.

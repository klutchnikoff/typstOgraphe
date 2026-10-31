# A.3 — Premier lot de conservation

**31 octobre 2026.** La conservation privée est livrée ; A.3 reste ouvert.
Le [contrat](a3-conservation.md) décrit la séparation entre état confirmé,
saisie locale et reprise explicite. Le [lot suivant](a3-recovery-results.md) livre la reprise graphique des formulaires,
les choix de fermeture et la coordination des transitions entre fenêtres.
Les limites ci-dessous décrivent ce premier lot historique.

## Garanties exercées

- Notes, liens, préférences de widgets et autres bandeaux passent par le collecteur
  commun. Il conserve les contrôles natifs, zones riches et données typées propres
  à l’objet, sans soumission du formulaire.
- Les brouillons d’auteurs/affiliations Article et les images choisies mais non
  appliquées restent privés. Deux fenêtres ne partagent pas leurs formulaires.
  L’annulation explicite retire le brouillon concerné.
- Les saisies refusées, leurs réglages invalides et les champs en attente restent
  conservables sans demander au propriétaire de les publier. Le checkpoint confirmé
  provient du propriétaire Rust, et non de la réplique rejetée.
- Une composition IME conserve sa dernière saisie validée. La validation d’un
  formulaire pendant sa préédition reste bloquée. Les scénarios web emploient des
  événements de composition synthétiques ; ils ne remplacent pas un clavier IME natif.
- Le délai de pause, l’échéance en saisie continue, la sérialisation des écritures,
  les changements pendant une écriture et les nouvelles tentatives après erreur
  sont vérifiés. La fermeture locale attend aussi les écritures nécessaires.
- Un enregistrement confirmé ne supprime pas les formulaires privés. Recharger une
  vue retient sa copie non reprise au lieu de l’écraser avec la nouvelle vue vide.
  Une ancienne génération ou séquence ne peut pas remplacer la copie courante.
- Une copie trop volumineuse, une ressource invalide ou un échec d’écriture ne
  consomme pas la copie précédente. Les chemins de récupération refusent les liens
  symboliques ; les lectures sont bornées et le format est versionné.

## Recette native macOS

Le [rapport brut](../experiments/a3-conservation/report-macos.json) vient du commit
`1473bea7209511ee850ab7ea44905d3f857bdcbe`, frontend de production embarqué et véritable
WebView Tauri. Il ne contient que des données synthétiques de recette.

1. Ouvrir le bandeau d’une note existante et y saisir un texte sans le valider.
2. Attendre la copie **automatique** ; aucun flush explicite n’accélère ce contrôle.
3. Enregistrer l’archive confirmée et vérifier que la copie du formulaire subsiste.
4. Interrompre seulement ce processus par `SIGKILL`, sans exécuter le parcours de
   fermeture ou ses nettoyages.
5. Démarrer une autre instance, rouvrir l’archive par le parcours natif normal,
   vérifier la présence du formulaire privé sur disque et l’absence de son texte
   dans le document confirmé. Revalider le checkpoint confirmé conservé.

Cette recette ne prétend pas restaurer le formulaire dans l’interface : elle
vérifie la conservation durable avant le lot de reprise. Aucun screenshot,
vidéo ou trace graphique n’a été produit.

Pour la rejouer après `npm run build:web` puis
`cargo build --locked -p typstographe --features recipe-driver` :

```sh
npm run test:draft-conservation-native -- /chemin/vers/un/dossier/vide
```

Le dossier doit être nouveau ou vide. Le script ne tue que son propre processus
enfant. Sous macOS, garder la session graphique active pendant cette recette.
`TOG_SPIKE_BINARY` permet d’utiliser une copie du binaire afin de l’isoler d’une
compilation simultanée dans un autre checkout.

## Vérifications complémentaires

- **308 tests Rust** du workspace réussis, doctest inclus ; un diagnostic de
  performance reste volontairement ignoré. Cela comprend les fichiers, verrous,
  conservation et propositions MCP, dont **9 tests dédiés** aux copies privées,
  avec un vrai sous-processus interrompu sans nettoyage et une erreur de stockage
  reproductible.
- **330 tests web** et **3 contrôles des langues natives**.
- **29 recettes de contrat** Yjs/Yrs ↔ Tiptap et propriétaire Rust.
- Dernière passe figée : **18/18 parcours Chromium/WebKit** de conservation et
  ressources réussis. Autres parcours ciblés : nouvelles copies privées, formulaires Article
  concurrents, composition, champs d’en-tête, images/signatures et navigation des
  bandeaux et de la recherche. Les échecs de délai/navigation pendant les premières
  passes ont été rejoués séparément ; aucun allongement de timeout ou retry automatique
  n’a été ajouté aux assertions.
- Build TypeScript/Vite, Clippy avec et sans `recipe-driver`, rustfmt, Prettier,
  limites de modules, dépendances du cœur et **15 tests Python** des scripts.

Les tests locaux n’établissent pas une validation native Windows/Linux ni l’état
futur de la CI. Les recettes de concurrence autour d’une transition déclenchée
depuis une autre fenêtre et les décisions de conservation/abandon restent dans
les lots suivants. Les copies privées retenues attendent leur parcours explicite
de reprise et de nettoyage.

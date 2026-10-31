# A.3 — Reprise des brouillons et fermeture

**31 octobre 2026.** Le lot d’interface validé est livré sur
`feat/a3-draft-conservation`. A.3 reste ouvert pour les recettes natives
multisystèmes, les incidents d’enregistrement et les changements externes.
Le [contrat](a3-conservation.md) et les [parcours validés](a3-recovery-interface.md)
restent les références ; les snapshots durables relèvent d’A.4.

## Parcours livrés

- L’accueil **Travail à reprendre** distingue les copies par projet et fenêtre,
  sans ouvrir de document automatiquement. Supprimer exige une confirmation et
  ne supprime pas les copies des autres fenêtres.
- **Reprendre** ouvre une copie indépendante, sans prendre le verrou du fichier
  initial. Le premier enregistrement demande un emplacement ; le nom suggéré
  comporte « récupéré ».
- Le panneau temporaire **Saisies à reprendre** ouvre les auteurs/affiliations dans
  leur formulaire existant, les notes/liens/widgets dans leur bandeau habituel,
  les champs simples dans leur onglet et leur contrôle. Aucun éditeur parallèle.
- Ouvrir, modifier le brouillon ou annuler sa reprise ne consomme pas la source.
  La validation utilise les commandes typées ordinaires. La consommation de la
  file attend la conservation durable ; un échec remet l’élément dans la file.
- Un texte refusé ou une cible modifiée/incompatible reste isolé, consultable,
  copiable et exportable. Les brouillons JSON exportés peuvent être importés.
  Leur contenu privé n’est jamais appliqué automatiquement.
- Les décisions de fermeture distinguent document propre, modifications
  ordinaires et saisies privées. L’action par défaut est à droite ; la
  destruction à gauche. Échap annule, Entrée choisit l’action par défaut.
  Une erreur laisse la fenêtre ouverte et permet de réessayer ; annuler le
  sélecteur d’enregistrement annule aussi la fermeture.
- Quitter avec plusieurs fenêtres explicite leur fermeture et la conservation
  de leurs saisies. L’abandon ne vise que la copie de la fenêtre demandeuse.
  Fermer une fenêtre native conserve les autres vues du projet.
- Nouveau/Ouvrir/Reprendre/Fermer le document/Quitter utilisent une barrière
  commune. Toutes les vues enregistrées, y compris celles dont la synchronisation
  est suspendue, doivent conserver leur travail avant le changement de génération.
  Les formulaires détachés sont également bloqués pendant cette barrière.
  Une composition, une erreur, une annulation ou une révision modifiée empêche
  le basculement.

## Garanties vérifiées

Les nouveaux tests couvrent notamment l’échec d’une copie de destination sans
consommation de la source, l’abandon strictement local, un deuxième incident après
réouverture d’un formulaire, la conservation de sa dernière saisie sans doublon,
la base d’un champ antérieure à une modification concurrente, l’import de contenu
refusé invalide sans publication, et la disponibilité de la copie d’une vue fermée
alors que d’autres vues restent ouvertes.

Ils ont aussi conduit à corriger deux nettoyages : supprimer une copie ne doit pas
effacer le résumé d’un autre projet ; annuler le dernier formulaire d’un projet
déjà enregistré ne doit pas laisser une fausse proposition de reprise. Le focus
d’un champ repris utilise sa liaison documentaire réelle et la section qui le
contient, plutôt qu’un nom d’onglet supposé.

Le clavier des bandeaux reste commun. Ctrl/Cmd + Entrée valide aussi un bandeau
contextuel repris. Les tableaux, listes, légendes et liens conservent leur voie
habituelle de publication et de restitution du contexte d’édition.

## Recette native macOS

Le [rapport brut](../experiments/a3-recovery/report-macos.json) correspond au commit
`56eba4da633cbe6ec39e316d70d0abad8fefab4c`, avec le frontend de production embarqué et
une véritable WebView Tauri. Les données sont synthétiques.

1. Saisir une note sans la valider, attendre la copie automatique, puis enregistrer
   l’archive confirmée sans appliquer le formulaire.
2. Interrompre uniquement le processus enfant de recette avec `SIGKILL`.
3. Rouvrir l’archive confirmée et vérifier que le formulaire subsiste sur disque,
   sans que son texte figure dans le document confirmé.
4. Redémarrer sur l’accueil, choisir **Reprendre**, vérifier l’indépendance du
   projet et l’absence de verrou sur le fichier initial.
5. Exporter le brouillon via la commande native ; vérifier son enveloppe, sa base
   confirmée et son contenu privé.
6. Rouvrir le bandeau ordinaire de la note, vérifier son texte et l’absence de
   publication, quitter avec Échap et constater que la saisie reste à reprendre.
7. Rouvrir, valider avec Cmd + Entrée, puis vérifier le document confirmé, la file
   vide et leur conservation durable.

Rejouer après le build web, puis celui du binaire natif :

```sh
npm run build:web
cargo build --locked -p typstographe --features recipe-driver
npm run test:draft-conservation-native -- /chemin/vers/un/dossier/vide
```

Le dossier doit être nouveau ou vide. La recette ne tue que son propre processus.
`TOG_SPIKE_BINARY` permet d’isoler son binaire d’une compilation simultanée dans
un autre checkout. Aucun screenshot, vidéo ou trace n’est produit.

## Vérifications et portée

- **319 tests Rust** du workspace, doctest inclus, dont **29 contrôles de récupération** ;
  un diagnostic de performance reste volontairement ignoré.
- **337 tests web** et **3 contrôles des langues natives**.
- **29 recettes de contrat** Yjs/Yrs ↔ Tiptap ↔ propriétaire Rust.
- **42 parcours Chromium/WebKit réussis** : reprise auteur et note, annulation, validation,
  deux fenêtres, champ refusé, remplacement d’image, import JSON, conservation
  existante et restitution du focus/contexte des bandeaux.
- TypeScript/Vite, Clippy avec et sans `recipe-driver`, rustfmt, Prettier,
  limites des modules Rust, dépendances du cœur et **15 tests Python**.

Ces contrôles locaux ne valent pas une validation native Windows/Linux, une
recette de clavier IME matériel ni un résultat de CI. Les compositions web sont
synthétiques. L’import JSON est exercé via le transport de recette et l’interface
réelle de consultation ; le sélecteur de fichiers natif reste un contrôle manuel.
Les scénarios d’incident/changement externe et la publication multisystème doivent
encore compléter A.3 avant sa clôture.

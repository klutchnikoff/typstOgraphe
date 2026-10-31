# A.5 — Documentation, nettoyage et recette finale

**31 octobre 2026 — implémentation et recette locale validées.** La qualification
CI multisystème du candidat reste distincte et doit réussir avant fusion.
Ce bilan distingue les contrôles locaux de cette qualification distante. Il ne vaut pas validation matérielle
ni autorisation de distribution signée.

## Périmètre

Les choix validés sont conservés : documentation courante séparée des bilans,
README bref, nettoyage ciblé sans réécriture du moteur, diagnostics Rust sous
`recipe-driver`, frontend Vite de recette distinct, puis même corpus Courrier/
Article sur Tauri, CLI et MCP. Aucun nouveau jeu de captures n’est produit.
Les rapports, résultats, licences et références visuelles existants sont conservés.

## Nettoyage et documentation

- [Index documentaire](README.md), guide de développement et contrats à jour ;
  phases closes retirées des roadmaps, indications historiques explicites.
- Diagnostics natifs et IPC de contrôle absents de la construction normale ;
  options de recette refusées avant tout accès aux projets ou création de fenêtre.
  La recette exige un répertoire explicitement isolé.
- Frontend normal sans pont `__spike`, commande d’essai Rust ni modules de mesure.
  `check:distribution` contrôle les **assets réellement produits**, aussi en CI.
- `build:recipe` associe `dist-recipe/` et `recipe-driver` ; il conserve `dist/`.
  Les exécutables sont copiés dans `target/recipes/<profil>/` et leur capacité
  STDIO est vérifiée, pour résister au remplacement de `target/debug` par une
  compilation normale simultanée. La CI utilise ce même parcours. Les deux modes utilisent le même moteur
  documentaire ; aucune validation simplifiée n’est introduite pour la recette.
- Commande `recover` sans appelant et commande d’essai `prepare_rust_edit` retirées ;
  la recette utilise maintenant `block.insert` du contrat public. `jsonschema`
  direct de l’hôte et `lib0` direct du frontend retirés ; `zip` de l’hôte déplacé
  en dépendance de test. Les dépendances transitives utiles restent verrouillées.
- Licence MIT du projet déclarée ; licences, provenances et inventaire Typst tiers
  conservés. Aucun changement de l’identifiant natif ni des préférences existantes.

## Recette depuis un checkout propre

Depuis un worktree propre, installer avec les lockfiles, construire les assets
normaux et de recette, exécuter les tests et lancer réellement la WebView native.
La recette `test:closure-native` couvre le même Courrier et Article avec édition
et métadonnées, MCP rattaché/autonome, refus/stale/acceptation humaine, version
nommée, reprise par la modale et une annulation/rétablissement, sauvegarde,
réouverture et exports hors ligne. Les sources, ressources et dépendances sont
comparées octet par octet entre adaptateurs ; les PDF sont identifiés et les
dimensions de pages comparées, sans exiger l’identité de leurs métadonnées variables.

La recette native réussie porte sur **`3329f78`**, depuis un worktree détaché
propre : `npm ci --offline`, frontend normal et de recette, vraie WebView native,
CLI et processus MCP. Le cache Cargo partagé a été réutilisé ; aucune dépendance
ni exemple personnel n’a été copié dans ce checkout. Les sources versionnées
restent propres après les essais. Les [rapports conservés](../experiments/a5-closure/README.md)
distinguent la recette complète, la conservation des versions et le lancement normal.

| Corpus | Pages finales | Fichiers de chaque export vérifiés | Versions conservées |
| --- | --- | --- | --- |
| Courrier | 2 | 791 fichiers (sources, ressources et paquets Datify inclus) | 1, état canonique vérifié après CLI/MCP/réouverture |
| Article | 3 | 13 fichiers | 1, état canonique vérifié après CLI/MCP/réouverture |

Le courrier part du script public, l’article du `.typsto` **versionné**. L’exemple
local modifié par l’utilisateur est resté intact. Les propositions rattachées
exercent refus, péremption et acceptation dans le comparateur ; l’agent n’obtient
ni droit d’acceptation ni droit de fichier. Le MCP autonome exerce les opérations
publiques, leurs gardes, lots atomiques, annulation et export. Les versions gardent
exactement le contenu du jalon malgré les éditions ultérieures de ces adaptateurs.

### Autres contrôles réussis

- **388 tests Rust** du workspace depuis le checkout propre, doctest compris ;
  un diagnostic de performance volontairement ignoré. Les tests Fluent sont
  également rejoués après retrait du dernier libellé de démonstration.
- **375 tests web**, **6 tests Node**, **29 contrats Yjs/Yrs**.
- **34 parcours navigateur sélectionnés** : 32 réussites lors de la passe, puis
  les deux scénarios dépendant de l’ancien bouton d’essai remis à jour et réussis
  sous Chromium et WebKit. Ils passent désormais par une vraie commande typée.
- Frontend normal, contrôle des assets distribués, binaires normaux/de recette,
  langues natives Mach-O, Clippy des deux configurations, TypeScript/Vite,
  Prettier, rustfmt, Actionlint, **15 tests Python**, frontières du cœur,
  limites de modules et liens de la documentation courante.
- Lancement réel du processus normal sans projet et refus des options de recette
  avec code 2 avant création d’un répertoire. Les snapshots et la récupération
  restent distincts ; la composition ne sauvegarde pas automatiquement.

Aucune capture, vidéo ou trace n’a été produite. Le répertoire technique
`coverage` et les références visuelles existantes restent conservés.

### Limites et qualification distante

Le contrôle de lancement normal prouve le démarrage du processus, pas une
inspection visuelle. Les interactions des recettes sont synthétiques ; claviers,
IME et presse-papiers matériels, installation, signature et notarisation restent
dans H. Le sélecteur de fichiers macOS partiellement anglais reste un problème
[consigné et reporté](native-localization-investigation.md). Les choix du jalon
facultatif à l’export restent inscrits au plan, sans nouvelle fonctionnalité ici.

La chaîne de CI occasionnelle inclut maintenant cette recette native en plus des
recettes A.3 et conserve ses rapports. Un **seul candidat complet sans packaging**
est à lancer après la poussée finale, sous les trois systèmes, avec la suite
navigateur exhaustive, les audits et le contrôle WebAssembly. La réussite
locale ne remplace pas cette qualification : consulter le workflow
[Validation complète](https://github.com/klutchnikoff/typstOgraphe/actions/workflows/exhaustive.yml).
Aucune réussite Windows/Linux ni aucun audit réseau récent n’est déduit des essais locaux.

### Problème de recette identifié et corrigé

La première tentative native depuis le checkout propre a lancé un binaire
normal à la place de celui de recette : une construction de développement
concurrente peut remplacer la sortie Cargo conventionnelle. Les gardes ont
refusé les options ; aucun document n’a été ouvert par erreur. Le lanceur copie
désormais les artefacts dans `target/recipes/<profil>/`, puis contrôle le support
STDIO avant de les annoncer utilisables. Tous les runners utilisent cet emplacement
par défaut. Le cache des dépendances Cargo reste partagé. La recette obsolète
qui cherchait « Accepter le lot » utilise désormais le comparateur et sa
validation chiffrée, puis vérifie la reprise d’une version par la modale commune.

La recette native attend la fin effective de la validation, la disponibilité de
la fermeture puis le retour du focus avant de demander Annuler. Des étapes de
progression rendent ces attentes observables dans le journal. La publication des
exécutables utilise elle aussi une copie temporaire vérifiée avant remplacement
atomique : un binaire chargé n’est pas tronqué par une construction suivante.

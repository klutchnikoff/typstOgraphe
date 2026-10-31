# A.3 — Recette de clôture et qualification native

**31 octobre 2026.** L’implémentation et les recettes de sortie sont livrées.
La première qualification native a réussi sous [macOS, Windows et Linux](https://github.com/klutchnikoff/typstOgraphe/actions/runs/41188235007).
Les rapports sont conservés ci-dessous. La qualification du commit courant et
l’archivage de ses rapports sont exécutés par le workflow [Validation complète](https://github.com/klutchnikoff/typstOgraphe/actions/workflows/exhaustive.yml).
Le lot A.4 commence après réussite de cette qualification et validation de ses propres choix.

Le [contrat de conservation](a3-conservation.md), les [parcours de reprise](a3-recovery-interface.md)
et leur [première recette](a3-recovery-results.md) détaillent les étapes précédentes.

## Critères de sortie

| Garantie | Recette et preuve |
| --- | --- |
| État Yjs/Yrs, arbre, métadonnées, modèle et Typst cohérents | Archive validée et état canonique du propriétaire Rust comparé exactement à celui de la CLI, pour Courrier et Article. Aucun nettoyage ad hoc des différences de contenu. |
| Enregistrement explicite, secours distinct | `.typsto` écrit sur demande ; formulaire privé conservé séparément après l’enregistrement et après incident. |
| Conservation après interruption | Arrêt forcé du seul processus enfant de recette, puis nouveau processus et réouverture réelle dans Tauri. |
| Échec d’écriture | Cible dont le parent est un fichier : refus déterministe, contenu courant, révision, CRDT, formulaire, ressources et verrou inchangés. |
| Changement externe | Un autre processus remplace réellement le fichier ouvert ; l’enregistrement est refusé et ses octets externes restent intacts. Enregistrer sous un autre nom permet de conserver le travail. |
| Ouverture concurrente | Un remplacement pendant la lecture/compilation est détecté avant publication et nettoyage de la récupération ; l’ancien document et son verrou restent ouverts. |
| Archive refusée | Une archive invalide n’efface ni l’état courant ni le formulaire privé. Les tests d’archives couvrent également les contradictions entre source, arbre et Yjs. |
| Plusieurs vues et transitions | Chaque vue enregistrée doit produire un checkpoint postérieur au début de la transition, y compris après suspension de sa synchronisation. Refus, composition et annulation bloquent le remplacement. |
| Ressources et export | Sources, dépendances et ressources des exports natifs, après réouverture et CLI comparées octet par octet ; octets de l’image vérifiés et dimensions des pages comparées. Les PDF sont vérifiés comme PDF, sans prétendre comparer leurs métadonnées variables. |
| Reprise et fermeture | Accueil sans réouverture implicite, copie indépendante, file de saisies, formulaire habituel, annulation non destructive, validation explicite et consommation durable. |

Les protections renforcées dans ce lot disposent de tests ciblés : une copie
antérieure à la transition ne permet plus de l’acquitter ; une cible modifiée
pendant l’ouverture ne permet plus de remplacer le document courant. Un troisième
cas vérifie qu’un échec de nettoyage après Enregistrer sous ne bascule pas déjà
le document vers la copie : le verrou et l’empreinte du fichier d’origine restent
protecteurs contre une modification externe. Pour un enregistrement sur place,
la nouvelle tentative reconnaît au contraire les octets déjà écrits par elle-même.

## Qualification native multisystème conservée

La première passe native au commit `f7b3785` est réussie sur les trois systèmes :
[Linux](../experiments/a3-closure/ci-linux.json),
[macOS](../experiments/a3-closure/ci-macos.json) et
[Windows](../experiments/a3-closure/ci-windows.json).
Ces JSON proviennent des sorties des scripts natifs des jobs réussis, sans
normalisation de leur contenu. Le chemin des artefacts a ensuite été corrigé :
la passe finale doit également conserver les fichiers de rapport et ses tests
de gardes ciblés. Un archivage manquant fait désormais échouer le job.

## Résultats locaux conservés

- [Publication Courrier/Article sous macOS](../experiments/a3-closure/publication-macos.json),
  au commit `282f135` : erreurs réelles, sauvegarde indépendante, interruption,
  réouverture et comparaison Tauri/CLI des exports et dimensions des pages.
- [Reprise après incident sous macOS](../experiments/a3-closure/recovery-macos.json) :
  retour à l’accueil, copie indépendante, restauration de note dans son bandeau,
  Échap sans perte, validation et export du brouillon.
- **323 tests Rust** réussis, doctest inclus ; un diagnostic de performance ignoré.
- **337 tests web**, **3 contrôles des langues natives**, **29 recettes du contrat
  partagé**, puis **20 parcours Chromium/WebKit** ciblés sur conservation/reprise.
- TypeScript/Vite, Clippy avec et sans `recipe-driver`, rustfmt, Prettier,
  Actionlint, limites des modules, dépendances du cœur et **15 tests Python**.

Les rapports locaux identifient leur commit ; ils ne revendiquent pas une
exécution native Windows/Linux. Celle-ci est identifiée par les rapports et le
SHA du workflow exécuté, disponibles dans ses artefacts `a3-acceptance-<système>`.

## Rejouer les recettes

Depuis un checkout propre, construire le frontend avant les binaires qui l’embarquent :

```sh
npm ci
npm run build:web
cargo build --locked -p typstographe -p typstographe-cli --features typstographe/recipe-driver
npm run test:draft-conservation-native -- /chemin/vers/un/premier/dossier/vide
npm run test:publication-native -- /chemin/vers/un/second/dossier/vide
```

Chaque dossier doit être nouveau ou vide. Les recettes n’interrompent que les
processus qu’elles ont créés et ne touchent pas aux projets personnels. Une session
graphique est requise. Sous Linux, la CI utilise Xvfb et une session D-Bus ; sous
macOS/Windows, la WebView native est lancée directement. `TOG_SPIKE_BINARY` et
`TOG_CLI_BINARY` permettent d’isoler les exécutables d’une compilation simultanée.
Aucun screenshot, vidéo ou trace graphique n’est produit.

## Coût et portée de la CI

Les recettes natives font partie des validations complètes hebdomadaires, des tags
et des lancements manuels. Elles ne sont pas ajoutées à chaque PR ou push ordinaire.
Pour ne lancer que cette qualification, choisir **acceptance_only = true** et
**packages = false** dans le workflow manuel. Ce mode compile l’application et la
CLI, puis les gardes de stockage/récupération ciblés et les recettes natives ; il évite de répéter le workspace, Clippy,
les audits, WebAssembly, tous les navigateurs et la construction des paquets.
Les rapports et journaux sont conservés même après un échec.

Les interactions de clavier des recettes sont synthétiques : elles n’établissent
pas une qualification de tous les IME matériels ni des sélecteurs de fichiers.
La fermeture de session OS, les coupures matérielles, l’installation, la signature
et la notarisation restent des contrôles de distribution du chantier H. La fenêtre
de deux/dix secondes de la copie de secours et les limites du stockage demeurent
celles du contrat ; aucune absence absolue de perte n’est promise.

Les snapshots nommés, comparaisons historiques, politique de rétention et annulation
entre séances restent dans A.4 ; ils ne sont pas ajoutés par cette clôture.

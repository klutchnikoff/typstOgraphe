# Latence de l’aperçu Typst — 26 octobre 2026

La réactivité est un critère d’architecture de typstOgraphe. L’interface web doit
être évaluée avec le vrai moteur Typst, le vrai transport Tauri et un affichage
préparé avant publication. Un délai artificiel de développement ne doit pas
fausser cette évaluation. Une mesure côté Rust ne représente pas à elle seule
le délai entre une édition et l’apparition du rendu.

## Chemin retenu

Le rendu interactif utilise les pixels RGBA de `typst-render`, transmis par une
réponse binaire Tauri (`Response` → `ArrayBuffer`) vers un canvas. Le petit en-tête
JSON contient les dimensions, révision, tailles des pages et mesures ; les pixels
suivent directement. Le lecteur vérifie format, bornes et longueur des pixels.
Les vues JavaScript référencent le buffer reçu ; l’IPC et le canvas peuvent
néanmoins faire des copies. Il ne s’agit pas d’une mémoire partagée avec Rust.

La résolution reste de deux pixels par point Typst, soit 1 191 × 1 684 pixels
pour une page A4. La transparence est conservée : `ImageData` reçoit des couleurs
non prémultipliées, identiques à celles des PNG précédents. La mise en page,
les polices et la carte de synchronisation restent celles du cœur Rust.

La file de rendu démarre à la prochaine microtâche, sans attente de 180 ms. Elle
regroupe les demandes du même événement et compile un seul instantané à la fois.
Pendant une compilation, seule une nouvelle demande pour l’état courant est
conservée. Les validations de révision, version et identité du document empêchent
un résultat périmé de remplacer le dernier aperçu valide.

La préparation de la carte de synchronisation résout chaque span de syntaxe une
seule fois par compilation et réutilise les métriques de chaque portion de texte.
Ce cache est local à la compilation ; les cartes des révisions précédentes restent
indépendantes. Un test réutilise le même monde Typst pour plusieurs sources tout
en conservant leurs anciennes cartes. Le profil de développement optimise aussi
`typst-render`, comme les bibliothèques de pixels déjà optimisées.

L’attente du relais Yjs couvre également les mises à jour qui s’ajoutent pendant
un accusé de réception. L’instantané est pris après cette attente complète.

Les pages sont préparées hors du DOM affiché puis publiées ensemble. Les images
PNG/SVG des chemins de comparaison attendent leur décodage et libèrent leurs URL
Blob après remplacement, annulation ou erreur. Le canvas reçoit ses pixels avant
publication. Aucun fichier temporaire, base64 ou serveur d’images n’intervient
dans le chemin interactif natif retenu. L’export PDF garde son chemin existant.

## Méthode

Le banc `test:preview-native` démarre la vraie application Tauri avec les assets
frontend de production embarqués, sans Vite et sans le relais HTTP des tests de
navigateur. Chaque essai possède son dossier de session isolé.

Quatre chemins partagent validation, compilation et synchronisation : PNG en
URL base64 (ancien chemin optimisé), PNG binaire, RGBA binaire et SVG natif de
Typst en binaire. Le SVG est chargé comme une image Blob, sans injecter son XML
dans le DOM de l’application. Les autres chemins servent à la comparaison ;
aucun sélecteur technique n’est ajouté à l’interface utilisateur.

Trois contenus : courrier d’une page, fixture avec tableau, note et image sur
deux pages, puis 60 paragraphes sur cinq pages. Dix tours modifient le texte ;
l’ordre des transports tourne à chaque tour. Le premier tour est écarté comme
échauffement, après initialisation de l’application et des polices. Chaque ligne
résumée contient donc neuf mesures ; le p95 est ici le maximum de ce petit
échantillon. Les résultats bruts restent disponibles.

- `backendMs` : durée du rendu côté Rust, sans projection préalable ni IPC.
- `requestMs` : invocation jusqu’à réception, avec validation/projection et IPC.
- `prepareMs` : préparation de l’affichage après lecture du résultat jusqu’à sa
  publication ; pour PNG/SVG cela inclut `decode()`, pour RGBA le dessin du canvas.
- `frameMs` : requête jusqu’à deux `requestAnimationFrame` après publication,
  pour laisser une occasion de peindre. Ce n’est pas un horodatage de présentation
  GPU ni une mesure physique de l’écran. La cadence de rafraîchissement quantifie
  fortement cette valeur, particulièrement sur les petits documents.
- `typing` : transaction de l’éditeur jusqu’au même point de mesure, par la voie
  automatique normale, avec file de rendu, relais Yjs et attente de synchronisation.
  La rafale insère 15 caractères espacés de 40 ms, compte les publications, puis
  mesure le rendu du dernier caractère. Les transactions sont synthétiques ; le
  passage par le clavier système et la méthode de saisie n’est pas chronométré.

La fenêtre doit rester visible. Le banc refuse une mesure si elle devient
masquée ou si les frames sont suspendues. Les premiers essais ont rencontré
cette condition et des parcours de navigateur ont également été interrompus ;
ils ne servent pas à la conclusion. Sur macOS, `caffeinate -di` empêche la mise
en veille automatique uniquement pendant la recette. Aucun réglage de limitation
en arrière-plan de WebKit n’est désactivé dans l’application.

## Résultats

Machine : **Apple M1 Max, 32 Gio, macOS (Darwin 25.6.0)**, WebKit natif,
fenêtre de 1 280 × 868 pixels CSS, facteur de pixels 2. Mesures à chaud en
millisecondes, **médiane / p95** (neuf mesures par cellule).

### Rust optimisé (`release`)

| Contenu | PNG base64 | PNG binaire | RGBA binaire | SVG binaire |
| --- | ---: | ---: | ---: | ---: |
| Courrier · 1 page | 50 / 50 | 50 / 50 | 33 / 34 | 33 / 34 |
| Tableau, note et image · 2 pages | 50 / 62 | 50 / 50 | 33 / 34 | 33 / 34 |
| Texte · 5 pages | 110 / 117 | 83 / 519 | 50 / 66 | 200 / 217 |

Le RGBA est retenu : il rejoint le SVG sur les petits contenus et reste plus
rapide sur le texte dense de cinq pages, malgré un transfert plus volumineux.
Le SVG a un faible coût d’invocation, mais sa préparation/décodage dans WebKit
augmente fortement sur ce dernier contenu. Une pointe isolée de 519 ms apparaît
pour le PNG binaire ; ce petit échantillon et l’alternance des transports ne
permettent pas de l’attribuer avec certitude au seul format.

Dans la chaîne automatique de rédaction, la médiane est de **33 ms** pour les
modifications unitaires des deux contenus (12 mesures chacun). Les deux rafales
ont publié **15 aperçus pour 15 caractères** ; le rendu du dernier caractère
arrive en **32 ms** sur le courrier et **48 ms** sur la fixture. Ce résultat
concerne ce corpus et cette machine, sans constituer une garantie universelle.

### Profil de développement

| Contenu | PNG base64 | PNG binaire | RGBA binaire | SVG binaire |
| --- | ---: | ---: | ---: | ---: |
| Courrier · 1 page | 83 / 100 | 84 / 95 | 50 / 50 | 67 / 78 |
| Tableau, note et image · 2 pages | 133 / 150 | 117 / 195 | 50 / 82 | 67 / 144 |
| Texte · 5 pages | 450 / 466 | 317 / 349 | 100 / 117 | 401 / 467 |

Le chemin RGBA, la résolution et les contrôles de cohérence sont identiques.
La rédaction complète mesure environ **50 ms** en médiane pour le courrier et
**67 ms** pour la fixture. La rafale artificielle à 25 caractères/s dépasse la
cadence soutenue du rendu debug sur la fixture : trois aperçus intermédiaires
sont publiés, puis le dernier caractère est rendu en **74 ms**. Il n’y a pas de
file d’anciennes compilations à vider. La recette optimisée ci-dessus est la
référence pour évaluer le confort final.

La mutualisation des recherches de spans réduit la préparation de la carte de
synchronisation de **117,7 à 12,8 ms** en médiane sur les cinq pages en debug.
Le délai total RGBA correspondant passe de **216 à 100 ms**. La mesure
intermédiaire conserve explicitement son contexte d’arbre Git non committé.

Rapports :

- [Optimisé](../experiments/web-editor/results/preview-native-release.json).
- [Développement](../experiments/web-editor/results/preview-native-dev.json).
- [Développement avant le cache de spans](../experiments/web-editor/results/preview-native-dev-before-span-cache.json).

Captures fonctionnelles du canvas, vérifiées visuellement, via le relais de tests
de navigateur (leurs compteurs ne sont pas des mesures d’IPC natif) :
[mode clair](tauri-interface-results/preview-rgba-light.png),
[mode sombre](tauri-interface-results/preview-rgba-dark.png).

## Reproduire

Depuis la racine du dépôt, après `npm ci` :

```sh
npm run build:recipe -- --release --offline
npm run test:preview-native
```

Sur macOS, avec une session graphique déverrouillée et la fenêtre visible :

```sh
caffeinate -di npm run test:preview-native
```

Le script affiche le dossier du rapport `native-performance-report.json`, sous
`output/web-editor/`. `TOG_SPIKE_BINARY` permet de choisir explicitement un autre
binaire et `TOG_BENCH_PROFILE` en précise le profil dans le rapport. Pour le profil
de développement, compiler aussi `typstographe` (frontend embarqué par défaut), puis pointer
sur `target/debug/typstographe`. Il utilise les mêmes mécanismes
et la même résolution ; seules les optimisations Rust diffèrent.

Pour un essai manuel optimisé, le binaire release compilé ci-dessus se lance
directement : `./target/release/typstographe`. Aucun serveur
Vite n’est nécessaire dans ce mode. `npm start` reste disponible pendant
le développement et utilise lui aussi le transfert RGBA natif.

## Vérifications et limites

Validation finale : **276 tests du cœur et 2 doctests, 10 tests Tauri** (un
diagnostic de performance ignoré par défaut), **63 tests frontend et 66 parcours
Chromium/WebKit** passent ; formatage et Clippy du cœur et de l’adaptateur passent.

Les tests Rust comparent les pixels RGBA binaires au décodage des PNG pour un
modèle opaque et un modèle transparent. Les tests frontend vérifient les bornes
du paquet, la file sans accumulation et la libération des ressources. Les parcours
Chromium/WebKit vérifient le vrai rendu Rust, le remplacement atomique des pages,
la conservation du dernier canvas après une erreur, la sélection synchronisée,
la navigation, les widgets, l’enregistrement et la collaboration.

Le relais STDIO/SSE des tests de navigateur utilise une enveloppe base64 interne
pour porter les réponses binaires. Cette enveloppe ne participe jamais aux
mesures natives ; ses durées ne représentent pas celles de Tauri.

Le RGBA coûte environ 8 Mo par page A4, soit 40 Mo transférés pour cinq pages.
Les images, buffers et surfaces anciennes/nouvelles peuvent coexister pendant
une mise à jour. La file empêche l’accumulation de compilations, mais le rendu
porte encore sur toutes les pages. Pour les documents longs, les prochains axes
sont la réutilisation des pages inchangées et la préparation des pages visibles,
avec des contrôles de cohérence équivalents. Ces optimisations restent à faire.
Les résultats macOS ne préjugent pas des latences WebView2 et WebKitGTK ; les tests
fonctionnels et la recette native restent utilisables sur les autres systèmes.

| Avant | Après | Pourquoi |
| --- | --- | --- |
| Attente de 180 ms relancée après chaque modification. | Demande immédiate et au plus une compilation active. | Réduire le délai et éviter une file de rendus obsolètes. |
| Pixels → PNG → base64 → décodage d’image pour tout aperçu. | Pixels → IPC binaire → canvas préparé avant publication. | Éviter compression, sérialisation base64 et décodage PNG. |
| Seul le compteur Rust permettait de discuter des performances. | Mesures natives par étape et après publication, avec comparaison de quatre transports. | Inclure le coût réellement ajouté par l’interface. |

# Phase 4 — CLI documentaire

Décisions validées le **24 octobre 2026** avant implémentation : D23 et D24 dans
[ROADMAP-ARCHITECTURE.md](https://github.com/klutchnikoff/typstOgraphe/blob/7b0d6fd3e6ae25e5704ed43ce2f4efefa9c2fa0b/ROADMAP-ARCHITECTURE.md). **Phase terminée le 24 octobre 2026.**

## Premier lot

La crate `typstographe-cli` dépend uniquement du cœur, de l’i18n partagée, de clap
et de serde_json. Aucun écran, profil personnel, signature ou dictionnaire personnel
global n’est chargé implicitement. Modèles, packages, ressources et exceptions
orthographiques propres au document proviennent de l’archive.

- `commands list [--query]` et `commands describe <id>` exposent le catalogue et ses
  schémas exacts, sans redéclarer les capacités dans l’adaptateur.
- `inspect` regroupe les lectures de session, métadonnées, structure et statistiques.
- `check` utilise la validation du moteur ; `--for-export` exige aussi les champs
  requis et `--spelling` demande les dictionnaires français/anglais embarqués.
  Une langue non couverte est signalée, sans prétendre que le texte est vérifié.
- `export --format pdf|typst|both --output dir [--pages]` appelle l’export commun.
  PDF par défaut, sans PNG implicites ; répertoire absent ou vide, jamais écrasé.
- `--json` donne un seul objet versionné, erreurs comprises, sans prose sur stdout.
  `--lang fr|en` choisit la langue de présentation, indépendante du document.
  Les descriptions techniques et schémas du registre gardent leur texte contractuel.
- `commands list --markdown` génère [docs/cli.md](cli.md), comparé en test à la
  référence enregistrée. Le catalogue décrit aussi les opérations du moteur dont
  le déclencheur CLI reste à développer dans les lots suivants.

## Accès aux fichiers

Le verrou appartient à la session documentaire privée du cœur. `document.open`
accepte l’option additive `read_only`, fausse par défaut. Le catalogue reste à
49 commandes et 19 requêtes. `document.inspect` expose le booléen `read_only`.

Une session d’écriture réserve un compagnon `.nom.typsto.lock` avec
`std::fs::File::try_lock`. Les chemins sont canonicalisés, liens symboliques compris.
Le compagnon n’est pas supprimé au déverrouillage : le remplacer permettrait de
verrouiller deux fichiers différents pour la même archive. Le verrou OS disparaît
à la fermeture ou au plantage ; le fichier vide peut subsister sans bloquer la suite.

Le verrou reste détenu pendant les éditions et sauvegardes atomiques. Les candidats
des lots partagent seulement sa durée de vie via `Arc<File>`, sans session mutable
partagée. Un échec conserve le verrou et le travail publié ; un Enregistrer sous
réussi transfère la réservation à la nouvelle destination.

Les lecteurs ne créent pas de compagnon et chargent la dernière version enregistrée
depuis un même descripteur. Ils peuvent consulter, sélectionner, chercher, vérifier,
rendre et exporter. Mutations, historique et saisie native sont refusés par le moteur.
Une copie vers un autre fichier redevient éditable.

L’UI propose la lecture seule en cas de verrou, l’indique dans le titre et désactive
les commandes et champs d’édition. L’éditeur garde sélection et copie possibles
avec un tampon egui non mutable.

Une empreinte SHA-256 du fichier lu est comparée avant sauvegarde. Une modification
ou suppression externe refuse l’enregistrement (`file_changed`) et préserve le travail
en mémoire. L’empreinte écrite est calculée sur le temporaire avant publication :
aucune lecture faillible après l’écriture atomique, notamment pour les lots.
`file_locked`, `file_read_only` et les erreurs d’accès utilisent l’i18n commune.

Le protocole protège les processus coopérants sur les chemins locaux. L’empreinte
ne rend pas transactionnel un outil externe ignorant les verrous et écrivant pendant
la sauvegarde. Liens physiques et systèmes distants ne constituent pas un protocole
collaboratif. La gestion des sessions d’agents MCP sera discutée en phase 5.

## Vérification du lot

- Processus CLI : parité catalogue/schémas, lecture avec un écrivain actif, parité
  métadonnées/structure/compteurs, brouillon distinct de l’export et de l’orthographe,
  exports PDF/Typst autonomes et PNG optionnels, refus d’écrasement, codes de sortie,
  JSON propre, langues fr/en et référence générée.
- Moteur : écrivain unique, lecteurs indépendants, verrou après remplacement atomique,
  fermeture et copie, modification externe de même taille, suppression externe,
  échec de remplacement et de lot, alias symboliques sur Unix, deux processus
  et libération après arrêt brutal du détenteur.
- egui : proposition de lecture seule, titre, saisie refusée, disponibilité des
  commandes et retour à l’édition après copie.
- Tests inclus dans `cargo test --locked --workspace`, donc dans la matrice CI
  macOS/Windows/Linux. L’exécution locale est macOS.

Contrôles locaux du 24 octobre : **378 tests Rust réussis** (220 cœur, 138 UI,
7 i18n, 7 processus CLI et 6 diagnostic de tableau), **14 tests Python** réussis,
formatage et Clippy sans avertissement sur tout le workspace et ses cibles,
frontière de dépendances du cœur et limite de 800 lignes de production respectées.
La CI distante des trois systèmes reste à confirmer après la poussée.

## Deuxième lot — création et appels du registre

Livré le **24 octobre 2026** :

- `new --type <id> [--model <id>] --output fichier.typsto` consulte
  `document_types.list`, sans liste parallèle des types/modèles dans la CLI.
  Le modèle est choisi automatiquement seulement s’il est unique. Les valeurs
  initiales et la signature éventuelle sont explicites, sans profil GUI.
- `query <id> [fichier.typsto]` exécute les requêtes du registre. Les lectures
  documentaires exigent un fichier ; découverte globale et suggestions sur un
  mot explicite restent disponibles sans document.
- `run <id> fichier.typsto` travaille par défaut sur un moteur indépendant construit
  à partir de la dernière archive enregistrée. Le résultat contient source et
  métadonnées résultants, avec `saved: false` et un message explicite. `--output`
  crée une nouvelle copie, même avec un écrivain actif sur la source. `--save`
  prend le verrou avant de lire, puis utilise la sauvegarde atomique du moteur.
  Aucune destination existante n’est écrasée. Les deux options sont exclusives.
- Les paramètres JSON UTF-8 se donnent avec `--params` ou `--params-file` (`-` pour
  stdin), avec une limite de 4 Mio. Les ressources et chemins du JSON sont relatifs
  au répertoire courant, même si le JSON est lu ailleurs.
- `--select` recherche un texte exact sensible à la casse. Une ambiguïté exige
  `--occurrence N`. `--block N` utilise l’ordre de `document.structure` ; les
  numéros commencent à 1. Recherche, blocs, images, orthographe et remplacement
  utilisent les sélections/cibles typées du moteur courant, sans indices egui.
- Les gardes sont fournies automatiquement. Les références de session copiées
  depuis un ancien processus sont refusées ; les cibles symboliques `current/all`
  restent utilisables. Les lectures de géométrie compilent leur propre rendu
  et résolvent leur référence dans le même processus, sans écrire de PNG.
- Les appels génériques refusent les opérations de fichiers, lots, historique
  et récupération avant d’ouvrir le document. Les fichiers utilisent les commandes
  dédiées ; les scripts atomiques restent le lot suivant. Aucun choix sur les
  sessions MCP n’est anticipé.

Le code de production du cœur et de l’application ne change pas dans ce lot.
Les modules CLI de lecture des paramètres, résolution des cibles et exécution
des appels sont séparés pour leur réutilisation lors des scripts.

Vérification locale : **385 tests du workspace réussis**, dont 20 tests de processus
CLI (7 existants, 4 de création, 9 d’appels). Ils couvrent parité avec le moteur,
réouverture par `document.open`, sélection Unicode d’une occurrence, modes aperçu/
copie/sauvegarde, verrous, refus d’écrasement, paramètres fichier/stdin, bornes,
familles interdites, références périmées et requêtes de géométrie sans fichiers.
Clippy sur toutes les cibles, formatage et gardes d’architecture passent.

Le parcours manuel CLI crée un courrier avec ses métadonnées, insère et met en
forme son texte, le valide pour export et produit son PDF autonome. Les données et
résultats de l’essai sont conservés dans `output/phase-4-next-manual/` (ignoré par Git).
L’archive ainsi créée a été ouverte dans l’application native et sa capture
`courrier-gui.png` inspectée : texte, gras, métadonnées et japonais sont correctement
affichés dans l’éditeur et le rendu. Les 14 tests Python d’architecture passent ;
l’arbre des dépendances normales de la CLI ne contient ni egui, eframe, winit, muda
ou rfd. Aucune dépendance supplémentaire n’a été ajoutée dans ce deuxième lot.

## Troisième lot — scripts JSON atomiques

Livré le **24 octobre 2026** :

- `script job.json` ou `script -` lit une enveloppe stricte de version 1 : un document
  à ouvrir ou créer, des étapes `{id, params, target?}` et une sauvegarde facultative.
  Les champs inconnus sont refusés ; 1 à 256 étapes, 4 Mio maximum.
- `document.kind: open` charge la dernière version enregistrée ; `new` utilise le
  catalogue de types et modèles et les paramètres de création existants. Aucun
  profil personnel n’est chargé implicitement.
- Sans `save`, le résultat reste en mémoire et la réponse contient le source et les
  métadonnées résultants avec `saved: false`. `save.kind: current` prend le verrou
  avant la lecture ; `as` crée une nouvelle archive sans écraser une destination.
  Une copie reste possible avec un écrivain actif sur l’original.
- Les chemins des archives, modèles, images et signatures sont relatifs au dossier
  canonique du script ; sur stdin, au répertoire courant. Le répertoire global de
  travail ne change pas. Seuls les chemins des paramètres typés sont adaptés, jamais
  les textes du document ou ses métadonnées.
- Les sélecteurs facultatifs `target.select/occurrence` et `target.block` suivent les
  règles de `run`. Ils sont résolus juste avant chaque étape, sur le brouillon
  courant : une image ou du texte insérés plus tôt peuvent donc être ciblés ensuite.
  Sans cible, la sélection courante continue entre les étapes, notamment pour
  remplir un P.-S. ou une mention de P.J.
- Le cœur expose `run_resolved_batch` pour préparer les cibles dans le véritable
  brouillon privé. Il réutilise l’exécution, les contrôles de ressources et la
  publication des lots existants : une seule transaction annulable, au plus une
  sauvegarde finale, aucune sauvegarde intermédiaire. Les gardes sont calculées
  dans le processus, sans références de session importées.
- Les identifiants et leur éligibilité aux lots sont contrôlés avant le chargement
  du document, puis les commandes résolues sont vérifiées dans le cœur. Fichiers,
  export, historique, récupération, requêtes et lots imbriqués sont interdits dans
  les étapes. Le budget couvre aussi les paramètres après résolution.
- Une erreur tardive ou de sauvegarde conserve la session publiée et les octets du
  fichier précédent. Les erreurs d’étape portent leur indice à partir de 0,
  l’identifiant de commande et l’erreur d’origine ; les messages traduits numérotent
  à partir de 1. Un compagnon de verrou vide peut subsister après l’échec.

Le catalogue reste à **49 commandes et 19 requêtes**, avec les mêmes schémas :
le résolveur est une API Rust pour les adaptateurs, l’enveloppe du script appartient
à la CLI. `serde`, déjà utilisé dans le workspace, est réutilisé pour la décoder ;
aucune nouvelle bibliothèque externe n’est introduite.

Vérification locale : **396 tests Rust du workspace réussis** (223 cœur, 138 UI,
7 i18n, 28 processus CLI). Les 3 nouveaux tests du cœur vérifient le brouillon évolutif,
l’historique en une transaction, les échecs tardifs/finaux et la protection du
résolveur. Les 8 nouveaux tests de processus couvrent le courrier complet, les modes
aperçu/copie/sauvegarde, les verrous, les chemins depuis un autre répertoire, les
images et leur nettoyage, la signature, stdin, l’enveloppe stricte et les limites.
Formatage, Clippy sur toutes les cibles, 14 tests Python, frontière du cœur et
limite de taille des fichiers de production passent. La CI distante reste à confirmer.

Le script suivi dans `examples/cli/courrier.json` crée un courrier personnel avec
expéditeur, destinataire, date fixe, objet, corps, gras, P.-S. et P.J. L’archive
`output/courrier-script.typsto` est validée pour export, puis exportée séparément
en PDF et Typst dans `output/courrier-script-export/`. Sa réouverture native et la
capture `output/phase-4-script-courrier-gui.png` ont été inspectées. Les résultats
JSON, captures et journaux du lot sont conservés dans `output/` (ignoré par Git).

### Contrôle Windows des fins de ligne

La CI des deuxième et troisième lots a révélé un échec Windows dans la comparaison
de la documentation générée : Git extrait le préambule Markdown en CRLF, tandis
que les lignes du catalogue sont générées en LF. Le générateur normalise désormais
le préambule en LF, sans assouplir le contrôle de dérive. Un test reproduit les deux
extractions et exige une sortie identique sans CR ; les 29 tests de la CLI passent
localement. macOS, Linux et couverture ont réussi sur la poussée du troisième lot ;
le correctif fait l’objet d’une nouvelle vérification distante.

Cette vérification a confirmé la réussite de Windows et Linux, puis révélé un
échec intermittent macOS dans le test de changement de format : si le worker
termine l’aperçu A3 avant la frame suivante, sa texture de 2381 pixels dépasse la
limite de 2048 pixels du contexte egui sans fenêtre. L’adaptateur graphique borne
désormais les textures d’aperçu à `max_texture_side`, avec rééchantillonnage bicubique
par tiny-skia déjà disponible via resvg. Le document Typst, le PDF et les coordonnées
de synchronisation restent intacts. Un test injecte un véritable rendu A3 terminé
avant la frame pour vérifier cette limite, les proportions et la synchronisation.

Ces deux corrections ont été confirmées par la CI du commit `8cc9554` :
[macOS/Windows/Linux](https://github.com/klutchnikoff/typstOgraphe/actions/runs/40099125227)
et [couverture](https://github.com/klutchnikoff/typstOgraphe/actions/runs/40099125202).

## Dernier lot — page PNG et retrait du diagnostic graphique

- `render fichier.typsto --page N --png page.png` ouvre la dernière archive
  enregistrée en lecture seule, utilise la compilation commune puis ne rasterise
  que la page demandée. Numérotation CLI à partir de 1, options obligatoires ; un
  brouillon compilable peut être rendu sans les champs exigés pour export.
- La réponse JSON expose le chemin, le numéro, le nombre de pages et leurs
  dimensions en points Typst. Le PNG reste dans le fichier, pas dans stdout.
  Les codes de sortie et la localisation fr/en restent ceux de la CLI.
- L’API Rust `Engine::render_page_png` lit le cache de rendu gardé : aucun accès
  mutable à la session, aucune recompilation, aucune modification d’historique.
  Cette lecture binaire pour les adaptateurs ne modifie pas les **68 contrats JSON**
  du catalogue. La rasterisation/encodage est commune avec les PNG d’export.
- Le PNG conserve la résolution Typst (2 pixels par point, 144 ppp), indépendamment
  des textures egui. Publication depuis un temporaire voisin avec
  `tempfile::NamedTempFile::persist_noclobber`, après écriture et synchronisation :
  pas d’écrasement, y compris en cas de création concurrente. Destinations occupées,
  dossiers et liens symboliques sont refusés ; le dossier parent doit exister.
  Un échec ne laisse pas de PNG partiel et préserve le document.
- `--render-demo`, sa branche de lancement et ses messages d’aide sont supprimés.
  Les captures restent des outils de développement. Le script de références construit
  les deux binaires et prépare sa fixture avec `new` puis `export --format typst`,
  à partir de `fixtures/capture-letter-params.json`. Aucun profil implicite ou
  commande spéciale de production n’est nécessaire. `--binary`/`--cli-binary`
  permettent de réutiliser les binaires déjà construits.

Vérification locale de clôture : **404 tests Rust du workspace réussis** (224 cœur,
139 UI, 7 i18n, 34 CLI dont 33 tests de processus). Le nouveau test du cœur vérifie
le cache gardé, les pages et l’absence de mutation. Les 5 nouveaux tests de processus
vérifient parité exacte avec moteur et export, écrivain actif, absence d’écritures
annexes, pages hors limites, arguments obligatoires, erreurs de compilation/écriture,
refus d’écrasement, liens symboliques sur Unix, chemins Unicode/relatifs et langues.
Ces tests sont inclus dans la matrice CI des trois systèmes ; l’exécution locale
est macOS. Le formatage, Clippy de toutes les cibles, les 14 tests Python et les
gardes de dépendances et de taille des fichiers de production passent.
`tempfile` était déjà utilisé dans le workspace et en test CLI ; aucune nouvelle
bibliothèque externe n’est ajoutée et la CLI n’a pas de dépendance graphique.

La comparaison native conserve **les huit références strictement identiques**
(dimensions et empreintes), sans réenregistrer leur manifeste. Captures et journaux
du contrôle : `output/baseline/run-0_krgait/` et `output/phase-4-final-captures.log`.
Le lanceur refuse maintenant `--render-demo` avant toute fenêtre.

Le scénario de clôture repart de `examples/cli/courrier.json` avec une destination
isolée : création, métadonnées, texte et gras, P.-S./P.J., sauvegarde, validation
pour export, exports PDF/Typst/PNG puis rendu de la page 1 par la commande dédiée.
Ce PNG a été inspecté visuellement et comparé octet pour octet à `page-1.png`
de l’export. Résultats conservés dans `output/phase-4-final-acceptance/`.
La [documentation CLI](cli.md) a été régénérée et son contrôle de dérive passe.

## Suite

Les critères de sortie de la phase 4 sont remplis. La phase 5 concerne MCP ; ses
choix principaux, notamment la politique de sessions et d’accès aux fichiers pour
les agents, seront validés ensemble avant de coder. Aucun serveur MCP ou canal
de contrôle de l’application n’est ajouté dans la phase 4.

Outils réutilisés : [clap](https://docs.rs/clap/4.6.7/clap/),
[assert_cmd](https://docs.rs/assert_cmd/2.2.2/assert_cmd/),
[verrous Rust](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock).

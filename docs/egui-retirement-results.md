# Retrait d’egui et reprises utiles

Lot du **27 octobre 2026**, après l’[inventaire préalable](egui-retirement.md).
Tauri est désormais la seule interface graphique compilée. Les modifications
de présentation restent un travail distinct, à discuter ensuite.

## Retrait et conservation

- Suppression de `crates/app/`, de ses vues, du tampon plat, de sa galerie et
  de ses outils de capture. Retrait d’eframe et des dépendances directes propres
  à cette interface ; le verrou Cargo perd **195 paquets**, sans nouvelle version
  ajoutée. `resvg` reste nécessaire au cœur. Les dépendances transitives natives
  de Tauri, dont muda, restent normales.
- Retrait des anciens exécutables d’essai de tableau et du pilote MCP egui.
  Leurs bilans et résultats restent dans `experiments/` et `docs/` ; Git conserve
  leurs sources. Les références d’implémentation egui sont signalées comme historiques.
- Suppression des appels de capture des tests actifs ; captures, vidéos et
  traces Playwright désactivées, même en échec. Les assertions de comportement,
  de données et de géométrie restent utiles. Aucune capture d’écran produite
  pour ce lot ; aucun rendu de l’ancienne interface à reproduire.
- Conservation du cœur, de Fluent, de Spellbook, des polices, des icônes et de
  l’accueil animé, des licences, des fixtures, rapports et résultats historiques.
- La génération du catalogue statique passe à la CLI :
  `cargo run --locked --offline -p typstographe-cli -- commands export --markdown`.
  Sans `--markdown`, elle fournit le JSON du registre ; un test vérifie la
  correspondance avec la documentation.

## Comportements repris dans Tauri

**Récupération après incident.** Chaque processus possède un emplacement unique
et un verrou OS, partagé par ses fenêtres. Les processus indépendants ne peuvent
ni écraser, ni restaurer, ni supprimer leurs copies respectives tant qu’ils sont
vivants. La liste présente les copies des processus terminés avec nom et date,
restauration et suppression explicites. Une copie illisible ne bloque pas le
démarrage et reste supprimable. La restauration valide les données et copie
l’état dans l’emplacement courant avant de consommer l’original. Les écritures
sont atomiques, limitées en taille et privées sur Unix.

Sauvegarde, abandon, fermeture et remplacement du document nettoient sa copie
sans affecter un autre processus. Une copie tardive de la même révision ne peut
pas réapparaître après ce nettoyage ; une nouvelle édition peut être récupérée.
La dernière copie de l’ancien emplacement global reste proposée, sans rétablir
une restauration automatique au lancement. Les diagnostics gardent leur dossier
isolé explicite. Ces copies ne constituent pas des snapshots durables du projet.

**Relecture MCP.** Les versions avant/après sont présentées avec leur structure,
styles, tableaux, images et annotations. Les changements de lien, note ou texte
alternatif sont explicités. La mise en forme seule ne produit plus deux textes
aplatis identiques. Les gardes, l’acceptation ou le refus du lot entier et son
annulation partagée sont conservés.

**Commandes et préférences.** Reprise de l’occurrence suivante/précédente
(Ctrl/Cmd+G et Ctrl/Cmd+Maj+G), de l’effacement de mise en forme, du retrait de
lien, de l’activation du correcteur, de la navigation entre erreurs et du
réexamen des mots ignorés. Le dictionnaire personnel devient consultable et
modifiable à la validation des préférences. Les commandes reprises passent par
le registre web ; les menus natifs vérifient également leur disponibilité avant
exécution. Le ratio des colonnes est enregistré après redimensionnement, sans
écriture périodique. Aperçu et suppression de la signature globale sont repris,
sans modifier les signatures déjà incorporées aux projets.

**Modèles et glyphes.** Import d’un `.typ` depuis Général, compilation de
validation avant publication et modèle embarqué dans l’archive et l’export,
même s’il n’est pas installé au catalogue. Ce n’est pas encore une bibliothèque
de modèles. Les fichiers Noto CJK existants servent aussi de secours CSS hors
ligne, chargés à la demande ; les polices de l’éditeur restent indépendantes du
rendu Typst. Un chemin documentaire positionnel est raccordé à l’ouverture au
lancement de l’application.

## Vérifications

- **407 tests Rust** du workspace, doctests compris ; **15 tests de l’hôte**
  également rejoués après les derniers ajustements de récupération. Un diagnostic
  de performance reste ignoré et s’exécute explicitement.
- **67 tests frontend et 3 tests de langues natives**, **9 tests des scripts**,
  frontière des dépendances du cœur, formatages et Clippy du workspace avec
  avertissements refusés.
- **72 parcours Chromium/WebKit** vérifiés. La première passe a détecté une
  condition résiduelle après retrait d’un appel de capture dans le test
  d’insertion ; correction puis reprise des deux parcours d’insertion et des
  six parcours d’accueil. Import de modèle invalide sans publication et
  persistance du ratio/correcteur sont également couverts.
- Recette native **macOS** complète puis réouverture : édition, Typst réel,
  ressources, archive autonome sous verrou, export PDF/Typst, échange MCP réel,
  relecture, acceptation du lot, sélection conservée et annulation partagée.
- Construction du frontend et du binaire natif après les dernières modifications.
  L’animation d’accueil et les ressources d’icônes restent celles validées en A.1.

Les rapports locaux sont dans `output/egui-retirement/` et
`output/egui-retirement-native/`, sans captures. Les tests de récupération
multi-processus exercent les verrous et la restauration du gestionnaire ; ils ne
remplacent pas une recette manuelle d’arrêt brutal sur chaque système.

La CI Rust ne construit plus egui et conserve la vérification indépendante du
cœur, de la CLI et du MCP. La CI Tauri reste présente sur les trois systèmes.
La couverture Linux inclut désormais **tout le workspace Rust, hôte Tauri
compris**, avec construction du frontend et bibliothèques WebKit. Le nouveau
pourcentage dépend de cette exécution distante ; aucune mesure de couverture
ni recette native Windows/Linux n’est annoncée comme exécutée sur ce Mac.

## Travaux maintenus dans la feuille de route

- **A.2** : contrat structuré commun, erreurs typées, disponibilité et retrait
  des duplications ; migration de la CLI et du MCP autonome. Leurs modules
  historiques du cœur sont conservés puisqu’ils servent encore ces adaptateurs.
  Leur compatibilité avec les archives structurées n’est pas annoncée.
- **B/D** : bibliothèque de modèles et régimes des options de présentation
  (libres, valeurs par défaut ou imposées), au-delà de l’import embarqué.
- **E/H** : nouveaux choix de présentation, sélection précise de polices système,
  recette matérielle IME, accessibilité et essais natifs sur les trois systèmes.
- **F** : ACP et évolution des droits/fonctions de l’assistance. Le MCP structuré
  conserve son périmètre de lecture et propositions soumises à validation.

Ce lot ne prétend pas une parité exhaustive test par test avec l’ancienne vue ;
il conserve les garanties réutilisables et reprend les parcours utiles sans
porter ses choix de géométrie ni maintenir artificiellement son tampon plat.

## Suivi de la CI

La CI du lot a confirmé Rust, la couverture et Tauri/macOS. Elle a révélé deux
écarts corrigés ensuite : la règle LF de `.gitattributes` visait encore l’ancien
répertoire du prototype, et la police CJK pouvait devenir la police latine de
secours sur Linux, puis modifier les lignes après chargement. La règle vise
maintenant `app/` ; les déclarations CJK sont limitées à leurs plages Unicode.
Un nouveau parcours vérifie, même sans la police système choisie, l’absence de
chargement CJK pour le français puis son chargement pour le japonais/chinois.
Les assertions de stabilité des lignes restent inchangées. Les quatre parcours
ciblés passent dans Chromium/WebKit ; un checkout avec `core.autocrlf=true`
conserve les octets LF et ceux de l’icône binaire contrôlée.

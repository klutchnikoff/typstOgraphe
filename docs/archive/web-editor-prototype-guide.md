> Archive du guide du prototype, avant le chantier A.1. Les commandes courantes
> sont dans le [guide de l’application](../../app/README.md).

# Diagnostic Tauri / Tiptap / Yjs

Prototype isolé, sur `codex/spike-tauri-editor`. Les
[choix validés](../../docs/architecture-web-editor-spike.md) précèdent son code.
Il réutilise `typstographe-core` sans modifier l'application egui, le catalogue
documentaire ni le format `.typsto`.

## Lancer

Depuis ce répertoire, avec Rust, Node.js et les
[prérequis Tauri](https://v2.tauri.app/start/prerequisites/) :

```sh
npm ci
npm run tauri dev
```

« Deuxième fenêtre » ouvre un deuxième participant. Chaque fenêtre possède son
propre `Y.Doc` ; les mises à jour et présences passent par un relais Tauri local.
« Hors connexion » permet de travailler sans recevoir les éditions du voisin,
puis « Connecté » fusionne les états. L'annulation est celle de Tiptap/Yjs,
propre à chaque participant ; le moteur Rust ne maintient pas un second historique
de frappe. « Essai Rust » insère dans ce même circuit un paragraphe préparé par Rust.

Le diagnostic s'ouvre sur une fixture ou la session expérimentale enregistrée.
Le dossier par défaut est `output/web-editor/default/` à la racine du dépôt.
Pour un essai indépendant :

```sh
npm run tauri -- dev -- -- --session-dir /chemin/absolu/vers/un/dossier-essai
```

## Mesurer et tester la réactivité native

L’aperçu utilise le vrai rendu Typst et le transfert RGBA binaire vers un canvas,
sans délai de frappe. Pour tester le frontend embarqué avec Rust optimisé :

```sh
npm run build
cargo build --release --locked --offline --manifest-path src-tauri/Cargo.toml --features custom-protocol
../../target/release/typstographe-web-editor-spike
```

`npm run test:preview-native` lance le banc de comparaison dans une session
isolée. La fenêtre doit rester visible et l’écran déverrouillé. Sur macOS,
`caffeinate -di npm run test:preview-native` empêche la mise en veille automatique
pendant cette recette. Les mesures du relais HTTP des tests de navigateur ne
représentent pas celles du transport Tauri natif. Voir la
[méthode et les résultats](../../docs/tauri-preview-performance.md).

## Édition et rendu

### Langues et services du système

Les catalogues de `crates/i18n/i18n/` définissent les langues disponibles ;
`crates/i18n/i18n.toml` définit leur repli. Il n’existe pas de seconde liste
de langues actives dans la configuration Tauri. L’interface choisit une seule
langue selon les préférences du système via le chargeur Fluent partagé et
la déclare sur `<html>`.
Le corps de l’éditeur porte séparément la langue du document, actualisée lors
d’un changement de ses paramètres, y compris reçu d’une autre fenêtre.

- **macOS** : les langues et le repli natifs sont générés depuis ces mêmes
  catalogues et réglages Fluent. Tauri les fusionne dans le bundle `.app` ;
  les menus et dialogues système peuvent ainsi négocier une langue prise en charge.
  Le même fichier généré `src-tauri/Info.plist` est intégré dans l’exécutable
  de développement et fusionné dans le bundle ; les deux parcours sont requis
  pour vérifier cette déclaration.
- **Windows** : Wry configure déjà la langue de WebView2 avec
  `GetUserDefaultUILanguage`. Les langues des installateurs NSIS et WiX sont
  dérivées des mêmes catalogues, avec le repli Fluent en premier. La langue de
  l’installateur ne fixe pas celle des documents.
- **Linux** : Fluent suit les préférences de locale ; GTK et WebKitGTK utilisent
  les traductions et locales installées sur le système. Il n’existe pas de
  déclaration de langues de bundle équivalente à celle de macOS à ajouter ici.

`npm run tauri -- dev|build|bundle …` passe par `scripts/tauri.mjs` : celui-ci
lit les catalogues puis génère `src-tauri/Info.plist` et une surcharge de
configuration Windows dans `src-tauri/gen/native-locales/`, avant que la CLI ne lise les paramètres
de bundle. Ces fichiers sont ignorés par Git et ne se modifient pas à la main.
Utiliser cette commande plutôt que d’appeler directement la CLI Tauri.
Les noms NSIS et les codes WiX sont des identifiants propres aux installateurs :
leur correspondance est isolée dans `native-locales.mjs`, sans décider des langues
disponibles. Un identifiant encore inconnu provoque une erreur explicite.

Ne pas déplacer le fichier généré `src-tauri/Info.plist` en se contentant de
`bundle.macOS.infoPlist` : avec notre version de Tauri, cette option ne suffit
pas au codegen de développement. Celui-ci lit uniquement l’emplacement
conventionnel. Après un changement, vérifier aussi les clés de langue de la
section Mach-O `__TEXT,__info_plist` de l’exécutable debug, et pas seulement
le `Contents/Info.plist` du bundle release.

La déclaration de plusieurs langues n’affiche pas une interface multilingue :
elle indique au système lesquelles l’application sait présenter. Les composants
natifs gardent leur propre négociation. Notamment, si la langue système n’existe
pas dans Fluent, l’interface utilise son repli tandis que WebView2 ou GTK peuvent
conserver la langue système. Un futur sélecteur de langue dans les préférences
devra aussi traiter cette limite native ; modifier uniquement `<html lang>`
ne traduit pas les menus système.

Spellbook est l’unique producteur des soulignements orthographiques du document.
Le corps porte `spellcheck="false"` et `autocorrect="off"`, hérités dans les
listes, cellules, citations et légendes. Les services natifs explicitement
demandés dans le menu contextuel restent un complément propre au système ;
leur présence ne garantit ni leur disponibilité hors ligne ni leur compatibilité
avec toutes les structures et l’historique Yjs.

La recette Chromium/WebKit vérifie les attributs réellement hérités, la séparation
des langues et un seul marquage Spellbook par occurrence dans le corps et les
widgets. Le menu natif et les éventuels soulignements dessinés par le système
restent à contrôler sur chaque OS : clic secondaire sur un mot erroné, correction,
annulation, puis contrôle d’une liste, d’une cellule et d’une légende. Vérifier
aussi les dialogues ouvrir/enregistrer en français et en anglais. Les tests de
navigateur ne remplacent pas ce contrôle natif.

### Capacités

- Paragraphes, titres 1–3, gras, italique, liens, listes non imbriquées.
- Dans le corps libre : Entrée reste dans le paragraphe (espace dans le rendu),
  deux Entrées créent un paragraphe. Maj+Entrée force un saut de ligne Typst.
  Listes, cellules, titres et légendes gardent leur navigation structurelle.
- Image avec légende directement éditable ; paramètres pour le texte alternatif.
- Tableau sans fusion ; commandes de lignes et colonnes lorsque le curseur y est.
- Note en ligne avec bandeau de texte, Entrée pour appliquer, Échap pour fermer.
- Cmd/Ctrl+Entrée ouvre les paramètres d'une note ou d'une image ciblée.
- Cmd/Ctrl+S enregistre. Le séparateur des colonnes se déplace à la souris ou
  avec les flèches après Tab ; double-clic pour revenir à moitié.

Rust valide l'instantané structuré, génère un sous-ensemble Typst et le compose
avec le monde, les polices, paquets et modèle personnel du cœur. Les réponses
de rendu périmées sont refusées. Un contenu invalide reste éditable et n'écrase
pas le dernier aperçu valide.

Les exports comprennent PDF et projet Typst autonome. Les écritures de sources
de l'adaptateur sont expérimentales : le projecteur de production ne connaît pas
encore les tableaux et notes. Cet essai conserve toutes les ressources de la session,
y compris celles d'images retirées, pour les retours en arrière et clients différés.

## Sauvegarde expérimentale

`session.json` est écrit atomiquement après validation et compilation. Il contient
le schéma versionné, l'instantané structuré, le Typst validé, l'état binaire Yjs et
les images. Une demande périmée est refusée ; les exports utilisent un nouveau
dossier et n'écrasent pas les précédents.

À la reprise, Rust vérifie source, instantané et ressources ; le client vérifie
que l'état Yjs reconstruit le même document. Les identités CRDT sont conservées,
mais les piles d'annulation propres aux vues ne sont pas persistées. Ce fichier
n'est pas un `.typsto` de production. Aucun import de source Typst arbitraire
ni migration de document existant n'est livré par ce diagnostic.

## Vérifier

```sh
npm test
npm run build
npm run test:rust
npm run check:rust
npm run check:format
```

Les tests de collaboration utilisent deux éditeurs et deux Y.Doc distincts :
initialisation unique, fusion hors ligne, annulation locale, cellules/légendes,
reprise binaire et presse-papier HTML. Les tests Rust vérifient refus, sauvegarde
et véritable compilation/export du corpus avec tableaux, notes et image.
La composition IME du système reste un essai manuel à distinguer des événements
simulés et des tests du navigateur.

Pour les parcours graphiques Chromium et WebKit, avec le **vrai** cœur Rust via
un transport de recette STDIO/SSE (aucun rendu factice) :

```sh
cargo build --locked --manifest-path src-tauri/Cargo.toml
npx playwright install chromium webkit
npm run test:browser
```

Le serveur de test est strictement local et n'est activé qu'en mode `browser-test`.
Les captures et mesures de saisie se trouvent dans `test-results/`. La CI dédiée
compile et teste l'adaptateur sur les trois systèmes ; les parcours de navigateur
s'exécutent sous Linux. Cela ne vaut pas un essai graphique natif sur ces systèmes.

Pour automatiser deux **fenêtres Tauri natives**, fermer l'application puis
rouvrir le fichier enregistré, avec un affichage disponible :

```sh
npm run build
cargo build --locked --features custom-protocol --manifest-path src-tauri/Cargo.toml
npm run test:native
```

La recette crée sa propre session dans `output/web-editor/`. Elle garde deux
rapports, les journaux et le pic de RSS échantillonné du seul processus Rust
(hors processus WebKit, donc pas la consommation totale de l'application).
Le [bilan](../../docs/web-editor-spike-results.md) distingue les résultats
automatisés, les limites et les vérifications manuelles restantes.

## Dépendances et icône

Les versions JavaScript sont exactes et verrouillées dans `package-lock.json` ;
le projet Rust possède son propre `Cargo.lock`. Aucun paquet Universe supplémentaire
n'est introduit. L'icône de diagnostic est générée par `tauri icon` depuis
`crates/app/assets/icons/lucide/pen-line.svg`, sous les licences Lucide présentes
dans ce même répertoire. Les fontes Inter utilisent les fichiers et la licence
du projet ; les polices de composition proviennent du cœur existant.

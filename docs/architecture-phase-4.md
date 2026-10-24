# Phase 4 — CLI documentaire

Décisions validées le **24 octobre 2026** avant implémentation : D23 et D24 dans
[ROADMAP-ARCHITECTURE.md](../ROADMAP-ARCHITECTURE.md). La phase reste en cours.

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

## Suite

1. Création à partir du catalogue des types/modèles et ouverture croisée GUI/CLI.
2. `query` et `run` : JSON en argument, fichier ou stdin ; résolution des cibles
   dans la session ; sauvegarde explicite (`--save` ou `--output`).
3. Scripts versionnés : étapes `{id, params}`, chemins relatifs au script, limites
   et atomicité du moteur réutilisées. Export distinct après enregistrement.
4. Page PNG, retrait de `--render-demo` et adaptation des références visuelles,
   puis scénario complet d’acceptation.

Outils réutilisés : [clap](https://docs.rs/clap/4.6.7/clap/),
[assert_cmd](https://docs.rs/assert_cmd/2.2.2/assert_cmd/),
[verrous Rust](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock).

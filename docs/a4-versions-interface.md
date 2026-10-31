# A.4.3 — Commandes et consultation Versions

## Périmètre confirmé

Ce lot livre la gestion et la consultation des versions. La comparaison commune,
puis la restauration dans une session active, restent deux lots distincts,
conformément au découpage confirmé avec l’utilisateur. La modale reprend la
[maquette validée](../ROADMAP.md) : la place de Comparer et de Restaurer sera
raccordée quand ces actions seront fonctionnelles.

Les références internes des relectures ne sont pas affichées dans Versions.
Les propositions MCP restent privées et ne sont pas transformées en versions
confirmées par ce lot.

## Interface

L’entrée **Versions** est disponible dans la barre latérale, le menu et la palette
pour un document actif. Les groupes **Versions manuelles** et **Avant restauration**
trient leurs entrées de la plus récente à la plus ancienne, avec nom, date et tags.
La date est affichée dans la locale de l’interface ; la date UTC conservée dans
le fichier ne change pas lors d’un renommage.

**Créer une version** propose un nom libre, des tags séparés par des virgules et
une description facultative. Un nom vide à la création demande le nom daté
fourni par le cœur. Le nom de profil disponible sert de créateur ; modifier les
informations depuis l’interface conserve cette attribution. Le bouton de création
attend le chargement de l’index et est désactivé au quota de 100 versions. Aucune
suppression automatique n’est introduite.

Le premier enregistrement demande l’emplacement du `.typsto`. Annuler ce choix
conserve le formulaire et ne crée pas de fichier ni de version. La création
écrit l’état confirmé courant et son snapshot dans une seule publication atomique.
Les formulaires et champs non validés restent dans la récupération privée A.3.

Un clic sur une version ouvre une modale centrée occupant 90 % de la fenêtre,
sur fond atténué et légèrement flouté :

- **Contenu** : projection DOM statique du schéma riche commun, sans éditeur,
  liaison Yjs ou commandes de widget. Les images viennent de la version ; les
  notes et citations gardent des badges distincts.
- **Aperçu** : rendu Typst des sources et du modèle de cet état, avec les
  bibliothèques du [catalogue de l’application](typst-dependencies.md).
  Son moteur isolé ne remplace pas le rendu ni la synchronisation du
  document en cours.
- Nom/tags/description et suppression : bandeau de décision dans cette même
  modale, sans empiler un second dialogue.

Les flèches naviguent dans la liste, les onglets et les actions via les composants
clavier communs. Tab reste dans la modale. Échap ferme d’abord le bandeau, puis
la consultation et rend le focus à sa ligne. La suppression propose **Supprimer**
à gauche et **Annuler**, action sûre par défaut, à droite ; Entrée ne supprime
pas accidentellement la version dès l’ouverture de la confirmation.

Les réponses asynchrones d’une ancienne consultation ne remplacent pas la vue
suivante. Une sauvegarde terminée après le changement de document ne rattache pas
son chemin à ce nouveau document. Les URL temporaires des images sont libérées
à la fermeture ou au changement de vue.

## Contrat partagé et publication

`content::project::VersionChange` définit les opérations typées `create`, `update`
et `delete`, utilisées par les adaptateurs. Les données documentaires ne passent
pas dans ces paramètres : seul le propriétaire Rust capture l’état confirmé.

- `versions.list` renvoie `project_id`, `history`, `limit` et `versions`.
- `versions.read` prend l’UUID d’une version et renvoie son état structuré,
  ses images d’origine et son index bibliographique. Aucun changement de session.
- `versions.change` prend `history` et une `action` typée. Il prépare le candidat,
  l’enregistre sous le verrou courant, puis publie le nouvel index en mémoire.

`history` est une empreinte de l’index. Elle complète la garde du document : deux
opérations sur des labels ne peuvent pas s’écraser silencieusement sous prétexte
que la révision du texte n’a pas changé. Identité, dates, origines et descripteurs
immuables restent gérés par le cœur. Supprimer une version ne permet pas de
retirer une référence interne de relecture. Le nettoyage CAS ne retire que les
objets devenus inaccessibles.

Les opérations ne créent pas d’entrée d’annulation et ne déplacent pas la sélection
de l’éditeur. Les autres fenêtres sont informées du nouvel index et du chemin
commun de sauvegarde. Si l’écriture échoue, le fichier et l’index en mémoire
restent inchangés. Si l’écriture réussit mais que le nettoyage de la récupération
échoue, la version enregistrée reste publiée et l’interface reçoit un avertissement :
la sauvegarde durable ne sera pas perdue lors de l’opération suivante.

Tauri demande le chemin du premier enregistrement ; la CLI et MCP autonome
exigent un projet enregistré. La CLI expose les requêtes par `query` et une
commande de fichier explicite `versions`. Exemple de capture :

```sh
# Lire d’abord le champ history.
cargo run --locked --offline -p typstographe-cli -- query versions.list courrier.typsto --json
cargo run --locked --offline -p typstographe-cli -- versions courrier.typsto \
  --params '{"history":"sha256:…","action":{"kind":"create","details":{"name":"v1","tags":["brouillon"]}}}' --json
```

MCP autonome vérifie ses racines d’écriture avant `versions.change`, même si le
fichier est déjà ouvert. Le MCP rattaché à l’application n’obtient aucun droit
sur les fichiers ou l’historique ; son futur parcours reste celui des propositions.
Le [catalogue généré](commands.json) et le [guide CLI](cli.md) sont actualisés.

## Validation

Vérifications locales macOS :

- Workspace Rust avec `--locked --offline --features typstographe/recipe-driver` :
  347 tests réussis et un diagnostic de performance explicitement ignoré, dont
  103 tests natifs et 189 tests du cœur. Les nouveaux tests de processus CLI/MCP
  ont ensuite été exécutés séparément : création/lecture, garde d’index,
  refus sous verrou concurrent et permissions en lecture seule.
- Test ciblé du contrat Versions du cœur relancé après l’ajout de l’index
  bibliographique aux réponses de consultation : deux tests réussis.
- Suite web : 344 tests Vitest et cinq tests de déclarations natives réussis.
  Les recettes couvrent les refus conservant le formulaire, l’annulation du
  sélecteur, le quota, le focus, l’isolation des images et un changement de
  document pendant une réponse de sauvegarde.
- Quatre parcours Playwright réussis : deux dans Chromium et deux dans WebKit,
  avec vrai moteur Rust et rendu Typst. Création depuis un document non
  enregistré, conservation du texte courant, modale à 90 %, navigation des
  onglets, renommage, annulation et confirmation de suppression, retour du focus.
- Vérification visuelle dans une session isolée : Contenu avec notes, tableau et
  image, puis Aperçu multipage, fond flouté et proportions de la modale.
- Clippy du workspace, tous les targets et `recipe-driver`, avec `-D warnings` :
  réussi. Build TypeScript/Vite, formatage Rust/Prettier, contrôle des modules
  Rust de moins de 800 lignes et `git diff --check` : réussis.

La recette graphique a révélé un bouton de création actif avant le chargement
de l’index. Il est désormais désactivé jusqu’à la réponse ; les parcours ont
été relancés avec succès après cette correction. Les binaires ordinaires et de
recette pouvant partager leur nom de lancement, les parcours graphiques utilisent
l’artefact `recipe-driver` explicitement identifié.

Ces résultats sont locaux ; ils ne valent pas validation CI Windows/Linux.
Aucun fichier de capture, vidéo ou trace graphique n’a été enregistré.

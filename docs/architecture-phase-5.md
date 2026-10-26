# Phase 5 — MCP documentaire

Décisions du 24 octobre 2026 : D11/D12, D25–D28 et complément D14 dans
[la feuille de route](https://github.com/klutchnikoff/typstOgraphe/blob/7b0d6fd3e6ae25e5704ed43ce2f4efefa9c2fa0b/ROADMAP-ARCHITECTURE.md). **Phase terminée le 24 octobre 2026.**
La [recette finale](mcp-acceptance.md) distingue essai avec un agent, inspecteur
et scénario automatisé rejoué dans la CI.

## Contrat validé

- Outils explicites et schémas dérivés du registre, liste stable. Capacités d’un
  type, disponibilité dans le contexte et autorisations du serveur sont distinctes.
- Découverte des types, champs, capacités, paramètres, défauts, effets et exemples
  via les outils ; ressources complémentaires, aucune dépendance obligatoire à un skill.
- Une fiche de modèle ne contient que son identifiant, nom, type, description et
  mots-clés. Les modèles d’un type sont interchangeables ; leur effet est visuel.
- Une session autonome, un document en mémoire par processus, identifiant explicite.
  Pas de profil personnel implicite ni de connexion à l’application en phase 5.
- Références gardées, texte littéral, commandes dédiées et lots atomiques. Les
  changements s’annulent ; une référence périmée n’est pas réancrée silencieusement.
- Sauvegarde explicite, éventuellement finale dans un lot ; export distinct.
  L’ouverture éditable réserve le fichier ; lecture/copie possible avec un écrivain actif.
- Résultats structurés et résumé ; validité, champs d’export et orthographe facultative
  sont séparés. Un contrôle ne corrige rien. Un aperçu PNG peut être renvoyé sans disque.
- SDK officiel `rmcp`, stdio, anglais par défaut ; accès aux fichiers refusé sans
  racines explicitement déclarées, pas de réseau ni de lancement de processus par outil.

## Premier lot — prototype de faisabilité

Le premier lot valide le SDK avant de développer la politique complète des fichiers :
initialisation avec un client réel, génération dynamique des outils depuis le registre,
découverte des types/capacités et premier courrier en mémoire avec contrôle et aperçu.
L’exécution des appels reste sérialisée autour d’un moteur documentaire propriétaire.
Le protocole désigne le document, sans supposer qu’il est toujours issu d’un chemin.

Le prototype refuse les imports par chemin, ouvertures, sauvegardes, exports et
récupération. Il ne propose pas de faux paramètres de racines non appliqués. Les
ressources embarquées restent utilisables. Les opérations indisponibles sont
signalées comme telles dans la découverte ; leur présence dans un type n’est pas
une autorisation de les exécuter dans le prototype.

Mesurer avant/après la taille du binaire CLI, le temps de compilation incrémental
et le coût propre des nouvelles dépendances, sans nettoyer les rapports existants.
Vérifier que les dépendances asynchrones du transport restent en aval du cœur.

### Résultat du prototype — 24 octobre 2026

**Faisabilité confirmée avec [rmcp 3.7.0](https://docs.rs/rmcp/3.7.0/rmcp/).**
Le repli JSON-RPC manuel n’est pas nécessaire. La production active seulement
`server` et `transport-io`, sans macros ni transport HTTP ; `client` et
`transport-child-process` servent aux tests de processus. `tokio`, le SDK et les
types MCP restent dans la crate CLI ; les dépendances normales du cœur ne
contiennent ni Tokio, rmcp, Fluent ou dépendance graphique.
Le garde-fou de dépendances interdit désormais aussi Tokio/rmcp et leurs sous-crates,
avec les tests correspondants, pour maintenir cette frontière dans la CI.

Le serveur expose **62 outils** : 59 opérations du registre et 3 outils de
découverte/rendu. La liste est construite à partir des schémas du registre, triée,
stable et indépendante du contexte. Le contrôle vérifie les références `$ref`
après adaptation des schémas. Les images par chemin sont retirées des schémas
du prototype et refusées à l’exécution. Les 68 contrats JSON du cœur sont conservés.

`document_types.describe` et `capabilities.describe` appellent des API de
documentation du cœur, construites depuis les champs et le registre existants.
Les premiers exemples couvrent la rédaction ; compléter toutes les fiches reste
à faire. Les fiches de modèle réutilisent `name`, `description`, `keywords` ajoutés
au catalogue commun ; l’adaptateur MCP omet les contraintes visuelles techniques.
La [documentation MCP](mcp.md) précise les schémas effectifs, la séquence d’usage
et les opérations non exposées, sans skill obligatoire pour les découvrir.

Un worker dédié possède le moteur, sérialise les demandes et garde la session
hors du transport. Les appels portent un identifiant UUID propre au document MCP
et la garde du moteur ; une référence étrangère ou périmée est refusée. Le même
point d’aiguillage pourra être relié au moteur de l’application en phase 6.
Les sélecteurs sont résolus par les helpers CLI et les lots utilisent le brouillon
atomique existant ; une sélection de confort et son édition échouent ensemble.
Les limites du moteur et une borne de 4 Mio sur les arguments JSON décodés sont
réutilisées. Il n’y a pas de promesse d’annulation d’une commande déjà en cours.

Contrôles locaux : **415 tests Rust du workspace réussis** (226 cœur, 139 UI,
7 i18n, 43 CLI : 7 unitaires et 36 de processus), **14 tests Python**, formatage,
Clippy sur toutes les cibles, gardes de dépendances et limite de 800 lignes.
Les 6 tests du diagnostic de tableau passent séparément. Les nouveaux scénarios
couvrent références étrangères/périmées, schémas et documentation, mutation
concurrente sur la même révision, historique, échecs tardifs, limites et refus
des fichiers, lecture seule, localisation, ressources, PNG et stdout propre.
Les tests sont inclus dans la CI des trois systèmes ; exécution locale sur macOS.

Les **huit captures natives restent strictement identiques** aux références :
`output/baseline/run-k9acf2yt/` et `output/phase-5-captures.log`.
Une trace JSON-RPC manuelle en 2025-11-25 crée un courrier avec image embarquée,
légende, gras, P.-S./P.J., le valide et reçoit son PNG. Le serveur termine avec
le code 0 après fermeture de stdin et n’a créé **aucun fichier** dans son répertoire
de travail. Le client de test conserve la trace, le source et le PNG dans
`output/phase-5-manual/` ; l’image a été inspectée. Il s’agit d’un essai protocolaire,
pas encore du test final avec un agent/LLM sans skill.

### Coût mesuré

Mesures sur cette machine, profil `dev` avec `debug=0` et workspace déjà compilé,
sans nettoyage ; ce ne sont pas des tailles de distribution optimisées.

| Mesure | Résultat |
| --- | --- |
| Binaire CLI avant | 249 014 816 octets (237,48 Mio) |
| Binaire CLI avec serveur | 259 268 032 octets (247,26 Mio) |
| Variation | +10 253 216 octets, soit +9,78 Mio / +4,12 % |
| Première construction du prototype avec ses nouvelles dépendances/features | 30,56 s |
| Reconstruction après variation des features workspace/tests | 10,63 s |
| Construction sans modification, cache chaud, avant/après | 0,61 s / 0,34 s |

Les temps décrivent leurs conditions de cache ; ils ne constituent pas une
comparaison de constructions propres. Mesures, journaux et chronogramme Cargo
sont conservés dans `output/phase-5-measurements.json`,
`output/phase-5-first-build.log` et `output/phase-5-build-timings.html`.

## Deuxième lot — fichiers autorisés et cycle de vie

Livré le 24 octobre 2026, après validation du prototype. L’adaptateur expose désormais
**67 outils** (64 opérations du cœur et les 3 outils complémentaires), avec une liste
stable même sans racines. Les 68 contrats du moteur restent inchangés.

### Autorisations et décisions d’implémentation

- `--read-root` et `--write-root` acceptent plusieurs répertoires **existants** et
  restent indépendants. Ils sont canonicalisés au démarrage ; un répertoire invalide
  arrête le serveur avant MCP. Sans racine, aucun accès documentaire aux fichiers.
- Les chemins relatifs utilisent le répertoire de lancement. Les composants `..`
  sont refusés. Les liens et ancêtres existants sont résolus avant le contrôle
  d’appartenance ; les nouvelles destinations d’export peuvent avoir des parents
  à créer. Le compagnon `.<nom>.lock` est contrôlé lui aussi. Les lectures et les
  cibles d’archive/verrou existantes doivent être des fichiers ordinaires ; les
  chemins doivent être représentables en JSON UTF-8.
- Chaque réponse réussie expose `permissions` ; `commands.list` et
  `capabilities.describe` distinguent présence d’un outil et autorisation du serveur.
  L’autorisation d’un chemin est toujours vérifiée à l’exécution. Une permission
  générale d’image n’autorise pas tout chemin : la variante embarquée reste possible
  sans racines, la variante fichier exige une racine de lecture.
- Une ouverture éditable exige **lecture et écriture**, car elle réserve le même
  verrou coopératif que l’application et la CLI. Elle peut créer le compagnon avant
  toute sauvegarde ; ce fichier persistant n’est jamais supprimé au déverrouillage.
  Un conflit donne `file_locked` sans attendre ni basculer silencieusement.
- `document.open` avec `read_only: true` lit la dernière archive enregistrée, même
  avec un écrivain actif. `save_as` vers un autre chemin autorisé crée une copie
  éditable. Le serveur globalement `--read-only` refuse cette copie, les mutations,
  sauvegardes et exports ; il autorise requêtes, ouverture explicitement en lecture
  seule et fermeture. Ce sont les seules exceptions de cycle de vie au mode lecture seule.
- Images, signatures et modèles par chemin suivent la même politique, y compris
  dans les lots et à la création. Leurs ressources sont copiées en mémoire et restent
  autonomes. Un modèle n’ouvre pas implicitement des fichiers voisins : son monde
  Typst ne lit que les ressources et dépendances embarquées disponibles.

### Sauvegarde, export et lots

`document.save` et `save_as` réutilisent la sauvegarde atomique, les verrous et les
empreintes du moteur, sans duplication d’une voie de stockage. Une modification
externe donne `file_changed` ; remplacer une destination différente existante exige
`overwrite: true`, sans contourner son verrou. Une nouvelle ouverture/remplacement
exige aussi la référence documentaire courante ; l’abandon d’un brouillon reste explicite.

Le champ final `save` d’un lot est désormais disponible (`current` ou `as`). Sa
permission est vérifiée avant les étapes ; sa publication et son contrôle d’empreinte
restent ceux de la transaction commune. Une erreur tardive ou finale conserve la
session publiée et l’historique. Sauvegarde, export et cycle de vie restent interdits
**dans** les étapes. L’annulation du lot restaure le brouillon, pas le fichier enregistré.

L’export est un appel distinct vers un répertoire nouveau ou vide : PDF, Typst autonome,
ou les deux, avec PNG facultatifs. Il utilise le contenu courant et ne marque pas
l’archive comme sauvegardée. Comme dans la CLI et l’application, une erreur d’IO
peut laisser un export partiel ; l’export multi-fichiers n’est pas atomique.

La vérification des chemins est applicative et conserve une limite connue : un
processus local hostile peut remplacer un lien/répertoire entre contrôle et ouverture.
Ce n’est pas une sandbox OS ni une protection contre les liens physiques. Les
répertoires autorisés doivent être de confiance. Cette limite, prévue par D12,
est documentée dans [le guide MCP](mcp.md), avec les budgets et l’annulation des appels.

### Vérifications

- **426 tests Rust du workspace** passent sous macOS : 226 cœur, 139 application,
  7 i18n et 54 CLI (15 unitaires, 39 de processus). Les tests ciblés ont été rejoués
  après l’enrichissement de l’aide et le dernier contrôle des chemins UTF-8.
- Les **14 tests Python**, formatage, Clippy toutes cibles, limite de 800 lignes,
  garde de dépendances et vérification de dérive de la documentation CLI passent.
- Les nouveaux clients SDK vérifient sauvegarde/réouverture, deux processus en conflit
  de verrou, lecture d’un instantané, copie, export et ses dépendances, lecture seule,
  refus de chemins et échecs tardifs. Les tests unitaires couvrent aussi les imports,
  liens sortants/pendants, compagnon détourné, racine remplacée, empreinte modifiée,
  sauvegarde finale et absence d’écriture implicite. Les tests de liens sont sous Unix ;
  le contrôle d’un alias vers un nom non UTF-8 réel est réservé à Linux (APFS le refuse).
- Les **huit captures natives sont strictement identiques** aux références :
  `output/baseline/run-fup1uaj_/`, journal `output/phase-5-files-captures.log`.
- Un parcours JSON-RPC manuel crée et met en forme un courrier, sauvegarde le lot,
  valide, exporte PDF/Typst/PNG, ferme/réouvre l’archive et reçoit le PNG. Celui-ci
  est identique au PNG exporté ; l’image a été inspectée. Arrêt propre, stderr vide,
  répertoire de travail du serveur inchangé. Trace, script d’essai, archive, PDF,
  sources et PNG sont conservés dans `output/phase-5-files-manual/`. Ce contrôle
  protocolaire ne remplace pas l’essai final avec un agent sans skill ni lecture du dépôt.

Les journaux de tests et de Clippy sont dans `output/phase-5-files-*.log`.

## Troisième lot — documentation et ressources

Livré le 24 octobre 2026. Le diagnostic de `getrandom` est consigné dans le
[bilan technique de phase 3](architecture-phase-3.md#diagnostic-wasm--24-octobre-2026) :
le contrôle WASM reste informatif, sans activation de `wasm_js` ni chantier Web.

### Capacités documentées

Les **68 opérations du cœur** possèdent une fiche explicite, indépendante du SDK :
contrat commun, détails, défauts extraits du schéma, restrictions, résultat attendu,
effets, transaction et exemple. Un test exige une fiche pour chaque entrée du
registre et décode tous les exemples. Les **3 helpers MCP** ont aussi leur fiche,
disponible par `capabilities.describe` : l’agent peut documenter n’importe lequel
des **67 outils exposés**, ainsi que comprendre les 4 opérations du cœur non exposées.
Les identifiants et schémas d’entrée JSON du cœur restent inchangés.

Les exemples sont des objets `{params, bindings}`. Les paramètres littéraux sont
séparés des références contextuelles : une liaison indique le pointeur JSON à
remplacer, le genre de référence et la lecture dont elle provient. Les cibles
d’image, occurrences de recherche, fautes orthographiques, rendus et références
de récupération ne sont pas des identités fictives à copier. La documentation
MCP explique l’enveloppe `document`, les résultats sous `outcome`, les prérequis
et les permissions de fichiers applicables.

La vérification utilise **jsonschema 0.60.1** uniquement en dépendance de test de la
CLI, avec `default-features=false` et `.offline()` : pas de fonctionnalité de
récupération HTTP/fichier, aucun ajout à la production ou au cœur. La version
requiert Rust 1.85, compatible avec le minimum du projet (1.92). Voir la
[documentation de validation hors ligne](https://docs.rs/jsonschema/0.60.1/jsonschema/#offline-validation).

Ce contrôle a permis de rendre explicites les adaptations déjà faites par le
résolveur MCP : `selector` peut fournir une cible/occurrence/faute requise, les
cibles et sélecteurs sont exclusifs, et le rendu peut être compilé si `render`
est absent. Les commandes sans cible ne proposent plus un sélecteur sans effet.
Les exemples liés à de **vraies réponses du moteur** valident les 67 schémas effectifs,
avec des contrôles négatifs. `commands.list`, les fiches et les ressources utilisent
les mêmes schémas et règles d’autorisation ; la disponibilité contextuelle reste distincte.

### Ressources du moteur courant

Cinq URI exactes et stables sont proposées : guide, types, commandes, document et
modèles. `typstographe://commands` présente 71 fiches (68 + 3), la liste des 67 outils,
les schémas effectifs et le contexte courant. `typstographe://document` fournit
état, texte, structure, métadonnées et source du corps ; aucun modèle source ou
binaire image. `typstographe://models` garde les fiches visuelles succinctes de
tous les types et le modèle fourni actif du document courant.

Toutes les lectures passent par le **même worker propriétaire que les outils** :
chaque réponse est un instantané cohérent avec référence documentaire et permissions.
Elles ne modifient ni contenu, sélection ou historique, ne compilent pas et ne
lisent pas de fichier. Sans document, l’état est fermé, les parties textuelles
sont null et les fiches globales restent disponibles. Après édition, Annuler/Rétablir
ou remplacement, une nouvelle lecture renvoie l’état courant ; les références
anciennes sont toujours gardées. Aucun abonnement ni notification de mise à jour
n’est annoncé. Les URI arbitraires, `file:` et variantes avec paramètres sont
refusées ; le SDK adapte l’erreur de ressource inconnue à la version du protocole.

### Contrôles

- **432 tests Rust du workspace** passent sous macOS : 226 cœur, 139 application,
  7 i18n, 60 CLI (19 unitaires, 41 de processus). Le contrôle des exemples du cœur
  est renforcé, avec 4 tests unitaires MCP et 2 tests de processus supplémentaires.
- Les clients SDK en 2025-11-25 et version courante vérifient le cycle des ressources,
  la cohérence des gardes, l’absence de mutation/écriture, l’historique, le mode lecture
  seule, le catalogue, les erreurs et la stabilité de la liste. Ce sont des tests de
  protocole ; l’acceptation avec un agent/LLM reste distincte.
- **14 tests Python**, formatage, Clippy toutes cibles, gardes de dépendances et limite
  de 800 lignes passent. La documentation CLI est générée puis contrôlée pour dérive.
- Les **8 captures natives sont strictement identiques** aux références :
  `output/baseline/run-_xq0bi2a/`, journal `output/phase-5-docs-captures.log`.

Les journaux du lot sont conservés dans `output/phase-5-docs-*.log`.

## Quatrième lot — recette et clôture

Livré le 24 octobre 2026. L'inspecteur officiel **2.12.0**, en interface web,
se connecte sur stdio en MCP 2025-11-25 : 67 outils, cinq ressources, découverte des
types, création d'un courrier et lecture de son instantané. Son contrôle CLI strict
passe sans erreur après correction du schéma des paramètres de `document.batch`.
Les avertissements restants portent sur les unions de types avec `null`, valides
mais inégalement acceptées par les fournisseurs de modèles.

Un agent dans un contexte neuf a rédigé un courrier à partir d'une seule consigne
métier, en découvrant ses outils et leur documentation **uniquement via MCP** :
aucun skill ni lecture du dépôt. Il utilise une session autonome, via le client
persistant officiel `mcpdo` et un mince relais d'invocation/journalisation.
**27 appels d'outils, aucune erreur applicative**, validation prête pour l'export,
aperçu inspecté, sauvegarde `.typsto`, export PDF/Typst/PNG et réouverture avec
texte, source et métadonnées identiques. Il a repéré puis retiré le gras hérité
dans le P.-S. et la mention de pièce jointe. Cette restriction d'accès de l'agent
est une consigne d'essai, pas une sandbox du système d'exploitation.

Deux défauts trouvés pendant la recette sont corrigés et couverts :

- Schéma MCP de `document.batch` : les `params` d'étape sont annoncés comme des
  objets, conformément au contrôle déjà appliqué à l'exécution. Le contrat gelé
  du cœur reste inchangé ; les formes non objets et l'absence de publication
  d'un lot invalide sont vérifiées.
- Langue MCP par défaut : choisir l'anglais avant que la construction de l'aide
  clap initialise le chargeur sur la langue du système. Les choix explicites
  `--lang fr/en` sont conservés ; le test de processus vérifie les résumés.

La fixture issue du courrier réel et `crates/cli/tests/mcp_acceptance.rs` rejouent
le parcours sous les deux versions de protocole testées, avec des références
obtenues par recherche, vérification des styles et annotations, PNG, export,
sauvegarde et réouverture. Ce scénario déterministe s'exécute dans la CI sur les
trois systèmes ; l'essai LLM et l'inspecteur graphique ont été réalisés sur macOS.
Les sources exportées ont aussi été recompilées avec le CLI Typst 0.15.1, sans
polices système, avec répertoires de packages/cache vides et sans téléchargement.

Contrôles locaux : **435 tests Rust** (226 cœur, 139 application, 7 i18n,
63 CLI), **14 tests Python**, formatage, Clippy toutes cibles, frontière du cœur et
contrôle de taille passent. Pas de changement de l'interface graphique ni du cœur ;
les huit captures de référence du troisième lot restent le dernier contrôle visuel
de l'application. Les preuves de cette recette sont dans `output/phase-5-acceptance/` ;
méthode, limites et commandes reproductibles dans [la recette](mcp-acceptance.md).

## Phase 6

L’assistance à la composition est un objectif retenu : l’agent rejoint le moteur
du document ouvert dans l’application. Les révisions protègent les changements de
l’utilisateur ; l’historique et l’affichage restent communs. Ce canal n’est pas
implémenté par le prototype autonome.

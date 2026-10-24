# Phase 5 — MCP documentaire

Décisions du 24 octobre 2026 : D11/D12, D25–D28 et complément D14 dans
[la feuille de route](../ROADMAP-ARCHITECTURE.md). La phase est en cours.

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

## Lots suivants

1. Racines de lecture/écriture, ouverture sous verrou, copie et sauvegarde/export
   avec confinement des chemins, y compris imports de modèles/images/signatures
   et sauvegarde finale des lots. Mode lecture seule et limites documentées/testées.
2. Compléter la documentation des capacités et la parité de tous les outils,
   ressources et erreurs. Test de création/vérification/export d’un courrier
   sans skill ni consultation du dépôt, puis essai avec un agent dans un client MCP.

## Phase 6

L’assistance à la composition est un objectif retenu : l’agent rejoint le moteur
du document ouvert dans l’application. Les révisions protègent les changements de
l’utilisateur ; l’historique et l’affichage restent communs. Ce canal n’est pas
implémenté par le prototype autonome.

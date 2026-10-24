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

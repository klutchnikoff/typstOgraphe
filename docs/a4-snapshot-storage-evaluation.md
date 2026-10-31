# A.4 — Évaluation du stockage des snapshots

**31 octobre 2026 — évaluation locale ; CAS retenu après examen du bilan.**
Les [décisions fonctionnelles](../ROADMAP.md#a4--snapshots-durables) sont consignées.
Ce lot compare deux stockages réellement exécutés en Rust release, sans modifier
le cœur, le format public du `.typsto` ou les dépendances de l’application.
L’interface Versions, la restauration et le comparateur restent à implémenter
après les choix et maquettes prévus dans A.4.

## Méthode

Trois historiques de 100 états confirmés, produits par les commandes de la CLI
actuelle : Courrier personnel, Article versionné et ce même Article enrichi de
250 paragraphes. Les états conservent leur historique Yjs ; chaque moteur reçoit
exactement les mêmes fichiers. L’exemple Article modifié localement n’a pas été
utilisé ni modifié. L’empreinte de l’exemple committé effectivement testé et les
empreintes des scripts sont dans le rapport brut.

Les scénarios comprennent texte Unicode, ajout/remplacement/suppression d’image,
image commune et modèle modifié. Ils sont mesurés à 20 puis 100 versions, avec
un tour de chauffe et sept échantillons sur macOS arm64, Apple M1 Max. Les caches
du système sont chauds ; les chiffres ne prédisent pas une ouverture à froid sur
un autre poste. La machine, les versions et toutes les valeurs sont enregistrées.

- **Git embarqué** : git2 0.21.0 et libgit2 1.9.7 embarquée, SSH/HTTPS désactivés.
  Arbres/blobs/commits des fichiers documentaires et pack différentiel. Commits
  indépendants, index applicatif pour la chronologie ; pas de chaîne d’ancêtres
  qui empêcherait de libérer un état supprimé.
- **Objets dédupliqués (CAS)** : fichiers identifiés par leur SHA-256 et référencés
  par chaque version, compression ZIP indépendante, sans deltas entre états.

Les deux candidats utilisent le **même stockage dédupliqué** pour les images,
la bibliographie et les dépendances, ainsi que les mêmes manifestes de ressources
partagés. L’index des versions reste compact. Les ressources sont lues à la
demande ; Git doit également importer son pack dans une base temporaire.
Le fichier autonome est remplacé avec `atomic-write-file` pour les deux moteurs.
Ce sont des enveloppes expérimentales, pas des archives de production à diffuser.

La recette et les API utilisées sont décrites dans le
[banc reproductible](../experiments/snapshot-storage/README.md).

## Mesures

Les temps portent sur le stockage : capture des fichiers déjà produits,
indexation/compression et publication atomique, ou ouverture et restitution d’une
version. Ils excluent génération du corpus, projection/validation du cœur,
compilation Typst et interface. **Ce ne sont pas des temps de frappe ou d’aperçu.**

**100 versions — tailles en Mio, temps médians en millisecondes.**

| Corpus | Stockage | Taille | Ajouter et enregistrer | Ouvrir et restituer | Supprimer et enregistrer¹ |
| --- | --- | ---: | ---: | ---: | ---: |
| Courrier | Git | 2.63 | 128.7 | 24.0 | 39.4 |
| Courrier | CAS | 3.20 | 84.1 | 9.6 | 35.2 |
| Article | Git | 2.48 | 242.7 | 23.5 | 23.8 |
| Article | CAS | 3.81 | 92.2 | 1.6 | 20.1 |
| Article + 250 paragraphes | Git | 2.49 | 375.3 | 59.0 | 30.5 |
| Article + 250 paragraphes | CAS | 4.47 | 140.9 | 1.7 | 21.5 |

¹ Conserver uniquement le premier et le dernier snapshot, puis nettoyer et republier.

**20 versions :**

| Corpus | Stockage | Taille (Mio) | Ajouter et enregistrer (ms) | Ouvrir et restituer (ms) |
| --- | --- | ---: | ---: | ---: |
| Courrier | Git | 1.79 | 65.8 | 17.0 |
| Courrier | CAS | 1.88 | 52.8 | 9.5 |
| Article | Git | 1.56 | 69.3 | 11.8 |
| Article | CAS | 1.80 | 45.2 | 1.8 |
| Article + 250 paragraphes | Git | 1.57 | 104.5 | 17.8 |
| Article + 250 paragraphes | CAS | 1.93 | 51.0 | 1.3 |

[Mesures finales brutes](../experiments/snapshot-storage/results/macos.json).

Le p95 brut est disponible pour chaque opération ; avec seulement sept mesures,
il correspond au maximum observé. Les temps de restitution d’un état dans un
stockage déjà ouvert, de première création et de suppression sont également
conservés, sans les confondre avec l’ouverture complète du projet dans Tauri.

## Garanties vérifiées

- **720 restitutions exactes** : 20 + 100 états × trois corpus × deux moteurs,
  comparés octet pour octet, y compris arbre, Yjs, métadonnées, modèle et ressources.
  Les dates, noms, tags et l’ordre de l’index sont conservés à la réouverture.
- **54 contrôles de refus/conservation** : échec avant publication laissant
  l’ancienne archive intacte, ressource absente ou altérée, ZIP tronqué et, pour
  Git, pack corrompu dans un ZIP pourtant valide.
- Suppression des états intermédiaires, puis réouverture : premier et dernier
  états intacts, états supprimés inaccessibles, aucun objet partagé orphelin dans
  l’archive republiée. Les ressources encore utiles restent disponibles.
- **30 projets reconstruits** (versions 0, 10, 20, 30, 99) : validation par le cœur,
  export Typst et compilation hors ligne réussis pour les deux moteurs et les
  trois corpus. Cela vérifie notamment l’ancienne image remplacée ou supprimée
  et les sources des modèles successifs.

Ces contrôles ne sont ni une qualification Windows/Linux, ni une simulation
d’une coupure de courant. Le lecteur de la sonde n’est pas un lecteur d’archive
de production durci ; les limites, gardes de fichiers et erreurs traduites du
cœur devront être étendues lors de l’intégration.

## Enseignements et recommandation

**Recommandation validée : retenir le stockage par objets dédupliqués pour A.4.**
Il est plus rapide à créer/écrire et à ouvrir sur les
trois corpus mesurés, en s’appuyant sur les briques déjà utilisées par le projet
(SHA-256, ZIP et écriture atomique), sans ajouter de moteur Git embarqué.

Git gagne sur la taille : à 100 versions, environ 18 % de moins pour le Courrier,
35 % pour l’Article et 44 % pour l’Article long. Ce gain est réel. Sur ce corpus,
le surcoût CAS reste de l’ordre de 0,6 à 2 Mio ; les temps de publication et la
simplicité du cycle de suppression favorisent CAS pour le premier périmètre.
Cela ne prouve pas que cet équilibre restera identique à des milliers de versions
ou sur des documents longs plus variés.

Sur l’Article long à 100 versions, l’ajout/publication demande environ 141 ms
avec CAS contre 375 ms avec Git, et ouverture/restitution environ 1,7 ms contre
59 ms. Une création de snapshot est une action explicite, pas une étape de chaque
frappe ; la persistance devra rester hors du chemin critique de l’aperçu. Le
coût total d’un futur bouton Versions inclura encore les validations du moteur.

Le CAS est retenu ; son format de production reste à préciser et à valider avant
implémentation. Git n’est pas adopté dans l’application. Cette décision ne clôt
pas A.4.

Deux points concrets révélés et corrigés **pour les deux candidats** :

1. Lire toutes les ressources de l’historique à chaque consultation est inutile.
   Le parcours sélectif ouvre l’index puis seulement l’état demandé.
2. Le Courrier embarque près de 800 entrées, notamment ses dépendances de date.
   Recopier leurs chemins et empreintes dans chaque ligne d’index alourdit cet
   index. Les manifestes de ressources identiques doivent être partagés et lus
   à la demande, indépendamment du choix de Git ou CAS.

Les premiers relevés avec index répété et ouverture globale, puis avec ouverture
sélective seule, sont conservés comme diagnostics. Leurs tailles et temps ne
doivent pas être mélangés aux mesures finales avec manifestes dédupliqués.

Le moteur de différences reste un **choix séparé** : aucun besoin de stocker dans
Git pour présenter des groupes de différences lisibles. Le stockage CAS ne
justifie pas d’écrire un nouvel algorithme de diff ; évaluer une bibliothèque
existante et l’adaptation documentaire restent des travaux A.4 distincts.

## Suite à valider

1. Le CAS étant retenu, valider la [proposition de format](a4-project-format.md) :
   schéma versionné du `.typsto`, index, données Yjs conservées,
   les contrôles d’intégrité et les budgets de taille/nombre d’entrées. Conserver
   un snapshot complet plutôt qu’une seule frontière causale dépourvue de contenu.
2. Maquetter Versions, la comparaison commune et les confirmations avant de
   coder les parcours validés. Préparer le lien de référence avec une future
   relecture et garder la validation locale MCP en F.3.
3. Étendre les commandes/requêtes communes de création, liste, consultation,
   nom/tags, suppression et restauration, avec autorisations par adaptateur.
4. Vérifier le vrai parcours atomique et gardé : création enregistrée, sauvegarde
   automatique avant restauration, refus sans perte, références protégées,
   nettoyage des ressources, sauvegarde/réouverture et exports des versions choisies.

Pas de rétention destructive automatique, de reconstruction silencieuse des
identités collaboratives ou de nouvelle dépendance de production dans ce lot.

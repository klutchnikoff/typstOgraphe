# Recette MCP — 24 octobre 2026

**Parcours réussi : un agent crée, rédige, vérifie, sauvegarde, exporte et rouvre
un courrier en découvrant l'application uniquement par MCP.** La phase 5 autonome
est terminée ; le rattachement au document ouvert dans egui relève de la phase 6.

## Conditions de l'essai

Sur macOS, serveur `target/debug/typstographe-cli mcp`, racines de lecture et
d'écriture limitées au dossier des documents d'essai. Une session distincte par
client ; aucun profil personnel ni signature de l'utilisateur importés.

L'[inspecteur officiel](https://github.com/modelcontextprotocol/inspector) **2.12.0**
est installé localement dans `output/phase-5-acceptance/tooling/`, avec Node 26.9.0.
Il s'agit d'un outil de recette, sans nouvelle dépendance de production.

L'agent est lancé sans historique de la conversation. Il reçoit la consigne métier,
les destinations et uniquement la syntaxe d'un relais de transport : lister les
outils, obtenir un schéma, lister/lire les ressources, appeler un outil. Ce relais
Python invoque le client persistant officiel **mcpdo**, conserve sa connexion stdio
et journalise appels et réponses. Il ne compose ni commande documentaire ni contenu.
Les PNG reçus sont écrits localement pour inspection, au lieu d'afficher leur base64.

L'agent a pour instruction de ne consulter aucun skill, source, README, test ou
autre fichier du dépôt, et de ne pas utiliser le web. Sa documentation applicative
provient des réponses MCP. **C'est une restriction d'essai par consigne, pas une
isolation OS** ; le serveur applique séparément sa politique de chemins. Ce test
ne démontre pas que tous les agents ou fournisseurs de modèles sont compatibles.

## Consigne métier

Créer un courrier personnel français, daté du 24 octobre 2026, envoyé de Lyon par
Camille Martin, 18 rue des Érables, 69003 Lyon, camille.martin@example.org, au Service
des équipements, Mairie de Valmont, 12 place de la République, 35000 Valmont.
Objet : « Demande de réservation d'une salle » ; référence CM-2026-042.

Demander la disponibilité de la salle des Tilleuls le **12 novembre 2026**, de 14 h
à 18 h, pour trente personnes, et les modalités de réservation, sans annoncer de
confirmation. Mettre la date en gras. Ajouter un P.-S. sur l'horaire adaptable et une
mention de pièce jointe « Présentation de la rencontre », sans incorporer de fichier.
Conserver l'espace de signature manuscrite. Choisir type et modèle par découverte,
contrôler validité et champs requis, consulter l'aperçu, sauvegarder, exporter PDF
et Typst autonome, puis fermer et rouvrir pour vérifier la conservation.

Toutes les coordonnées sont fictives. Aucun envoi de courrier n'est effectué.

## Résultats observés

| Contrôle | Résultat |
| --- | --- |
| Inspecteur web | Connexion stdio, MCP 2025-11-25, 67 outils et cinq ressources |
| Interactions graphiques | Découverte des types, création d'un courrier, lecture de `typstographe://document` |
| Inspecteur CLI strict après correction | Aucune erreur ; 104 avertissements répètent les unions légales avec `null` dans les schémas des outils |
| Découverte par l'agent | Liste des outils, ressources, 16 demandes de schéma et quatre lectures de ressources |
| Outils documentaires et de documentation | 27 appels, aucune erreur applicative |
| Vérification documentaire | Valide et prêt pour l'export |
| Aperçu | Une page, informations et mise en forme vérifiées visuellement |
| Sauvegarde et export | `.typsto`, PDF, sources Typst et PNG produits |
| Réouverture | Texte, source et métadonnées identiques ; archive propre, signature sans image |
| Autonomie de l'export | Recompilation avec Typst 0.15.1, polices système ignorées et dossiers de packages/cache initialement vides, sans téléchargement |

L'agent a constaté que le gras appliqué à la date devenait le style de frappe et
était hérité par les nouvelles annotations. Il a trouvé les deux textes par
`search.find` et retiré leur gras avec `text.bold`, puis contrôlé à nouveau l'aperçu.
Le comportement existant est conservé ; l'essai montre que l'agent a pu le corriger
avec les opérations documentées. Une éventuelle évolution de cette règle relève
du bilan des interactions, sans être intégrée silencieusement à cette recette.

## Corrections issues de la recette

1. **Portabilité du lot** : le schéma MCP annonçait `true` pour les paramètres
   d'étape, alors que l'adaptateur exige déjà un objet. Il annonce désormais
   `type: object` ; les paramètres détaillés restent ceux de chaque outil. Tests
   des valeurs non objets, de l'absence de publication et du contrat du cœur inchangé.
2. **Langue par défaut** : l'aide clap initialisait le chargeur sur la langue du
   système avant le choix anglais de MCP. Le choix intervient maintenant avant
   cette construction. Tests des résumés anglais par défaut, de `--json`, du
   français explicite et de `--lang=en`.

Les avertissements de l'inspecteur sur les types avec `null` ne sont pas des erreurs
JSON Schema. Ils signalent une limite pour les clients qui traduisent les schémas
vers un dialecte plus restreint. Aucun aplatissement ne supprime la distinction
entre une propriété absente et sa valeur `null`.

## Régression et preuves conservées

[La fixture](../fixtures/mcp-agent-letter.json) contient les valeurs initiales et
les textes réellement produits. Le test `crates/cli/tests/mcp_acceptance.rs` rejoue
le parcours avec le client SDK officiel, sous les deux versions de protocole déjà
testées. Il obtient les références par recherche, contrôle les styles, les annotations,
le PNG et l'export, puis compare source, texte et métadonnées après réouverture.
Ce test est inclus dans les jobs Rust sur macOS, Windows et Linux. Il reste distinct
de l'essai interactif avec le LLM, réalisé ici sur macOS.

```sh
cargo test --locked -p typstographe-cli --test mcp_acceptance
```

Preuves locales dans `output/phase-5-acceptance/`, conservées hors Git :

- `agent/agent-transcript.jsonl` : échanges du relais, y compris les deux opérations
  de connexion préparatoires ; l'échec initial de liaison locale dans la sandbox
  précède l'agent et a été résolu par l'autorisation du socket local.
- `agent/documents/courrier-agent.typsto` et `agent/documents/export-agent/` :
  document, PDF, sources et PNG contrôlés.
- `inspector/tools.json`, `inspector/schema-lint.log`, captures `resource.png`,
  `protocol.png` et `batch-fixed.png` : contrôle du client réel et correction.
- `result.json` : comptages et empreintes SHA-256 des fichiers exportés.
- `offline/` : recompilation indépendante, caches de packages et journal.
- Journaux des contrôles Rust, Python, Clippy, formatage et frontière du cœur.

Pour refaire le contrôle dans l'inspecteur, créer un fichier `inspector.json` avec
une entrée `mcpServers.typstographe` de type `stdio`, le chemin absolu du binaire,
les arguments `mcp --read-root … --write-root …` et un `cwd` dédié. Les racines
doivent déjà exister. Puis :

```sh
npx @modelcontextprotocol/inspector@2.12.0 --web --config inspector.json
npx @modelcontextprotocol/inspector@2.12.0 --cli --config inspector.json --server typstographe --method tools/list --strict --format json
```

Chaque connexion démarre son propre serveur autonome. Les méthodes, choix de
sécurité et limitations sont détaillés dans [le guide MCP](mcp.md).

# MCP typstOgraphe — prototype en mémoire

Lancer le binaire CLI construit avec `cargo build --locked -p typstographe-cli` :

```sh
typstographe-cli mcp
typstographe-cli mcp --read-only
typstographe-cli mcp --lang fr
```

Le transport est **stdio**, avec le SDK Rust officiel `rmcp 3.7.0`. stdout est
réservé au protocole MCP ; `--json` ne lui ajoute pas l’enveloppe de la CLI.
Les messages d’exécution sont en anglais par défaut, indépendamment du système,
ou en français avec `--lang fr`. Identifiants, schémas et documentation technique
restent en anglais ; la langue du document reste une donnée distincte.

Ce premier lot garde **un document en mémoire par processus**, sans lien avec
le document ouvert dans l’application. Aucun fichier documentaire n’est ouvert
ou écrit. Il n’offre pas encore de racines, de sauvegarde ou d’export : fermer
le serveur perd le brouillon. Ces opérations arrivent dans le prochain lot,
avec permissions et verrou. Ne pas utiliser ce prototype pour conserver un travail.

## Découverte

Le serveur fournit 62 outils : 59 issus du registre documentaire et 3 outils
de découverte/rendu. La liste et ses schémas restent stables pendant la session.
Les 68 identifiants du cœur ne changent pas.

| Outil | Utilisation |
| --- | --- |
| `document_types.list` | Types intégrés, création possible, capacités et fiches de modèles |
| `document_types.describe` | Objectif du type, champs, obligations d’export, défauts, capacités et modèles |
| `capabilities.describe` | Description, exemples, schéma commun et schéma MCP effectif d’une commande |
| `commands.list` | Catalogue et disponibilité contextuelle, complétés par le support MCP |
| `document.inspect` | État et référence courante du document ; fonctionne aussi sans document |
| `models.list` | Fiches des modèles du type actif, avec le modèle actuellement reconnu |
| `render.page` | PNG d’une page renvoyé directement comme contenu image MCP |

Le type porte les champs et les capacités. Le modèle n’en fournit qu’une
présentation : sa fiche contient identifiant, nom, type, description et mots-clés,
avec l’indicateur du modèle actif lorsque le contexte le permet. Aucun contrat
Typst de modèle n’est à apprendre pour rédiger. Les modèles d’un même type sont
interchangeables. Le type `compte-rendu` existe, mais n’a pas encore de modèle
fourni : il est signalé comme non créable.

Les capacités d’un type, la disponibilité à la position courante et l’autorisation
du serveur sont distinctes. `commands.list` expose `context_availability` du cœur
et `mcp_support` ; `capabilities.describe` précise `prototype_support` et donne
`mcp_input_schema`, qui est le schéma à utiliser pour appeler l’outil MCP.
`command.params_schema` reste le contrat commun du moteur pour les autres adaptateurs.
Les exemples détaillés couvrent les premières opérations de rédaction ; leur
complétion pour tout le catalogue reste un lot de phase 5.

Les ressources `typstographe://guide`, `typstographe://types` et
`typstographe://commands` complètent les outils. Elles sont disponibles sans
lecture d’un fichier, sans dépendre de leur inclusion automatique par un client.
Aucun skill n’est nécessaire pour découvrir l’interface.

## Références documentaires et appels

Chaque réponse réussie fournit `structuredContent` avec `version`, `ok`, `outcome`,
`summary` et `document`. Les résultats restent également disponibles comme texte
JSON pour les clients qui n’exploitent pas le contenu structuré.

`document` vaut `null` sans document ouvert, sinon un objet contenant un identifiant
UUID propre à cette session et une garde du moteur. Renvoyer cet objet dans le
champ `document` de l’appel suivant. Une édition peut changer sa révision ; une
création/remplacement change aussi son identifiant. Aucun chemin de fichier ne
tient lieu d’identité documentaire.

Séquence minimale, avec les arguments des outils :

```json
{"type":"courrier","model":"courrier.model.personal"}
```

Ces arguments s’appliquent à `document.new`. Conserver la propriété `document`
de sa réponse, puis appeler `text.insert` avec cette référence et le champ `text`.
Par exemple, un appel réel à `text.insert` a la forme suivante ; remplacer la
référence illustrative par celle reçue, sans fabriquer de valeurs :

```json
{
  "document": {
    "id": "UUID retourné par le serveur",
    "guard": { "document_id": 4, "revision": 2 }
  },
  "text": "Bonjour Élodie.\n\nUn nouveau paragraphe."
}
```

Utiliser ensuite `text.bold` avec la nouvelle référence, `value: "on"` et
`selector: { "select": "Élodie" }`. Une recherche de sélecteur est exacte et
sensible à la casse ; une ambiguïté exige `occurrence`, à partir de 1.
`selector: { "block": 1 }` cible un bloc dans l’ordre de `document.structure`.
Une cible explicite du moteur et un sélecteur de confort sont exclusifs.

Les gardes de premier niveau sont préparées par le serveur depuis `document`.
Ne pas fournir un champ `guard` séparé. Les cibles logiques retournées par les
lectures conservent leurs propres gardes : elles ne sont jamais réancrées en silence.
En cas de référence périmée, lire `document.inspect` sans référence pour retrouver
l’état courant, puis relire les cibles nécessaires.

`text.insert` reçoit du texte littéral ; gras, italique, liens et structures
passent par les outils dédiés. Un retour simple reste une espace dans le même
paragraphe ; deux retours créent un paragraphe.

## Lots, contrôle et ressources embarquées

`document.batch` reçoit `document` et les étapes `{ id, params }` du moteur.
Les gardes des étapes sont calculées sur le brouillon courant. Le champ
`params.selector` permet de cibler du texte ou une image créés plus tôt dans le lot.
Sans sélecteur, la sélection laissée par l’étape précédente est utilisée.
Une erreur ne publie rien ; un lot réussi s’annule en une fois.

Dans ce prototype, `save` doit être absent ou null. Les familles de fichiers,
récupération et les lots imbriqués restent interdits dans les étapes. Les budgets
du moteur s’appliquent, dont 256 étapes et 4 Mio de paramètres ; chaque appel
MCP est lui aussi limité à 4 Mio après décodage JSON. Ce n’est pas une limite
annoncée sur les octets bruts du transport du SDK.

`insert.image`, `image.replace` et `document.set_signature` acceptent uniquement
`ImageInput` embarqué : `kind: "embedded"`, `extension` et `data` en base64 standard.
Les chemins sont refusés, même dans une étape tardive ou à la création d’un document.
Les modèles fournis et les packages embarqués fonctionnent hors ligne.

`document.validate` sépare validité du document/modèle et champs manquants pour
export. `spelling.issues` et `spelling.suggestions` sont des lectures distinctes,
sans correction automatique et sans vérification grammaticale.
`render.page` reçoit `document` et `page`, numérotée à partir de 1. Son résultat
comprend un bloc image `image/png` et les dimensions de page en points Typst.

Les erreurs d’exécution sont des résultats MCP avec `isError: true`, code stable,
paramètres et message traduit. Outil inconnu/non exposé et ressource inconnue
sont des erreurs de protocole. La politique et les gardes sont vérifiées côté
serveur, indépendamment des annotations indicatives de MCP.

`--read-only` refuse toutes les commandes de mutation, y compris création,
sélection et historique ; les lectures de découverte restent disponibles.
Un appel déjà en cours peut terminer si le client l’annule ; l’annulation n’est
pas une promesse de restauration. Les appels annulés avant leur prise en charge
par le worker sont ignorés lorsque leur canal de réponse est déjà fermé.

## Vérification et suite

```sh
cargo test --locked -p typstographe-cli --test mcp
```

Les tests utilisent un véritable client SDK lancé contre le processus CLI sur
stdio : découverte, rédaction, validation, PNG, historique, erreurs, lecture seule
et appels concurrents. Ils couvrent 2025-11-25 et la version par défaut du SDK.
Les tests du cœur et de l’adaptateur couvrent aussi la documentation, les schémas,
les limites, les références étrangères et les refus de fichiers.

Restent en phase 5 : racines et politique de fichiers, ouverture sous verrou,
sauvegarde/export et sauvegarde finale de lot ; documentation complète, essai
dans l’inspecteur MCP et avec un agent sans skill ni lecture du dépôt. La phase 6
rattachera l’agent au moteur du document ouvert dans l’application.

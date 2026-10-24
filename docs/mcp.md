# MCP typstOgraphe — session documentaire autonome

Le [contrat général des interfaces](interface-contract.md) décrit le moteur commun.
Ce guide précise l'enveloppe MCP, sa découverte et ses règles de session/fichiers.

Construire avec `cargo build --locked -p typstographe-cli`, puis lancer :

```sh
typstographe-cli mcp
typstographe-cli mcp --read-root /chemin/courriers --write-root /chemin/courriers
typstographe-cli mcp --read-root /chemin/courriers --read-only
typstographe-cli mcp --read-root /chemin/entrees --write-root /chemin/sorties --lang fr
```

`--read-root` et `--write-root` sont répétables et désignent des **répertoires
existants**. Une racine invalide arrête le lancement, sur stderr, avant le protocole.
Le serveur ne crée pas les racines. Sans racine, la rédaction et l’aperçu en mémoire
restent possibles, mais aucun accès documentaire aux fichiers n’est autorisé.

Le transport est **stdio**, avec `rmcp 3.7.0`. stdout est réservé au protocole ;
`--json` ne lui ajoute pas l’enveloppe de la CLI. Les messages d’exécution sont en
anglais par défaut, ou en français avec `--lang fr`. Identifiants, schémas et aide
technique restent en anglais ; la langue du document est une donnée distincte.

Une session possède **un document en mémoire par processus**, indépendant de celui
ouvert dans l’application. Rien n’est enregistré automatiquement : fermer le
serveur perd les modifications non sauvegardées. La phase 6 reliera l’assistant
au moteur du document ouvert dans l’application.

## Découverte

Le serveur fournit **67 outils** : 64 issus du registre et 3 outils de découverte/rendu.
La liste et ses schémas restent stables pendant la session, quelles que soient les
permissions. Les 68 identifiants du cœur ne changent pas. Récupération et
`models.presentation` ne sont pas exposées.

| Outil | Utilisation |
| --- | --- |
| `document_types.list` | Types intégrés, création possible, capacités et fiches de modèles |
| `document_types.describe` | Objectif du type, champs, obligations d’export, défauts, capacités et modèles |
| `capabilities.describe` | Description, exemples, schéma commun et schéma MCP effectif d’une commande |
| `commands.list` | Catalogue et disponibilité contextuelle, complétés par les autorisations MCP |
| `document.inspect` | État, référence courante et permissions ; fonctionne aussi sans document |
| `models.list` | Fiches des modèles du type actif, avec le modèle actuellement reconnu |
| `render.page` | PNG d’une page renvoyé directement comme contenu image MCP |

Le type porte les champs et les capacités. Le modèle n’en fournit qu’une
présentation : sa fiche contient identifiant, nom, type, description et mots-clés,
avec l’indicateur actif lorsque le contexte le permet. Les modèles d’un même type
sont interchangeables. Le type `compte-rendu` existe mais reste non créable,
faute de modèle fourni.

Capacité du type, disponibilité à la position courante et autorisation du serveur
sont distinctes. `commands.list` expose `context_availability` et `mcp_support` ;
`capabilities.describe` précise `prototype_support` et `mcp_input_schema`, le schéma
à utiliser pour appeler MCP. `command.params_schema` reste le contrat commun du
moteur. `authorized` indique une permission générale ; chaque chemin et chaque
commande sont encore contrôlés à l’exécution. Une image embarquée, par exemple,
reste possible sans racine de lecture ; une image par chemin exige cette racine.

Chaque opération du cœur a une fiche explicite : détails, défauts extraits du schéma,
restrictions, résultat attendu, effets et exemple. Les 3 helpers MCP ont aussi leur
fiche. Une capacité ajoutée sans documentation fait échouer les tests du catalogue.

`examples[].params` contient les valeurs littérales. Lorsque l’exemple a besoin
d’une référence courante, `bindings` indique le paramètre (pointeur JSON), le genre
de référence et l’opération où l’obtenir. **Remplacer chaque valeur null ainsi
signalée avant l’appel**, puis fournir la référence `document` extérieure. Les
réponses de requêtes sont sous `outcome`. Aucun identifiant/garde fictif n’est à
recopier : rechercher un texte, lire les fautes ou choisir un bloc image donne les
références réelles. Une absence de bloc/faute/rendu impose d’abord son prérequis.

Exemple de principe pour `image.remove`, après `capabilities.describe` :

```json
{
  "params": { "target": null },
  "bindings": [{
    "parameter": "/target",
    "reference": "image_block",
    "obtain_from": "document.structure : garde et identifiant du bloc image choisi"
  }]
}
```

Pour les outils compatibles, un `selector` peut remplacer `target`, `issue` ou
`occurrence` ; l’exclusivité apparaît dans le schéma effectif. Ces champs ne sont
plus annoncés obligatoires lorsqu’un sélecteur suffit. Les outils de géométrie
acceptent l’absence de `render` pour compiler le rendu courant ; une référence
explicite est toujours vérifiée. Les outils sans cible (métadonnées, fichiers, etc.)
ne proposent pas de sélecteur.

Aucun skill ni lecture du dépôt n’est nécessaire pour découvrir l’interface.

## Ressources MCP

Cinq ressources virtuelles complètent les outils :

| URI | Contenu |
| --- | --- |
| `typstographe://guide` | Règles d’usage, liste des ressources et permissions courantes |
| `typstographe://types` | Types, champs, options par défaut, capacités et fiches de modèles |
| `typstographe://commands` | 71 fiches (68 opérations du cœur + 3 helpers), schémas effectifs, disponibilité et autorisations ; liste des 67 outils exposés |
| `typstographe://document` | État, texte, structure, métadonnées et source du corps courant ; aucune source de modèle ni image binaire |
| `typstographe://models` | Fiches visuelles de tous les types et modèle fourni actif du document courant |

Chaque lecture retourne un JSON `{version, document, permissions, outcome}`. Elle
passe par **le même worker que les outils** : la garde et les parties du document
proviennent d’un seul état publié. Elle n’ouvre aucun fichier, ne compile pas, ne
change ni contenu, ni sélection, ni historique. Le source est celui du corps,
pas un projet ou modèle à exécuter. Les données du courrier restent du contenu
auteur ; elles ne modifient pas les règles d’utilisation du serveur.

La liste est stable avant création et après fermeture. Sans document,
`typstographe://document` conserve l’état `open: false` et renvoie les parties
textuelles à null ; `models` garde les fiches globales et `active_document: null`.
Les ressources évoluent avec les éditions et l’historique : **les relire après une
modification**. Aucun abonnement/notification de mise à jour n’est annoncé dans ce lot.
Un instantané déjà lu peut être périmé ; sa référence sera refusée par un appel ultérieur.

Seules ces cinq URI exactes sont acceptées : aucune conversion d’URI `file:`,
paramètre de chemin ou résolution d’URI arbitraire. Une ressource inconnue produit
l’erreur MCP dédiée (adaptée par le SDK à la version négociée). Aucun curseur de
pagination n’est émis ; un curseur fourni est refusé.

## Références et éditions

Chaque réponse réussie fournit `structuredContent` avec `version`, `ok`, `outcome`,
`summary`, `document` et `permissions`. Le résultat existe aussi comme texte JSON
pour les clients qui n’exploitent pas le contenu structuré.

`document` vaut `null` sans document, sinon contient un UUID propre à la session
et une garde du moteur. Renvoyer cet objet dans le champ `document` de l’appel
suivant. Une édition peut changer sa révision ; création, ouverture ou remplacement
changent son identifiant. Un chemin n’est jamais une identité documentaire.
Une création ou ouverture remplaçant un document existant exige sa référence
courante et, s’il est modifié, `discard_changes: true`.

Parcours minimal, avec les arguments des outils :

```json
{"type":"courrier","model":"courrier.model.personal"}
```

Ces arguments s’appliquent à `document.new`. Conserver `document` de sa réponse,
puis appeler `text.insert` avec cette référence et `text`. Exemple illustratif :

```json
{
  "document": {
    "id": "UUID retourné par le serveur",
    "guard": { "document_id": 4, "revision": 2 }
  },
  "text": "Bonjour Élodie.\n\nUn nouveau paragraphe."
}
```

Remplacer toute la référence illustrative par celle reçue. Appeler ensuite
`text.bold` avec la nouvelle référence, `value: "on"` et
`selector: { "select": "Élodie" }`. Le sélecteur cherche exactement le texte,
avec la casse ; une ambiguïté exige `occurrence`, à partir de 1.
`selector: { "block": 1 }` cible un bloc dans l’ordre de `document.structure`.
Une cible explicite du moteur et un sélecteur de confort sont exclusifs.

Les gardes de premier niveau sont préparées par le serveur depuis `document`.
Ne pas fournir `guard` séparément. Les cibles retournées par les lectures gardent
leurs propres gardes et ne sont jamais réancrées silencieusement. Après une erreur
de révision, lire `document.inspect` sans référence, puis relire les cibles utiles.

`text.insert` reçoit du texte littéral ; styles et structures utilisent les outils
dédiés. Un retour simple reste une espace ; deux retours créent un paragraphe.
Les sélections de confort et leur édition échouent ensemble.

## Fichiers, verrous et copies

Le champ extérieur `permissions` décrit les racines canonicalisées, le répertoire
de lancement qui sert de base aux chemins relatifs, `read_only` et `write_enabled`.
Les racines de lecture et d’écriture sont **indépendantes** : l’une n’accorde pas
l’autre. Une ouverture éditable exige les deux, car elle lit l’archive et réserve
son écriture. La création d’un brouillon en mémoire n’exige aucune racine.

| Outil | Contrat et autorisations |
| --- | --- |
| `document.open` | `path` sous une racine de lecture ; avec `read_only: false` (défaut), aussi sous une racine d’écriture, verrou inclus |
| `document.save` | `path` facultatif, sinon chemin courant ; écriture autorisée, sauvegarde atomique, historique conservé |
| `document.save_as` | `path` requis ; écriture autorisée, `overwrite: false` par défaut, verrou toujours respecté |
| `document.export` | `directory` autorisé en écriture, nouveau ou vide ; `format: pdf/typst/both` et `page_images` (true par défaut) |
| `document.close` | Libère la session et son verrou ; refus des modifications non enregistrées sans `discard_changes` |

L’ouverture éditable prend le **même verrou coopératif** que l’application et la
CLI. Un second écrivain reçoit `file_locked`, sans attente ni repli silencieux.
Le compagnon `.<nom>.lock` peut être créé dès l’ouverture, même avant une sauvegarde ;
il reste sur disque après fermeture et ne doit pas être supprimé. Sa présence
n’indique pas un verrou actif : c’est le verrou du système qui fait foi.

Pour lire un fichier déjà réservé, ouvrir avec `read_only: true`. Le serveur lit
la **dernière version enregistrée**, sans voir les modifications en mémoire de
l’autre session ni réserver d’écriture. Les éditions sont alors refusées.
`document.save_as` vers **un autre chemin autorisé** crée une copie indépendante
et rend cette copie éditable. Le document original reste protégé. Un serveur lancé
avec `--read-only` interdit aussi cette copie, toute sauvegarde et tout export ;
il permet uniquement les requêtes, l’ouverture avec `read_only: true` et la fermeture.

Une sauvegarde vérifie l’empreinte du fichier avant de le remplacer ; un changement
externe donne `file_changed`. Elle refuse une destination existante différente,
sauf décision explicite `overwrite: true` avec `save_as`. Le verrou d’un autre
écrivain reste prioritaire sur cette option.

L’export utilise le contenu courant en mémoire, ne sauvegarde pas l’archive et ne
réinitialise pas son état modifié. L’export Typst conserve ressources, packages et
polices nécessaires au fonctionnement hors ligne. Une erreur d’écriture peut
laisser un export partiel ; il ne s’agit pas d’une transaction multi-fichiers.
Une nouvelle tentative exige une destination nouvelle ou vide.

## Imports et lots atomiques

`insert.image`, `image.replace`, `document.set_signature` et la signature initiale
acceptent soit une image embarquée (`kind: embedded`, `extension`, `data` en base64),
soit un fichier (`kind: path`, `path`) sous une racine de lecture. La ressource est
copiée en mémoire : les sauvegardes et exports ne dépendent pas du fichier initial.

`document.set_model` accepte une source Typst ou un fichier UTF-8 autorisé en lecture.
Il valide le contrat du type et les dépendances avant publication. Ce n’est pas
un import arbitraire de projet Typst : seules les dépendances embarquées disponibles
sont utilisables, sans téléchargement ni lecture implicite à côté du modèle.
Le modèle conserve son rôle de présentation.

`document.batch` reçoit `document` et des étapes `{ id, params }`. Les gardes des
étapes sont calculées sur le brouillon courant ; `params.selector` permet de cibler
un élément créé plus tôt. Une erreur ne publie rien ; un lot réussi s’annule une fois.
Chaque `params` est un **objet JSON**, comme dans l'appel de l'outil correspondant,
sans `document` ni `guard` extérieur. Le schéma MCP exprime cette contrainte ; la
fiche de chaque outil décrit les champs de cet objet.

Le champ `save` est facultatif. Pour une sauvegarde finale explicite :

```json
{"kind":"current"}
```

ou :

```json
{"kind":"as","path":"courriers/lettre.typsto","overwrite":false}
```

Ces objets sont la valeur de `save`, pas des étapes. La destination et son compagnon
sont autorisés avant les éditions ; la sauvegarde finale utilise le verrou et le
contrôle d’empreinte du moteur. Son échec conserve le document publié et son historique.
Les étapes excluent les opérations de fichiers, la récupération et les lots imbriqués ;
les imports d’images/modèles restent autorisés selon les racines de lecture.
L’export est toujours un appel distinct. Annuler un lot déjà sauvegardé modifie
le brouillon, sans annuler l’écriture disque ; il faut sauvegarder à nouveau.

## Gras et italique : délimiter les insertions

`text.bold` et `text.italic` ont deux usages, selon la cible :

- **Une sélection non vide** : `on` applique le style, `off` l'enlève au texte
  sélectionné. Le style de frappe est repris de la sélection après modification.
- **Un point d'insertion vide** : `on` ou `off` règle le style des prochaines
  insertions, sans modifier le texte déjà présent.

Utiliser explicitement `value: "on"` et `value: "off"` pour une composition
déterministe. Le défaut `toggle` dépend de l'état courant. Gras et italique sont
indépendants : désactiver l'un conserve l'autre.

**Un style actif persiste** : un nouveau paragraphe, P.-S. ou autre structure
ne garantit pas son arrêt. Une navigation peut aussi reprendre le style du texte
environnant. `document.inspect` expose `typing_style` et la sélection pour contrôler
l'état. `text.insert` laisse un point d'insertion vide après le texte inséré :
c'est là qu'il faut désactiver le style pour préserver le passage formaté tout
en reprenant une rédaction sans ce style. `off` sur une sélection non vide
retirerait au contraire la mise en forme de cette sélection.

Ces exemples partent d'un **point d'insertion vide à l'endroit voulu, avec le gras
et l'italique désactivés**. Si du texte est sélectionné, se déplacer d'abord vers
un point d'insertion logique vide avec `text.select`, à partir des références
gardées courantes ; ne pas inventer d'identifiants ou d'offsets.

Arguments de `document.batch` pour le gras : **ajouter au premier niveau la
propriété `document` reçue dans la dernière réponse**, puis envoyer cet objet à
`document.batch`. Les étapes n'ont ni `document` ni `guard` extérieur.

```json
{
  "steps": [
    { "id": "text.bold", "params": { "value": "on" } },
    { "id": "text.insert", "params": { "text": "Passage en gras" } },
    { "id": "text.bold", "params": { "value": "off" } },
    { "id": "text.insert", "params": { "text": " Puis la suite." } }
  ]
}
```

Même enchaînement pour l'italique, avec la même référence documentaire courante
à fournir au premier niveau de l'appel :

```json
{
  "steps": [
    { "id": "text.italic", "params": { "value": "on" } },
    { "id": "text.insert", "params": { "text": "Passage en italique" } },
    { "id": "text.italic", "params": { "value": "off" } },
    { "id": "text.insert", "params": { "text": " Puis la suite." } }
  ]
}
```

Seul le premier passage est formaté ; la suite ne l'est pas. Chaque lot constitue
une seule modification annulable. La dernière insertion illustre la reprise de
rédaction ; elle peut être omise si rien ne doit être ajouté immédiatement.

Pour mettre en forme du texte existant, `selector: { "select": "passage exact" }`
peut cibler directement ce texte avec `text.bold` ou `text.italic` ; une ambiguïté
exige de préciser l'occurrence. Cela ne constitue pas une remise à zéro automatique
du style de frappe.

Les mêmes lots sont disponibles dans `batch_examples` des fiches
`text.bold`, `text.italic`, `text.insert` et `document.batch`, consultables par
`capabilities.describe` et `typstographe://commands`. Chaque exemple précise
l'opération à appeler (`id`), ses `params`, ses préconditions et le résultat attendu ;
la fiche d'une commande de style peut donc présenter un exemple appelant
**`document.batch`**, sans que ses `steps` soient des paramètres de `text.bold`.

En mode `input_rules: "literal"` (défaut), `*gras*` et `_italique_` restent du
texte littéral : ce n'est pas un format de texte enrichi pour MCP. Utiliser les
commandes dédiées pour la mise en forme.

## Contrôles et limites

Les chemins sont résolus avant contrôle, y compris leurs ancêtres existants pour
les nouvelles destinations. Tous les composants `..` sont refusés, même s’ils
resteraient dans une racine. Un lien symbolique vers l’extérieur, un lien pendant,
ou un compagnon de verrou redirigé hors racine sont refusés. Les lectures acceptent
des fichiers ordinaires ; les cibles d’archive et de verrou existantes doivent
aussi être des fichiers ordinaires. Les limites de taille du moteur s’appliquent.

Il s’agit d’un contrôle applicatif de chemins, **pas d’une sandbox du système**.
Un autre processus local pouvant remplacer un lien ou un répertoire entre la
vérification et l’ouverture peut encore créer une course. Les liens physiques
ne prouvent pas l’origine d’un contenu. Les racines doivent donc désigner des
répertoires de confiance. Les verrous sont coopératifs ; l’empreinte détecte les
changements externes observés, sans rendre une écriture hostile concurrente impossible.

Les lots sont bornés à 256 étapes et 4 Mio de paramètres. Chaque appel MCP est aussi
limité à 4 Mio **après décodage JSON** ; ce n’est pas une limite sur les octets bruts
du transport SDK. Les ressources ont leurs budgets propres, y compris dans les lots.
Un appel annulé avant sa prise en charge est ignoré si son canal est fermé ; un appel
déjà en cours peut terminer. L’annulation ne promet pas de restauration.

`document.validate` sépare validité et champs manquants pour export. L’orthographe
est une lecture distincte, sans correction automatique ni grammaire. `render.page`
reçoit `document` et `page` à partir de 1, et renvoie une image PNG sans écrire de fichier.

Les erreurs d’exécution ont `isError: true`, un code stable, des paramètres et un
message traduit. Outil non exposé ou ressource inconnue sont des erreurs de protocole.
Le serveur ne propose ni commande réseau ni outil de lancement de processus.

## Vérification et suite

```sh
cargo test --locked -p typstographe-cli --test mcp --test mcp_files --test mcp_resources
```

Les tests font dialoguer de vrais clients SDK avec plusieurs processus CLI :
rédaction, garde, sauvegarde, conflit de verrou, lecture/copie, réouverture, export,
lecture seule, erreurs et PNG. Ils couvrent 2025-11-25 et la version par défaut du SDK.
Les tests unitaires ajoutent les chemins et liens, imports, empreintes et échecs
finaux de lots. Les tests de liens symboliques de l’adaptateur sont exécutés sous Unix.
Les exemples avec références sont liés à de vraies réponses du moteur et validés
contre les 67 schémas publiés avec `jsonschema 0.60.1`, dépendance de test seulement,
sans fonctionnalités HTTP/fichier et avec récupération externe désactivée.
Les ressources sont comparées aux outils au fil de création, édition, historique,
lecture seule et fermeture, avec vérification de l’absence de mutation/écriture.

Restent en phase 5 : essai dans l’inspecteur MCP et acceptation par un agent sans
skill ni consultation du dépôt, jusqu’à création, vérification et export du courrier.

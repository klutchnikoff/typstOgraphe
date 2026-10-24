# MCP typstOgraphe — session documentaire autonome

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

Les ressources `typstographe://guide`, `typstographe://types` et
`typstographe://commands` complètent les outils. Aucun skill ni lecture du dépôt
n’est nécessaire pour découvrir l’interface. Compléter l’aide de toutes les
capacités et les ressources du document actif reste un lot de phase 5.

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
cargo test --locked -p typstographe-cli --test mcp --test mcp_files
```

Les tests font dialoguer de vrais clients SDK avec plusieurs processus CLI :
rédaction, garde, sauvegarde, conflit de verrou, lecture/copie, réouverture, export,
lecture seule, erreurs et PNG. Ils couvrent 2025-11-25 et la version par défaut du SDK.
Les tests unitaires ajoutent les chemins et liens, imports, empreintes et échecs
finaux de lots. Les tests de liens symboliques de l’adaptateur sont exécutés sous Unix.

Restent en phase 5 : aide complète du catalogue, ressources du document actif,
inspection MCP et essai d’un agent sans skill ni consultation du dépôt.

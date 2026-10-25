# Fichiers MCP rattachés — lot 6.5

Le rattachement au document ouvert conserve les choix D34/D35/D36/D39.
L’acceptation d’un lot publie ses modifications en mémoire ; elle ne sauvegarde
ni n’exporte. Les opérations de fichiers sont explicites et indépendantes.

## Autorisation dans l’application

Le panneau **Assistance par un agent** propose deux droits facultatifs avant la
création de l’invitation, désactivés initialement :

- **Enregistrer le fichier courant** : disponible après un premier enregistrement
  dans l’application et seulement avec un rattachement en écriture. L’autorisation
  fixe le chemin canonique de ce fichier, sans donner accès à son répertoire.
- **Exporter dans un dossier choisi** : l’utilisateur choisit une racine existante.
  L’agent peut y créer un dossier d’export ou utiliser un dossier vide situé dessous.

L’invitation montre ses droits effectifs. Pour les modifier, révoquer et recréer
l’invitation. Ils ne sont ni des préférences globales ni une autorisation de
lecture des fichiers locaux. Les propositions ne peuvent donc pas importer une
image ou un modèle local sous prétexte qu’un dossier d’export est autorisé.

Un **Enregistrer sous** effectué par l’utilisateur conserve le document en mémoire,
mais ne transfère pas le droit de sauvegarde à son nouveau fichier. Créer une
nouvelle invitation pour l’autoriser. Fermeture/remplacement du document,
révocation et déconnexion conservent leurs règles de fin d’invitation.

## Contrat MCP

Les mêmes 70 outils restent exposés en mode rattaché, avec leurs schémas communs.
Les permissions de chaque réponse, `commands.list`, `capabilities.describe` et
les ressources décrivent les droits distincts de l’autorisation d’une proposition.
Exemple du champ spécifique de `document.inspect` :

```json
{
  "attached_files": {
    "current_file": "/documents/courrier.typsto",
    "export_roots": ["/documents/exports"],
    "save_as": false,
    "published_only": true,
    "export_requires_current_revision_before_write": true
  }
}
```

Un chemin absent vaut `null`, et l’absence de racine donne `[]`. `write_enabled`
peut être vrai pour ces droits précis alors que les racines génériques de lecture
et d’écriture restent vides. Cela n’autorise aucune autre commande de fichiers.

| Outil | Contrat rattaché |
| --- | --- |
| `document.save` | Référence documentaire courante et droit de sauvegarde requis. Omettre `path` ; un chemin explicite n’est accepté que s’il désigne le même fichier autorisé et toujours courant. Écriture atomique avec le verrou déjà détenu par l’application et contrôle des modifications externes. |
| `document.export` | Référence courante, `directory`, `format` et `page_images` selon le schéma. Dossier absent ou vide sous la racine autorisée ; aucun remplacement de fichiers existants. Le résultat identifie l’instantané exporté. |
| `document.save_as`, `new`, `open`, `close`, récupération, annuler/rétablir | Toujours refusés en appel direct rattaché. Aucun dialogue graphique déclenché par l’agent. |

Les lots proposés ne contiennent toujours ni opération de fichiers, ni sauvegarde
finale. Les fichiers contiennent uniquement **le document publié**, même si une
proposition est en attente ou ouverte en relecture. Le candidat reste privé.
L’enregistrement ne crée pas d’étape d’annulation. Un export ne marque jamais le
document comme enregistré et ne retire pas sa récupération.

## Exécution et décisions différées

Les demandes rejoignent le point d’exécution de la frame après publication de la
saisie native. Identité, garde complète, invitation et permissions sont vérifiées
à l’exécution, y compris pour une demande restée dans la file du transport.
IME, bandeau, dialogue ou export graphique en cours provoquent un refus explicite ;
rien n’est exécuté automatiquement à la fermeture de cette interruption.

Un enregistrement utilise immédiatement le moteur et son verrou courant. Son
effet `Saved` est transmis une fois à egui : retrait de la récupération par la
même fonction que pour une sauvegarde humaine, confirmation discrète et nouveau
titre/état modifié. Aucun déplacement, changement de style ou prise de focus.
La sélection renvoyée par MCP reste celle du contexte agent ; la réponse à une
opération de fichiers ne transmet pas implicitement la sélection de l’utilisateur.

L’export comporte deux étapes en arrière-plan, avec **un seul export MCP en vol** :

1. Préparer l’instantané publié et compiler sans écrire de fichier.
2. À la réception de ce résultat, revérifier la garde complète, l’absence
   d’interruption, la demande encore attendue, l’invitation et la destination.
   Refuser si l’utilisateur a modifié le document ; ne pas réancrer ni réessayer.
   Lancer alors l’écriture de cet instantané, avec un dernier contrôle de la
   destination et de la révocation avant son démarrage.

La politique commune résout les liens, refuse `..`, vérifie la racine canonique
initiale et le compagnon de verrou de sauvegarde. Une destination redirigée ou
une racine remplacée pendant la compilation ne reçoit aucun export. Le writer
commun revérifie le dossier vide et utilise `create_new` pour les fichiers.

Une annulation ou déconnexion avant l’écriture abandonne le résultat de la
compilation. Une compilation déjà lancée conserve le créneau d’export jusqu’à
sa fin, même après annulation : les annulations répétées ne créent pas une série
de workers concurrents. Après le début de l’écriture, une modification utilisateur n’altère
pas l’instantané exporté ; une coupure n’annule pas rétroactivement les fichiers
déjà produits. L’export n’est pas une transaction multi-fichiers (D16). Ces
contrôles sont coopératifs, pas un bac à sable contre un autre processus qui
modifierait simultanément le système de fichiers.

La réponse conserve la garde de l’instantané dans `outcome.guard` et
`outcome.value.guard`. La référence extérieure `document` et les permissions sont
rafraîchies au traitement de la fin d’export, avant de répondre au client.
`document.inspect` permet de relire le document publié après l’opération.

## Vérification

Tests : permissions facultatives, document non enregistré ou en lecture seule,
contenu publié contre candidat privé, sauvegarde sous verrou sans nouvelle entrée
d’historique, émission unique de l’effet, refus de redirection/Save As/lot avec
sauvegarde, garde périmée, interruption, conflit externe, changement de fichier,
export asynchrone PDF et Typst, annulation, dossier devenu non vide et racine
remplacée par un lien. Un client **SDK rmcp réel** exécute sauvegarde et export
avec une lecture concurrente sur la session possédée par l’interface.

Le test egui active le droit par sa case à cocher et vérifie récupération, focus,
sélection et style après l’effet de sauvegarde. La [recette 6.6 avec agent LLM](mcp-attached-acceptance.md) confirme maintenant
la rédaction simultanée et le parcours graphique jusqu’aux fichiers.

Validation locale macOS : **542 tests Rust et deux doctests**, 14 tests Python,
Clippy workspace/toutes cibles avec `-D warnings`, formatage, frontières du cœur,
limites de taille et construction des binaires réussis. Un test du protocole
rmcp annule réellement une requête et vérifie son abandon avant dispatch propriétaire.
Linux et Windows
restent vérifiés par la CI.

Capture native du panneau inspectée : `output/phase6-5/assistance.png` (conservée
localement, ignorée par Git). Reproduction :

```sh
cargo build --locked -p typstographe
mkdir -p output/phase6-5
target/debug/typstographe --capture-assistance output/phase6-5/assistance.png
```

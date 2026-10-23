# Lots documentaires atomiques

Contrat de phase 2, implémenté le 24 octobre 2026 conformément à D15/D16.
La commande `document.batch` utilise le même registre, les mêmes validations et le
même moteur que les éditions individuelles. Elle prépare un document en mémoire,
puis publie son résultat en une seule transaction d’annulation. Aucun transport
CLI/MCP ni nouveau bouton n’est ajouté dans ce lot.

## Paramètres

| Champ | Contrat |
| --- | --- |
| `guard` | Identité et révision du document ouvert au démarrage du lot |
| `steps` | Entre 1 et 256 appels `{ id, params }` du registre documentaire |
| `save` | Absent ou `null` : publication en mémoire uniquement ; sinon une sauvegarde finale explicite |

Les paramètres JSON sérialisés sont limités à 4 Mio, sans allouer une deuxième
copie pour les compter. Le schéma annonce les bornes du nombre d’étapes. Pour une
grosse image, utiliser un chemin explicite plutôt qu’une chaîne base64.
Les imports sont également limités à 64 Mio de ressources supplémentaires par
rapport à la session de départ, y compris les images ensuite supprimées du corps.
Les ressources déjà conservées pour l’historique du document restent disponibles.
Les limites ordinaires du document, des images et des modèles s’appliquent aussi.

Chaque entrée du [catalogue](commands.md) expose `batch_step`. Cette propriété est
explicite dans la déclaration centrale du registre : une nouvelle commande doit
indiquer si elle peut être utilisée comme étape.

Sont admises les éditions de contenu, structure, métadonnées, options, images,
signature et modèle, ainsi que la sélection/navigation et l’orthographe documentaire.
Les imports lisent uniquement leurs fichiers explicites, comme hors lot.
Sont exclus : création/ouverture/fermeture, sauvegardes, export, récupération,
annuler/rétablir, requêtes, lots imbriqués et actions globales de profil/préférences/
dictionnaire. Tous les appels sont décodés et leur admissibilité est vérifiée
avant d’exécuter une étape ou de lire un fichier importé.

## Cibles et gardes

`current` et `all` se résolvent sur le brouillon au moment de chaque étape.
Les sélections explicites, objets, occurrences et fautes gardés conservent leurs
règles ordinaires : identité, révision et frontières de zones sont vérifiées.
Aucune garde ni position n’est réécrite automatiquement.

Le brouillon commence avec la même garde que le document et avance comme une
suite d’appels individuels : chaque mutation réelle incrémente sa révision,
une sélection ou une opération sans changement ne l’incrémente pas. Les champs
`guard` des commandes qui les exigent restent obligatoires et doivent désigner
l’état du brouillon visé. Une cible de la révision initiale devenue périmée après
une étape fait échouer tout le lot. Ce premier contrat ne propose pas de références
aux résultats d’étapes intermédiaires.

Les révisions intermédiaires ne sont pas publiées. Le résultat final porte la
garde et la sélection publiées ; un lot modifiant le document avance sa révision
d’une unité. Un lot sans changement documentaire peut déplacer la sélection ou
changer le style de frappe, sans étape d’annulation ni suppression du rétablissement.

## Sauvegarde finale

```json
{"kind": "current"}
```

Exige un chemin déjà associé à la session et réutilise `document.save`.

```json
{"kind": "as", "path": "/chemin/courrier.typsto", "overwrite": false}
```

Réutilise `document.save_as` ; `overwrite` vaut `false` par défaut. Une destination
existante est refusée sans autorisation explicite. La publication sans écrasement
reste protégée si un fichier apparaît pendant la préparation.

La sauvegarde n’est pas une étape : elle intervient une seule fois, après toutes
les éditions et la préparation validée de leur transaction. Elle réutilise l’écriture
atomique existante et le filtrage des ressources inutilisées. Si elle échoue, le
brouillon n’est pas publié et le repère enregistré n’est pas changé. Après sa
réussite, il ne reste qu’un transfert en mémoire sans IO ni validation faillible.
L’export reste une opération distincte ; aucune atomicité multi-fichiers n’est promise.

## Exemple

Remplacer la garde illustrative par celle de `document.inspect`. La première
étape utilise la garde initiale, les suivantes des cibles symboliques.

```json
{
  "id": "document.batch",
  "params": {
    "guard": {"document_id": 7, "revision": 12},
    "steps": [
      {
        "id": "metadata.set",
        "params": {
          "guard": {"document_id": 7, "revision": 12},
          "field": "letter.subject",
          "value": "Demande de rendez-vous"
        }
      },
      {"id": "text.insert", "params": {"text": "Madame, Monsieur,"}},
      {"id": "text.bold", "params": {"target": {"kind": "all"}, "value": "on"}}
    ],
    "save": {"kind": "current"}
  }
}
```

`Outcome.value` donne le nombre d’étapes, le chemin sauvegardé ou `null` et la
présentation du modèle lorsqu’elle change. Les effets de rendu, de révélation de
sélection et de sauvegarde ne sont transmis qu’après réussite. L’interface conserve
le focus pour les lots ne modifiant que les métadonnées ou le modèle ; elle ne
nettoie la copie de récupération qu’après une sauvegarde réussie.

## Erreurs et vérifications

Une erreur d’étape garde son code et ajoute `reason: batch_step`, son index
`step` à partir de zéro, `command` et l’erreur structurée d’origine dans `error`.
L’interface affiche un numéro à partir de un et le message traduit. Les erreurs
initiales de garde/limites et celles de la sauvegarde finale restent des erreurs
ordinaires du moteur.

Les tests de `session/batches.rs`, `engine/tests/batch.rs` et
`app/tests/batch_engine.rs` couvrent la publication unique, les refus atomiques,
les frontières/cibles gardées, l’historique et le rétablissement, les ressources,
la configuration orthographique de lecture, les fichiers autonomes et la parité
des appels typés/JSON avec l’adaptateur graphique.

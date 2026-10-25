# Propositions MCP — préparation de 6.4b

Statut au 25 octobre 2026 : **parcours et protocole proposés, à valider avant leur
intégration**. D36 valide déjà l’accord humain sur un lot entier. Le socle Rust
d’acceptation atomique est implémenté et testé ; le MCP rattaché reste en lecture
seule. Ce document ne vaut pas décision D39 et n’annonce aucun nouvel outil livré.

## Présentation proposée

Une notification discrète annonce « Une proposition à relire », avec une action
« Voir la proposition ». Elle ne prend ni focus ni sélection et ne coupe pas la
frappe. Un seul lot peut attendre une décision pour une invitation.

L’ouverture volontaire passe dans un mode de **relecture à deux colonnes** :
« Actuel » et « Proposition ». Il remplace temporairement les panneaux rédaction
et aperçu, sans créer une troisième colonne. Le retour à la rédaction conserve
le lot en attente, la position de lecture et le contexte utilisateur.

L’onglet **Contenu** présente les blocs avec la représentation de l’éditeur, sans
code Typst. Les retraits sont biffés à gauche, les ajouts soulignés à droite, avec
des fonds discrets et une légende. Le gras, les liens, les changements de type de
bloc et les paramètres doivent aussi apparaître, même à texte identique. Le
comptage des changements est calculé par l’application, pas fourni par l’agent.

L’onglet **Informations** compare les métadonnées, options, modèle, signature et
autres ressources modifiées. Il n’apparaît que si nécessaire, avec le nombre de
changements dès le bandeau principal. Images et signatures doivent montrer les
vignettes avant/après ; un modèle est identifié par son nom et sa description.
Le code source d’un modèle personnalisé ne constitue pas une explication suffisante
de son changement : sa modification doit être signalée explicitement et son effet
visuel doit pouvoir être examiné dans l’aperçu candidat. L’intégration devra prévoir
une vue d’aperçu dans le panneau de proposition, réutilisant le rendu Typst.

Les seules décisions sont **Refuser le lot** et **Accepter le lot**. Revenir à la
rédaction ou appuyer sur Échap ne décide rien. Il n’y a ni acceptation partielle ni
édition directe du candidat dans cette première version. Aucune touche globale
n’accepte le lot ; Tab permet d’atteindre les boutons, puis Entrée/Espace d’activer
explicitement celui qui a le focus. Le focus initial va au bouton de retour, jamais
au bouton d’acceptation. Les bandeaux usuels gardent leur contrat clavier propre.

Si l’utilisateur modifie le document, l’acceptation devient indisponible. La
comparaison conservée est alors clairement étiquetée **Version de départ** /
**Proposition**, avec « Le document a changé. L’agent doit préparer une nouvelle
proposition. » Elle ne prétend pas afficher le texte actuellement publié. Même
revenir au même texte par Annuler ne réactive pas la proposition.

Une préédition IME ou un autre dialogue interdit temporairement soumission et
acceptation. L’utilisateur peut toujours abandonner la relecture. Terminer la
saisie ne déclenche aucune acceptation différée : une nouvelle action explicite
est nécessaire. Les brouillons de bandeaux et les cibles capturées doivent être
revalidés avant toute publication.

Après accord, retour à la rédaction avec sélection et passage visible reportés,
style de frappe conservé et une seule étape d’annulation. Une confirmation discrète
propose « Voir » pour naviguer volontairement vers une modification. Refuser
retire le lot sans modifier le document ni son historique. L’accord ne sauvegarde
pas et n’exporte pas : ces opérations restent dans le lot 6.5.

La maquette interactive est conservée dans `docs/mockups/mcp-review.html`. Elle
illustre contenu, informations, acceptation/refus et péremption ; elle simule
localement ces actions. Elle ne représente pas une fonction déjà livrée dans egui,
ni une comparaison structurée calculée par le moteur. Les interactions ont été
contrôlées dans un DOM simulé ; le contrôle visuel appartient à cette validation.

## Protocole proposé

Ajouter trois helpers à l’adaptateur MCP, documentés et signalés comme disponibles
uniquement en mode rattaché. Ils réutilisent les étapes du registre et les sélecteurs
existants ; ils ne dupliquent pas les commandes documentaires du cœur.

| Outil proposé | Entrée | Effet |
| --- | --- | --- |
| `proposals.submit` | Référence documentaire gardée, `request_id`, étapes du lot, résumé facultatif | Préparer un candidat privé, puis renvoyer immédiatement son identifiant et l’état `pending`. Aucun changement publié. |
| `proposals.get` | Identifiant opaque du lot | Lire son état dans cette invitation, y compris après une nouvelle révision documentaire. |
| `proposals.cancel` | Identifiant opaque du lot | Retirer un candidat encore en attente. Ne peut ni annuler une acceptation ni modifier un état terminal. |

Il n’existe **aucun outil `approve` / `accept` MCP**. L’agent ne peut pas simuler
un accord par une commande de texte, une métadonnée ou un paramètre du lot.
Les commandes d’édition ordinaires et `document.batch` restent refusées en appel
direct rattaché : il faut soumettre explicitement une proposition. Le mode autonome
conserve son exécution immédiate et sa politique de fichiers.

Les états du lot sont `pending`, `accepted`, `rejected`, `stale` ou `cancelled`.
Un état terminal ne revient jamais à `pending`. Une péremption exige un nouveau
lot, une nouvelle lecture et une nouvelle approbation ; il n’y a pas de fusion.
Révocation, fermeture/remplacement du document ou déconnexion détruisent les
candidats et l’invitation. Ils ne sont pas récupérables par une nouvelle connexion.

`submit` utilise la garde complète actuelle. `get` et `cancel` utilisent seulement
l’identifiant du lot dans la connexion authentifiée : une garde devenue ancienne
ne doit pas empêcher l’agent d’apprendre qu’un lot est accepté ou périmé. Ces appels
ne donnent aucun accès à un autre document ou à une autre invitation.

Le `request_id` rend une répétition de soumission identifiable si la réponse a été
perdue : même identifiant et même requête renvoient le même lot et son état, sans
nouveau candidat. Réutiliser l’identifiant pour un autre contenu échoue. La
déduplication précède le contrôle de fraîcheur d’une répétition identique, afin de
pouvoir retrouver une acceptation déjà effectuée. Un identifiant inconnu n’est
jamais interprété comme un accord ou une invitation à rejouer une édition.

Pour borner la mémoire : un seul candidat en attente et au plus 64 enregistrements
de soumission par invitation. Après un état terminal, libérer le candidat et ses
ressources ; conserver seulement les données compactes de statut/déduplication.
À la limite, refuser une nouvelle soumission explicitement, sans éviction qui
rendrait possible un rejeu. L’utilisateur peut recréer une invitation. Les budgets
actuels de 256 étapes, 4 Mio de JSON et de ressources documentaires restent appliqués.
Ces valeurs sont des bornes techniques initiales, pas une extension du périmètre.

Les opérations de fichiers, d’historique, de récupération et les lots imbriqués
restent interdits dans les étapes. Pas de sauvegarde finale d’un candidat. Les
imports gardent la politique des ressources autorisées ; aucun chemin local
arbitraire n’est acquis par le seul rattachement. Les déclarations d’autorisation
et les fiches de capacités doivent distinguer lecture, proposition et effets de
fichiers disponibles, plutôt que de prétendre que les commandes sont exécutées.

Le contexte agent demeure séparé. Les étapes d’un lot s’y enchaînent, mais sa
sélection et son style finaux ne deviennent courants qu’après acceptation.
Refuser/annuler laisse ce contexte inchangé, hors report des positions imposé par
la rédaction humaine. Le transfert d’une sélection utilisateur reste volontaire.
La navigation propre à l’agent pourra réutiliser `text.select` dans son contexte,
sans navigation graphique ; cette autorisation doit être déclarée séparément et
ne confère aucune édition directe. Les lectures continuent à voir le publié.

Exemple de forme proposée, avec une référence fournie par `document.inspect` :

```json
{
  "document": { "id": "invitation-document", "guard": { "document_id": 42, "revision": 7 } },
  "request_id": "correction-salutation-1",
  "summary": "Corriger la salutation",
  "steps": [
    { "id": "text.insert", "params": {
      "selector": { "select": "Cheres collègues" },
      "text": "Chères collègues"
    } }
  ]
}
```

Les noms et la forme ci-dessus restent une proposition, pas un schéma disponible.
La réponse devra dire explicitement `pending` et `published: false`. L’agent rend
la main à l’utilisateur ; il ne prétend pas que l’édition a eu lieu et ne lance pas
une boucle de consultation. Un prochain échange peut interroger `get`, qui
renverra notamment la garde publiée après acceptation. Aucun appel ne reste
suspendu indéfiniment en attendant une décision humaine.

## Ordre d’intégration après validation

1. Stockage borné des lots et transitions d’états dans l’adaptateur local ; tests
   de déduplication, fermeture, déconnexion et péremption.
2. Soumission isolée depuis MCP, disponibilité et documentation générées ; tester
   qu’aucun appel agent ne rejoint `accept_proposal`.
3. Relecture egui, comparaison de toutes les modifications, rendu du candidat,
   accord/refus explicites et protection IME/modalités.
4. Publication via le socle commun, report de sélection/ancre visible et retour
   du focus, annulation/rétablissement, notification et navigation volontaire.
5. Recette intégrée avec captures, puis 6.5 et 6.6. Le lot 6.4b reste ouvert jusque-là.

## Contrôles du socle livré

Les quatre nouveaux tests de `engine/tests/proposals/acceptance.rs` couvrent :
contexte utilisateur déplacé et style modifié pendant la relecture, contexte
agent final, autre participant reporté, Unicode, une seule annulation et son
rétablissement, refus des gardes anciennes/autres contextes/lecture seule,
lot limité au style sans perte du rétablissement, et publication groupée
image/signature/métadonnées sans toucher au fichier enregistré.

La suite Rust complète et Clippy avec `-D warnings` passent sur macOS. Les tests
de sockets locales nécessitent une exécution hors bac à sable. Le cœur conserve
ses dépendances sans GUI, Fluent, Tokio, MCP ni IPC. Aucun parcours graphique
d’acceptation n’est encore intégré ou annoncé comme testé.

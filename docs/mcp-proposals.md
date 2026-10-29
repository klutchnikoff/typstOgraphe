# Propositions MCP — préparation de 6.4b

> Contrat historique de la projection plate/egui, conservé avec ses recettes.
> Le contrat actif est décrit dans [Adaptateurs structurés](structured-adapters.md),
> [MCP](mcp.md) et [rattachement Tauri](mcp-local.md).

Statut au 25 octobre 2026 : **parcours et protocole livrés en 6.4b (D36/D39)**.
Le MCP rattaché peut préparer des lots privés ; seule une acceptation explicite dans
egui publie leurs modifications. Sauvegarde et export publiés sont livrés en
[6.5](mcp-files-attached.md), sous autorisations distinctes.

## Présentation

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
pas et n’exporte pas : ces opérations explicites suivent le contrat du lot 6.5.

La maquette validée est conservée dans `docs/mockups/mcp-review.html`. Le parcours
natif réutilise les polices, blocs, images et annotations de l’éditeur en lecture
seule. [similar](https://docs.rs/similar/latest/similar/) compare les mots Unicode
avec un temps de calcul borné ; le formatage, les liens, la structure, les
métadonnées et les ressources sont aussi comparés. Ce calcul ne sert jamais à
réancrer une commande périmée : la publication garde ses traces exactes du moteur.

L’aperçu candidat utilise un worker Typst distinct et les ressources du candidat,
sans installer son rendu dans le moteur publié. Les vignettes avant/après ont des
caches distincts, même si une ressource conserve son chemin. Les actions Voir,
Revenir, Accepter, Refuser et Voir le passage sont dans le registre graphique,
sans raccourci d’acceptation par défaut.

## Protocole rattaché

Trois helpers de l’adaptateur MCP sont documentés et disponibles
uniquement en mode rattaché. Ils réutilisent les étapes du registre et les sélecteurs
existants ; ils ne dupliquent pas les commandes documentaires du cœur.

| Outil | Entrée | Effet |
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
La navigation directe `text.select` réutilise uniquement son contexte, sans
navigation graphique ; `permissions.agent_navigation_enabled` déclare cette
autorisation, qui ne confère aucune édition directe. Une interruption ou un lot
en attente bloque cette navigation. La commande graphique **Confier la sélection
à l’agent** assure le transfert explicite (menu Édition et palette). Les lectures continuent à voir le publié.

Exemple de soumission, en remplaçant la référence par celle fournie par `document.inspect` :

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

Les schémas de `tools/list` font autorité. La réponse indique explicitement
`pending` et `published: false`. L’agent rend
la main à l’utilisateur ; il ne prétend pas que l’édition a eu lieu et ne lance pas
une boucle de consultation. Un prochain échange peut interroger `get`, qui
renvoie notamment la garde publiée après acceptation. Aucun appel ne reste
suspendu indéfiniment en attendant une décision humaine.

## Intégration livrée

Le stockage et les transitions sont dans l’adaptateur MCP ; le cœur reste sans
transport. `LocalSession::poll_with_input` reçoit l’état d’interruption natif et
traite les requêtes après publication de la saisie. `LocalSession::decide` est une
entrée Rust réservée à la décision humaine, absente du dispatch et du catalogue MCP.

La sélection et l’ancre visible sont reportées à l’acceptation. La restauration
utilise les réglages publics egui pour un défilement immédiat, sans animation qui
pourrait être annulée par un reliquat de molette. Le focus et le style utilisateur
sont conservés. Une publication de vue identique ne réétiquette pas un résultat
Agent comme User ; installer son rendu ne provoque donc pas de suivi implicite.

« Voir le passage modifié » emploie une cible gardée distincte de la sélection,
révèle les deux panneaux et la repère pendant deux secondes. Un mouvement de
sélection ou une nouvelle publication retire ce repérage. Une cible périmée n’est
pas réancrée ; une nouvelle identité documentaire efface les états d’assistance.

L’enregistrement/export MCP rattaché du lot 6.5 porte uniquement sur le publié,
sous permissions propres à l’invitation. La navigation agent directe et le
transfert graphique volontaire sont complétés en 6.6 ; les sélecteurs des étapes
permettent aussi de cibler les lots.

## Contrôles du socle livré

Les quatre nouveaux tests de `engine/tests/proposals/acceptance.rs` couvrent :
contexte utilisateur déplacé et style modifié pendant la relecture, contexte
agent final, autre participant reporté, Unicode, une seule annulation et son
rétablissement, refus des gardes anciennes/autres contextes/lecture seule,
lot limité au style sans perte du rétablissement, et publication groupée
image/signature/métadonnées sans toucher au fichier enregistré.

Les tests supplémentaires couvrent les quatre scénarios du stockage MCP
(déduplication, états, budgets, refus et ressources), les outils sur une connexion
rmcp locale réelle, les sept parcours egui (accord/refus, interruptions, focus,
péremption, révocation, passage visible et rendu sans suivi), et deux comparaisons
Unicode/formatage/liens. La [recette agent LLM de 6.6](mcp-attached-acceptance.md)
confirme le parcours natif, les refus, les accords et les fichiers.

Captures natives conservées dans `output/phase6-4b/` : contenu clair/sombre,
informations avec vignette, aperçu candidat et proposition périmée. Reproduction :

```sh
cargo build --locked -p typstographe-egui
mkdir -p output/phase6-4b
target/debug/typstographe-egui --capture-widgets output/phase6-4b/review-light.png review
target/debug/typstographe-egui --capture-widgets output/phase6-4b/review-dark.png review-dark
target/debug/typstographe-egui --capture-widgets output/phase6-4b/information.png review-information
target/debug/typstographe-egui --capture-widgets output/phase6-4b/preview.png review-preview
target/debug/typstographe-egui --capture-widgets output/phase6-4b/stale.png review-stale
```

Les 527 tests Rust, les deux doctests, les 14 tests Python, fmt et Clippy avec
`-D warnings` passent sur macOS ; la CI vérifie les trois systèmes. Les tests de sockets locales
nécessitent une exécution hors bac à sable. Le cœur conserve ses dépendances sans
GUI, Fluent, Tokio, MCP ni IPC.

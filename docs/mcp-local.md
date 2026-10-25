# MCP local — début de phase 6

Les choix D34/D35/D36 et les lots sont consignés dans
[ROADMAP-ARCHITECTURE.md](../ROADMAP-ARCHITECTURE.md#phase-6--assistance-à-la-composition-dans-lapplication-d14).
Le rattachement propose désormais les **lots soumis à approbation humaine** de
6.4b (D39), avec les contextes séparés et positions de 6.3. Le [contrat des
propositions](mcp-proposals.md) précise `proposals.submit/get/cancel`, la comparaison
native, l’accord/refus et la péremption. Les éditions directes restent refusées ;
l’enregistrement et l’export publiés sont livrés en [6.5](mcp-files-attached.md),
sous droits facultatifs distincts propres à l’invitation.

## Décision pour les futures modifications rattachées (D36)

Validé le 25 octobre 2026 : l'agent soumettra un **lot proposé**, préparé et validé
techniquement sur un brouillon isolé. Le document publié restera inchangé jusqu'à
l'approbation explicite de l'utilisateur. L'agent ne pourra pas approuver son lot.

L'utilisateur acceptera ou refusera **le lot entier**, sans acceptation partielle
pour cette première version. Accepter publiera une seule transaction annulable ;
refuser ne changera ni le document ni son historique documentaire. À l'acceptation,
l'identité et la révision seront vérifiées à nouveau : une proposition périmée
devra être recalculée puis soumise à une nouvelle approbation, sans fusion automatique.
La sauvegarde et l'export resteront explicites et utiliseront uniquement le document
publié, jamais le brouillon d'une proposition en attente.

La présentation et le protocole sont validés en D39 et livrés en 6.4b : contenu
Actuel / Proposition, informations modifiées, aperçu candidat isolé et décision
explicite dans egui. Le bouton « Voir » après publication reste une navigation
volontaire qui conserve la sélection utilisateur. Aucun outil agent ne peut
approuver un lot. Le fonctionnement du MCP autonome reste inchangé.

## Parcours

1. Ouvrir ou créer un document dans typstOgraphe ; il peut rester non enregistré.
2. Choisir **Édition → Assistance par un agent…** (également dans la palette).
3. Créer l'invitation pour ce document, puis copier la configuration MCP.
4. Ajouter cette configuration au client MCP. `typstographe-cli` doit être dans son
   `PATH`, ou remplacer `command` par le chemin absolu du binaire construit.
5. L'agent commence par `document.inspect`, découvre les capacités et leurs
   autorisations, puis lit le document ou son rendu.
6. Soumettre un lot avec `proposals.submit` : il reste privé jusqu’à la décision
   dans l’application. L’agent rend la main ; `proposals.get` peut consulter ensuite
   le statut sans référence documentaire courante.
7. Le panneau et la barre d'état indiquent l'invitation prête ou l'agent connecté.
   Le bouton de révocation coupe la connexion et supprime l'invitation.

Avant la création, le panneau permet aussi d’autoriser la sauvegarde du fichier
courant et de choisir une racine d’export. Ces droits sont désactivés initialement ;
ils ne donnent pas accès aux autres documents ni aux imports locaux. Leur détail
et la revalidation des exports différés sont dans le [contrat de fichiers](mcp-files-attached.md).

La commande générée équivaut à :

```sh
typstographe-cli mcp --attach /chemin/prive/invitation.json
```

`--attach` est incompatible avec `--read-only`, `--read-root` et `--write-root` :
l'autorisation vient de l'application et concerne son document. Le jeton secret
reste dans le fichier privé, pas dans les arguments ni dans la configuration MCP.
La configuration contient son chemin et doit rester locale. Chaque invitation
accepte **une seule connexion authentifiée**, sans réutilisation après déconnexion.
Créer une nouvelle invitation pour reconnecter l'agent.

Le mode [MCP autonome](mcp.md) reste inchangé et se lance sans `--attach`.

## Frontières

```text
client MCP ← stdio → CLI passerelle ← socket locale → rmcp dans l'application
                                                        ↓ file bornée + réveil
                                                 frame egui après publication
                                                        ↓ emprunt temporaire
                                                 Engine du document ouvert
```

La crate `typstographe-mcp`, extraite de la CLI en 6.1, contient l'adaptateur,
les schémas, les ressources, la documentation et les sélecteurs partagés. Les deux
binaires en dépendent ; aucune dépendance application → CLI n'est introduite.
Le cœur reste sans egui, Fluent, Tokio, rmcp ni interprocess. Les types de sélection
communs conservent leurs annotations clap ; ils restent dans cette couche
adaptateur, pas dans le cœur.

Le mode autonome possède son moteur. Le mode rattaché l'**emprunte pendant le
traitement de la requête sur le thread de l'interface**. Il ne rouvre pas le fichier,
ne reconstruit pas une seconde session et ne contourne pas son verrou. Le thread
Tokio s'occupe du protocole et échange seulement des requêtes/réponses. La
passerelle transfère le flux MCP après authentification ; aucun second catalogue
ni protocole documentaire n'est créé.

`LocalSession::poll` est appelé après `sync_published_selection`, donc après les
éditions natives et les actions de la frame. Les lectures voient le texte publié,
y compris pendant une préédition IME ou un dialogue. La préédition reste locale.
Les lectures peuvent alimenter des caches dérivés, sans modifier sélection,
contenu, style ni historique. Les calculs synchrones du cœur restent exécutés à
ce point ; leur coût et la préparation asynchrone des lectures coûteuses devront
être traités avant la recette de composition simultanée (6.6).

## Autorisation et durée de vie

- Aucun canal n'est créé au lancement de l'application.
- L'invitation porte une adresse aléatoire et un jeton temporaire de deux UUID v4.
  L'authentification, de taille fixe, précède le traitement de tout message MCP.
- macOS/Linux : socket dans un répertoire temporaire court, créé en mode `0700` ;
  invitation `0600`. Le chemin court respecte la limite des sockets Unix macOS.
- Windows : tube nommé local aléatoire avec DACL protégée accordant l'accès au
  propriétaire (`OW`). L'invitation réside dans le répertoire temporaire utilisateur.
- La référence du rattachement est opaque. Son identité documentaire reste fixe ;
  les révisions peuvent évoluer avec la frappe de l'utilisateur. Les anciennes
  références sont refusées ; relire `document.inspect` pour se remettre à jour.
- Un document fermé ou remplacé révoque immédiatement son invitation. Le nouveau
  document n'hérite d'aucune autorisation. Enregistrer sous un autre nom ne change
  pas l'identité du document en mémoire.
- Révocation, fermeture de l'application, EOF ou coupure terminent la connexion ;
  aucune reconnexion ni répétition automatique n'est effectuée. Après un incident,
  un éventuel fichier temporaire résiduel n'est pas une autorisation réutilisée.
- L'authentification incomplète expire après trois secondes. La passerelle borne
  sa connexion à cinq secondes. File de requêtes : 16 ; au plus 16 traitées par
  frame. rmcp borne les messages MCP entrants.

## Catalogue et restrictions de ce lot

Les 67 outils communs et cinq ressources sont partagés avec le serveur autonome ;
trois helpers de proposition portent le catalogue rattaché à 70 outils. Cette liste
reste stable ; `commands.list`, `capabilities.describe` et `permissions` précisent
les autorisations propres au rattachement. `session_mode` vaut `attached`.

Les éditions directes, sélection, annulation, lots, création, ouverture, fermeture
et Enregistrer sous restent refusés **avant résolution et exécution**. Les étapes
de propositions suivent leur contrat d’approbation. `document.save` et
`document.export` sont les deux exceptions explicites, sous droits de l’invitation.
Les requêtes documentaires, la découverte et le PNG de page restent disponibles.
Les informations sur le document et le rendu proviennent du même moteur que l’interface.

## Vérifications

Les tests du workspace couvrent la connexion locale avec le SDK officiel rmcp,
les ressources et schémas partagés, une référence périmée après frappe utilisateur,
le refus des commandes, un jeton invalide, une seconde connexion, la consommation
de l'invitation, ses permissions Unix et son retrait à la révocation/remplacement.

Un test d'intégration lance réellement `typstographe-cli mcp --attach` en
sous-processus stdio, rejoint un propriétaire de moteur en mémoire, lui fait
publier une édition, vérifie sa lecture par l'agent et refuse les écritures. Les
tests egui couvrent l'action publique, l'invitation, sa copie/révocation et la
fermeture/remplacement du document.

Les tests sont dans la suite existante des trois systèmes en CI ; leur exécution
locale macOS ne prouve pas à elle seule le fonctionnement des tubes Windows.

### Recette locale des 24 et 25 octobre 2026

Code vérifié : `c83e5b2` (les modifications restantes concernent les guides et le
suivi). Les tests d'arrêt de la passerelle couvrent aussi une révocation pendant
que le client conserve stdin ouvert : le runtime de cette passerelle CLI utilise
`shutdown_background` pour ne pas attendre la lecture bloquante non annulable de
Tokio stdin. La connexion se ferme sans nouvelle entrée du client.

- `cargo test --locked --workspace --offline` : **467 tests réussis**, aucun
  échec ni test ignoré ; journal dans `output/phase6-local/tests.log`.
- Clippy workspace/toutes cibles avec `-D warnings`, fmt, frontières du cœur et
  limites de taille : réussis. Les 14 tests Python des garde-fous passent.
- Les huit captures natives sans assistance active correspondent exactement aux
  références conservées : `output/baseline/run-3orzok94`.
- Panneau d'autorisation inspecté visuellement :
  `output/phase6-local/assistance.png`, produit avec
  `target/debug/typstographe --capture-assistance output/phase6-local/assistance.png`.
  Ce diagnostic n'ouvre aucun canal ni invitation.
- Journaux de construction, Clippy, captures et tests conservés dans
  `output/phase6-local/` (ignoré par Git).

La validation locale a été faite sous macOS ; la CI des trois systèmes reste
nécessaire pour confirmer Linux et les tubes Windows. À l'issue de 6.1/6.2, il
restait les contextes séparés, le report des positions, les éditions,
sauvegardes/exports rattachés et la recette de composition. Le lot suivant est
décrit ci-dessous.

### Lot 6.3 — contextes et préparation des propositions

Livré le 25 octobre 2026 ; [contrat détaillé](editing-contexts.md).

- Une invitation possède maintenant son contexte d'édition pour toute sa durée,
  indépendant de la sélection et du style utilisateur. Les lectures `Current`,
  `inspect`, le catalogue contextualisé et la ressource du document l'utilisent.
  Le moteur est toujours emprunté ; aucun fichier n'est rouvert.
- Les positions conservées sont reportées à partir des opérations réellement
  publiées, y compris Unicode, normalisations, remplacements multiples, lots,
  annulation et rétablissement. Les anciennes références externes restent refusées.
- `ViewSync` reporte sélection et ancre du passage visible sans prendre le focus
  ni créer une demande de navigation. egui capture cette ancre hors préédition.
- Le moteur prépare un candidat de proposition isolé, à partir du contexte agent.
  Aucun changement du document, de ses contextes, de l'historique, de la révision
  ou du fichier. Images/signature restent privées au candidat ; aucun verrou de
  fichier n'est retenu. La vérification de fraîcheur ne vaut pas accord humain.
- Aucun outil de proposition/approbation exposé, aucune publication de candidat.
  Les 67 outils et cinq ressources gardent leurs schémas ; toutes les commandes
  rattachées restent refusées. Le serveur autonome conserve son fonctionnement.

La présentation des différences, l'acceptation/refus du lot entier, la conservation
effective du focus et du passage visible lors de l'acceptation et le filtrage des
interruptions seront traités en 6.4. Les propositions périmées devront être
recalculées et présentées à nouveau, sans fusion automatique.

#### Recette du 25 octobre 2026

Code : `02dff51` (positions), `17db142` (contextes/candidats), `f80261d`
(MCP/vues), `2b19c74` (diagnostic de capture).

- `cargo test --locked --workspace --offline` : **485 tests réussis**, aucun
  échec ni test ignoré, dont les deux exemples Rustdoc du contrat d'interface.
- Clippy workspace/toutes cibles avec `-D warnings`, formatage, frontières du
  cœur et limites de 800 lignes : réussis. Les 14 tests Python passent.
- Construction des deux binaires réussie. Les huit captures natives correspondent
  exactement aux références préservées : `output/baseline/run-jvk0u8_4`.
- Les tests ajoutés couvrent report Unicode et textes répétés, remplacements
  multiples, normalisation, saisie native et brouillons refusés, historique groupé,
  lots, contextes indépendants, disponibilité contextualisée, péremption,
  propositions sans effets (y compris ressources et verrou), ancre visible egui
  et hors IME, lectures MCP dans leur contexte persistant.

La première capture orthographique a révélé un ancien diagnostic qui soumettait
une projection indépendante comme brouillon natif. Le contrôle des traces le
refusait ; la capture attendait ensuite des fautes non chargées. Le diagnostic
installe désormais sa fixture par le chargeur documentaire, avec un test de
régression. Un second essai avait uniquement le cadre de focus de la recherche
différent ; la série finale, sans autre lancement pendant les captures, est
identique sur les huit scénarios. Aucune référence n'a été remplacée.

Journaux conservés dans `output/phase6-3/` (ignoré par Git) : `tests.log`,
`clippy.log`, `build.log`, `guards-tests.log` et les essais intermédiaires.
Les captures intermédiaires restent dans `output/baseline/run-e77nqmip` et
`run-kf018kr8`. Vérification locale macOS ; Linux et Windows restent à confirmer
par la CI. Le lot 6.3 ne clôt pas la phase 6 et n'active aucune édition rattachée.

## Livraison 6.4b

Le catalogue autonome conserve 67 outils ; le catalogue rattaché en compte 70,
avec les trois helpers de proposition. Les permissions et fiches indiquent
`proposal_supported` sans autoriser les appels d’édition directs. Aucun chemin
local arbitraire, préférence globale, opération de fichier ou historique n’est
admis dans un lot. `proposals.get/cancel` sont limités à leur invitation.

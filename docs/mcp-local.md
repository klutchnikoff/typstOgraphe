# MCP local — début de phase 6

Les choix D34/D35 et les lots sont consignés dans
[ROADMAP-ARCHITECTURE.md](../ROADMAP-ARCHITECTURE.md#phase-6--assistance-à-la-composition-dans-lapplication-d14).
Ce premier lot livre **le rattachement en lecture seule**. L'édition partagée,
l'enregistrement et l'export rattachés attendent les contextes séparés et les
protections D32/D33 ; ils restent prévus en 6.3 à 6.5.

## Parcours

1. Ouvrir ou créer un document dans typstOgraphe ; il peut rester non enregistré.
2. Choisir **Édition → Assistance par un agent…** (également dans la palette).
3. Créer l'invitation pour ce document, puis copier la configuration MCP.
4. Ajouter cette configuration au client MCP. `typstographe-cli` doit être dans son
   `PATH`, ou remplacer `command` par le chemin absolu du binaire construit.
5. L'agent commence par `document.inspect`, découvre les capacités et leurs
   autorisations, puis lit le document ou son rendu.
6. Le panneau et la barre d'état indiquent l'invitation prête ou l'agent connecté.
   Le bouton de révocation coupe la connexion et supprime l'invitation.

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

Les 67 outils et cinq ressources sont partagés avec le serveur autonome. La liste
reste stable ; `commands.list`, `capabilities.describe` et `permissions` précisent
les autorisations propres au rattachement. `session_mode` vaut `attached`.

Toutes les commandes sont refusées **avant résolution et exécution**, notamment
sélection, mise en forme, annulation, lots, création, ouverture, fermeture,
enregistrement et export. Aucun chemin documentaire n'est autorisé. Les requêtes
documentaires, la découverte et le PNG de page restent disponibles. Les informations
sur le document et le rendu proviennent du même moteur que l'interface.

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
nécessaire pour confirmer Linux et les tubes Windows. Le premier lot n'est pas
une clôture de phase 6 : les contextes séparés, le report des positions, les
éditions, sauvegardes/exports rattachés et la recette de composition restent à faire.

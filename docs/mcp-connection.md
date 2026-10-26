# Raccordement MCP fixe à typstOgraphe

Choix validé le 25 octobre 2026 : **configuration fixe, reconnexion explicite**.
L’autorisation reste temporaire et liée à un document. La configuration de Codex
ou d’un autre client MCP n’a plus à contenir le chemin de chaque invitation.

> Ce guide décrit le raccordement de la référence egui. Tauri est désormais
> l’application principale ; ses commandes sont dans le [guide courant](../app/README.md).

## Configuration, une seule fois

Construire la CLI et l’application (`cargo build --locked --offline -p typstographe-cli -p typstographe-egui`).
Dans le client MCP, enregistrer un serveur **STDIO** :

- programme : chemin absolu de `typstographe-cli` (`typstographe-cli.exe` sous Windows) ;
- arguments : `mcp`, `--attach-active`.

Dans le prototype, le programme est par exemple
`/Users/nicolas/Documents/git/typstOgraphe/target/debug/typstographe-cli`.
**Édition → Assistance par un agent… → Copier la configuration MCP** fournit le
JSON correspondant. Le programme voisin de l’application est utilisé s’il existe ;
sinon `typstographe-cli` doit être dans le `PATH` du client, ou remplacé une fois
par son chemin absolu. Le chemin du programme doit rester valable après déplacement
du prototype ; le packaging de l’application distribuée reste un chantier distinct.

La configuration copiée contient aussi la variable `TYPSTOGRAPHE_MCP_DIRECTORY`,
fixée au répertoire de découverte de l’application. Sa valeur reste identique
d’une invitation à l’autre. Pour l’emplacement par défaut du même utilisateur,
les seuls programme et arguments suffisent. Cette variable est notamment utile
aux tests et diagnostics isolés ; l’UI et le client doivent alors utiliser le même
répertoire absolu.

## À chaque séance

1. Ouvrir le courrier dans typstOgraphe.
2. Ouvrir **Édition → Assistance par un agent…**, choisir les éventuels droits de
   sauvegarde/export, puis créer l’invitation pour ce document.
3. Démarrer ou **reconnecter** le serveur MCP dans le client, sans modifier sa configuration.
4. Travailler avec l’agent et accepter/refuser les propositions dans typstOgraphe.

Le client doit être lancé/reconnecté après création de l’invitation. S’il démarre
avant, il reçoit un message explicite : aucune invitation disponible. Il ne reste
pas en attente et ne se reconnecte pas automatiquement. Après révocation,
fermeture/remplacement du document, EOF ou déconnexion, créer une nouvelle
invitation puis reconnecter explicitement le serveur.

La connexion établie reste attachée à ce document. Créer une autre invitation
ou changer de fenêtre ne redirige pas cet agent. Si plusieurs invitations non
connectées sont disponibles, le raccordement fixe refuse de choisir ; révoquer
les autres, ou utiliser le raccordement explicite `mcp --attach INVITATION` dans
un contexte où le choix du document est volontairement géré séparément.

## Contrat technique

- Le répertoire partagé est `~/.typstographe/mcp`, sous le dossier personnel résolu
  par `home`, indépendamment des préférences egui, du projet et du fichier `.typsto`.
  Il n’est créé qu’à l’autorisation. L’override explicite doit être absolu.
- Chaque publication a un nom UUID distinct, un compagnon de verrou OS et un
  petit JSON atomique contenant seulement version et chemin d’invitation. **Aucun
  jeton, adresse de socket, chemin documentaire ou donnée de profil dans ce JSON.**
- Le répertoire est privé (`0700`) et les fichiers `0600` sur Unix. Les liens
  symboliques sur le répertoire final, le record ou son verrou sont refusés.
  Sous Windows, ils se trouvent dans le dossier personnel ; le canal conserve sa
  DACL propriétaire et son authentification existantes.
- Le verrou, conservé par la session, indique que la publication est vivante.
  Un résidu de crash sans verrou est ignoré, même si son fichier d’invitation existe
  encore. La découverte est bornée à 1 024 entrées et 4 Kio par record.
- Le propriétaire retire sa publication dès qu’il observe l’authentification réussie,
  ou lors de la révocation/fermeture/remplacement/fin de session. Chaque publication
  ne retire que ses propres fichiers. Un résidu de crash ne confère aucun droit.
- La passerelle résout une invitation unique une seule fois, puis réutilise le
  handshake et le transfert stdio existants. Zéro ou plusieurs invitations : erreur,
  sans repli autonome. Si l’invitation disparaît ou est consommée entre découverte
  et connexion, l’essai échoue ; aucun autre document n’est recherché automatiquement.
- `--attach-active` exclut `--attach`, `--read-only`, `--read-root` et `--write-root`.
  Aucun nouvel outil MCP ni changement de catalogue, de droits, d’approbation ou
  de source documentaire. Le cœur n’est pas modifié.

La publication et les écritures atomiques utilisent `atomic-write-file` et les
verrous de `std::fs::File`. `home` était déjà une dépendance d’eframe ; les deux
crates sont réutilisées dans l’adaptateur MCP, sans nouvelle version téléchargée.

## Vérifications

Les tests couvrent publications successives, résidu de crash, retrait indépendant,
ambiguïté, permissions Unix, liens et records invalides, puis l’authentification et
le retrait de la découverte. Un véritable client SDK rmcp rejoint deux invitations
successives **avec les mêmes arguments et environnement** : une nouvelle invitation
ne change pas la connexion déjà établie. Les refus CLI n’écrivent rien sur stdout
et ne démarrent jamais le MCP autonome. Les tests egui vérifient configuration
identique, révocation et échec de publication sans changement documentaire.

Le pilote natif opt-in utilise son propre répertoire de découverte. La connexion
Inspector par `--attach-active`, la lecture du document et les captures clair/sombre
sont conservées dans `output/mcp-fixed-connection/`, hors Git. Aucune configuration
Codex personnelle ou préférence réelle n’est modifiée par les tests.

Validation locale : **563 tests Rust, deux doctests et 14 tests Python réussis** ;
formatage, Clippy (y compris le pilote opt-in), frontières du cœur et tailles des
modules contrôlés. Les captures ne constituent pas une mesure de performance :
la capture sombre a dépassé le délai du contrôleur pendant les contrôles parallèles,
puis s’est achevée et a été vérifiée. La révocation a renvoyé `Connection closed`
à l’inspecteur. Aucun nouvel essai graphique Windows/Linux n’est revendiqué.

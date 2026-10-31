# MCP rattaché à Tauri

L’application et `typstographe-cli mcp --attach INVITATION` partagent une seule
passerelle MCP. Le fichier d’invitation est celui produit dans **Assistance à la
rédaction** : adresse TCP de bouclage et jeton privé. Aucun projet n’est rouvert et
aucun verrou n’est repris par la passerelle. Seul l’état confirmé du propriétaire
Rust est exposé ; aucun instantané JavaScript différé ne sert de source de vérité.

Le binaire Tauri accepte aussi `--mcp --invitation INVITATION`. Pour une
configuration fixe, la CLI propose `mcp --attach-active` : elle lit l’invitation
Tauri au démarrage. Après une nouvelle invitation, **reconnecter explicitement**
le serveur dans le client. Une connexion déjà lancée reste liée à son invitation.
Une invitation absente/invalide n’ouvre jamais une session autonome à sa place.
Les anciens chemins de découverte propres à egui ne sont plus utilisés.

## Lecture et propositions

Le [catalogue commun](mcp.md) décrit les opérations documentaires. Le catalogue
rattaché n’expose directement que ses requêtes compatibles et `context.set` ;
`commands.list` distingue `attached_direct` et `attached_proposal`. La géométrie
Typst et `commands.available` restent réservées au serveur autonome pour le moment.
Le contexte de l’agent est distinct de chaque fenêtre et ne déplace aucun focus.

Pour éditer, l’agent appelle `proposals.submit` avec la dernière référence
`document`, un `request_id` unique, un résumé et des étapes structurées. Les
sélecteurs et les cibles passent par le **même résolveur et le même moteur** que la
CLI et l’éditeur. Le lot est préparé sans modifier le document ni l’historique.
Maximum 128 étapes, requête transport de 5 Mio, aperçu de relecture de 8 Mio.

Le périmètre rattaché reste le **corps**, ses cellules et annotations. L’en-tête,
les ressources, les modèles, les signatures et les fichiers restent en lecture.
Leur édition demanderait une relecture qui les montre explicitement. Les anciens
contrats d’[autorisations de fichiers](mcp-files-attached.md) décrivent egui ; ils
ne constituent pas des droits disponibles dans Tauri.

Seul l’utilisateur peut accepter ou refuser les groupes applicables dans le comparateur commun. Les changements liés restent atomiques. Aucun outil MCP ne peut
l’approuver. À l’acceptation, Rust revérifie identité, révision, IME et état du lot,
revalide son rendu et publie les commandes enregistrées. Le navigateur reçoit le
delta Yrs confirmé ; il ne reconstruit plus une modification indépendante. Une
seule contribution est ajoutée à l’historique de la fenêtre qui accepte ; les
identités et sélections des zones non touchées sont conservées.

Un changement humain périme la proposition, sans report automatique. Un même
`request_id` avec le même contenu relit son reçu ; avec un autre contenu il est
refusé. `proposals.get/cancel` consulte/annule, sans publier. Après soumission,
l’agent rend la main à l’utilisateur. Les saisies IME et dialogues bloquent la
préparation et l’acceptation. Révocation et remplacement du document rendent
l’invitation inutilisable ; reconnecter après autorisation humaine.

## Vérification

Les tests couvrent le client officiel rmcp, les schémas effectifs, les sockets de
bouclage, la reconnexion explicite et la préparation/acceptation native. La recette
Rust ↔ Yjs vérifie un lot texte/tableau, la conservation des nœuds existants et une
seule étape d’annulation/rétablissement. Les essais d’agent sans skill historiques
sont conservés dans `experiments/` ; ce lot ajoute une recette SDK de découverte
jusqu’à l’export hors ligne, pas un nouvel essai avec un LLM externe.

Les [recettes egui historiques](archive/flat-mcp-local.md) restent archivées.
Aucune capture n’est nécessaire pour cette migration du contrat.

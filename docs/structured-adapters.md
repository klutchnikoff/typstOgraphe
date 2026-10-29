# Adaptateurs structurés — A.2, 29 octobre 2026

## Une seule édition documentaire

`content::api` est la façade sans interface de `EditingSession` : catalogue,
requêtes, création/ouverture, conservation et export. La CLI et le serveur MCP
s’y raccordent ; leurs sources n’utilisent plus le moteur de projection plate,
`StoredDocument`, le parseur de source ou un interprète indépendant des widgets.
Les restrictions du catalogue de capacités par type sont contrôlées dans le cœur,
y compris pour un nœud générique ou une archive : un P.-S. ne peut pas contourner
le type Article en utilisant `block.insert`.

Le moteur historique reste dans le cœur avec ses tests et ses catalogues archivés,
mais il ne sert plus de chemin d’exécution à ces adaptateurs.

Tauri conserve son propriétaire vivant et ses contextes par fenêtre. Le MCP
rattaché prépare ses appels dans ce même propriétaire. Les sélecteurs sont résolus
sur le brouillon isolé, puis passent aux mêmes `commands::Command` typées. La
validation porte sur l’état final : aucun canal ne publie une étape intermédiaire.

Les commandes textuelles reçoivent une zone et une plage UTF-16 ; les commandes
structurelles reçoivent une racine et un chemin. Les enregistrements d’en-tête ont
des identifiants stables. Les chemins doivent être relus après une modification.
Une sélection privée n’est ni une édition ni une cible documentaire persistante.

## Découverte et différences d’adaptateurs

Les schémas des éditions proviennent de l’énumération exécutée. Les signatures des
requêtes et du cycle de vie sont regroupées dans la façade. Les validateurs des
paramètres sont mis en cache, hors chemin de frappe locale. Le serveur rattaché
amorce cette validation avant de prendre le verrou du document vivant. Les schémas MCP
ajoutent l’enveloppe de référence ; ils sont vérifiés aussi à l’exécution.

Les capacités sémantiques fournissent des exemples d’étapes : créer une liste ou
une citation en bloc ne demande pas un second interprète par adaptateur. Une
nouvelle capacité doit déclarer son nœud, sa validation et ses exemples avant de
pouvoir être annoncée comme exécutable. Les noms `search.find`, `edit.undo/redo`
et `render.selection_boxes` sont conservés ; leurs cibles sont désormais typées.
Les anciennes positions et références de projection ne sont pas simulées.

| Adaptateur | Édition et contexte | Conservation |
| --- | --- | --- |
| Tauri | Propriétaire commun, contexte privé par fenêtre, frappe confirmée asynchroniquement | Parcours graphique existant |
| CLI | Même propriétaire ; `run` explicite et scripts atomiques ; aperçu par défaut | Verrou avant lecture pour `--save`, copie indépendante pour `--output` |
| MCP autonome | Même façade, références document/révision strictes, contexte privé | Racines lecture/écriture explicites ; sauvegarde/export séparés |
| MCP rattaché | Lecture confirmée ; contexte agent privé ; propositions du corps, cellules et annotations | Acceptation humaine uniquement ; aucune permission de fichier |

En mode rattaché, la préparation ne publie rien. À l’acceptation, le propriétaire
rejoue le lot gardé et produit le delta Yrs confirmé. Le navigateur ne calcule plus
un autre delta à partir de l’aperçu. Le cache d’instantanés transmis au MCP est
supprimé, ainsi que sa copie et sa validation supplémentaire dans le parcours
d’aperçu ; ses lectures utilisent directement le propriétaire confirmé. L’historique de la fenêtre qui accepte reçoit
une seule contribution. Les cibles d’en-tête restent refusées, même si un sélecteur
textuel les trouve : l’actuelle relecture ne présente que le corps.

## Fichiers et ressources

Les archives structurées `.typsto` conservent l’état Yjs et les ressources
nécessaires à leur autonomie. Les ressources admises en mémoire restent
accessibles pour annuler ; seules celles utilisées sont conservées dans l’archive
et l’export. Les modèles sont des sources Typst intégrées au projet.

Une ouverture éditable réserve le verrou avant lecture et vérifie l’empreinte
après chargement. Une lecture seule ne prend pas le verrou d’un écrivain. Une
copie ne remplace jamais implicitement une cible. Un lot peut sauvegarder son
candidat en dernière action ; un échec conserve l’état, les contextes et
l’historique confirmés. L’export multi-fichiers reste une opération séparée.

Les chemins d’import implicites de l’ancien contrat disparaissent : `resource.add`
reçoit les octets validés (base64 sur le fil), `document.set_model` le source
Typst. Seuls les chemins documentaires d’ouverture, sauvegarde et export sont
résolus par les adaptateurs. Les préférences personnelles ne sont pas implicites.

## Recettes et limites

Les tests vérifient les archives structurées, les brouillons incomplets, les
références d’auteurs/affiliations, zones riches, cellules, Unicode et frontières,
styles, ressources/signatures, état modifié après admission d’une ressource,
orthographe selon la langue de chaque résumé, sélecteurs évolutifs, lots refusés, erreurs
localisées, verrous, modifications externes et export hors ligne.

Le client officiel rmcp vérifie le protocole, la découverte jusqu’à l’export d’un
courrier, la concurrence, les permissions et les ressources virtuelles. La
passerelle native et la CLI utilisent le même format d’invitation ; la reconnexion
explicite relit la nouvelle invitation sans déplacer les clients déjà lancés.
La recette Rust ↔ Yjs vérifie aussi l’acceptation d’un lot texte/tableau, les
identités conservées et une seule annulation/rétablissement. Aucun nouvel essai
avec un LLM externe, aucune capture dans ce lot.

A.2 reste ouvert : transformations supplémentaires (dont fusion de paragraphes),
mode individuel allégé et raccordement global de la disponibilité dans l’interface.
La conservation durable des brouillons refusés relève d’A.3. Étendre la relecture
MCP à l’en-tête/aux ressources et réintroduire des droits de fichiers rattachés
demanderont des parcours explicites ; les droits historiques egui ne sont pas
réactivés par cette migration.

Guides courants : [CLI](cli.md), [MCP](mcp.md), [invitation Tauri](mcp-local.md).

### Validation du lot

Recette locale macOS, sans captures :

- `cargo test --locked --offline --workspace` : **456 tests réussis**, aucun
  échec ; le diagnostic de performance natif `profile_preview_pipeline` reste
  explicitement ignoré par défaut, comme auparavant.
- `npm test` : **202 tests web et 3 tests de déclaration des langues** réussis.
- `npm run test:contract` : **20 recettes Rust ↔ Yjs/Tiptap** réussies.
- `npm run test:browser -- tests/browser/editing-owner.spec.ts tests/browser/editing-context.spec.ts` :
  **20 parcours Chromium/WebKit** réussis (contextes, focus, historique, refus et
  compositions simulées).
- Clippy workspace/toutes cibles avec `-D warnings`, rustfmt, Prettier,
  constructions Tauri/production web, frontière des dépendances du cœur et
  limites de taille des modules : réussis ; les **9 tests des garde-fous Python** passent.

La recette native `app/scripts/mcp-recipe.mjs` est aussi passée sans fenêtre :
passerelle MCP STDIO réelle, socket de bouclage et propriétaire Rust via le pilote
de recette, proposition sans publication, acceptation puis annulation/rétablissement.
Le script et son rapport sont conservés localement dans
`output/a2-structured-adapters/` (ignoré par Git). Aucun LLM externe n’est intervenu.

Les catalogues générés sont contrôlés contre la CLI. Les contrats antérieurs de
projection restent dans `docs/archive/`, avec leurs recettes. Ces vérifications
locales ne constituent pas une validation de la CI Windows/Linux distante.

## Nettoyage ciblé après migration

Le premier interpréteur `content::operations`, sans appelant de production, est
supprimé avec ses cibles locales, ses styles de frappe et son catalogue parallèle.
Le calcul de longueur UTF-16 encore utilisé par les commandes communes appartient
désormais à `Node` ; les notes, citations et retours conservent une longueur unité.
Les deux tests propres à cet interpréteur disparaissent ; les recettes du moteur
commun continuent de vérifier Unicode, styles, contextes et annotations.

Le catalogue ne porte plus d’indicateurs `legacy` ni d’annonces de migration
inachevée. Les vérifications de signatures utilisent le catalogue exécutable de
`content::api`. Treize motifs d’erreur sans émetteur CLI/MCP et leurs traductions
françaises et anglaises sont retirés.

Ce nettoyage ne remplace pas les tests manquants de sauvegarde native, de
récupération et d’acceptation MCP. La couverture est remesurée après nettoyage des
artefacts instrumentés, en excluant les implémentations des tests ; le taux de
référence corrigé avant ce lot est de 81,39 % sur Linux.

Vérification locale du nettoyage : **454 tests Rust**, **206 tests web**, les
**3 tests de déclaration des langues natives** et **20 recettes Rust ↔ Yjs**
réussis. Clippy sur tout le workspace avec `-D warnings`, le formatage et les
gardes de dépendances et de taille passent également. Le binaire Tauri est
reconstruit avant les recettes de contrat. La déclaration UUID de l’application
hérite désormais de celle du workspace, sans changement de version ni de fonctions.

## Conservation et acceptation MCP — tests natifs

Les opérations de sauvegarde/ouverture et l'acceptation d'une proposition ont
maintenant des points d'entrée indépendants des fenêtres Tauri. L'application
et les tests utilisent ces mêmes fonctions ; le pilote Rust ↔ Yjs passe aussi
par l'acceptation complète avec compilation réelle. La validation
Typst s'effectue sans verrouiller la saisie ; les gardes sont revérifiées avant
l'écriture ou la publication. Les interruptions sont injectées à cette frontière,
sans temporisation arbitraire ; deux acceptations sont aussi mises en concurrence
sur de vrais threads synchronisés.

Vingt tests Rust supplémentaires vérifient notamment :

- la sauvegarde `.typsto`, sa réouverture et la suppression de la récupération
  seulement après succès ;
- les erreurs de validation, d'écriture, de nettoyage de récupération, les fichiers
  corrompus, les verrous et les changements externes, sans perte du travail courant ;
- une nouvelle saisie pendant la validation d'une ouverture ou d'une sauvegarde,
  ainsi qu'une déconnexion pendant l'ouverture ;
- une récupération incohérente ou impossible à recopier : conservation de la
  source et du document courant, puis restauration après résolution du problème ;
- le refus des liens symboliques de récupération sur Unix ;
- la compilation réelle avant acceptation, l'annulation unique, la double
  acceptation concurrente et la conservation du statut final ;
- les interruptions pendant cette compilation : saisie humaine, composition IME,
  refus, révocation, fermeture de fenêtre ou remplacement du document ;
- les requêtes MCP sur une véritable connexion locale : authentification,
  paramètres, lectures et préparation privée sans droit de sauvegarde ;
- un échec de notification après acceptation : le travail confirmé reste
  sauvegardable et une nouvelle tentative ne rejoue pas le lot.

Deux défauts révélés par ces tests sont corrigés :

1. Un état Yjs vide ou reconstruit à partir du même texte ne peut plus remplacer
   l'état confirmé dans une sauvegarde. Le contenu **et l'histoire causale** sont
   vérifiés par la même fonction pour le projet et pour la récupération.
2. Une seconde acceptation ne peut plus marquer « périmée » une proposition déjà
   acceptée. La publication et sa notification restent ordonnées sous le verrou
   du propriétaire.

Ces recettes ne simulent pas une panne électrique ni les dialogues natifs du
système. Elles vérifient les refus et interruptions à des frontières déterministes.
Les tests sont exclus de la couverture ; les modules extraits ne sont pas exclus.
Pour comparer les taux, regrouper `files.rs` avec `files/persistence.rs`, et
`agent.rs` avec `agent/approval.rs` et `agent/transport.rs`.

Validation locale de ce lot : **474 tests Rust**, **206 tests web et 3 tests de
langues natives**, **20 recettes Rust ↔ Yjs**, et **8 parcours Chromium/WebKit**
réussis. Les parcours navigateur couvrent le brouillon refusé, la composition
simulée, la sauvegarde entre deux fenêtres et une édition distante pendant la
sauvegarde. Un premier passage Chromium a expiré lors de l'initialisation de la
seconde fenêtre, avant la composition ; la recette isolée puis la série complète
ont réussi au nouvel essai. Clippy workspace/toutes cibles avec `-D warnings`,
formatage, production web et garde-fous d'architecture passent. Les dialogues de
fichiers du système ne sont pas automatisés par ces parcours navigateur.

## Garanties complémentaires — ressources, décisions et cibles

Le lot suivant ajoute **20 tests Rust**, centrés sur les résultats observables :

- Une image avec légende et une signature survivent aux sauvegardes et à la
  réouverture. Leur suppression retire les octets de l'archive ; Annuler les
  rétablit depuis la mémoire de séance. Une ressource inutilisée n'est pas
  conservée dans le projet.
- Un Article conserve son fichier bibliographique original, y compris ses
  commentaires, sa citation et son image après réouverture et compilation Typst.
- La création et la fermeture refusées gardent document, verrou et récupération.
  Une erreur de nettoyage est levée avant remplacement ; une fermeture confirmée
  libère le verrou et invalide les anciennes références de fenêtre. Une nouvelle
  signature doit être présente et décodable avant création.
- Les reprises d'une requête MCP après acceptation, refus, annulation ou péremption
  ne publient rien et conservent le reçu terminal après une nouvelle proposition.
  Un identifiant réutilisé avec d'autres étapes est refusé. Les limites d'étapes
  et de résumé ne consomment pas la place de relecture.
- Un échec d'écriture de l'invitation garde l'invitation et le lot précédents.
  Le renouvellement invalide l'ancien jeton, y compris sur un socket déjà
  connecté, sans modifier le document. Le fichier d'invitation conserve ses
  permissions privées sur Unix ; son écriture reste hors du verrou documentaire.
- Une cible de note suit les insertions Unicode. Une note, une cellule ou une
  légende supprimée fait perdre la sélection : une insertion ultérieure est
  refusée, sans se reporter sur un voisin. Le curseur entre deux notes reste
  entre les badges.
- La recherche retrouve des cibles UTF-16 exactes dans le corps, les notes,
  cellules, légendes et résumés. Leur remplacement en un lot est annulable en
  une fois ; une plage coupant une paire de substituts Unicode est refusée.
  La recherche bibliographique utilise auteurs et titres sans changer le projet.
- Préférences corrompues, trop volumineuses, signature invalide et écriture refusée
  ne remplacent pas les données enregistrées. Les projets récents sont bornés et
  dédoublonnés sans perdre profil, signature ou dictionnaire personnel.
- Le raccordement orthographique natif fournit les bons indices UTF-16 après un
  emoji ou un accent combiné, distingue les segments et respecte les mots ignorés.
  Il traite les langues indisponibles, suggestions et limites de requête.

Une régression a été confirmée avant correction : une récupération dont le texte
et Yjs concordaient pouvait être publiée malgré une signature absente des
ressources. La restauration remplaçait alors le travail courant avant de constater
que le propriétaire ne pouvait pas s'ouvrir. `Session::from_checkpoint` appelle
maintenant la validation structurelle commune de l'état avec ses ressources,
avant toute publication. Cette validation reste distincte des champs obligatoires
pour l'export : les brouillons incomplets sont admis.

Les extractions de fermeture, d'installation d'invitation et de gestion des récents
réutilisent les mêmes fonctions dans l'application et les tests. Les dialogues
natifs, une panne électrique réelle et le clavier IME du système ne sont pas
simulés par ces tests. Le code de test reste exclu de la mesure de couverture.

La recette navigateur des préférences a révélé un second défaut. Pendant
l'enregistrement du panneau, une modification de largeur pouvait réécrire
l'ancien thème et l'ancienne taille. Le défaut est reproduit en retenant la réponse
d'enregistrement puis en modifiant la largeur ; le test échouait avec « system »
au lieu de « dark ». Toutes les écritures de préférences d'une fenêtre passent
maintenant par une file commune d'instantanés immuables. Les producteurs voient
les nouveaux réglages pendant l'écriture, avant de préparer une autre demande.
Un échec est rapporté à l'appelant sans bloquer les demandes suivantes. Cette
sérialisation ne constitue pas une synchronisation des préférences entre fenêtres.
Deux tests web vérifient l'ordre, l'immuabilité et la reprise après erreur ; la
régression navigateur vérifie la largeur, le thème et la taille après réouverture.

Validation locale finale : **494 tests Rust** réussis (diagnostic de performance
natif toujours ignoré par défaut), **208 tests web et 3 tests de langues natives**,
**20 recettes Rust ↔ Yjs**, **12 parcours Chromium/WebKit**. Ces derniers incluent
la régression déterministe de concurrence des préférences. Le premier passage
WebKit des préférences avait échoué ; le test isolant ce chevauchement a confirmé
le défaut avant correction. La série finale complète passe avec la correction.
Clippy workspace/toutes cibles avec `-D warnings`, rustfmt, Prettier, construction
web/du binaire Tauri, frontière des dépendances et garde-fous de taille passent.
Aucune capture ni sollicitation d'un agent externe n'a été nécessaire.

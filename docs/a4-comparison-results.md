# A.4.4 — Comparateur commun et décisions locales

## Livraison

Le comparateur Rust et la modale servent aux Versions et aux propositions MCP.
La [maquette validée](mockups/a4-comparison.html) et le
[contrat détaillé](a4-comparison-contract.md) sont conservés dans le projet.

- Avant à gauche, après à droite. Versions reprend explicitement un passage de
  gauche ; MCP applique une modification de droite. Les états restent figés.
- Les différences entre ancrages de blocs identiques forment des groupes. Le
  texte utilise les mots Unicode ; liens, styles, structure, annotations, en-tête,
  modèle et inventaires de fichiers sont aussi pris en compte. Les projections
  riches et les deux aperçus Typst utilisent leurs propres ressources.
- Chaque décision est préparée sur un checkpoint détaché du propriétaire actif,
  puis compilée hors de son verrou. La publication revérifie document, révision,
  identité de comparaison et époque des décisions. Une décision refusée ou un
  candidat invalidé ne publient rien.
- Une publication connue fait avancer la garde des groupes restants. Une édition
  extérieure bloque leur application ; Annuler ne réactive pas une ancienne garde.
- Les décisions passent par l’historique du participant qui accepte. Les images
  nécessaires restent disponibles pour Annuler/Rétablir. La réponse
  transporte aussi le delta confirmé, indépendamment de la notification IPC.
- L’acceptation ne sauvegarde ni n’exporte implicitement. Les formulaires non
  validés ne sont pas publiés pour ouvrir une comparaison. Les propositions
  privées ne deviennent pas des versions enregistrées et restent hors du quota.
- Les deux versions historiques se comparent en consultation seule. Les noms
  sont affichés et l’ordre de leurs dates détermine gauche/droite.

Le raccordement MCP fige les archives complètes dès la soumission. Une proposition
périmée avant sa première ouverture reste consultable. Les décisions et leur
révision de travail figurent dans les reçus ; la révision initiale reste distincte.
Les outils rattachés ne peuvent ni accepter, ni sauvegarder, ni exporter.

## Garanties exercées

Les tests du moteur couvrent les deux sens, l’ordre arbitraire des reprises,
les refus, l’annulation séparée, les styles à texte identique, Unicode et espace
insécable, les frontières structurelles, les sélections/styles de frappe sur les
branches inchangées et le refus d’un candidat préparé pour une autre paire.

Les tests documentaires vérifient une note et son texte, une image avec sa
ressource, une collision de chemins immuables, puis un Article : affiliation,
résumé riche, citation et fichier bibliographique, avec Annuler/Rétablir.

Les tests natifs couvrent conservation du fichier enregistré et des snapshots,
concurrence, composition IME, refus après préparation, isolation des fenêtres,
comparaison historique en lecture seule, proposition partiellement acceptée,
consultation périmée, fermeture tardive et écriture après capture du checkpoint.
La [clarification des dépendances](typst-dependencies.md) retire l’historique des
paquets : leurs copies archivées ne créent aucun groupe ni couplage entre passages.
Les tests vérifient que reprise, Annuler/Rétablir, aperçu historique et récupération
gardent les bibliothèques de l’application. Une copie transportée défectueuse ne
doit pas être exécutée. Une ancienne bibliothèque sans les paquets requis par la
version installée reste rendue/exportée avec le catalogue complet de cette version.

Les tests web couvrent le gel des états entre onglets, les décisions indépendantes,
les réponses tardives, les groupes liés, les contrôles en lecture seule, les
flèches, le retour du focus, les options générales et la signature archivée.
Ils vérifient aussi l’installation sans notification et le rejet d’une réponse
d’un document remplacé. Une notification ne peut pas remplacer un autre panneau
latéral par celui de l’assistance.

## Vérification initiale locale macOS

- Workspace Rust : **368 tests réussis**, un diagnostic de performance ignoré.
  Après le dernier durcissement : les **12 tests du comparateur** puis la suite
  native complète (**111 réussis**, un diagnostic ignoré) ont été relancés avec
  succès, dont les **8 tests natifs de ce lot**. Les tests MCP utilisant des
  sockets localhost ont été exécutés avec cette capacité autorisée.
- Clippy du workspace, tous les targets, avec `recipe-driver` et `-D warnings` :
  réussi après les derniers changements Rust.
- Suite Vitest (**354 tests**) et locales natives (**5 tests**), TypeScript/Vite, Prettier, formatage
  Rust, contrôle des modules Rust de moins de 800 lignes et `git diff --check` :
  réussis.
- **10 parcours Playwright réussis** : cinq sous Chromium et cinq sous WebKit,
  avec le vrai propriétaire Rust et le rendu Typst. Reprise locale, maintien de
  l’autre colonne, deux aperçus chargés, édition depuis une seconde fenêtre,
  consultation historique, clavier, acceptation/refus MCP et annulation.
- Vérification visuelle et clavier supplémentaire dans une session isolée avec
  titres, tableau, image et note : modale à 90 %, fond atténué/flouté, sélecteur
  compact, chevrons, différences de mots, reprise et retour au déclencheur.

La recette a révélé deux défauts corrigés : l’état « fermé » calculé avant le
rattachement initial d’une consultation MCP, et une invitation de test non remise
à zéro entre les moteurs de navigateur. Les binaires de recette sont identifiés
par leur artefact Cargo et copiés à part : un lancement normal de l’application
ne peut plus remplacer celui utilisé pour la vérification. Aucune capture,
vidéo ou trace graphique n’a été enregistrée.

Ces vérifications locales ne constituent pas une validation CI Windows/Linux.

## Vérification de la clarification des paquets

Le contrat validé ensuite retire le registre des jeux de paquets et le jeton
`comparison:bundle` de l’historique Yjs. Le comparateur ignore les différences
de copies transportées ; Tauri et la session commune CLI/MCP résolvent les
bibliothèques dans le catalogue applicatif, y compris pour les anciennes versions.
Les snapshots conservent leur état documentaire et leurs ressources sans promettre
l’identité du PDF entre versions de typstOgraphe. La conversion explicite des
formats incompatibles reste un parcours futur, non implémenté.

Contrôles locaux après cette simplification :

- **202 tests du cœur et son doctest réussis**, dont les reprises dans les deux
  sens et l’absence de groupes/couplages créés par des paquets différents.
- **111 tests natifs réussis**, un diagnostic de performance ignoré. Les sockets
  locales MCP ont nécessité l’exécution hors du bac à sable ; les quatre refus
  initiaux étaient des interdictions système, pas des régressions du code.
- Le test commun CLI/MCP a ensuite été enrichi avec une bibliothèque historique
  ne transportant aucun paquet : rendu PNG, sauvegarde et export hors ligne
  réussissent avec les bibliothèques de l’application. Sa relance ciblée réussit.
- Clippy workspace/tous les targets avec `recipe-driver` et `-D warnings`,
  formatage Rust, contrôle des longueurs et `git diff --check` réussis.

La présentation web n’a pas changé dans cette correction ; les recettes
graphiques ci-dessus décrivent la vérification initiale du comparateur.

## Limites et suite

### Correctif des surlignages près des badges

Les snapshots locaux de `examples/article-working.typsto` ont révélé une lacune
de présentation : le mot `doit` remplacé par `peut` et l’ajout de `xx` étaient
calculés, mais aucun marquage n’apparaissait dans leur paragraphe contenant une
citation. L’ancien affichage exigeait l’égalité du texte DOM avec la projection
interne du moteur ; le libellé humain du badge rendait cette égalité impossible.

Le cœur produit maintenant des plages par chemin de feuille/objet. La projection
riche les applique comme marques temporaires, avec conservation des styles et
des widgets, indépendamment des libellés localisés. Les notes modifiées peuvent
aussi être marquées comme objets entiers. Un extrait minimal des deux snapshots
est conservé dans `fixtures/comparison-article-badges.json` ; le projet local de
l’utilisateur n’a pas été modifié ni ajouté au commit.

Validation du correctif : **14 tests du comparateur**, **356 tests Vitest** et
**5 tests de locales natives** réussis ; TypeScript/Vite, Prettier, Clippy du
workspace/tous les targets, formatage Rust et contrôle des longueurs réussis.
Les **8 parcours Chromium/WebKit** passent avec le propriétaire Rust réel, dont
la comparaison historique Article reproduisant `doit` → `peut` et `xx`, avec
vérification du rouge barré/vert souligné en modes clair et sombre. Ces contrôles
sont locaux ; aucune capture n’a été enregistrée.

### Périmètre restant

Le regroupement reste volontairement conservateur : un paragraphe ou un widget
n’est pas découpé en décisions textuelles plus fines. Un changement de réglages,
ressources documentaires ou bibliothèque bibliographique regroupe le contenu associé ; aucune
indépendance supplémentaire n’est déduite de la proximité visuelle des changements.
Les deux aperçus permettent d’examiner aussi les différences de présentation.

Les décisions de comparaison restent temporaires, en mémoire. Le recalcul après
édition extérieure est explicite : nouvelle comparaison Versions ou nouvelle
proposition MCP. La reprise vers un tiers état n’est pas proposée.

La restauration complète avec snapshot `before_restore`, ses confirmations et sa
coordination avec les autres fenêtres restent le prochain lot A.4. Les relectures J
conservent leurs outils dédiés. F.3 peut enrichir le cycle de vie et les reçus des
propositions sans construire un second moteur de décisions.

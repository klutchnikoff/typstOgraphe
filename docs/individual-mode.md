# Mode individuel et services de synchronisation

## Contrat retenu et livré

Le mode individuel est le défaut d’une nouvelle session documentaire. Il conserve
Yjs dans l’éditeur, Yrs et la validation dans Rust, l’historique par participant,
les ressources, l’aperçu Typst et la récupération. Les fichiers `.typsto` restent
identiques : le mode ne constitue ni une conversion ni une option persistée.
Rouvrir un fichier démarre une nouvelle session individuelle ; si plusieurs
fenêtres s’y rattachent, la règle d’activation automatique s’applique.

La différence porte sur les services facultatifs : aucun objet `Awareness`,
heartbeat, abonnement de présence ou plugin de curseurs distants en individuel.
Le transport de confirmation documentaire reste actif. Les événements nécessaires
aux propositions MCP et aux autres vues restent écoutés. Le propriétaire ne
renvoie plus à son unique fenêtre les deltas dont il lui a déjà rendu l’accusé.
Cette dernière économie s’applique aussi à une synchronisation activée sans
seconde fenêtre ; elle dépend des destinataires, jamais du mode documentaire.

## Parcours

- Le bouton de la barre d’état **Activer la synchronisation** active les services
  en un clic. Les mêmes commandes figurent dans le menu et la palette.
- Aucun remplacement de `Y.Doc`, de l’éditeur ou de l’historique : identité,
  ressources, sélection, styles de frappe et focus sont conservés. Le bouton
  n’accapare pas le focus à la souris ; une activation ne crée pas d’annulation.
- Le clic attend les confirmations en cours. Pendant une composition IME, il
  reste mémorisé jusqu’à sa validation, sans demander un second clic. Une présence
  déjà active suspend sa publication pendant la composition sans reconfigurer
  sa vue ; les curseurs ne sont republiés qu’après confirmation.
- Les formulaires explicitement privés restent privés. Un brouillon refusé doit
  être résolu avant activation. Un échec du service ne remplace pas la saisie.
- Ouvrir une seconde fenêtre du même document est une demande explicite de
  participation : elle active automatiquement les services locaux pour les vues.
- Le départ d’une fenêtre retire immédiatement sa présence. Le service reste
  activé jusqu’à une action explicite. **Arrêter la synchronisation** est
  disponible lorsqu’il ne reste qu’une fenêtre participante ; Rust vérifie aussi
  cette condition, y compris en cas de course avec une nouvelle ouverture.
- Une fenêtre momentanément déconnectée ne participe pas à la présence. Sa
  reconnexion réactive les services si une autre fenêtre participe. La
  déconnexion diagnostique existante ne désigne pas le mode individuel.
- Le rattachement d’un agent MCP n’active pas la présence humaine. Les
  propositions, leur validation et leur publication continuent par le
  propriétaire commun, même avec un unique rédacteur en mode individuel.

## Frontière technique

`Session` possède un état éphémère `Synchronization` : génération du document,
version des services, activation et fenêtres participantes. Il ne modifie pas la
révision documentaire, les paramètres, l’historique ou l’archive. Les versions
protègent contre une notification ou un paquet de présence tardif après un arrêt,
un redémarrage du service ou un changement de session.

`LocalPresence` possède uniquement la présence et son cycle de vie. Le montage du
plugin public `yCursorPlugin` utilise la reconfiguration du même éditeur. Aucun
second moteur, format ou historique « solo » n’est ajouté. Les horloges de
présence restent monotones lorsque le service redémarre sur le même `Y.Doc`.
Un nouveau curseur au début d’un texte vivant référence son premier caractère,
pour ne pas confondre cette nouvelle position avec une ancienne frontière effacée
dans les migrations Tiptap. Les ancrages historiques du document restent intacts. Les primitives
Yjs/Yrs et ProseMirror utilisées restent celles du projet.

Le transport de recette SSE reproduit les décisions du propriétaire natif,
y compris le départ des fenêtres et le filtrage des échos. Ses connexions portent
un identifiant propre à chaque chargement pour qu’une ancienne fermeture ne
retire pas une nouvelle vue du même nom.

Il s’agit de synchronisation entre fenêtres locales. Aucun serveur distant,
partage réseau ou droit supplémentaire n’est introduit ; voir le chantier G.

## Mesure et vérification

Le banc natif utilise la WebView Tauri, les assets de production et le véritable
propriétaire Rust. Il distingue les confirmations documentaires, les échanges au
repos et les compilations Typst. Les recettes fonctionnelles n’utilisent aucune
capture d’écran. Les chiffres et leurs limites sont consignés dans le
[rapport de mesure](../experiments/individual-mode/README.md).

La [recette finale Courrier/Article](a2-closure-results.md) clôture A.2 le
31 octobre 2026. Elle confirme notamment qu’une réouverture repart en individuel,
y compris après activation sur le précédent document ; les autres fenêtres
restent enregistrées pour l’activation automatique lorsqu’elles rejoignent.
Les brouillons durables restent dans A.3, le réseau et les permissions dans G.

Vérifications du lot : **295 tests Rust**, **320 tests web + 3 tests de langues
natives**, **29 recettes de contrat** et **14 parcours Chromium/WebKit** réussis.
Ces parcours couvrent l’activation après rédaction, les styles de frappe et
l’historique conservés, les deux fenêtres, les curseurs distants réellement
visibles au début du texte, la reconnexion, les refus et le clic pendant une
composition. Clippy, TypeScript, formats et contrôles de frontière du cœur passent.
La composition est simulée dans ces recettes ; un clavier ou IME physique reste
une vérification manuelle, sans prétendre l’avoir exercé ici.

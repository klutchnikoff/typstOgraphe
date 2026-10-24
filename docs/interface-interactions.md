# K.1 — Inventaire des interactions

État du code relu : `af81da4`, le 24 octobre 2026. Inventaire documentaire, sans
modification de comportement. D29 et D30 sont validées ; les événements et les
politiques à définir en K.2 ou en phase 6 ne sont pas figés par ce document.

Le moteur porte déjà les règles documentaires. Ce qui reste à extraire concerne
principalement la coordination entre sélection, rendu et demandes de révélation.
Cette coordination est distincte de la saisie, du placement et du focus propres
à chaque interface. [Le relevé des états et événements](interface-view-sync.md)
détaille les transitions à conserver.

Suite livrée : le [composant de K.2](view-sync.md) reprend ces règles et leurs
tests sans egui ; son branchement dans l'application reste prévu en K.3.

## Classement et périmètre

| Classe | Responsabilité | Suite |
| --- | --- | --- |
| D — Documentaire | Validité du document, cibles gardées, édition, historique, capacités et lectures. | Déjà commune dans `core` ; la réutiliser. |
| V — Coordination de vue | Décider de demander une révélation ou des surbrillances, différer une demande, éviter le retour inverse. | Candidats pour le composant distinct du moteur et instancié par vue, après K.1. |
| A — Adaptateur | Gestes, saisie/IME, focus natif, placement, pixels, fenêtres, préférences et services de l'hôte. | Reste dans l'adaptateur ; certaines politiques de l'hôte devront être explicites en phase 6. |

Une fonction peut mélanger D, V et A. La colonne « suite » décrit le partage visé,
pas un déplacement intégral de son fichier. Une décision n'est pas automatiquement
documentaire : choisir le champ qui garde le focus relève aussi de l'adaptateur.
L'inventaire couvre les modules de production de `crates/app/src/app/`, les lectures
et services du cœur qu'ils appellent, ainsi que les tests pertinents.

## Appels, sélection et historique

| Interaction et origine | Règle actuelle et propriétaire | Classe / suite | Repère |
| --- | --- | --- | --- |
| Menu, palette, barre ou raccourci | `command_enabled` réutilise la disponibilité et les intentions du moteur ; le registre du type et les états locaux ajoutent leurs conditions. | D existant ; A pour modalité, préférence orthographique et champ de métadonnées actif. | [availability.rs][availability], [command-routing.md](command-routing.md) |
| Déclenchement d'une action | `apply` annule l'IME, construit la commande documentaire ou ouvre une fonction d'interface ; `dispatch_action` refuse les modalités et demande un rafraîchissement. | D pour la commande ; A pour l'aiguillage des événements et des fenêtres. | [commands.rs][commands] |
| Exécution documentaire | `execute_document_command` publie d'abord la sélection egui dans le moteur, appelle `run_call`, puis adapte les effets. | D pour validation/publication ; V pour les demandes de révélation ; A pour caches, messages et focus. | [engine.rs][app-engine] |
| Sélection graphique en fin de frame | `sync_published_selection` synchronise le cache natif avec `set_adapter_selection` ; cette méthode ne recalcule pas le style. | D pour les bornes ; A pour la conversion `CCursorRange` et le moment de publication. | [engine.rs][app-engine], [engine/mod.rs][core-engine] |
| Déplacement natif sans modification du texte | `update_adapter_cursor` publie la sélection, reprend le style environnant et termine le groupe de frappe, sans changer le contenu ni sa révision. | D existant ; V pour le suivi de la nouvelle position ; A pour le geste. | [editor_view.rs][editor-view], [native.rs][native] |
| Gras, italique, liens et blocs | Les commandes du moteur appliquent les règles, les frontières et l'historique ; gras et italique peuvent aussi régler le style de frappe. | D existant. L'interface peint une représentation indépendante du modèle. | [run.rs][run], [guide MCP](mcp.md#gras-et-italique--délimiter-les-insertions) |
| Réponse de commande réussie | `Render`, `RevealSelection`, `DocumentReplaced`, `Saved` sont produits par le moteur ; leur adaptation n'est pas équivalente à « le texte a changé ». | D pour les effets ; V pour la révélation ; A pour leur exécution visuelle et les services locaux. | [run.rs][run], [engine.rs][app-engine] |
| Échec de commande ou de saisie | Le moteur refuse l'édition ; `accept_edit` restaure le curseur antérieur en cas de rejet natif, puis l'application présente l'erreur. | D pour le refus ; A pour la restauration du widget et les messages. | [commands.rs][commands] |
| Lot atomique | Le moteur agrège les effets et publie une transaction. L'application préserve le focus si source et sélection sont inchangées ; elle actualise la présentation du modèle si nécessaire. | D existant ; A pour la comparaison servant au focus. Ne pas inventer de révélation intermédiaire des étapes. | [batch.rs][batch], [engine.rs][app-engine] |
| Annuler/rétablir | L'historique documentaire appartient au moteur. `apply_document_command` compare le document précédent, vide les textures d'images et conserve éventuellement le focus du champ actif. | D existant ; A pour le champ egui et les caches ; V pour l'adaptation des effets. | [engine.rs][app-engine], [commands.rs][commands] |

## Saisie, navigation et annotations

| Interaction et origine | Règle actuelle et propriétaire | Classe / suite | Repère |
| --- | --- | --- | --- |
| Saisie, collage, suppression | egui modifie un `TextInput` possédé ; `commit_native_edit` valide et publie projection, sélection et historique. La première erreur condamne le brouillon. | D existant ; A pour `TextEdit`, les conversions de curseur et le regroupement des événements. | [editor_view.rs][editor-view], [frontière Rust](engine-adapters.md#saisie-et-composition-natives) |
| Préédition IME | La projection provisoire et son curseur restent locaux ; le texte publié, les compteurs et l'aperçu ne reçoivent pas les prédictions. | A. `NativeEdit` reste un instantané opaque du cœur, sans contrat réseau. | [ime.rs][ime], [editor_view.rs][editor-view] |
| Validation ou abandon IME | Commit passe par la validation commune. Commande, changement de cible/garde, perte de focus, clic ou fermeture peuvent annuler la préédition ; le Commit tardif est filtré. | D pour la garde et la publication ; A pour la politique d'interruption, les événements et le curseur provisoire. Phase 6 : arbitrer l'arrivée d'une édition d'agent. | [ime.rs][ime], [native.rs][native] |
| Raccourcis textuels | L'interface reconnaît un événement candidat ; `transforms_input` et la commande avec `input_rules=typing` appliquent les transformations du moteur. | D existant ; A pour la consommation de la touche et l'exclusion de l'IME. | [text_input.rs][text-input], [typing.rs][typing] |
| Tab/Shift-Tab dans une annotation ou description | L'adaptateur choisit l'action à partir de la disponibilité ; le moteur résout `body`, `annotation`, `term` ou `definition`. | D pour les destinations ; A pour Tab et le verrouillage du focus du widget ; V pour la révélation demandée par les effets. | [text_input.rs][text-input], [annotations.rs][annotations], [blocks.rs][blocks] |
| Entrée et sortie de structure | `block.split`/`block.exit` déterminent les frontières, la scission ou la sortie ; le moteur décide aussi quand émettre `RevealSelection`. | D existant ; A pour la touche ; V pour la demande de révélation. | [text_input.rs][text-input], [run.rs][run], [blocks.rs][blocks] |
| Clic sur le terme d'une description | Le dessin détecte le point ; `select_description_term` construit une cible gardée et demande la partie `term`. | A pour la détection ; D pour la destination. Ne pas assimiler cette sélection à un ordre explicite de défilement. | [editor_view.rs][editor-view], [engine.rs][app-engine] |
| Image : sélection, légende et suppression | Boutons et clavier passent par `select_image_part`/`remove_image` ; le moteur protège les frontières et gère les ressources. | D existant ; A pour les boutons, gestes, boîtes et textures ; V pour les effets de navigation. | [images.rs][images], [text_input.rs][text-input] |
| P.-S. et P.J. | La commande insère une structure et place le curseur ; l'application ajoute une demande de révélation de l'éditeur pour ces deux déclencheurs. | D existant ; V pour cette règle encore spécifique dans l'application. | [run.rs][run], [engine.rs][app-engine] |

## Aperçu, recherche et orthographe

| Interaction et origine | Règle actuelle et propriétaire | Classe / suite | Repère |
| --- | --- | --- | --- |
| Demander un rendu après modification | L'application attend plus de 220 ms après le changement, compare `submitted` à la révision et prépare un instantané du moteur. | D/service existant pour l'instantané ; politique de rendu interactive à relever avec V ; A pour l'horloge, l'envoi et les réveils. Ne pas transformer ce délai en règle de compilation CLI/MCP. | [preview.rs][preview], [render.rs][core-render] |
| Exécuter le rendu asynchrone | Le worker natif du cœur conserve les caches Typst, coalesce les aperçus en attente et réveille l'hôte par rappel. Il ne dépend pas d'egui. | Service commun existant ; ne pas recréer un second worker dans `ViewSync`. | [rendering.rs][rendering] |
| Recevoir un rendu | L'application filtre par garde complète ; `install_render` la vérifie aussi, puis l'application installe textures, présentation et statut. | D pour la validité ; V pour réarmer le suivi de position ; A pour les pixels/texels, l'erreur affichée et la durée. | [preview.rs][preview], [render.rs][core-render] |
| Suivi source → aperçu | Une demande attend un rendu courant ; la lecture `locate` fournit un point à partir de l'extrémité primaire du curseur. | D pour la géométrie ; V pour le choix de la position, l'attente et la consommation de la demande. | [preview.rs][preview], [relevé des transitions](interface-view-sync.md) |
| Surbrillance de sélection dans l'aperçu | Si le rendu est courant et sans erreur locale, l'application demande `selection_boxes` ; sinon les boîtes sont absentes. | D pour les quadrilatères ; V pour l'éligibilité ; A pour échelle, couleur, clip et dessin. | [preview.rs][preview], [render.rs][core-render] |
| Navigation aperçu → source | Le **double-clic** sur une page peinte devient un point Typst. Le moteur localise ; l'application sélectionne, révèle l'éditeur et annule le suivi inverse. | A pour le double-clic et les pixels ; D pour `locate`/`text.select` ; V pour les demandes et l'absence de boucle. | [preview.rs][preview] |
| Faire défiler ou redimensionner les colonnes | L'adaptateur conserve la proportion, convertit les points en pixels et ne recentre l'aperçu que si le petit rectangle cible n'est pas déjà visible. | A. Géométrie des widgets et préférences de largeur restent locales. | [editor_view.rs][editor-view], [preview.rs][preview] |
| Recherche incrémentale et remplacement | Le moteur calcule les occurrences et valide les remplacements. L'application garde requête, remplacement, résultats graphiques et sélection active, liés à la garde. | D existant ; A pour le panneau et le cache des plages à peindre. | [search.rs][search], [availability.rs][availability] |
| Navigation et clic sur une occurrence | Navigation utilise les commandes `edit.find_*` ; un clic sur une plage peinte sélectionne sa cible gardée et demande explicitement de révéler l'éditeur. Une sélection remplace l'autre. | D pour les cibles et le déplacement ; V pour la révélation ; A pour la détection et le maintien du focus de recherche. | [search.rs][search], [editor_view.rs][editor-view] |
| Diagnostics orthographiques et suggestions | Les calculs/références sont communs ; le checker asynchrone alimente le soulignement. La disponibilité relit le document publié, sans attendre ce cache graphique. | D existant ; A pour temporisation, activation globale, popup et cache du checker. | [spelling.rs][spelling], [availability.rs][availability] |
| Clic droit, correction, suivant/précédent | Le clic droit est converti en référence ; correction et navigation passent par le moteur. Le menu capture une cible revalidée à l'application. | D existant ; V pour les effets de navigation ; A pour hit-test et popup. | [spelling.rs][spelling], [commands.rs][commands] |
| Masquer un soulignement sous la frappe | Pendant 700 ms, une faute sous un curseur vide et focalisé n'est pas peinte ; le résultat documentaire n'est pas supprimé. | A, politique visuelle de l'éditeur. | [editor_view.rs][editor-view] |
| Ignorer/apprendre un mot | Ignorer est documentaire et annulable ; apprendre écrit le dictionnaire personnel dans les préférences, puis configure le moteur avec ces valeurs. | D pour ignorer ; A pour apprendre et persister le profil. | [spelling.rs][spelling], [preferences.rs][preferences] |
| Compteurs document/sélection | Le moteur compte les mots et graphèmes du contenu publié ; la barre d'état formate le résultat et ses info-bulles. | D existant ; A pour la présentation. | [chrome.rs][chrome] |

## Fichiers, champs et services de l'hôte

| Interaction et origine | Règle actuelle et propriétaire | Classe / suite | Repère |
| --- | --- | --- | --- |
| Création et choix d'un modèle fourni | Types créables et modèles viennent des définitions intégrées. L'application fournit les valeurs explicites du profil ; le moteur crée le document. | D pour le contrat et la publication ; A pour les choix visuels et l'accès au profil. | [files.rs][files], [dialogs.rs][dialogs] |
| Modification de métadonnées | Le widget fournit valeur, champ et garde ; le moteur valide et regroupe `typing` ou `discrete`. L'undo natif du widget est désactivé. | D existant ; A pour saisie, champ actif et restitution de la valeur publiée. | [information_fields.rs][information-fields] |
| Date du jour | L'hôte résout « aujourd'hui » en date fixe avant d'envoyer la valeur ; le type valide cette donnée. | A pour l'horloge et le raccourci de saisie ; D pour le champ et son format. | [information_fields.rs][information-fields] |
| Options et modèles | Le moteur valide format, langue et puces selon le modèle. L'interface peut garder un brouillon de puce invalide sans le publier. | D existant ; A pour brouillon, rubriques et messages. | [information.rs][information], [models.rs][models] |
| Import d'image, signature, modèle ou lien | Les dialogues capturent une cible/garde ; le moteur revalide à l'application. Glisser-déposer et sélecteur de fichier sont des mécanismes d'hôte. | D pour validité et ressources ; A pour dialogues, brouillons et accès au profil. | [images.rs][images], [signature.rs][signature], [links.rs][links], [models.rs][models], [dialogs.rs][dialogs] |
| Remplacement/fermeture d'un document modifié | L'application propose sauvegarder/abandonner/annuler. Le moteur exige garde et abandon explicite ; l'application les fournit lors de l'action finale. | D pour le refus ; A pour la décision humaine et `pending_action`. Phase 6 : revalider le contexte de cette décision si le document évolue. | [files.rs][files], [files du moteur][core-files] |
| Sauvegarde | Le moteur assure archive atomique, empreinte et verrou ; l'application choisit le chemin et, après succès, nettoie sa récupération locale. | D existant ; A pour dialogues, messages et service de récupération. Une sauvegarde seule ne force pas le suivi de l'aperçu. | [files.rs][files], [engine.rs][app-engine] |
| Fichier déjà réservé | Le moteur refuse un second écrivain ; l'application propose l'ouverture en lecture seule et permet ensuite une copie. | D pour verrou/copie ; A pour l'offre et son chemin en attente. | [files.rs][files], [files du moteur][core-files] |
| Export en arrière-plan | `prepare_export` valide et capture le document demandé ; une tâche possédée s'exécute hors du thread graphique. Modifier le document ensuite ne modifie pas cet export. | D pour validation/instantané ; A pour thread, boîte de dialogue, canal et notification finale. Ce n'est pas l'installation d'un aperçu. | [export.rs][export], [dialogs.rs][dialogs], [guide des lectures](engine-queries.md) |
| Récupération après incident | L'hôte choisit le répertoire, planifie les points de reprise et configure les entrées. Lister/restaurer/supprimer utilisent les références du moteur ; restaurer remplace le document. | D pour les opérations gardées ; A pour service périodique, adoption de l'entrée et dialogue. | [recovery.rs][recovery] |
| Après remplacement réussi | Réinitialisation de recherche, cibles de dialogues, historique du champ, images, présentation, sélection et aperçu ; reconstruction du registre du type. | D pour l'identité neuve ; V pour annuler les demandes de l'ancienne vue et amorcer la nouvelle ; A pour les caches/fenêtres. | [files.rs][files] |
| Préférences et apparence | Profil, signature globale, dictionnaire personnel, police, thème et largeur des colonnes sont persistés par l'hôte. Le moteur reçoit des valeurs, pas ces préférences. | A. Aucun déplacement de profil ou de stockage de préférences vers `ViewSync`. | [preferences.rs][preferences], [setup.rs][setup] |
| Cycle de frame, menus et diagnostics visuels | Ordre de dessin, modales, titre, langue des libellés, capture et réveils sont gérés par l'application. | A ; l'ordre actuel de consommation des demandes est un fait à préserver en K.2/K.3. | [mod.rs][app-mod], [chrome.rs][chrome], [setup.rs][setup] |

## Conséquences pour le périmètre de K.2

1. Extraire les **règles V**, en réutilisant `locate`, `selection_boxes`, les gardes
   et les effets existants. Le moteur n'a pas besoin de savoir si un menu est ouvert.
2. Distinguer une demande de révélation d'un simple changement de sélection ; une
   nouvelle sélection n'incrémente pas nécessairement la révision documentaire.
3. Conserver côté adaptateur focus, disposition, pixels, état IME, cache des textures,
   exécution des temporisations et interfaces des services. La place des politiques
   de planification reste à préciser ; le coordinateur pourra recevoir une horloge,
   des faits ou des résultats sans imposer une technologie d'exécution.
4. Le composant par vue ne crée pas plusieurs sélections dans le moteur : celui-ci
   possède aujourd'hui une sélection et un style de frappe. La politique des
   commandes d'un agent dans la session ouverte reste à définir en phase 6.
5. Les politiques existantes qui mériteraient de changer sont consignées dans
   [le relevé](interface-view-sync.md#choix-à-reprendre-ensemble), pas corrigées ici.

K.1 n'ajoute ni interface publique, ni transport, ni nouvelle sémantique d'effet.
Le [contrat Rust existant](engine-adapters.md) reste la référence d'intégration ;
`docs/interface-contract.md` sera le livrable de K.4 après l'extraction.

[availability]: ../crates/app/src/app/availability.rs
[commands]: ../crates/app/src/app/commands.rs
[app-engine]: ../crates/app/src/app/engine.rs
[app-mod]: ../crates/app/src/app/mod.rs
[editor-view]: ../crates/app/src/app/editor_view.rs
[ime]: ../crates/app/src/app/ime.rs
[text-input]: ../crates/app/src/app/text_input.rs
[preview]: ../crates/app/src/app/preview.rs
[search]: ../crates/app/src/app/search.rs
[spelling]: ../crates/app/src/app/spelling.rs
[files]: ../crates/app/src/app/files.rs
[export]: ../crates/app/src/app/export.rs
[recovery]: ../crates/app/src/app/recovery.rs
[images]: ../crates/app/src/app/images.rs
[signature]: ../crates/app/src/app/signature.rs
[links]: ../crates/app/src/app/links.rs
[models]: ../crates/app/src/app/models.rs
[information]: ../crates/app/src/app/information.rs
[information-fields]: ../crates/app/src/app/information_fields.rs
[preferences]: ../crates/app/src/app/preferences.rs
[dialogs]: ../crates/app/src/app/dialogs.rs
[chrome]: ../crates/app/src/app/chrome.rs
[setup]: ../crates/app/src/app/setup.rs
[core-engine]: ../crates/core/src/engine/mod.rs
[run]: ../crates/core/src/engine/run.rs
[batch]: ../crates/core/src/engine/batch.rs
[native]: ../crates/core/src/engine/native.rs
[typing]: ../crates/core/src/engine/typing.rs
[annotations]: ../crates/core/src/engine/annotations.rs
[blocks]: ../crates/core/src/engine/blocks.rs
[core-render]: ../crates/core/src/engine/render.rs
[rendering]: ../crates/core/src/rendering.rs
[core-files]: ../crates/core/src/engine/files.rs

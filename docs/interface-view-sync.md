# K.1 — États, événements et coordination des vues

> Les références à egui et aux captures ci-dessous décrivent l’implémentation
> historique conservée dans Git, avant le [retrait d’egui](egui-retirement-results.md).
> Ses outils ne sont plus des commandes de développement actives.

Relevé du code `af81da4`, le 24 octobre 2026. Complément de
[l'inventaire des interactions](interface-interactions.md), en préparation de
K.2/K.3 et des choix de phase 6. Les noms d'événements proposés par D30 restent
provisoires ; ce document décrit d'abord ce que le code fait aujourd'hui.

Suite livrée : [K.2 — coordinateur de vue](view-sync.md). L'inventaire ci-dessous
reste le relevé du branchement egui avant K.3 ; K.2 ajoute le composant pur et
14 tests, sans modifier ce branchement ni livrer l'assistance de phase 6.
Le [branchement K.3 et sa recette](view-sync-egui.md) sont désormais livrés ; les
deux indicateurs décrits dans cet inventaire historique ont été retirés d'egui.
Le [contrat général livré en K.4](interface-contract.md) est la référence
d'intégration ; cet inventaire conserve les observations de départ.

## États et propriété

Les structures sont dans [app/state.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/state.rs), sauf
l'IME, déclaré dans [app/ime.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/ime.rs).

| État actuel | Sens observé | Partage à préparer |
| --- | --- | --- |
| `Engine::selection()` | Sélection publiée unique, utilisée pour les commandes à cible courante. `set_adapter_selection` valide ses bornes ; `update_adapter_cursor` actualise aussi le style. | État documentaire existant. Pas encore de sélection du moteur par vue ou par acteur. |
| `Projection::typing_style` | Gras, italique et lien hérités/réglés pour la saisie ; les commandes d'un agent peuvent affecter les insertions suivantes. | État documentaire existant ; politique de restitution à discuter pour la phase 6. |
| `document.selection` | Cache egui en caractères, direction et affinité ; distinct de la sélection native de préédition. | A : conversion et cache locaux. Le coordinateur utilise une cible logique validée. |
| `preview.forward_pending` | Demande booléenne de retrouver l'extrémité primaire du curseur dans l'aperçu. Plusieurs demandes se fusionnent ; le curseur est relu lors de la consommation. | V : intention commune par vue. Sa chronologie ne se réduit pas à la révision. |
| `preview.editor_scroll_pending` | Demande de révéler la sélection dans l'éditeur lors de son prochain retour de `TextEdit`. | V pour l'intention ; A pour focus, calcul de rectangle et défilement. |
| `document.force_selection` | Réinjecter le curseur dans le widget egui et restaurer éventuellement son focus. Peut être posé après un refus ou une fermeture de dialogue. | A. Ne pas le convertir mécaniquement en demande de suivi de l'aperçu. |
| `document.history_field` | Identifiant du champ natif dont le focus doit être conservé lors de l'historique. Recherche et palette ne deviennent pas cette cible. | A : aucun identifiant egui dans le coordinateur. |
| `preview.render` | Référence et tailles des pages du rendu installé dans le moteur ; l'application conserve un miroir de présentation. | D pour la validité de la référence, déjà vérifiée par le moteur ; A pour le miroir. |
| `preview.error` | Erreur locale affichée ; sa présence interdit actuellement localisation inverse et surbrillances, même si une ancienne texture reste visible. | A pour le message ; V pour la règle d'éligibilité à reproduire avec un statut de rendu. |
| `preview.pages`, `elapsed` | Textures GPU et durée affichée. | A ; ni pixels de widget ni textures dans `ViewSync`. |
| `preview.submitted` | Dernière révision envoyée au worker ; sert au débit de demandes, pas à valider une réponse. N'est pas remis à zéro par `after_document_replaced`. | Coordination du service de rendu à distinguer de la navigation ; ne pas le substituer à une garde complète. |
| `preview.revision` | Révision du dernier succès/échec pris en compte pour le statut et les réveils ; n'est pas la preuve de validité du rendu. | A/service. La synchronisation repose sur `render.reference.guard` et l'absence d'erreur. |
| `search.guard`, `occurrences`, `search.engine` | Références documentaires et plages dérivées, requête, remplacement et index actif pour le panneau. | D pour recherche et références ; A pour panneau/cache. La conversion des occurrences en indices n'est pas un nouveau ciblage public. |
| `spelling.checker`, `target`, `menu_pending`, `menu_position` | Résultats différés pour le soulignement et référence capturée de popup ; position en pixels. | D pour le diagnostic et sa revalidation ; A pour worker graphique, popup et position. |
| `ime.draft`, `enabled`, `discarding`, `reset_cursor`, `scroll_pending` | Composition provisoire, filtrage des événements tardifs, restitution du curseur et défilement de la préédition. | A. `ime.scroll_pending` n'est pas une demande source ↔ aperçu. |
| Cibles de lien, image, modèle, signature, export | Cibles/gardes capturées avant une interaction différée ; les valeurs de dialogue restent locales. | D pour le contrôle des cibles ; A pour leur capture et la durée du dialogue. |
| `file.pending_action`, `read_only_offer`, `allow_close` | Décision d'abandon/sauvegarde en attente, chemin de l'offre de lecture seule, fermeture de fenêtre. `PendingAction` ne capture pas lui-même de garde. | A. Revoir la validité d'une décision différée si un agent peut modifier/remplacer le document. |
| `file.export_task`, `export_guard` | Export d'un instantané, indépendant du rendu courant ; garde capturée à l'ouverture du dialogue. | D pour préparer l'instantané ; A pour thread/canal et notification. |
| `recovery.worker`, `entries`, `recovered_name` | Service de reprise configuré par l'hôte, références documentaires et nom d'une copie restaurée. | D pour opérations/références ; A pour planification, adoption et présentation. |
| Préférences, `image_textures`, `capture` | Réglages globaux, cache GPU et diagnostic visuel reproductible. | A ; hors de l'état du coordinateur documentaire. |

## Producteurs et consommateurs des demandes de navigation

Toutes les occurrences de production des deux indicateurs sont couvertes ici ; les
initialisations et les remises à zéro sont distinguées des événements utilisateur.

| Site | `forward_pending` | `editor_scroll_pending` | Condition exacte |
| --- | --- | --- | --- |
| [setup.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/setup.rs), `with_context` | `true` initial | `false` initial | Application initialisée sans document ; aucun rendu lancé tant que le document est fermé. |
| [engine.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/engine.rs), `execute_document_command` | Passe à `true` | Passe à `true` si `RevealSelection` | Suivi avant si `outcome.changed || !effects.contains(Saved)`. Un succès sans modification et sans `Saved` peut donc demander le suivi ; une sauvegarde seule ne l'ajoute pas et n'efface pas une demande antérieure. |
| Même fichier, `apply_document_command` | Déjà traité par la voie précédente | Passe aussi à `true` | Déclencheurs UI `InsertPostscript` et `InsertAttachment`. Cette demande supplémentaire ne vient pas d'un effet du moteur. |
| [editor_view.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/editor_view.rs), `editor_feedback` | Passe à `true` | Consommé, puis `false` | Le suivi avant est posé si la sélection finale diffère de `old_range`. La révélation éditeur utilise la sélection actuelle, centre son rectangle et ne reprend le focus que si aucun champ de recherche ne l'a. |
| [search.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/search.rs), `select_search_result` | Posé par la commande de sélection | Passe à `true` | Seulement si la sélection de l'occurrence gardée réussit. Le clic sur une plage peinte est distinct de `edit.find_next/previous`, dont le moteur produit déjà `RevealSelection`. |
| [files.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/files.rs), `after_document_replaced` | `true` | `true` | Après remplacement réussi, y compris fermeture, avec sélection ramenée à zéro et caches réinitialisés. |
| [preview.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/preview.rs), `rendering` | Passe à `true` | Inchangé | Après installation réussie d'un rendu lié à la garde courante, même sans déplacement de sélection. |
| Même fichier, `preview_ui` | Consommé, puis `false` | Inchangé | Seulement si `forward_pending && sync_is_current()`. La demande est consommée avant l'appel de localisation, même si celui-ci ne trouve aucun point. |
| Même fichier, `navigate_backward` | Mis à `false` après le succès de la commande | Passe à `true` | Seulement avec un rendu courant, une cible trouvée et une sélection réussie. La voie générique avait posé le suivi avant ; cette remise à zéro évite le retour immédiat dans l'aperçu. |
| [ime.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/ime.rs), `ime_event(Commit)` | Passe à `true` | Inchangé | Après tentative de publication d'un Commit non vide/non saut de ligne, via `accept_edit` ; ce dernier peut aussi signaler un refus. La préédition seule ne pose pas cette demande. |

Les reprises de focus supplémentaires de `force_selection` se trouvent dans
`engine.rs`, `commands.rs`, `files.rs`, `dialogs.rs`, `preferences.rs`, `search.rs`
et `spelling.rs`. Elles ne sont pas toutes des déplacements documentaires : par
exemple, fermer les préférences ou refuser une édition restaure le widget sans
créer de commande de sélection. `editor_frame` consomme ce drapeau en conservant
le focus de recherche lorsqu'il est actif ; `editor_feedback` peut aussi ajuster
le curseur du widget dans la frame courante.

Le relevé du rendu concerne le parcours interactif : `preview_request` fournit
`export_dir: None`. Le worker accepte aussi des demandes d'export ; `rendering`
comporte encore une branche `is_export`, dont les erreurs sont affichées même si
la garde n'est plus courante. L'export proposé aujourd'hui par le dialogue passe
par une tâche distincte (`file.export_task`), sans installer un nouvel aperçu.

## Effets documentaires déjà disponibles

Source : [engine/run.rs](../crates/core/src/engine/run.rs) et
[engine/batch.rs](../crates/core/src/engine/batch.rs).

| Effet | Émission actuelle | Adaptation actuelle |
| --- | --- | --- |
| `Render` | Révision changée ; dans un lot, publication finale changée. | Le rendu interactif est demandé par comparaison de révisions et temporisation, pas uniquement par la présence de cet effet. |
| `RevealSelection` | Image insérée/remplacée ; navigation recherche/orthographe ; remplacement courant ; sortie de bloc ; sélection de `body/annotation` ; certaines scissions de bloc. | Demande de révélation de l'éditeur. Les lots agrègent cet effet et visent leur sélection finale. |
| `DocumentReplaced` | Nouveau, ouverture, fermeture ou restauration de récupération réussis. | Réinitialisation des caches et de la navigation de l'application. |
| `Saved` | Sauvegarde/sauvegarde sous ; sauvegarde finale explicite d'un lot. | Nettoyage de la récupération locale après succès, message et gestion du focus. N'annule pas un `Render` si le lot a aussi changé le document. |

Une sélection ordinaire, un style de frappe ou un lot sans effet sur le source
peuvent avoir des suites visuelles sans `Render`. `RevealSelection` n'explique
pas à lui seul toutes les demandes actuelles : les P.-S./P.J. et le clic sur un
résultat de recherche en ajoutent dans l'application.

## Ordre observé dans une frame

Sources : [ui_frame](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/mod.rs),
[columns_ui et editor_feedback](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/editor_view.rs).

1. Vérifier/interrompre l'IME et traiter une demande de fermeture de fenêtre.
2. Recevoir les exports, traiter les réponses de rendu et l'orthographe, puis les
   fichiers déposés. Une installation de rendu peut poser une demande de suivi.
3. Relever le focus d'historique, traiter raccourcis/menus natifs, préparer l'IME,
   calculer la modalité et afficher titre/menus/barre d'état.
4. Dans les colonnes, **dessiner l'aperçu avant l'éditeur**, bien que l'éditeur soit
   à gauche. L'aperçu peut consommer une demande ou traiter un double-clic.
5. L'éditeur dessine sa barre, les informations et la recherche ; prépare la
   projection/IME ; reçoit le résultat de `TextEdit`. Il peut sélectionner un
   résultat de recherche ou un terme, consommer la révélation éditeur, puis publier
   une saisie ou un déplacement natif. Les actions d'image sont traitées ensuite.
6. Afficher et appliquer les dialogues, opérations de fichier et récupération ;
   checkpoint de reprise, actualisation orthographique et publication du cache de
   sélection ; mise à jour des menus et éventuelle capture.

La même action, survenue avant ou après l'aperçu dans cette séquence, peut être
consommée dans cette frame ou dans la suivante. Un appel natif à
`commit_native_edit` ne renvoie pas d'`Outcome` : l'adaptateur doit encore signaler
la publication/sélection au futur coordinateur, sans inventer une seconde mutation.

## Séquences à conserver lors de l'extraction

| Séquence | Comportement observé |
| --- | --- |
| Frappe native avec déplacement du curseur | Publication validée, mise à jour du curseur, demande de suivi ; l'aperçu a déjà été dessiné et utilisera la nouvelle position à une frame ultérieure. |
| Sélection seule, même révision | Recalcul du contexte de frappe et demande de suivi. Aucun nouveau rendu n'est nécessaire si la garde du rendu est courante. |
| Texte modifié, ancien rendu encore peint | Pas de localisation/surbrillance sur ce rendu ; `forward_pending` reste posé. Après plus de 220 ms, l'application envoie la nouvelle demande si cette révision n'est pas déjà soumise. |
| Rendu valide reçu | Filtre de garde puis `install_render` ; textures et présentation mises à jour, erreur effacée, suivi avant réarmé. Il prend la sélection du moment, pas celle de l'instantané compilé. |
| Erreur courante ou rendu périmé | `sync_is_current` est faux : aucune boîte ni navigation inverse ; une demande avant n'est pas consommée. Un échec courant renseigne la révision traitée ; pas de nouvelle soumission automatique de la même révision par cette seule erreur. |
| Localisation sans point sur un rendu courant | Aucun défilement, mais demande avant consommée. Ne pas ajouter silencieusement un mécanisme de répétition. |
| Double-clic dans l'aperçu | Point en coordonnées Typst, `locate(ToSource)`, sélection gardée ; demande éditeur et annulation du suivi inverse. La colonne éditeur arrive ensuite et peut révéler la cible dans cette frame. Un simple clic ne déclenche pas ce parcours. |
| Nouvelle installation après navigation inverse | Une installation réussie réarme actuellement le suivi avant. La suppression du retour inverse n'est donc pas une interdiction permanente : l'ordre des événements compte. |
| Plusieurs sélections avant consommation | Le booléen fusionne les demandes ; la dernière sélection publiée est utilisée. Il faut distinguer cette chronologie de la garde documentaire, inchangée lors d'un simple déplacement. |
| Recherche au clavier | Le moteur fournit sélection et effet de révélation ; le panneau reprend son focus. La révélation de l'éditeur ne doit pas détourner ce focus. |
| Préédition IME | Projection provisoire et défilement IME locaux ; aucune préédition dans l'aperçu ni les compteurs. Le Commit passe par la validation commune ; une prédiction annulée ne doit pas revenir par un Commit tardif. |
| Remplacement ou fermeture | La garde d'identité change ; les anciennes réponses de rendu ne s'installent pas. Les demandes de la nouvelle situation et les caches sont réinitialisés ; si aucun document n'est ouvert, l'application vide les réponses reçues. |
| Lot de métadonnées ou sauvegarde | Préservation conditionnelle du focus ; un lot peut rester sans changement de source et de sélection. Une sauvegarde seule conserve les demandes antérieures sans en ajouter une ; un lot modifié et sauvegardé réarme le suivi. |

La garde complète du rendu et son identifiant sont déjà contrôlés par le cœur
([render.rs](../crates/core/src/engine/render.rs)). La transformation des points
Typst en pixels, le petit rectangle de visibilité de 8 × 32 pixels, le centrage,
le clip et les couleurs appartiennent à l'adaptateur. Les temporisations doivent
être distinguées de leur exécution : une politique partagée peut recevoir une
horloge ou un statut, sans posséder `egui::Context` ni lancer un thread.

## Couverture existante et cas à préparer

K.1 n'ajoute pas de tests de comportement : il identifie ceux qui protègent déjà
le code et les cas à couvrir pendant l'extraction.

| Preuve existante | Ce qu'elle contrôle | Repère de test |
| --- | --- | --- |
| Défilements et rendu périmé/échoué | Suivi des deux colonnes, refus de navigation inverse, consommation des demandes. | [preview.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/tests/preview.rs), `synchronization_scrolls_both_panes_and_rejects_outdated_or_failed_renders` |
| Double-clic et mise à l'échelle | Retour du focus vers l'éditeur depuis une page redimensionnée. | Même fichier, `double_click_on_the_scaled_preview_returns_focus_to_the_editor` |
| Surbrillances | Échelle, direction de sélection, absence de boîtes si cible/rendu indisponible. | Même fichier, `preview_paints_selection_at_page_scale_and_clears_it_when_unavailable` |
| Identité documentaire | Refus d'une ancienne réponse, y compris à révision égale ; acceptation du rendu courant. | Même fichier, `render_responses_are_guarded_by_document_identity_even_at_the_same_revision` |
| Recherche | Focus recherche conservé, cibles de session/révision, une sélection active, clic sur une autre occurrence et surbrillance de l'aperçu. | [search.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/tests/search.rs) |
| Annotations et orthographe | Navigation égale aux commandes JSON, focus et historique, refus des cibles d'une autre session. | [annotations_engine.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/tests/annotations_engine.rs), [spelling_engine.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/tests/spelling_engine.rs) |
| Lots et métadonnées | Lot sauvegardé, récupération, focus de métadonnées, erreurs sans édition. | [batch_engine.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/tests/batch_engine.rs), [information_engine.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/tests/information_engine.rs) |
| Fichiers et export | Sauvegarde sans réinitialisation indue, remplacement réussi, export de l'instantané demandé, récupération. | [files_engine.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/tests/files_engine.rs) |
| IME | Préédition inter-frames sans publication, interruption et Commit tardif, garde périmée, sauvegarde du seul texte validé. | [ime.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/tests/ime.rs) |
| Déclencheurs et disponibilité | Aiguillage commun, sélection gardée, commandes indisponibles et paramètres différés. | [routing.rs](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/tests/routing.rs) |
| Géométrie sans egui | Localisation/boîtes gardées, pas de mutation documentaire. | [render.rs du cœur](../crates/core/src/engine/tests/render.rs) |

Pour K.2/K.3, rendre ces règles testables sans egui et compléter notamment : attente
du rendu puis consommation unique ; intention plus récente à révision égale ;
localisation sans point ; effet des no-op et de `Saved` ; clic inverse puis installation
de rendu ; réinitialisation de document ; indépendance de deux coordinateurs de vue.
Les captures de référence viendront vérifier le branchement en K.3 ; un test pur
du coordinateur ne remplace pas le contrôle du focus et de la géométrie natifs.

## Choix à reprendre ensemble

Les points ci-dessous conservent les questions relevées en K.1. Les choix de
périmètre, contextes, concurrence, focus/navigation et interruptions ont depuis
été validés : D31 à D33 et la liste de phase 6 dans
[la feuille de route](https://github.com/klutchnikoff/typstOgraphe/blob/7b0d6fd3e6ae25e5704ed43ce2f4efefa9c2fa0b/ROADMAP-ARCHITECTURE.md#phase-6--assistance-à-la-composition-dans-lapplication-d14).
Le transport, le rattachement et la revalidation des décisions différées restent
à préciser. Les choix de phase 6 ne sont pas implémentés par K.2.

| Étape | Choix à préciser avant le code concerné | Base observée |
| --- | --- | --- |
| K.2 | Événements assez précis pour traduire `Outcome`, publication native, changement de sélection et résultat de rendu ; conserver l'ordre et distinguer changement de contenu et demande de révélation. | Quatre noms génériques d'événements ne décrivent pas seuls les compléments P.-S./P.J., les no-op, les erreurs et `Saved`. |
| K.2 | Limite entre navigation de vue et planification du service de rendu ; garder l'horloge et le réveil indépendants de la plateforme. | Temporisation 220 ms dans l'application ; coalescence déjà dans le worker du cœur. |
| K.2/K.3 | Conserver les règles actuelles ou proposer séparément certaines évolutions : suivi sur no-op, réarmement à chaque installation, absence de nouvelle tentative après erreur. | L'extraction n'autorise pas à simplifier ces conditions en les modifiant. |
| Phase 6 | À qui appartient la sélection courante lors d'une opération d'agent ? L'agent cible-t-il explicitement du texte sans déplacer durablement le curseur utilisateur, et quel style de frappe doit-il laisser ? | Une sélection et un style dans `Engine` ; le cache egui est republié avant les commandes et en fin de frame. Deux `ViewSync` ne créent pas deux curseurs du moteur. |
| Phase 6 | Quand une modification d'agent doit-elle révéler un passage ou prendre le focus ? Distinguer correction de texte, modification de métadonnées et demande explicite de navigation. | La voie actuelle est conçue pour un seul utilisateur local ; les politiques de focus dépendent du déclencheur et du champ actif. |
| Phase 6 | Reporter, refuser ou traiter une édition d'agent pendant une préédition IME ou un dialogue ? Les lectures peuvent-elles continuer ? | Le moteur refuse les brouillons périmés ; l'adaptateur sait annuler une composition. Cela ne décide pas à lui seul du confort d'une intervention distante. |
| Phase 6 | Comment revalider les décisions et cibles différées, dont une confirmation d'abandon, si l'état change entre l'affichage et l'action ? | Les cibles de lien/image/modèle/signature/export sont gardées ; `PendingAction` est une intention locale sans garde propre. |
| Phase 6 | Quel point d'entrée applique une commande et adapte une fois ses effets, pour les gestes locaux comme pour le canal d'agent ? | `execute_document_command` et les publications natives sont des voies distinctes ; export/récupération ont leurs services dédiés. Le moteur partagé et le refus d'une seconde ouverture du fichier sont déjà validés par D14/D27. |

L'inventaire ne choisit ni le transport local, ni une cible WASM, ni un protocole
d'éditeur web. Il ne remet pas en cause le registre typé, les lots atomiques et
les gardes déjà livrés. Le périmètre de K.2 est validé ; les questions techniques
de rattachement de phase 6 seront précisées après la coordination des vues.

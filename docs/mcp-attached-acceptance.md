# Recette MCP rattachée — lot 6.6, 25 octobre 2026

**Parcours validé sur macOS : un agent découvre le document ouvert par MCP,
propose des corrections, attend une décision humaine puis sauvegarde et exporte
le contenu accepté. La phase 6 est terminée.** La recette a aussi révélé et permis
de corriger un blocage du thread graphique pendant la production d’un PNG MCP.

## Dispositif et portée

- Propriétaire natif egui réel, moteur et adaptateur MCP réels, passerelle CLI stdio.
  Un courrier fictif et des répertoires neufs ; aucun profil/signature ou document
  personnel chargé. Les droits de sauvegarde et d’export sont accordés à cette
  seule invitation. Catalogue inchangé : 70 outils rattachés et cinq ressources.
- Agent lancé sans historique, sans skill ni lecture du dépôt. Il reçoit la consigne
  métier et la syntaxe d’un relais de transport mcpdo (Inspector officiel 2.12.0).
  Ses schémas, capacités et exemples viennent exclusivement du MCP. Il peut voir
  seulement les PNG retournés. Cette restriction est une consigne d’essai ; elle
  ne constitue pas un sandbox OS. Le serveur applique séparément ses permissions.
- Un pilote distinct joue l’utilisateur : sélection explicite, événements de texte
  egui, ouverture des préférences et décisions par le registre graphique. L’agent
  n’a accès ni au pilote, ni à son répertoire, ni à un outil d’approbation.
- La préédition japonaise est injectée par événements egui, puis validée séparément.
  Cela vérifie la frontière événement/éditeur/moteur ; ce n’est **pas** une nouvelle
  vérification physique de l’IME macOS. Linux et Windows ne sont pas testés
  graphiquement ici ; le suivi de leur CI est laissé à l’utilisateur.

Le pilote est compilé seulement avec `mcp-acceptance` ; aucun contrôle par fichier
n’est présent dans l’application ordinaire. Sa [notice](../experiments/mcp-acceptance/README.md)
précise le lancement, les rôles et les données à garder privées.

## Consigne et parcours

Corriger la salutation « Cheres collègues », les erreurs « pourrai », « charactère »
et « importnt » et les lourdeurs du courrier sur la production de blé. Garder un
registre sobre, l’incertitude sur la production future et la demande de retour
assez rapide. Ne pas inventer d’information ni renforcer le ton. Conserver les
informations du courrier et les ajouts humains en italique hors du passage confié.
Le texte de départ est conservé dans le pilote et dans les traces de recette.

| Étape | Constat |
| --- | --- |
| Découverte | Outils, ressources, guide et fiches consultés par l’agent, sans lecture du dépôt. |
| Rédaction simultanée | Ajouts humains publiés pendant le travail de l’agent ; curseur/style utilisateur indépendants de sa sélection. |
| Proposition privée | `pending`, `published=false` ; aucun changement de contenu publié, d’historique ni de fichier. Comparaisons natives clair/sombre inspectées. |
| Péremption | Nouvelle frappe → `stale` ; l’acceptation est indisponible et ne publie rien. L’agent relit puis prépare un nouveau lot. |
| Refus | Un lot valide est refusé côté interface ; source et historique inchangés. Son état est lu par l’agent. |
| Accord | Lot entier accepté, une seule étape d’annulation ; sélection reportée et style italique conservé. « Voir » révèle volontairement le passage. |
| Historique | Annuler/rétablir retrouve exactement les sources ; une frappe après accord s’annule avant le lot. |
| IME | `にほん` reste absent du publié en préédition. Lecture MCP autorisée, proposition sonde refusée immédiatement. Validation → texte publié une seule fois. |
| Modalité | Pendant les préférences : lecture autorisée, sauvegarde refusée ; aucune exécution différée à la fermeture. |
| Rendu | PNG lu par l’agent ; après correctif, une frappe pendant le calcul est publiée et son ancien résultat PNG refusé `stale_revision`. Une nouvelle demande stable réussit. |
| Fichiers | Validation sans diagnostic, sauvegarde explicite, export PDF + Typst + PNG ; après export `dirty=false` et révision inchangée. |
| Coupure | Révocation utilisateur, unique tentative agent → « Connection closed », sans reconnexion ni répétition. Document conservé. |

Quatre soumissions préparées avec succès : le premier devient périmé, le deuxième est
refusé, le troisième est accepté ; une seconde session après correction du PNG
confirme un nouveau lot accepté puis sauvegardé/exporté. La sonde pendant l’IME
n’a créé aucun lot. Les décisions restent toutes du côté du pilote utilisateur.

L’archive, `contenu.typ`, les métadonnées et le modèle exportés ont été comparés
au contenu publié : concordance exacte. L’export final se recompile avec Typst,
polices système ignorées et répertoires de packages/cache vides, sans téléchargement.
Les empreintes SHA-256 et tailles sont dans `verification.json`.

## Corrections livrées

1. **Transfert explicite de sélection (D32)** : commande enregistrée **Confier la
   sélection à l’agent**, menu Édition et palette, sans raccourci par défaut.
   Aucun suivi implicite des déplacements humains. Navigation directe `text.select`
   limitée au contexte agent, sans édition, historique, focus ni défilement humain.
   Ces actions sont bloquées pendant interruptions et proposition en attente.
   Les commandes de relecture sont aussi disponibles depuis la palette.
2. **Rendu hors UI** : compilation manquante et rasterisation PNG sur un worker,
   via `RenderTask`/`PageImageTask` immuables. Aucune session ou verrou de fichier
   supplémentaire. Revalidation au retour : garde complète, invitation et demande
   encore active. Une seule lecture coûteuse en vol ; son annulation garde le slot
   occupé jusqu’à la fin du worker. Les lectures légères restent disponibles.
   Cibles `Current` différées capturées au départ ; rendu natif déjà courant
   conservé à la fin de compilation. Validation typée des paramètres maintenue.
3. **Diagnostic reproductible** : exemple natif opt-in, contrôleur utilisateur et
   relais de transport journalisés. Construction du diagnostic vérifiée en CI,
   sans lancer de fenêtre sur les runners.

Le premier lot remplaçait aussi la fin en italique : la comparaison l’a montré.
La sélection confiée suit les opérations du document selon l’affinité déjà
[documentée](editing-contexts.md#report-exact-des-positions) : une insertion
exactement à sa borne finale agrandit la sélection. Ce n’est pas une photographie
immuable du texte. Le passage a été confié de nouveau puis ciblé plus précisément,
et les lots suivants ont conservé les ajouts et leur style. La qualité sémantique
d’un agent reste à relire ; la recette valide le mécanisme d’approbation, pas une
correction linguistique automatiquement fiable.

## Mesures et limites

Mesures ponctuelles d’un build **debug** macOS, courrier d’une page avec japonais ;
ce n’est pas un benchmark interplateforme ni une mesure de frappe physique.

| Mesure | Résultat |
| --- | --- |
| Avant correctif, deux appels PNG | 4,421 s et 3,301 s ; frames correspondantes d’environ 4,122 s et 3,029 s. |
| Après correctif, PNG stable | 3,344 s sur le worker. |
| Échantillon stable après correctif | 3 429 frames ; médiane 11,41 ms, p95 18,07 ms, maximum 114,00 ms. |
| Traitement MCP dans cet échantillon | Maximum relevé 5,64 ms. |
| Frappe injectée pendant un autre PNG | Accusé de frame reçu en 78 ms, texte publié ; ancien PNG refusé. Inclut le protocole de contrôle et sa scrutation. |

La production du PNG reste coûteuse, mais ne s’exécute plus sur le thread graphique.
Les traces complètes conservent également des pointes de frame plus élevées lors
de phases non isolées, hors du point de traitement MCP mesuré. Cette recette ne
certifie pas la performance générale de l’éditeur et n’attribue pas ces pointes
à une cause non mesurée. Les corpus avant/après diffèrent par les ajouts humains.

## Vérifications et preuves

Validation locale : **553 tests Rust, deux doctests, 14 tests Python**, Clippy
workspace/toutes cibles avec le diagnostic activé et `-D warnings`, formatage,
frontière des dépendances du cœur, limites des modules et syntaxe Python.
Les nouveaux tests couvrent contexte indépendant, véritable SDK rmcp,
compilation/géométrie/PNG pendant interruption, arrivée tardive après frappe,
annulation et slot borné, cache natif conservé, cible capturée, diagnostic de
compilation et paramètres invalides. Les contrôles existants de propositions,
ViewSync, sauvegarde/export et fermeture/remplacement restent exécutés.

Preuves locales conservées, ignorées par Git :

- `output/phase6-6-acceptance/before-fix/` et `after-fix/` : traces agent et utilisateur,
  captures natives, archives et exports.
- `after-fix/typing-during-render.json`, `stable-render-measurement.json` et les
  événements du transport : mesures et correspondances de révision.
- `verification.json`, `offline-recompiled.pdf`, `workspace-tests.log`, `clippy.log`.

Aucun jeton, fichier d’invitation, configuration privée de daemon ou préférence
personnelle n’est copié dans ces preuves ou versionné. Les fenêtres et daemons
de diagnostic sont arrêtés après révocation.

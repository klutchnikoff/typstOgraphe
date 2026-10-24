# Lectures documentaires et découverte du moteur

Complément de phase 2 du 24 octobre 2026 : cinq requêtes ajoutées et deux réponses
enrichies. Le [catalogue](commands.md) et ses [schémas d’entrée](commands.json)
proviennent du registre Rust. Aucun transport CLI/MCP n’est livré ici.

## État publié et gardes

Extension additive de phase 4 : `document.inspect` expose `read_only` ;
`document.open` accepte `read_only: true` pour lire un instantané sans verrou
d’écriture. La disponibilité des mutations reflète ce mode. Voir
[les protections de fichiers](architecture-phase-4.md).

Toutes ces lectures concernent le document publié ; le brouillon IME reste privé
à l’adaptateur jusqu’à validation. Elles ne changent ni source, métadonnées,
sélection, style de frappe, état modifié, révision, historique ou fichiers.
`render.compile` peut alimenter le cache de rendu dérivé.

`document_types.list`, `document.source`, `document.assets`, `commands.list` et
`document.inspect` prennent `{}`. Les lectures de source et de ressources exigent
un document ouvert ; la découverte et l’inspection restent disponibles à l’accueil.

Les deux lectures coûteuses `document.validate` et `render.compile` exigent
`{ "guard": { "document_id": ..., "revision": ... } }`, obtenue par inspection.
Les gardes étrangères ou périmées sont refusées avant les calculs ; une session
fermée produit `no_document`. Aucun réancrage silencieux n’est effectué.

## Types, source et ressources

`document_types.list` renvoie `types`, dans l’ordre du catalogue intégré. Chaque
entrée fournit `id`, `can_create`, `models` et `capabilities`. Chaque modèle
fournit `id` et `fixed_page` (`{ paper, landscape }` ou `null`). Le compte-rendu
existe comme type mais reste sans modèle fourni : sa création est indisponible.
Les identifiants sont techniques, sans dépendance à la langue de l’interface.

`capabilities` est la liste des identifiants des commandes documentaires du type
admissibles comme étapes de lot. Elle décrit son vocabulaire ; elle ne signifie
pas que chaque commande est disponible à la position courante. Fichiers,
historique, services globaux et requêtes n’en font pas partie. Le registre commun
et les règles des commandes de Courrier sont la source de cette liste.

`document.source` renvoie `{ guard, source }`. `source` est le corps documentaire
Typst (`contenu.typ`), distinct du texte visible, du modèle et de la composition
`main.typ`. La requête n’exécute aucun code fourni par l’appelant.

`document.assets` renvoie `{ guard, assets, dependencies }`. Les deux listes
contiennent des objets `{ path, bytes }` : chemin interne et taille en octets,
sans contenu binaire ni chemin personnel d’origine. L’ordre est celui des chemins.
Les dépendances comprennent les fichiers des capacités et packages embarqués.

L’inventaire des images et signatures applique exactement la politique de
rétention de sauvegarde/export : images du corps, signature et ressources du
modèle, y compris les chemins déclarés dans des branches inactives ou calculés.
Il ne parcourt pas les instantanés d’annulation. Les images devenues inutiles
disparaissent de la liste sans être détruites en mémoire : Annuler peut les restaurer.
Comme la sauvegarde, il conserve prudemment les ressources candidates si une
erreur de compilation empêche de démontrer qu’elles sont inutiles. Il ne prétend
donc pas produire une analyse exacte de tous les chemins d’un programme Typst invalide.

## Inspection et disponibilité

`document.inspect` conserve garde, sélection orientée, état d’ouverture,
chemin, récupération, état modifié, undo/redo et rendu courant. Il ajoute :

- `document_type` : identifiant du type actif ;
- `capabilities` : vocabulaire documentaire du type, défini ci-dessus ;
- `typing_style` : `{ bold, italic, link }`, style actuel de frappe du moteur.

Sans document ouvert, ces trois champs et `selection` valent `null`. La garde
reste accessible pour protéger une création ou une ouverture depuis l’accueil.
Le style de frappe n’est pas un résumé des styles mélangés d’une sélection.
La lecture exhaustive des styles dans `document.structure` reste une extension.

`commands.list` renvoie `guard`, `selection` et `entries`. Garde et sélection
valent `null` sans document. Chaque entrée conserve la définition statique du
registre et ajoute `context_availability` :

- `{ state: "enabled" }` : disponible avec ses paramètres par défaut ;
- `{ state: "disabled", reason: { code, params } }` : indisponible dans le contexte,
  avec la même erreur structurée que les contrôles du moteur ;
- `{ state: "requires_parameters" }` : aucune interdiction identifiée dans le
  contexte, mais les arguments obligatoires et leurs validations restent à fournir.

Pour les commandes entièrement déterminées par leurs défauts, la lecture appelle
`Engine::available`. Les dialogues préparatoires utilisent `available_intent`
(lien, insertion d’image, export, fichiers, modèle) ; les autres appels paramétrés
appliquent les règles de portée connues. Aucun chemin, texte ou ressource fictif
n’est fabriqué pour simuler un appel complet. Une commande de récupération peut
ainsi rester `requires_parameters` jusqu’au contrôle de sa référence et du service.
L’exécution revalide toujours l’appel réel.

Cette réponse est contextuelle et ne constitue pas le fichier de référence
reproductible : `registry::catalogue_json()` et les options
`--commands-catalogue[-json]` produisent le catalogue statique sans état de session.
Il n’y a pas de filtre paramétrable par type dans ce lot ; les capacités et les
raisons d’indisponibilité permettent déjà d’identifier les différences entre types.

## Validation

`document.validate` renvoie `{ guard, valid, ready_for_export, diagnostics }`.
Il réutilise la rétention et la validation du stockage, valide le contrat du
modèle avec Typst, puis contrôle les options réelles et les champs requis à l’export.

Un diagnostic comporte `severity` (`error` ou `warning`), `code`, et éventuellement
`field` (identifiant anglais), `detail` (texte technique non contractuel) et
`cause` (erreur typée, quand disponible). Depuis le lot 3.2, l’interface traduit
la cause structurée ; le moteur ne produit plus de phrase Fluent. Les entrées
de `recovery.list` peuvent également ajouter une `cause` à leur champ `error`.
Les codes de ce lot sont `asset_inventory`, `document_storage`, `model_validation`,
les raisons des contrôles d’options (`model_fixed_page`, `bullet_marker_invalid`,
`bullet_marker_policy`, `model_presentation`) et `export_required_field`.

Une erreur rend `valid` faux. Un champ d’export absent est un avertissement : un
brouillon vide peut être valide mais pas prêt à exporter. `ready_for_export` exige
à la fois l’absence d’erreurs et tous les champs requis renseignés.
Le rapport agrège les contrôles ; une bibliothèque peut ne fournir que sa première
erreur dans une catégorie. Il ne garantit pas que toutes les erreurs d’un modèle
invalide seront découvertes en un seul passage. Il n’installe pas de rendu.

## Compilation sans interface

`render.compile` renvoie `{ guard, render, diagnostics }`. Sur succès, `render`
est `{ reference: { guard, id }, pages }` et `diagnostics` est vide. Les dimensions
des pages sont en points Typst ; la référence fonctionne immédiatement avec
`render.locate` et `render.selection_boxes`, sans passage préalable par l’application.

Sur erreur Typst, `render` vaut `null` et un diagnostic `error / render_compile`
fournit un détail. Les erreurs de paramètres ou de garde restent des
`CommandError`, distinctes des erreurs de compilation rapportées.

La requête est synchrone ; un futur transport pourra l’exécuter sur son worker.
Elle réutilise la compilation du worker natif, sans rasteriser les pages ni écrire
un PDF. Un rendu déjà courant est réutilisé avec le même identifiant. Une édition
invalide sa garde ; une nouvelle compilation installe une nouvelle référence.
Le worker graphique conserve son fonctionnement asynchrone et peut remplacer
le cache par un rendu de la même révision : la dernière référence installée fait foi.
Un seul rendu est détenu par le moteur, libéré à la fermeture ou au remplacement
du document. Rien de ce cache n’est archivé.

## Vérifications

Les suites [discovery.rs](../crates/core/src/engine/tests/discovery.rs) et
[validation.rs](../crates/core/src/engine/tests/validation.rs) couvrent découverte à vide,
types et paramètres, contexte de frappe, inventaire et annulation, brouillon
valide/incomplet, modèle/ressources/options invalides, gardes refusées,
géométrie sur plusieurs pages, réutilisation du cache, erreurs et préédition privée.
Les tests de rendu natif existants vérifient toujours les références remplacées
et les réponses tardives. Les 68 entrées ont leurs schémas stricts et tests
d’aller-retour ; les requêtes restent exclues des lots d’édition.

Le [lot 3.2](architecture-phase-3.md#lot-32--erreurs-et-libellés-sans-fluent)
détaille les causes sérialisées et leur traduction. `CommandError` garde ses codes,
raisons et détails ; `params.cause` est un ajout compatible, sans modification des
schémas d’entrée ou de la disponibilité des opérations.

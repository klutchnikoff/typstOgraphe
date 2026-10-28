# Socle du contrat structuré — premier lot A.2

Livré le **28 octobre 2026**, selon les
[décisions validées](structured-contract.md). Ce lot installe les fondations ;
il ne termine pas la migration des adaptateurs.

## Ce qui est en place

- `content::contract` possède désormais `Settings`, `Snapshot`, l'état explicite
  `State` versionné, `Guard` et les erreurs structurées. Tauri réutilise les
  mêmes valeurs au lieu de définir ses propres réglages et instantanés.
- `content::shared` utilise Yrs 0.28.0, avec `small-client`, `sync`, offsets
  UTF-16 et rétention des contenus supprimés. Le cœur ne dépend ni de Tauri,
  ni de Fluent, ni d'un transport ou serveur collaboratif.
- La projection Rust lit corps, tableaux, notes, citations, descriptions,
  images/légendes, champs simples et collections d'en-tête : textes partagés,
  fragments riches, ordre des enregistrements, références et options.
  Les descripteurs des types fournissent les liaisons des champs et collections.
- Un nouveau projet peut être initialisé en Rust, puis ouvert dans le vrai
  éditeur. Une édition d'un état existant conserve les identités et les racines
  privées de la liaison Tiptap.
- `SharedSession::prepare` travaille sur un candidat isolé et valide l'état
  final du lot. `publish` recontrôle identité/révision avant publication. Un refus
  laisse l'état confirmé intact ; la livraison répétée d'une même mise à jour
  n'ajoute pas de révision. Les mises à jour ayant des dépendances absentes sont
  refusées à cette frontière de publication.

Les erreurs ont un code sérialisable (`wrong_document`, `stale_revision`,
`invalid_shared_state`, `incomplete_shared_state`, `inconsistent_state`, etc.).
Les diagnostics détaillés de certains validateurs existants restent des chaînes ;
leur migration fine et leur présentation localisée accompagnent les commandes.

## Conservation cohérente

La sauvegarde et la lecture `.typsto`, ainsi que les copies de récupération,
contrôlent la concordance entre le contenu Yjs et l'arbre/réglages sauvegardés.
Les caches Typst ne suffisent plus à rendre acceptable une archive dont l'état
partagé décrit autre chose. Un refus ne remplace pas le fichier existant.

Un projet neuf sans état partagé est initialisé à l'enregistrement depuis ses
valeurs structurées. Un état Yjs non vide mais invalide ou incohérent est refusé ;
il n'est pas reconstruit silencieusement. La comparaison tolère les attributs
implicites de l'éditeur et le découpage équivalent des portions de texte, sans
ignorer un changement de contenu ou d'attribut significatif.

## Conservation des styles des annotations

Les recettes ont révélé un défaut antérieur : la liaison Yjs conserve les marques
des textes, mais sérialise les éléments en ligne non textuels par leurs attributs.
Le style d'une note/citation pouvait rester dans ProseMirror sans être partagé.

`SharedInlineMarks`, commun aux profils riches, conserve ces marques dans un
attribut privé de liaison et les restitue lors des mises à jour distantes.
L'extension limite son parcours à la région modifiée ; une restauration depuis
une mise à jour distante ne produit pas une deuxième édition partagée.
`documentContent` fournit les valeurs documentaires publiques, sans cet attribut
privé ; instantanés, bibliographie et champs riches passent par cette conversion.
Le lecteur et l'initialiseur Rust utilisent la même représentation.

## Vérifications

- Tests du cœur : initialisation des trois types intégrés, Unicode, garde
  périmée/étrangère, lot invalide, validation finale d'un lot, idempotence,
  dépendances manquantes, en-tête riche et conservation des identités.
- Tests de fichiers : initialisation du Yjs à la sauvegarde, refus d'une archive
  dont les caches Typst sont valides mais le Yjs incohérent, protection du fichier
  existant et récupération après incident.
- Recettes d'interopérabilité : Yjs/Tiptap réels vers Rust et retour, Courrier et
  Article, auteurs/résumés, styles sur les notes/citations, projection distante,
  annulation et ouverture d'un état initialisé par Rust.
- Compilation web et contrôles de format ; Clippy et tests du workspace ; garde
  des dépendances du cœur. Recettes navigateur sans captures, vidéos ni traces.

Relevé local : **147 tests d'interface**, 3 tests des déclarations natives de
langue et **7 recettes d'interopérabilité** passent. Tous les tests Rust du
workspace passent également, avec le diagnostic de performance déjà ignoré.
Les 152 parcours navigateur ont été exercés ; les trois échecs bibliographiques
ont conduit à corriger leur appel de validation, puis les six parcours de ce
fichier ont été rejoués avec succès sur Chromium et WebKit.

L'exemplaire local de `examples/article-working.typsto` a également été lu,
recopié, rouvert et compilé hors ligne en **3 pages**, sans modifier son original.

Lancer les recettes croisées depuis la racine :

```sh
npm run test:contract
```

Le lanceur construit le binaire natif puis exécute les tests Vitest dédiés. La
CI desktop les exécute sur macOS, Windows et Linux. Le relevé local est effectué
sur macOS ; la réussite distante des autres systèmes se lit dans la CI.

## Suite du chantier

Le [lot suivant](structured-commands.md) ajoute les premières commandes typées
communes, leurs cibles multizones et leur disponibilité dans le cœur. Les autres
commandes, requêtes et l'historique restent à migrer. Le relais de frappe actuel n'est pas encore
remplacé par `SharedSession` : ce remplacement doit livrer ensemble confirmation,
refus et conservation du brouillon, puis l'activation à la demande des services
de présence/collaboration. Le mode individuel allégé est une décision validée,
pas une fonctionnalité achevée par ce lot.

La CLI et MCP autonome utilisent encore le moteur historique. Les propositions
rattachées et l'acceptation de leurs lots doivent également rejoindre cette
nouvelle publication. Les gardes et la préparation Yrs de ce lot en fournissent
le socle ; la pile d'annulation des commandes et le parcours de relecture ne sont
pas déclarés migrés.

La préparation isolée utilise actuellement une copie encodée/décodée du document.
Les futures mesures de la frappe doivent couvrir le parcours complet, incluant
projection et validation ; les seuls chiffres de la sonde Yrs ne les préjugent pas.
Les décisions de rétention des snapshots durables restent dans A.4.

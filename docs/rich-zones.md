# Zones riches — profils et création commune

Contrat du 27 octobre 2026. Le point d’entrée est `app/src/rich-zone.ts`.
Les vues passent par `createRichZone` ; elles n’instancient pas Tiptap et
n’assemblent pas leurs propres listes d’extensions.

| Profil | Contenu | Retours | Ligne de reprise finale |
| --- | --- | --- | --- |
| `document` | Corps, titres, widgets et annotations documentaires | Retour simple dans le paragraphe ; double retour pour le suivant | Oui |
| `rich-text` | Paragraphes, gras, italique, liens ; résumé notamment | Même saisie et normalisation que le corps | Non imposée |
| `inline-text` | Un paragraphe avec mise en forme en ligne ; titre notamment | Entrée et Maj+Entrée ne créent pas de paragraphe | Non |

Le profil porte les règles de contenu et de saisie. La fabrique fournit aussi les
attributs accessibles, la langue passée par l’appelant, la politique de correction
native, les raccourcis de liens et de contexte, les protections pendant une
composition IME et le raccordement à l’historique. Le shell délègue les raccourcis
de texte à la zone identifiée par son profil ; il ne reconnaît plus une classe
particulière de la présentation des métadonnées.

La normalisation des paragraphes est commune au document et au texte riche :
pas de ligne vide avant le premier contenu, pas de retours vides répétés et
réutilisation d’un retour déjà présent devant le curseur. La ligne de reprise
finale est une option explicite du profil document. Elle ne s’ajoute pas aux
résumés, qui ne contiennent pas de widgets.

## Données et historique

La liaison est indépendante du profil :

- sans liaison, l’éditeur a son historique local Tiptap ;
- `document` relie le fragment principal d’un `Y.Doc`, avec son historique
  collaboratif et, si fournie, la présence des participants ;
- `fragment` relie un champ `Y.XmlFragment` à l’historique du document propriétaire.
  La fabrique vérifie qu’ils appartiennent au même `Y.Doc` et couvre le fragment
  dans la portée d’annulation si aucun ancêtre n’est déjà couvert. Fermer le
  champ ne détruit pas cet historique. Les commandes standard `undo` et `redo`
  et leurs raccourcis utilisent ce propriétaire, sans second gestionnaire.

Le schéma utilisé pour initialiser les fragments Yjs, lire le contenu et produire
les vues en lecture seule vient de `richZoneExtensions`, avec les mêmes profils
que la création interactive. `richFieldProfile` traduit les genres de champs
`rich` et `rich_inline` du contrat documentaire ; aucun type particulier ne
redéfinit ces comportements.

```ts
const zone = createRichZone({
  profile: richFieldProfile(field.kind),
  element: mount,
  label: field.label,
  language,
  binding: { kind: 'fragment', fragment, historyOwner: documentEditor },
  openLink: (editor) => properties.open(editor.state.selection.from, 'link'),
})
```

Les vues conservent leurs responsabilités de présentation : libellés, barre de
mise en forme, cadre, choix du bandeau et actions métier. Le corps fournit sa
résolution contextuelle (badge actif, orthographe, paramètres du widget) ; une
zone éditoriale fournit son ouverture du lien dans le bandeau commun. Ces
callbacks ne réimplémentent pas les touches clavier.

## Extension du dispositif

Pour un nouveau champ riche, déclarer son genre dans le schéma du type : les
composants de métadonnées choisissent alors son profil automatiquement. Pour
une nouvelle vue, appeler la fabrique. Une nouvelle règle commune doit être
ajoutée au point central ou à une extension commune existante, jamais recopiée
par une vue. Un nouveau profil n’est justifié que par une différence de contenu
ou de comportement explicite.

Les widgets du corps (cellules, légendes, attributions…) restent des zones de
contenu du même arbre ProseMirror ; ils héritent du profil document et ne sont
pas transformés en éditeurs indépendants. La vue de préparation des propositions
MCP utilise aussi la fabrique, sur une copie privée du `Y.Doc`. La relecture HTML
utilise le schéma du profil document, sans créer un autre éditeur interactif.

Ce lot centralise la création et les comportements déjà disponibles. Il ne
raccorde pas encore l’orthographe documentaire ni la synchronisation de l’aperçu
aux champs riches de l’en-tête et ne change pas le contrat CLI/MCP.

## Vérifications

Les tests couvrent les profils, les schémas de sérialisation, le collage, les
retours et les marques, l’IME, les liaisons d’historique, leur durée de vie et le
refus d’un propriétaire étranger. Un contrôle de structure interdit une nouvelle
construction `new Editor` ou un assemblage StarterKit ailleurs que dans la
fabrique. Les recettes navigateur utilisent aussi une zone indépendante de
l’en-tête, pour vérifier que les raccourcis n’altèrent pas le corps du document.
Les parcours Article et les bandeaux continuent de tester leurs vues réelles.

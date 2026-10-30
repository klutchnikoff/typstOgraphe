# Inventaire des transformations — A.2

État relu le **30 octobre 2026**, après la fusion de paragraphes et les améliorations
de tableaux et de badges. Cet inventaire remplace la rubrique ouverte
« transformations supplémentaires ». Il ne clôture pas A.2 et ne décide pas de
nouvelles signatures publiques.

## Périmètre et méthode

Lecture des commandes exécutées dans `content::commands`, des résolveurs Yrs,
des profils riches et des gestionnaires Tiptap, puis des recettes unitaires,
de contrat Rust ↔ Yjs et navigateur existantes. Les références utiles sont
[les commandes](structured-commands.md), [les adaptateurs](structured-adapters.md),
[les contextes](structured-editing-contexts.md) et [la fusion de paragraphes](paragraph-join.md).

Deux voies sont intentionnelles :

- Une action explicite (bouton, palette, bandeau, CLI/MCP) prépare un lot typé
  et le propriétaire Rust le publie sous garde.
- La frappe, l’IME, les règles textuelles et les gestes de saisie immédiate
  produisent une transaction locale, puis un delta confirmé par le même
  propriétaire. Leur affichage ne doit pas attendre un aller-retour natif.

La seconde voie n’est pas un ancien moteur à retirer. Elle doit respecter les
mêmes contenus, frontières, règles d’historique et refus. Une validation commune
du résultat ne prouve cependant pas, à elle seule, que deux transformations
réalisent la même opération ou préservent les mêmes identités.

## Ce qui est déjà raccordé

| Famille | Voie commune et état | Travail supplémentaire dans cet inventaire |
| --- | --- | --- |
| Texte littéral, suppression dans une zone, gras, italique, liens | Commandes textuelles typées ; styles de frappe dans le contexte privé | Pas de migration supplémentaire |
| Paragraphe et titres | Conversions `block.paragraph`, `block.heading_*`, profils riches partagés | Pas de migration supplémentaire |
| Séparation et fusion de paragraphes | `block.split`, `block.join`, comparaison Rust/Tiptap, sélection attachée à l’historique | Distinguer ces opérations des retours en ligne : R1 |
| Espaces et lignes vides | Politiques locales et Rust, réparation sans réécriture globale, recettes croisées | Étendre les scénarios d’entrée native : R3 |
| Insertion, conversion, réglages et suppression des widgets | Planification locale traduite en `block.splice`, `block.remove`, `node.set_attrs` | Gestes internes aux listes : R2 |
| Notes et citations bibliographiques | `inline.replace`, attributs publics, suppression d’une unité, bandeau commun | Pas de migration supplémentaire des badges |
| Tableaux et images | Figures, légendes, lignes/colonnes, rôles d’en-tête, ressources immuables | Presse-papiers : R3 ; fusion de cellules hors A.2 |
| Recherche/remplacement et orthographe documentaire | Lots textuels par zones et exceptions gardées | Pas de migration supplémentaire |
| Champs et collections d’en-tête | Champs typés, enregistrements identifiés et ordonnés, profils riches | Vérifier les restrictions de collage selon le profil : R3 |
| Ressources, signature, options et modèle | Commandes gardées, admission des ressources avec le lot | Pas de migration supplémentaire |
| Annuler/rétablir, sélections, styles privés et disponibilité | Propriétaire et contextes communs, déclencheurs raccordés | À conserver dans les recettes des trois points ci-dessous |

## R1 — Retours en ligne dans les commandes structurées

**Constat.** L’éditeur distingue `softBreak` (retour simple, espace dans Typst),
`hardBreak` (retour forcé) et séparation de paragraphes. `block.split` couvre la
dernière opération. `text.insert` refuse les caractères CR/LF dans une zone riche,
et `inline.replace` n’accepte actuellement que les notes et citations.

Les adaptateurs peuvent construire un paragraphe contenant un retour par une
commande de sous-arbre. Il manque une opération localisée pour l’insérer sans
remplacer le paragraphe entier. Le problème est la granularité et la préservation
des identités, pas la possibilité de représenter le document.

**Choix à valider avant codage.** Étendre une commande en ligne existante ou
introduire une commande dédiée, avec distinction explicite simple/forcé. Ne pas
donner une interprétation implicite de la touche Entrée à `text.insert`.

**Sortie attendue.** Comparaison de l’opération locale et de l’opération Rust dans
le corps, les cellules, citations, légendes et résumés autorisés ; refus dans les
titres d’en-tête sur une seule ligne et les champs bruts concernés. Vérifier
Unicode, badges/styles voisins, identités conservées, normalisation des retours,
garde et annulation. Ce lot ne modifie pas les règles de retours déjà validées.

Sources : `app/src/editing.ts`, `app/src/rich-zone.ts`,
`crates/core/src/content/shared/commands.rs`, `shared/objects.rs` et `shared/structure.rs`.

## R2 — Transformations à l’intérieur des listes non imbriquées

**Constat.** Création, changement de type et suppression du widget passent par les
commandes communes. En revanche, la séparation d’un item ordinaire par Entrée
vient de `ListItem` de Tiptap ; Retour arrière, la sortie d’un item vide et les
descriptions ont des gestionnaires locaux spécifiques. Le delta final est validé
par Rust. `block.split` sépare des paragraphes, pas deux items ; `block.join` ne
joint pas deux items situés dans des conteneurs distincts.

Les commandes génériques peuvent insérer, remplacer ou retirer des items par
lots. Elles ne définissent pas encore le même contrat de séparation/jointure
avec conservation des identités que les gestes de l’éditeur. Par ailleurs,
Suppr en fin d’item ne dispose pas d’un pendant explicite du gestionnaire de
Retour arrière : cette asymétrie demande un choix, pas une correction silencieuse.

**Choix à valider avant codage.** Définir séparation d’item, jointure et sortie
d’item vide, puis choisir commandes dédiées ou composition des opérations
existantes. Distinguer jointure de paragraphes internes et jointure d’items ;
préserver les frontières terme/définition. Préciser le comportement de Suppr et
celui des items contenant plusieurs paragraphes. Aucun nouvel identifiant public
n’est arrêté dans cet inventaire.

**Sortie attendue.** Même résultat pour les opérations équivalentes Rust/Tiptap,
une annulation, curseur à la jonction et absence de reconstruction des autres
items. Conserver les règles validées : premier item protégé, retour au texte
précédent sans déballer le widget, sortie d’un item vide vers la ligne de reprise.
Les listes imbriquées restent exclues.

Sources : `app/src/editing.ts` (`backspaceInList`, `DocumentInput`),
`app/src/widgets.ts` (`widgetKey`), `app/src/document-zone-extensions.ts`,
`crates/core/src/content/shared/structure.rs` et `shared/join.rs`.

## R3 — Entrées natives et sélections traversant des structures

**Constat.** Le presse-papiers et le glisser-déposer éditorial utilisent pour
l’essentiel le traitement de ProseMirror/Tiptap, suivi de la validation commune.
La suppression d’une sélection non réduite au curseur suit aussi les mécanismes
de l’éditeur ; les protections de jointure au curseur n’en définissent pas le
contrat complet.

Certaines recettes nommées « collage » passent par `setContent` ou
`insertContent`. Elles vérifient bien l’import de contenu et la normalisation,
mais n’exercent pas les mêmes événements que le vrai presse-papiers. Le résultat
de ces tests ne doit donc pas être présenté comme une recette complète de collage.

**Travail borné.** Compléter la matrice de comportement et les recettes par de
véritables entrées de collage/coupe et remplacements de sélection :

- texte multiligne et HTML avec styles, liens, retours et espaces ;
- remplacement sur plusieurs paragraphes, autour d’un badge et à une frontière
  de widget ; séparation du corps et de l’annotation, terme/définition ;
- sélection de cellules et collage de tableau, sans fusion de cellules ;
- corps, résumé riche, titre sur une ligne et champ brut, selon leur profil ;
- déplacement interne de texte ou de blocs si proposé par l’éditeur natif ;
- ressource ou référence absente : refus conservant le travail confirmé et le
  brouillon, sans import ou téléchargement implicite ;
- annuler/rétablir et seconde fenêtre, sans déplacement du focus distant.

Confronter le contenu produit aux mêmes commandes de zones/sous-arbres et à la
validation Rust. Il n’est pas nécessaire de créer une commande publique « paste »
pour chaque geste du système. La prise en charge des nœuds non admis et les
remplacements traversant des frontières doivent être explicités et validés avant
toute modification de comportement. Les imports bureautiques et les images depuis
le presse-papiers ne deviennent pas de nouvelles fonctionnalités de ce lot.

Sources : `app/src/rich-zone.ts`, `app/src/document-whitespace.ts`,
`app/tests/whitespace.test.ts`, `app/tests/browser/tables.spec.ts`,
`crates/core/src/content/shared/mod.rs` et `shared/structure_tests.rs`.

## Hors de cette liste

- Services individuels/collaboratifs à la demande : prochain lot A.2 distinct.
- Conservation durable et réimportation des brouillons refusés : A.3.
- Snapshots : A.4 ; listes imbriquées, fusion de cellules et nouvelles capacités : C.
- Extension de la relecture MCP à l’en-tête, ressources et fichiers : parcours
  à concevoir séparément, pas un contournement des autorisations actuelles.
- Tout raccourci textuel n’a pas à devenir une seconde interprétation Markdown
  ou Typst côté CLI/MCP : ces adaptateurs disposent des opérations explicites.

## Mode individuel — exigence de transition retenue

Le prochain lot conserve le moteur Yjs/Yrs et les mêmes fichiers. Seuls les
services collaboratifs facultatifs doivent rester inactifs quand ils ne sont
pas utiles ; la liaison éditeur ↔ propriétaire Rust continue de valider le travail.

**Depuis un document en cours, l’activation de la synchronisation doit être une
action explicite en un clic**, sans conversion du projet, fermeture/réouverture,
copie du document ni réinitialisation du contenu. Conserver son identité, son
historique, ses ressources, ses contextes et la position du curseur. L’activation
ne doit pas créer une étape d’annulation documentaire.

Traiter les confirmations en attente sans perdre la frappe ; ne jamais publier
une préédition IME, un formulaire privé ou un brouillon refusé. Une impossibilité
d’activer les services conserve le document et explique l’échec. Les recettes
devront couvrir activation après une longue rédaction, services déjà activés,
participant rejoignant ensuite, reprise de l’édition et annulation d’une frappe
antérieure à l’activation.

La présentation de cette action, son articulation avec plusieurs fenêtres et le
rattachement d’un agent restent à préciser avant implémentation. Ce contrat ne
choisit ni serveur distant ni ouverture de réseau ; partage et hébergement restent
dans G. Mesurer séparément le coût de la frappe/confirmation et celui de Typst.

## Vérifications de l’inventaire

Relancés localement sur macOS, sans captures : **50 tests unitaires ciblés**
(`paragraph-join`, `whitespace`, `horizontal-space`, `rich-zone`) et les
**24 recettes de contrat Rust ↔ Yjs/Tiptap** : réussis. Cette passe est un audit
du code et de ses garanties existantes ; elle ne prétend pas avoir exécuté les
nouvelles recettes R1–R3 ni les presse-papiers natifs des trois systèmes.

La clôture d’A.2 doit nommer le résultat des trois points, livrer le mode
individuel et conclure la recette Courrier/Article entre les adaptateurs.

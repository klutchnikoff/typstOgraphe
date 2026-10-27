# Inventaire avant retrait d’egui — archive

**Exécution :** le retrait et les reprises sont documentés dans le
[bilan du nettoyage](egui-retirement-results.md). Le texte ci-dessous conserve
le diagnostic préalable ; ses chemins egui renvoient à la référence Git.

Audit du **27 octobre 2026**, après A.1. Inventaire fondé sur le code et les
recettes existantes, à valider avant implémentation. Aucune suppression ni
correction fonctionnelle n’est réalisée par cet inventaire. Les tests A.1
démontrent leurs parcours précis, pas une parité exhaustive des deux interfaces.

**Orientation validée le 27 octobre 2026 : aucune capture d’écran pour le moment.**
L’interface va encore évoluer après le nettoyage. Ne pas migrer l’outillage de
capture egui, créer de références d’images, comparer des pixels ou faire des
captures une condition du retrait. Les tests fonctionnels web et natifs restent
utiles. Retirer ou désactiver la production automatique de captures des recettes
actives ; les rapports historiques restent des archives, sans obligation de
maintenir leur outillage. Le travail de présentation vient après le nettoyage.

## 1. Conserver indépendamment de l’interface

- Le cœur Rust : compilation et export Typst, types et modèles, dépendances,
  ressources, polices, orthographe Spellbook, statistiques et accès aux fichiers.
- Les garanties déjà testées : écritures atomiques, verrous, détection des
  modifications externes, validation des données et ressources conservées pour
  l’annulation. Les adapter au nouveau contrat plutôt que réécrire leur logique.
- Fluent et ses catalogues ; distinction langue d’interface/langue documentaire.
- L’état Yjs et son historique par participant ; les cibles structurées et
  offsets UTF-16 du nouvel éditeur. Ne pas réintroduire le tampon plat d’egui.
- `assets/branding/`, `assets/ui/`, licences, fixtures utiles, rapports et résultats
  d’essais. Les maquettes et images historiques documentent les travaux passés ;
  elles ne constituent pas une présentation figée à reproduire dans Tauri.

Attention : supprimer `crates/app/` ne signifie pas supprimer immédiatement les
modules historiques de `crates/core/engine` ou `document`. La CLI et le MCP
autonome les utilisent encore ; leur évolution relève de A.2.

## 2. État fonctionnel et écarts à décider

« Présent » indique une implémentation trouvée, pas une recette manuelle des
trois systèmes. Les écarts ci-dessous doivent recevoir une décision explicite :
reprise avant retrait ou report assumé dans le chantier correspondant.

| Sujet | État observé | Suite proposée |
| --- | --- | --- |
| Capacités de rédaction | Les 16 capacités du catalogue structuré sont raccordées à Tauri, dont tableaux et notes ; widgets et bandeaux communs existent. | Conserver, recetter les cas limites ; ne pas porter les vues egui. C/E. |
| Fichiers et export | Tauri utilise l’archive structurée, le verrou commun, l’écriture atomique et l’export PDF/Typst ; le format historique reste utilisé par CLI/MCP autonome. | Unifier les adaptateurs sur le contrat structuré, sans compatibilité artificielle avec les positions plates. A.2/A.3. |
| Erreurs | Le cœur historique possède des erreurs typées ; de nombreuses commandes de l’hôte Tauri renvoient encore `Result<…, String>` avec des textes français. | Retrouver codes et informations structurées, puis localiser à la frontière d’interface. A.2. |
| Récupération après incident | egui utilise un gestionnaire de copies avec liste/restauration/suppression. Tauri conserve un `session.json` après deux secondes d’inactivité dans un chemin global de récupération. | Définir isolation entre processus/projets, noms des copies et traitement de sauvegarde, abandon, remplacement, fermeture et fichier invalide ; conserver les ressources et l’écriture atomique. A.3, prioritaire. |
| Registre des commandes | egui centralise commandes, raccourcis et disponibilité. Tauri a un registre TypeScript, un catalogue Rust de capacités, des commandes ajoutées dans le shell et des menus natifs décrits séparément. | Inventorier l’ensemble et vérifier l’aiguillage menu/palette/bouton/clavier, les états disponibles et le focus ; mutualiser les descriptions qui doivent l’être. A.2/E. |
| Commandes de confort | Recherche/remplacement exacts présents ; navigation suivante/précédente dédiée et effacement de mise en forme présents dans egui ne sont pas exposés de la même façon dans le registre web. | Décider la reprise des commandes utiles et leurs raccourcis ; vérifier liens et recherche sans déplacer implicitement la sélection. E. |
| Orthographe | Le même moteur est réutilisé ; suggestions, ajout au dictionnaire personnel et mots ignorés du document existent. Les commandes egui d’activation, de navigation et de réinitialisation ne sont pas toutes reprises. | Reprendre ou reporter explicitement ces contrôles ; rendre le dictionnaire personnel consultable et modifiable. E. |
| Préférences d’édition | Thème et taille existent. Le choix des polices est devenu générique ; la largeur des colonnes suit une proportion en séance, mais n’est plus enregistrée dans les préférences. | Restaurer la persistance de la proportion ; préciser les polices proposées et les glyphes disponibles hors ligne. E. |
| Profil et signature | Profil global, import de signature et copie dans le projet existent. Le panneau global Tauri ne propose ni aperçu ni suppression de la signature, contrairement à egui. | Reprendre les actions utiles ; vérifier signature par défaut, remplacement, suppression et autonomie du projet. D/E. |
| Modèles personnalisés | egui propose l’import d’un `.typ`. Tauri vérifie actuellement que l’identifiant et le source correspondent exactement à un modèle du catalogue. | Distinguer conformité au type et présence dans la bibliothèque globale. Décider le périmètre d’import et d’ouverture d’un modèle embarqué, en lien avec le chantier D et les régimes validés. |
| Synchronisation éditeur/aperçu | Navigation dans les deux sens et surlignage de sélection existent dans Tauri ; rendu et coordonnées sont associés à une révision. | Conserver et recetter Unicode, notes, annotations, cellules et rendu périmé. Ne pas recopier la géométrie du tampon egui. C/E. |
| Relecture des propositions MCP | L’acceptation/refus du lot, les gardes et l’annulation existent. Tauri affiche actuellement Actuel/Proposition en texte aplati ; egui signale aussi styles, structure, métadonnées et ressources. | Rendre visibles les changements non textuels autorisés par le nouveau contrat. Un lot ne modifiant que le gras peut afficher deux textes identiques aujourd’hui. A.2/F, prioritaire avant de déclarer la relecture équivalente. |
| Droits et rattachement MCP | Le MCP Tauri a son raccordement propre et limite l’agent aux lectures/propositions ; ses instructions excluent sauvegarde/export. La référence egui possède le raccordement fixe et les droits de fichiers facultatifs. | Comparer explicitement découverte, révocation, contexte, interruptions, relecture et droits ; décider les reports. La CLI/MCP autonome ne doit pas être annoncée compatible avec les archives structurées avant A.2. |
| Ouverture par chemin et système | egui accepte un chemin en argument. Le parseur de lancement Tauri traite ses options de diagnostic/MCP mais ne raccorde pas un chemin documentaire positionnel. | Décider puis reprendre le parcours d’ouverture utile. L’association `.typsto` et les événements d’ouverture du système relèvent aussi de H. |
| Clavier, IME, accessibilité et glyphes | Des tests web et natifs automatisés existent ; les tests IME egui ne prouvent pas le chemin matériel de la webview. Les polices CJK du rendu Typst ne sont pas automatiquement des polices CSS de l’éditeur. | Recette matérielle macOS/Windows/Linux : accents, composition/annulation/validation japonaises, cellules/annotations, raccourcis, navigation et lecture d’écran. E/H. |

La récupération demande une attention particulière : plusieurs fenêtres d’une
même session collaborative peuvent partager un état, mais deux processus
indépendants ne doivent pas écraser la même copie de récupération. Le chemin
global actuel et l’absence d’isolation visible dans `Session::open/save`
ne suffisent pas à garantir ce second cas.

## 3. Outils et tests à transformer

| Élément actuellement lié à egui | Traitement proposé |
| --- | --- |
| `--commands-catalogue` / `--commands-catalogue-json` dans le binaire egui | Déplacer la génération utile dans un outil documentaire indépendant de l’interface, en cohérence avec A.2 ; garder les tests du catalogue. |
| `scripts/capture_baseline.py`, son lanceur et ses tests | Retirer l’outillage de capture et ses comparateurs, sans équivalent à reconstruire dans Tauri. Reprendre seulement les scénarios fonctionnels et les fixtures encore utiles dans les tests web/natifs. |
| `experiments/editor-model/` | Conserver le bilan des frontières de cellules et de l’IME ; le diagnostic du tampon plat peut sortir de la compilation active. Ne pas recréer ce tampon dans ProseMirror. |
| `experiments/mcp-acceptance/` | Reprendre les scénarios métier encore utiles sur le MCP structuré : concurrence humaine/agent, interruptions, droits, refus, acceptation, annulation. Conserver les résultats de l’essai d’agent sans skill. |
| Galerie `ui-kit-gallery` et kit graphique egui | Retirer la galerie et son rendu. Conserver les conventions fonctionnelles utiles ; les futures modifications de présentation seront discutées après nettoyage. |
| Tests `crates/app/src/app/tests/` et `editor/tests.rs` | Trier par garantie : réutiliser le cœur, traduire les scénarios de comportement en tests web/natifs, retirer les assertions de géométrie et d’événements propres à egui. Pas de copie automatique test pour test. |
| CI, couverture et gardes de taille | Remplacer les exemples et chemins egui ; conserver la frontière du cœur et les contrôles génériques. Préciser le périmètre de couverture de l’hôte Tauri avant de retirer celui de l’ancienne interface. |

Sources examinées : [commandes egui](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/commands.rs),
[récupération egui](https://github.com/klutchnikoff/typstOgraphe/blob/b26f2df/crates/app/src/app/recovery.rs),
[shell web](../app/src/shell.ts), [préférences web](../app/src/preferences-panel.ts),
[session Tauri](../app/src-tauri/src/session.rs),
[relecture web](../app/src/assistance.ts),
[droits MCP Tauri](../app/src-tauri/src/agent.rs) et
[validation des modèles](../app/src-tauri/src/settings.rs).

## 4. Ce que l’on pourra retirer

- L’exécutable et la bibliothèque `typstographe-egui`, puis `crates/app/`.
- Le tampon, les décorations, la géométrie, les menus et le kit visuel propres
  à egui, après reprise des comportements utiles.
- Les exemples, options de capture et entrées CI qui n’ont plus d’usage actif.
- Les dépendances exclusivement utilisées par egui, après examen du graphe Cargo.
  **Ne pas supprimer `resvg`**, utilisé aussi par le cœur. Retirer une dépendance
  directe `muda` ou `rfd` ne signifie pas la faire disparaître des dépendances
  transitives de Tauri. Garder les ressources partagées dans `assets/`.
- Les commandes de lancement obsolètes dans la documentation courante ; les
  bilans historiques gardent leur statut et leurs preuves dans Git et docs/.

## 5. Ordre proposé et critères de sortie

1. Valider les écarts à reprendre et ceux à reporter ; ne pas appeler « parité »
   une fonction absente ou une recette non effectuée.
2. Nettoyer l’outillage inutile, notamment captures, comparateurs et galeries,
   sans le porter vers Tauri. Traiter la convergence documentaire A.2 et la
   fiabilité A.3, en priorité la récupération et les garanties d’acceptation.
3. Reprendre le périmètre fonctionnel de commandes/préférences retenu et les
   tests nécessaires ; refaire les parcours clavier et fichiers sur Tauri.
4. Retirer egui et ses dépendances exclusives dans un lot dédié, puis vérifier
   lancement, édition, sauvegarde/réouverture, récupération, export et MCP sur
   les trois systèmes, ainsi que CI et couverture.
5. Reprendre ensuite les modifications de présentation de l’interface. La
   reprise d’un comportement ne demande pas de reproduire les anciennes vues.

Ce retrait n’exige pas de terminer l’article scientifique, les projets/versions,
ACP ou la collaboration distante. Il n’impose pas non plus un convertisseur
général des anciens formats ou préférences : aucune rétrocompatibilité
préventive n’est demandée. Identifier seulement les données utiles à conserver
avant de retirer un ancien parcours, sans effacer les rapports ni les résultats.

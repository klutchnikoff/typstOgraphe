# Sélecteurs de fichiers macOS — diagnostic reporté

**Statut au 31 octobre 2026 : problème confirmé par l’utilisateur, non résolu.
Investigation reportée à sa demande.**

## Reproduction et périmètre

Depuis la racine, arrêter complètement l’application, lancer `npm start`, puis
cliquer sur **Ouvrir…**. L’interface et le menu de l’application sont français,
mais les boutons du sélecteur de fichiers macOS restent anglais.
Le redémarrage complet et les corrections des menus ne résolvent pas ce cas.
La langue du document reste indépendante de celle de l’interface.

Les libellés standard de Tauri/Muda sont désormais fournis par Fluent :
« À propos de typstOgraphe », masquer, couper, copier, coller, tout sélectionner.
Cette correction est validée pour le menu. Le titre et le filtre du sélecteur
proviennent aussi de Fluent ; cela ne traduit pas ses commandes internes.
La traduction du sélecteur et celle du menu sont donc deux vérifications séparées.

## Ce qui a été vérifié

- Le véritable exécutable contient `CFBundleLocalizations = [en, fr]` et le repli
  `CFBundleDevelopmentRegion = en`, issus des catalogues Fluent. Le contrôle de
  section Mach-O couvre désormais aussi le lancement Cargo autonome.
- Une sonde insérée temporairement dans le véritable hôte Tauri, avec fenêtre
  invisible, indique `fr` pour Fluent, le bundle et AppKit, avant et après
  création de WebKit. Les préférences natives indiquent `fr-FR`.
- Dans ce lancement autonome/de développement, le chemin du bundle est
  `target/debug` et son identifiant natif est nul. Ce constat reste une piste à
  comparer avec une vraie application `.app`, pas une cause établie.
- Le plugin installé est `tauri-plugin-dialog 2.8.3`, utilisant `rfd 0.16.1` avec
  `tauri 2.12.5`. Sa voie asynchrone macOS attache normalement le sélecteur à une
  fenêtre avec `beginSheetModalForWindow`. La première sonde AppKit utilisait
  un autre parcours ; ses résultats ne suffisaient pas à qualifier Tauri.

**Correction du premier bilan :** le texte « Ouvrir » repéré dans l’arbre
d’accessibilité pouvait être le **titre** du panneau. Le rôle de bouton n’était
pas vérifié. La propriété `NSOpenPanel.prompt` vaut « Open » dans la sonde, mais
elle ne suffit pas non plus à déduire le texte affiché par le service distant.
Les sondes affinées n’exposent que deux éléments d’accessibilité, sans les
boutons du service ; l’inspection externe nécessite une autorisation qui n’a
pas été demandée. Aucune capture n’a été produite.

## Essais temporaires retirés

L’ajout de [`CFBundleAllowMixedLocalizations`](https://developer.apple.com/library/archive/documentation/General/Reference/InfoPlistKeyReference/Articles/CoreFoundationKeys.html)
a été essayé dans le générateur.
L’utilisateur signale que le sélecteur reste anglais après relance. Ce réglage
seul peut aussi laisser un framework choisir une langue système absente de
Fluent ; il ne garantit donc pas l’alignement demandé.

Un autre prototype a transmis la langue sélectionnée par Fluent au domaine
**volatile** [`NSArgumentDomain`](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/UserDefaults/AboutPreferenceDomains/AboutPreferenceDomains.html)
avant initialisation d’AppKit/WebKit. Ses sondes
de processus réussissent les cas suivants :

| Préférence simulée dans le processus | Fluent | Préférences du processus, bundle, AppKit |
| --- | --- | --- |
| `fr-FR` | `fr` | `fr`, avant et après WebKit |
| `en-US` | `en` | `en`, avant et après WebKit |
| `ja`, absente des catalogues | `en` | `en`, avant et après WebKit |

Le contrôle des préférences de langue persistantes confirme qu’elles ne changent
pas pendant cette recette. **Ces résultats portent sur le processus hôte, pas
sur la traduction visible des boutons du sélecteur.** Ils ne constituent pas
une validation du correctif.

À la fin de ce diagnostic, les réglages expérimentaux, le raccordement volatile,
la sonde temporaire et sa dépendance directe ont été retirés de l’application et
de la CI. Les corrections précédemment validées des menus et des métadonnées
restent en place. Aucune préférence de langue macOS enregistrée n’a été modifiée.

## Reprise prévue

Piste proposée par l’utilisateur :
[dbv-typst-editor](https://github.com/davidbuenov/dbv-typst-editor).
Son traitement de ce problème n’a **pas encore été examiné**. Comparer ses
réglages Tauri et macOS avant d’en déduire qu’il possède une solution réutilisable.

Pour reprendre :

1. Examiner ses déclarations de langues, ressources localisées et parcours de
   lancement en développement et en bundle ; rechercher les solutions amont
   Tauri/rfd déjà éprouvées.
2. Comparer `npm start`, l’exécutable Cargo et un `.app` fraîchement construit,
   notamment l’identité du bundle et l’activation des traductions du service
   macOS qui affiche le panneau.
3. Vérifier les **vrais boutons** d’ouverture, d’enregistrement et d’annulation,
   puis les dialogues de dossiers et les menus contextuels WebKit. Ne pas
   confondre titre, message, métadonnées et commandes du service système.
4. Couvrir français, anglais et repli, en conservant les préférences système et
   la langue documentaire ; ajouter la recette au niveau où le défaut est établi.

Les tests de métadonnées et de négociation Foundation existants restent utiles,
mais ne doivent pas être présentés comme une recette visuelle du sélecteur.

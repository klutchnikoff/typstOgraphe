# Disponibilité et aiguillage des actions

Lot de phase 2 du 24 octobre 2026. Les identifiants et schémas du
[catalogue documentaire](commands.md) restent inchangés : 49 commandes et
quatorze requêtes. La présentation graphique et les intentions de dialogue
ne créent pas de nouvelles commandes JSON.

## Une décision partagée

`App::command_enabled` évalue chaque action du registre courant. Les menus
natifs et egui, la barre d’outils et ses sous-menus, les boutons de recherche,
la palette et `App::apply` utilisent cette même décision. Les raccourcis passent
par `apply` ; une action désactivée ne peut donc pas être forcée au clavier.
La palette filtre les actions indisponibles avant sa recherche et sa navigation.

Le moteur vérifie les règles documentaires. L’adaptateur garde les conditions
propres à l’interface : appartenance au registre du type, présence d’un document,
préférence d’orthographe, accès à une copie de récupération et focus d’un champ
pour l’effacement des styles. L’absence d’exceptions orthographiques ne bloque
pas la remise à zéro : la commande documentaire conserve son fonctionnement
sans effet lorsqu’il n’y a rien à réinitialiser.

Les fenêtres modales bloquent les menus et le dispatch externe. L’évaluation
partagée ne dépend pas de cette modalité, afin que la palette puisse évaluer
les actions dans sa propre fenêtre. Les autres surfaces sont désactivées par
leur hôte graphique pendant une modalité.

## Routes explicites

`UiCommand::route` est exhaustif : toute nouvelle variante doit être classée
pour compiler. Les actions actuellement enregistrées ont une des voies suivantes :

- `Document` : paramètres disponibles immédiatement, par exemple gras, titres,
  effacement des styles, notes de courrier et historique ;
- `Contextual` : paramètres issus du contexte courant, par exemple recherche,
  remplacement, orthographe documentaire ou modèle fourni ;
- `Prepared` : un dialogue doit collecter des paramètres, par exemple un lien,
  une image, un modèle à importer, une destination d’enregistrement ou d’export ;
- `Interface` : fenêtre, palette, préférences, activation du soulignement,
  apprentissage global d’un mot, présentation des suggestions ou sortie de
  l’application. Ces actions composent des lectures/commandes documentaires
  quand elles en ont besoin.

Les variantes de blocs internes qui ne sont pas des actions publiques sont
`Unsupported` et ne peuvent pas être déclenchées depuis le registre.
Un test parcourt tous les registres de types et refuse une action publique
sans voie, une mauvaise correspondance d’identifiant ou une route documentaire
absente du catalogue du moteur.

## Paramètres connus et paramètres à collecter

`Engine::available` valide une commande munie de ses paramètres. La cible de
l’éditeur est capturée en sélection logique gardée pour les vérifications comme
pour l’exécution. Évaluer un bouton ne déplace jamais le curseur détenu par le
moteur et ne modifie ni révision, ni historique, ni contexte de frappe.

`Engine::available_intent` vérifie les conditions déjà connues avant un dialogue :
`New`, `Open`, `Save`, `SaveAs`, `Close`, `Export`, `Link`, `InsertImage`, `SetModel`.
Chaque intention référence un identifiant déjà présent dans le catalogue.
Les contraintes de document ouvert, cible de lien, frontières d’insertion d’image
et champs requis pour exporter sont partagées avec les commandes complètes.
Aucune URL, image ou destination fictive n’est construite pour cette vérification.
Aucun fichier n’est lu ou écrit par la vérification d’une intention.

Nouvelle ouverture et fermeture peuvent conduire à une confirmation du travail
non enregistré. L’intention autorise ce parcours ; la commande complète vérifiera
la garde et le choix explicite de conservation ou d’abandon. De même, une
intention d’export ne garantit ni la compilation du modèle ni l’accès à une
future destination. La confirmation du dialogue revalide les paramètres réels.

## Recherche et orthographe

La construction des paramètres de recherche et remplacement est commune à la
disponibilité et à l’exécution. Les contrôles ne dépendent pas de la liste de
résultats peinte par egui. Un remplacement individuel exige toujours l’occurrence
capturée et sa garde ; un remplacement invalide ou périmé reste indisponible.
Le moteur réutilise son calcul de recherche par identité/révision du document,
texte recherché et sensibilité à la casse. Cela évite de recompiler la recherche
pour chaque bouton et entrée de menu. Le cache n’est pas sérialisé.

Les actions orthographiques consultent les fautes du document publié via le
moteur, même si le worker de soulignement n’a pas encore terminé. Les références
de faute sont revalidées pour ignorer, corriger, apprendre ou afficher les
suggestions. Le worker conserve le dessin des soulignements et le calcul des
suggestions en arrière-plan. La correction reste locale avec **Spellbook 0.4.2**,
les dictionnaires français et anglais embarqués et le dictionnaire personnel.

## Vérifications

Les nouveaux scénarios contrôlent les intentions et commandes complètes, les
cibles étrangères, les frontières d’image, les champs d’export, les caches de
recherche, les occurrences périmées et l’absence de mutation lors des lectures.
Les tests d’interaction cliquent dans les menus egui et la barre d’outils,
choisissent une action dans la palette et utilisent un raccourci personnalisé :
ils vérifient le résultat, la sélection et une seule transaction annulable.
Ils vérifient aussi le blocage d’une action indisponible par ces différentes voies.

Les tests existants couvrent les paramètres des dialogues, les fichiers, images,
annotations, métadonnées, modèles et préférences. Le pont des menus natifs partage
`dispatch_action` et les mêmes états ; les tests de raccourcis conservent le
contrôle des modificateurs macOS/Windows/Linux et l’absence de double dispatch
pour les touches réservées par le menu natif.

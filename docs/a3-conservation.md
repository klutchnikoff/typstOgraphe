# A.3 — Conservation des saisies et reprise après incident

Décisions validées le **31 octobre 2026**. Ce contrat prolonge le socle structuré
d’A.2 ; il ne remplace ni son moteur documentaire ni ses garanties de publication.

## Responsabilités conservées

L’état Yjs/Yrs, l’arbre et les réglages validés, puis le Typst généré doivent
rester cohérents. Les commandes et lots passent par le propriétaire commun.
Les écritures atomiques, verrous, gardes et contrôles d’archives sont conservés.
Une copie de récupération est distincte du fichier `.typsto`, de l’historique
d’annulation en séance et des snapshots durables d’A.4.

## Enregistrement et déclenchement

- `Ctrl/Cmd + S` reste l’enregistrement explicite du projet. Aucune écriture
  automatique implicite de ce fichier n’est ajoutée.
- La copie de secours locale suit les changements utiles, après deux secondes
  de pause, avec une échéance maximale de dix secondes pendant une rédaction
  continue, en fonctionnement normal. Les timers suspendus par le système ou
  une erreur de stockage ne permettent pas de promettre une absence absolue de
  perte ; les erreurs de protection sont visibles.
- Les écritures sont regroupées, ordonnées et limitées au travail modifié.
  La récupération ne compile pas le document et ne bloque pas la frappe.
- La fermeture explicite attend une conservation nécessaire ; un échec ne doit
  pas permettre une fermeture silencieuse avec perte du travail.

## Ce qui est protégé

La récupération conserve un état confirmé cohérent et, séparément, les saisies
privées de chaque fenêtre : texte local en attente ou refusé, valeurs d’en-tête
non confirmées, formulaires ouverts et leurs ressources. Les brouillons de
formulaires comprennent aussi les saisies riches, options et fichiers choisis
avant validation, quand ils sont requis pour retrouver le travail.

Une composition de clavier en cours n’est jamais publiée comme une saisie
validée. La conservation privée respecte cette séparation et ne provoque ni
validation de formulaire, ni partage, ni entrée dans l’historique documentaire.
Une erreur de rendu ne justifie pas la suppression de la copie de secours.

Les formulaires utilisent un mécanisme commun de collecte et de notification.
Leur identité, cible, valeurs et ressources doivent être explicites ; le moteur
ne déduit pas leur sens des seuls libellés affichés. Les données de récupération
sont bornées, versionnées et traitées comme non fiables à la lecture.

## Isolation et cycle de vie

Une copie courante est conservée par document et par fenêtre, sans accumulation
de versions. Une écriture ne remplace l’ancienne copie qu’une fois la nouvelle
écrite correctement. Une fenêtre ne peut pas écraser ou supprimer la copie
privée d’une autre, ni une notification tardive atteindre le document suivant.

Enregistrer le document confirmé ne suffit pas à effacer une copie contenant
encore un formulaire ouvert ou une saisie privée non conservée ailleurs. Les
ressources utiles aux brouillons restent avec leur copie de récupération.
La disparition de ces données résulte d’une reprise durable ou d’un abandon
explicite ; le nettoyage des snapshots et ressources historiques reste dans A.4.

## Reprise et décisions explicites

Le prochain lot présentera à l’accueil le nom, la date et la présence de saisies
non confirmées. Le document confirmé peut être repris normalement ; un brouillon
refusé revient dans un espace de récupération isolé permettant de retrouver et
copier son travail. Aucune fusion automatique ni écrasement du fichier d’origine.
Un brouillon exporté manuellement doit également pouvoir être rouvert depuis
l’interface. Ce parcours doit être maquetté et validé avant implémentation.

En présence d’un brouillon non confirmé, fermer propose explicitement de le
conserver pour reprendre plus tard, de l’abandonner ou d’annuler. Un changement
externe conserve les protections existantes et peut conduire à une copie
indépendante. Une écriture échouée n’est jamais annoncée comme réussie.

L’historique d’annulation reste limité à la séance ; les états nommés et les
comparaisons historiques ne sont pas introduits par A.3.

## Lots et critères de vérification

1. **Conservation des brouillons** : collecte commune des zones, champs et
   formulaires, copie privée durable isolée de l’état confirmé, ordonnancement
   et échéances, protection des ressources et des transitions documentaires.
2. **Interface de reprise** : maquette, réouverture du document et des saisies
   privées, import d’un brouillon de secours, abandon et fermeture explicites.
3. **Recettes d’incident et de concurrence** : Courrier/Article, processus
   interrompu, saisie pendant une écriture, document sans chemin, deux fenêtres,
   composition, refus, erreur de stockage et changement externe. Vérifier les
   parcours natifs sur macOS, Windows et Linux sans confondre les tests locaux
   avec la couverture multisystème de la CI.

La livraison du premier lot ne vaut pas clôture d’A.3. L’interface de reprise,
les décisions de fermeture et les recettes natives restantes restent à traiter
dans les lots suivants.

## Référence étudiée

La lecture de DBV Typst Editor au commit
`48e65ceae0e3ad55e8cfe96263ac217c81b98e0e` fournit des exemples utiles de règles
pures de sauvegarde et rétention, ainsi que de restauration dans l’éditeur.
Son historique est une collection de textes par fichier, extérieure au projet ;
il ne fournit pas nos checkpoints structurés ni la conservation des brouillons
privés et ressources. Aucun code tiers n’est repris dans ce lot ; conserver les
mentions MIT si une reprise est décidée ultérieurement.

Sources : [historique](https://github.com/davidbuenov/dbv-typst-editor/blob/48e65ceae0e3ad55e8cfe96263ac217c81b98e0e/src-tauri/src/history.rs),
[règles de sauvegarde](https://github.com/davidbuenov/dbv-typst-editor/blob/48e65ceae0e3ad55e8cfe96263ac217c81b98e0e/src/app/autoSave.js).
Les workflows de distribution étudiés relèvent du chantier H.

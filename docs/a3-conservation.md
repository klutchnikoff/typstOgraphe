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

Le [parcours de reprise et les choix de fermeture](a3-recovery-interface.md), sa
maquette et son implémentation sont validés/livrés. L’accueil présente les copies
indépendantes et leurs saisies privées. Le brouillon refusé reste isolé et les
brouillons exportés peuvent être importés sans publication automatique.

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

Les trois lots sont implémentés. La [recette de clôture](a3-closure-results.md)
relie les critères aux contrôles locaux et à la qualification native des trois
systèmes. La réussite de cette qualification est requise avant de passer à A.4.

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

## Premier lot : conservation privée

Voir les [résultats et la recette native](a3-conservation-results.md).

L’implémentation garde un fichier atomique par génération documentaire et fenêtre,
sous `sessions/<session>/drafts/<generation>/<empreinte-fenetre>.json` dans les
données locales de l’application. Le format `typstographe-private-recovery-v1`
contient deux parties distinctes :

- le checkpoint confirmé, construit par le propriétaire Rust avec ses ressources,
  son état Yjs/Yrs, ses réglages et sa révision ;
- la saisie privée : état partagé local complet, arbre visible, réglages éventuellement
  invalides, champs en attente, formulaires identifiés avec leur cible et leurs
  valeurs, ressources et indication de refus.

Le collecteur commun des bandeaux et fenêtres de formulaire conserve les valeurs
brutes des contrôles, les zones riches et les données typées complémentaires.
Une image choisie mais non appliquée reste dans cette copie privée ; elle n’entre
pas dans les ressources du document. Les auteurs/affiliations en cours d’ajout ou
de modification restent également privés. Fermer leur interface sans validation
ne fait pas disparaître leur dernière saisie ; une annulation explicite retire
le brouillon correspondant. Les préférences globales du logiciel sont hors de
ce mécanisme documentaire.

Le contrôleur ne demande ni publication, ni validation, ni compilation avant la
conservation. Il reste donc utilisable après un refus et en mode individuel.
Les écritures attendent la pause de deux secondes ou l’échéance de dix secondes ;
une transition déclenchée depuis la fenêtre attend aussi l’écriture nécessaire.
La file d’écriture conserve les changements arrivés pendant une écriture. Un
échec laisse la demande à réessayer et affiche une erreur localisée. Une vue
obsolète, un mauvais document ou une séquence plus ancienne ne remplacent pas
une copie plus récente. Un enregistrement explicite nettoie les copies entièrement
confirmées, en conservant les copies privées ou illisibles.

Recharger une vue ne vaut pas abandon : sa copie privée non reprise est retenue
séparément avant de commencer la copie courante de la nouvelle vue. Cela ne crée
pas de versions au fil de la frappe ; ces copies attendent une décision explicite
dans le parcours de reprise.

Les fichiers sont limités à 64 Mio, lus avec une borne effective, et le schéma est
versionné. Les mises à jour Yjs sont décodées mais leur contenu privé n’a pas à
satisfaire le contrat documentaire. Les ressources nouvelles sont contrôlées sans
être admises ; les limites sont de 128 formulaires, 1 024 champs et 4 096 ressources
par copie. Les écritures utilisent `atomic-write-file` ; les liens symboliques aux
emplacements de récupération sont refusés. Les fichiers privés sont créés avec
le mode `0600` et leurs répertoires avec `0700` sous Unix. Il s’agit de données
locales privées, sans chiffrement supplémentaire.

### Reprise graphique et coordination livrées

Le [parcours validé](a3-recovery-interface.md) est implémenté. L’accueil liste les
copies disponibles ; reprendre construit une copie indépendante avec sa file de
saisies privées. Le fichier initial n’est pas ouvert ni verrouillé. Son premier
Enregistrer demande un emplacement, avec une suggestion de nom distincte.

Le collecteur et les interfaces de formulaire sont aussi ceux de la reprise :
`ContextPanel` hydrate les contrôles et zones riches, puis laisse la validation à
la commande ordinaire. Les champs simples passent par `MetadataCommands` en état
manuel, sans publication au simple réaffichage. La file n’est consommée qu’après
confirmation et copie durable ; un échec de conservation remet l’élément dans la
file. Annuler la reprise du formulaire conserve la source récupérée.

L’enveloppe conserve désormais aussi les éléments récupérés non encore traités,
une indication de rétention explicite et, dans les formulaires/champs rouverts,
l’identifiant de leur source. Un deuxième incident conserve la dernière saisie du
formulaire repris, sans dupliquer son ancienne version. Les brouillons JSON
exportés peuvent être importés ; le format historique de brouillon refusé reste
isolé, et une base confirmée absente est remplacée par un document vide valide.
Un contenu privé invalide n’entre jamais automatiquement dans le document.

Les transitions Nouveau/Ouvrir/Reprendre/Fermer le document/Quitter attendent une
barrière commune : toutes les fenêtres déclarées doivent conserver leur saisie
avant le changement de génération. Les interfaces sont temporairement figées,
y compris les fenêtres de formulaire détachées. Une erreur, une composition en
cours, une annulation ou un changement de révision empêche la transition. L’abandon
ne concerne que la copie de la fenêtre qui le demande. Fermer une fenêtre native
ne remplace pas le document des autres fenêtres.

Le catalogue ne propose pas les copies actives d’un processus encore vivant. Il
refuse les identifiants sortant du répertoire autorisé, les liens symboliques et
les sources non validables. Supprimer/reprendre une copie ne nettoie son résumé
confirmé que si celui-ci désigne la même base et qu’aucune autre enveloppe ne reste.
Les copies illisibles restent visibles et supprimables après confirmation.

### Clôture et limites de qualification

La [recette de clôture](a3-closure-results.md) exerce les incidents d’enregistrement,
les changements externes, la réouverture et l’export de Courrier et Article. Les
recettes natives sont disponibles dans la validation complète des trois systèmes ;
leurs résultats CI font foi pour Windows/Linux. Les saisies IME matérielles et
les sélecteurs système restent des contrôles manuels de distribution.

Les snapshots durables et l’historique entre séances restent dans A.4. Une
préédition IME non validée n’est pas publiée ni promue en copie de saisie validée.

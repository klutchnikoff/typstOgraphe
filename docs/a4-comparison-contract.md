# A.4 — Comparaison et décisions locales communes

Statut : périmètre et maquette interactive validés le 31 octobre 2026.
Comparateur commun et premier raccordement des décisions locales implémentés.
Voir le [bilan de validation](a4-comparison-results.md) et la
[maquette conservée](mockups/a4-comparison.html).

## Composition validée — choix locaux, application unique

Le même comparateur sert Versions et propositions MCP. **Avant à gauche en
rouge, après à droite en vert.** Deux versions historiques sont ordonnées par
leur date ; une comparaison avec le document courant le capture à l’ouverture.
MCP conserve sa base à gauche et son candidat à droite.

Chaque groupe possède deux boutons « Sélectionner », un sous chaque colonne.
Ils forment un choix exclusif accessible au clavier. Tous les verts sont actifs
au départ ; cliquer le rouge le sélectionne et désélectionne le vert, et inversement.
Le choix actif a un fond plein, un texte contrasté et une coche, sans dépendre de
la couleur seule. Les groupes liés et les widgets restent des unités indivisibles.

Les choix sont privés à la consultation. Ils persistent entre les onglets de
cette consultation, sans modifier le document, le fichier, l’historique
Annuler/Rétablir ou le reçu MCP. Fermer ou choisir une autre paire abandonne ces
choix, sans publication. Aucun outil de l’agent ne peut les valider.

**« Appliquer et enregistrer »** compose le résultat à partir de l’état de droite,
en remplaçant les groupes choisis en rouge par ceux de gauche. Les parties
inchangées proviennent également de droite, y compris entre deux états
historiques lorsque le document courant est différent des deux. Ce résultat
remplace le document courant et est enregistré dans son `.typsto`. Si le projet
n’a pas encore de chemin, le sélecteur Enregistrer propose une destination ; son
annulation conserve le document et les choix.

La validation publie une seule opération annulable. **Aucun snapshot automatique
n’est créé**, ni du résultat, ni « avant restauration » pour cette composition.
Créer une version reste une action volontaire. Les versions existantes et leurs
métadonnées restent intactes. Annuler revient à l’état précédent dans l’éditeur,
qui redevient alors modifié ; cela ne réécrit pas implicitement le fichier.
Le futur parcours distinct de restauration complète n’est pas livré par ce lot.

Les états de comparaison restent figés après validation. La consultation indique
que les choix ont été appliqués et enregistrés ; elle ne permet pas une seconde
publication de la même préparation. La vue Contenu et les aperçus sont statiques,
avec les ressources documentaires de chaque état et le catalogue Typst de la
version installée. Les copies de paquets transportées ne sont pas restaurables.

## Groupes applicables et dépendances

Un groupe visuel n’est pas automatiquement une unité de décision indépendante.
Une citation et la référence bibliographique qu’elle nécessite, ou une image et
sa ressource, doivent être traitées ensemble. Le plan regroupe les opérations
liées et vérifie la cohérence de l’état résultant avant publication. Une absence
de preuve d’indépendance impose un regroupement sûr, pas une application partielle
hasardeuse. Cela peut exceptionnellement produire une seule unité si tout est lié,
sans imposer ce traitement à tous les lots.

## Préparation, enregistrement et concurrence

Le calcul des différences reste descriptif. Le candidat exécutable est lié à la
paire, aux choix complets, à son identité/époque et à la révision courante capturée.
L’adaptateur prépare le candidat et le compile hors du verrou de rédaction, puis
recontrôle garde, autorisation humaine, contexte privé et composition IME avant
l’écriture atomique. Une erreur de validation, un verrou de fichier, une
modification externe du fichier ou un échec d’écriture ne publie rien et conserve
les choix pour réessayer.

Une intervention documentaire extérieure bloque la validation. Même un retour au
même texte par Annuler ne réactive pas l’ancienne garde. Il faut ouvrir une nouvelle
comparaison, ou demander une nouvelle proposition MCP depuis une base relue.
Les deux colonnes restent consultables. Les publications ne sont jamais
réancrées ni rebasées implicitement.

Après le commit durable, la publication confirmée et l’état enregistré sont
retournés directement au client, ainsi que diffusés aux autres fenêtres. Une
notification perdue ne doit pas faire perdre le résultat au validateur. Un échec
de nettoyage de récupération après le commit produit un avertissement, sans
présenter l’enregistrement déjà réussi comme un échec.

## Raccordement MCP

La soumission fige les archives complètes de base et de candidat. Les choix
rouge/vert n’affectent pas la proposition avant validation. Après l’application
unique, son reçu distingue les groupes retenus et écartés : `accepted`, `rejected`
ou `partially_accepted`. La révision de travail est mise à jour ; la révision
initiale reste celle de la base. L’enregistrement provient de l’action humaine
explicitement libellée, jamais d’un outil de l’agent. Aucun export n’est implicite.
Les propositions ne deviennent pas des snapshots, ni des annotations de relecture.

## Présentation commune

La modale à 90 %, ses onglets Contenu / Aperçu / Comparer, ses deux colonnes et
les actions locales reprennent la maquette interactive validée. Les actions de
chaque groupe sont alignées à droite ; l’action principale est à l’extrémité
droite. Les dépendances sont indiquées dans la ligne de décision. Les états
restent figés et les décisions sont affichées séparément. Une intervention
extérieure conserve la consultation et les choix, mais bloque leur validation.

Une différence de structure non identifiable sans ambiguïté reste groupée.
Les changements de paramètres et de ressources documentaires sont regroupés
avec le contenu associé lorsque leur indépendance n’est pas prouvée.

### Découpage par paragraphe validé

- Deux paragraphes modifiés consécutifs donnent deux décisions lorsque leurs
  frontières sont conservées. Plusieurs changements dans un même paragraphe
  restent une décision. Un titre conservant son identité suit la même règle.
- Un widget entier constitue une unité : liste avec ses items, tableau avec sa
  légende, citation en bloc avec son attribution, image avec sa légende. Aucun
  découpage interne n’est introduit par cette évolution.
- Une fusion, une scission, un changement de structure ou une correspondance
  ambiguë reste groupé. Les dépendances documentaires (bibliographie, réglages,
  ressources…) restent prioritaires sur le découpage visuel.

Le moteur commence par les ancrages de blocs identiques. Dans un passage
contenant plusieurs blocs modifiés, il sépare les paires lorsque les checkpoints
conservent exactement les mêmes identités de blocs, dans le même ordre et en
même nombre. Il ne déduit pas leur identité de la ressemblance des textes.
Des frontières ajoutées, supprimées ou recréées, ou l’absence de checkpoints
permettant cette preuve, conservent le passage groupé. Ces identités sont
internes au calcul ; les décisions publiques continuent à utiliser leurs groupes
et leurs gardes documentaires. Le même découpage sert Versions et MCP.

## Implémentation et limites explicites

`content::comparison::Comparison` possède la paire privée, les groupes et la garde
de travail. `prepare` produit un candidat opaque, lié à l’identité de cette
comparaison et à l’époque de ses décisions. `publish` recontrôle ces références et
passe par `EditingSession::publish_prepared`. Un candidat préparé pour une autre
paire, une décision concurrente ou une ancienne révision est refusé sans publication.

`similar` **3.2.1** a été retenu après évaluation de son API de séquences et de
mots Unicode : crate Rust sans backend natif supplémentaire, version déjà disponible
dans le cache local, maintien en amont et délai de calcul configurable. Les
[sources de la bibliothèque](https://github.com/mitsuhiko/similar) et son
[API de comparaison](https://docs.rs/similar/latest/similar/algorithms/index.html)
sont les références. L’alignement exact des blocs et le détail des mots utilisent
Myers avec échéance de 100 ms par alignement. Une limite atteinte produit un
regroupement conservateur ; elle n’autorise aucune correspondance approximative.

Le découpage applique les différences entre ancrages de blocs identiques, puis
sépare les paragraphes ou widgets adjacents dont les frontières sont conservées.
Plusieurs modifications dans un même paragraphe ou widget restent une décision.
Si les réglages, la bibliothèque bibliographique ou les ressources documentaires
changent, le contenu associé est regroupé en une seule décision documentaire :
le premier moteur ne prétend pas encore prouver des indépendances plus fines.
Les caractères, espaces significatives, liens et mises en forme sont conservés ;
les deux projections riches restent disponibles même lorsque le texte est identique.
Les fichiers modifiés sont inventoriés et les deux rendus Typst sont isolés.

Le moteur rattache aussi les plages de suppression et d’ajout aux chemins des
feuilles textuelles de chaque groupe (`before_highlights` / `after_highlights`,
positions UTF-16 locales). Une annotation modifiée est marquée comme un objet
entier. L’affichage applique ces plages dans la projection riche statique, sans
comparer le texte DOM aux chaînes internes de la différence. La présence d’un
badge de citation ou de note ne peut donc pas masquer les changements voisins.
Les marques visuelles `del` / `ins` ne rejoignent ni Yjs ni le contenu enregistré.

Les paquets appartiennent au catalogue figé de la version installée de
typstOgraphe, pas à l’historique documentaire. La comparaison ignore les
différences de leurs copies transportées ; préparer une reprise résout les
bibliothèques du modèle dans ce catalogue. Annuler/Rétablir ne change pas leurs
versions et aucun jeton de paquet n’est ajouté à Yjs. Les deux aperçus emploient
les mêmes bibliothèques applicatives avec leurs propres modèles et ressources.
On garantit l’état documentaire, pas un PDF identique entre versions de
typstOgraphe. Voir le [contrat des dépendances](typst-dependencies.md).

Les comparaisons et leurs choix sont temporaires, hors `.typsto` et hors des
snapshots. La récupération des choix de composition n’est pas ajoutée par ce lot.
Le bilan conserve les recettes des étapes précédentes et de cette évolution.

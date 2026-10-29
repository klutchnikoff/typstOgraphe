# Contrat documentaire structuré — décisions A.2

Décisions validées le **28 octobre 2026**. Elles guident l'unification entre
l'application Tauri, la CLI, MCP autonome et MCP rattaché. Ce document décrit
la cible ; les lots ci-dessous distinguent sa mise en œuvre progressive.

## Représentation et moteur partagé

- Le contrat appartient à typstOgraphe et porte sa version. Il reste proche des
  structures ProseMirror utiles, sans exposer ses objets ni ses positions globales.
- Yjs reste dans l'éditeur ; **Yrs est retenu côté Rust**, après
  [évaluation sur les structures réelles](../experiments/yrs-evaluation/README.md).
  Les mises à jour conservent les identités collaboratives. Les offsets des
  zones textuelles sont en UTF-16 ; les identifiants internes Yjs restent privés.
- L'état partagé et sa projection documentaire doivent être cohérents. Le Typst
  produit est une représentation dérivée. Une sauvegarde ou un export ne prend
  pas un arbre fourni par un adaptateur pour preuve de l'état partagé.
- Une valeur métier reste distincte de sa présentation : le modèle détermine
  la mise en page et les options effectives selon son contrat.

## Cibles et données d'en-tête

- Une cible nomme une zone : corps, cellule, annotation, titre, résumé ou autre
  champ riche. Les blocs/cellules sont adressés par chemins dans une révision.
- Auteurs, affiliations, références et autres enregistrements utilisent leurs
  identifiants stables. Une commande modifie un champ ou un enregistrement sans
  remplacer implicitement toute sa collection.
- Les opérations différées associent identité de session documentaire et révision.
  Une cible périmée est refusée ; une position relative de curseur n'autorise pas
  le réancrage silencieux d'une proposition.
- Les mêmes opérations documentaires sont disponibles aux adaptateurs concernés,
  dans les limites de leurs autorisations. Le catalogue décrit ces limites.
- La rédaction incomplète est autorisée : les contraintes structurelles restent
  impératives ; les champs exigés à l'export sont signalés séparément.

## Validation, publication et historique

- Une commande ou un lot est préparé sur un état isolé, avec les données et
  ressources nécessaires. La publication recontrôle sa garde et valide le tout.
  Un refus ne publie aucune partie du lot.
- Une acceptation publie une seule transaction et constitue une seule étape
  d'annulation pour le participant concerné. Les contributions distantes restent
  distinctes. La frappe conserve ses groupes naturels.
- Les propositions MCP rattachées exigent l'acceptation humaine du lot entier.
  Sauvegarde et export ont leurs opérations et autorisations propres.
- Le moteur fournit des erreurs structurées et les motifs d'indisponibilité.
  La traduction, les dialogues et la présentation appartiennent aux adaptateurs,
  via Fluent avec repli anglais. Les erreurs des archives conservent leur cause
  typée jusqu’à cette frontière ; une validation refusée ne déclenche pas un
  second lecteur sans reconnaissance explicite du format.
- La pile d'annulation en séance, la récupération et les snapshots durables ont
  des contrats distincts. Yrs ne persiste pas automatiquement la pile d'annulation.
  La politique de rétention des snapshots reste à décider dans A.4.

## Saisie et mode individuel

- Tiptap affiche la frappe immédiatement. Les mises à jour sont transmises
  de façon asynchrone au moteur Rust pour confirmation ; les commandes préparées
  gardent leur contrôle strict de révision.
- La frappe concurrente utilise la fusion Yjs/Yrs. Les états publiés sont
  validés ; la préédition IME reste locale jusqu'à sa validation.
- Un refus conserve le brouillon local et permet une remise en cohérence
  explicite. Il ne doit pas effacer silencieusement la saisie. Le parcours de
  refus doit être implémenté et testé avant de durcir le relais de production.
- Focus, sélection, style de frappe et présence appartiennent aux participants.
  Les panneaux et brouillons d'interface ne font pas partie du document partagé.
- **Mode individuel par défaut** : même moteur et mêmes fichiers, services
  collaboratifs et présence activés seulement à la demande. On conserve Yjs/Yrs ;
  on ne crée pas une seconde implémentation de l'édition. Le gain de réactivité
  sera mesuré sur le parcours réel, sans assimiler le coût CRDT au rendu Typst.

## Mise en œuvre et critères de sortie

Le [premier lot du socle](structured-contract-foundation.md) est livré : valeurs
partagées, projection Yrs, préparation/publication gardée et conservation cohérente.
Les cibles multizones et commandes sont livrées dans le cœur. Le relais et
l’historique Tauri y sont raccordés, ainsi que les tableaux, styles, liens,
titres, widgets, objets en ligne et contextes de sélection/styles de frappe.
Les autres commandes de l’interface et les adaptateurs CLI/MCP restent à migrer selon la séquence ci-dessous.

1. **Socle partagé** : valeurs de réglages et d'instantané dans le cœur,
   gardes/cibles/erreurs structurées, projection Yrs et contrôle de cohérence
   lors de la conservation. Éditions préparées et refus sans mutation publiée.
2. **Commandes structurées** : les [cibles multizones et premières commandes](structured-commands.md)
   sont livrées dans le cœur et vérifiées avec Tiptap, y compris l’en-tête, les
   ressources, les options et les modèles. Poursuivre les transformations restantes,
   le catalogue et la disponibilité communs. Préserver les identités lors des éditions.
3. **Application** : le relais et l’historique par fenêtre sont raccordés au
   moteur, avec groupement, accusés/refus et export du brouillon avant abandon.
   Les sélections et styles privés Tauri sont aussi raccordés, avec des cibles
   multizones et une frontière causale compacte. Poursuivre les autres commandes
   typées et le mode individuel. Conserver la réactivité et les comportements
   de l’éditeur ;
   étendre la recette IME aux systèmes réels.
4. **CLI et MCP** : migrer les opérations et requêtes documentaires, les fichiers
   et les exports ; rendre les capacités découvrables, avec les nouvelles cibles.
5. **Recette et retrait** : même corpus Courrier/Article sur tous les adaptateurs,
   propositions, annulation, enregistrement/réouverture, rendu/export hors ligne,
   vérifications macOS/Windows/Linux ; retirer les anciennes voies après recette.

Les lots sont livrés par commits cohérents, avec une poussée à la fin du lot.
Les captures restent suspendues. La clôture d'un lot ne vaut pas clôture de A.2.

## Modèle conservé dans le projet

Le source du modèle enregistré dans le `.typsto` fait foi pour ce projet. Son
identifiant indique sa provenance ; il ne contraint pas la copie enregistrée à
rester identique au catalogue de l’application. Une mise à jour ou un retrait
du modèle fourni ne doit donc pas empêcher l’ouverture d’un projet autonome.
Le contrat Typst, les ressources et la cohérence de l’état Yjs restent vérifiés.

L’interface n’identifie un modèle comme fourni que si son identifiant **et** son
source correspondent au catalogue courant. Sinon elle présente « Modèle embarqué »
et propose toujours de choisir explicitement une version du catalogue. Le simple
chargement ne remplace ni le modèle du projet ni son historique.

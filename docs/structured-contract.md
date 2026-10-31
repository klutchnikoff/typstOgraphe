# Contrat documentaire structuré — décisions A.2

Décisions validées le **28 octobre 2026**. Elles guident l'unification entre
l'application Tauri, la CLI, MCP autonome et MCP rattaché. Ce document décrit
le contrat courant ; les bilans liés conservent l’historique de sa mise en œuvre.

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
- Les propositions MCP rattachées passent par le comparateur humain commun :
  décisions par groupe indépendant, dépendances indivisibles et publication
  unique de la composition. Voir le [contrat](a4-comparison-contract.md).
  Sauvegarde et export ont leurs opérations et autorisations propres.
- Le moteur fournit des erreurs structurées et les motifs d'indisponibilité.
  La traduction, les dialogues et la présentation appartiennent aux adaptateurs,
  via Fluent avec repli anglais. Les erreurs des archives conservent leur cause
  typée jusqu’à cette frontière ; une validation refusée ne déclenche pas un
  second lecteur sans reconnaissance explicite du format.
- La pile d'annulation en séance, la récupération et les snapshots durables ont
  des contrats distincts. Yrs ne persiste pas automatiquement la pile d'annulation.
  Le [format CAS](a4-project-format.md) limite les versions manuelles à 100,
  sans suppression automatique. Les références de relecture sont distinctes.

## Saisie et mode individuel

- Tiptap affiche la frappe immédiatement. Les mises à jour sont transmises
  de façon asynchrone au moteur Rust pour confirmation ; les commandes préparées
  gardent leur contrôle strict de révision.
- La frappe concurrente utilise la fusion Yjs/Yrs. Les états publiés sont
  validés ; la préédition IME reste locale jusqu'à sa validation.
- Un refus conserve le brouillon local et permet une remise en cohérence
  explicite. Il ne doit pas effacer silencieusement la saisie. Le parcours de
  refus conserve le travail et suspend la synchronisation jusqu’au choix explicite
  de reprise ou d’export ; voir le [contrat de conservation](a3-conservation.md).
- Focus, sélection, style de frappe et présence appartiennent aux participants.
  Les panneaux et brouillons d'interface ne font pas partie du document partagé.
- **Mode individuel par défaut** : même moteur et mêmes fichiers, services
  collaboratifs et présence activés seulement à la demande. On conserve Yjs/Yrs ;
  on ne crée pas une seconde implémentation de l'édition. Le gain de réactivité
  sera mesuré sur le parcours réel, sans assimiler le coût CRDT au rendu Typst.
- **Activation depuis un document en cours, précisée le 30 octobre :** une action
  explicite en un clic, sans conversion ou réouverture. Conserver identité,
  contenu, ressources, historique, sélection et focus ; l’activation ne crée pas
  d’étape d’annulation. Traiter les confirmations en attente sans publier une
  préédition IME ou un brouillon privé/refusé, et conserver le travail en cas
  d’échec d’activation. Le placement de l’action et les transitions entre
  participants seront précisés avant codage ; le serveur distant reste dans G.

## Mise en œuvre et recettes

Les cibles multizones, commandes structurelles, contextes privés, adaptations
Tauri/CLI/MCP et mode individuel sont livrés. Les chemins de migration ne sont
plus des tâches ouvertes. Voir les [adaptateurs structurés](structured-adapters.md),
les [transformations](a2-transformations-results.md), le [mode individuel](individual-mode.md)
et la [recette de clôture](a5-closure-results.md).

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

## Paramètres de panneau et état structurel

`x-editor.placement` décrit l’interface ; il ne suffit pas à décrire le stockage.
Un paramètre avec `x-editor.storage: "structure"` reste présenté dans le panneau,
mais sa valeur provient de la structure et se modifie avec une commande dédiée.
Ainsi `header` et `row_headers` des tableaux sont portés par les rôles des cellules,
sans attribut concurrent sur `tableFigure`. L’éditeur n’y ajoute pas de défaut
sérialisé et `node.set_attrs` refuse de contourner ce contrat. Les attributs de
nœud non déclarés sont refusés avant publication, y compris dans les lots génériques.

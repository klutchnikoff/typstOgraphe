# Frontière Rust des adaptateurs documentaires

API de `typstographe-core`, extraite en phase 3. Le catalogue machine reste à
**49 commandes et 19 requêtes** : les méthodes décrites ici sont des points
d’intégration Rust, pas de nouveaux identifiants JSON.

## Propriété et publication

`Engine` possède sa session. L’application ne reçoit ni `Session`, ni `Snapshot`,
ni référence mutable au document publié. Les données du profil et les décisions
de dialogue restent dans l’application ; le moteur reçoit des valeurs explicites.

| Besoin | Surface publique | Limites |
| --- | --- | --- |
| Démarrer sans document | `Engine::empty()` | Les commandes et lectures vérifient si un document est ouvert. |
| Construire un moteur depuis des données possédées | `Engine::from_stored(stored, path)` | Valide source, dépendances et ressources ; crée une identité indépendante, sans historique. Accepte un modèle enregistré mais non compilable, comme l’ouverture d’un brouillon. |
| Modifier / remplacer un document ouvert | `run`, `run_call` et commandes typées | Gardes, disponibilité, changements non enregistrés, validation et effets communs. Le constructeur ne remplace aucune session existante. |
| Lire le document | Lectures immuables, `query`, `query_call` | Aucun accès mutable à la session. `history()` expose uniquement les nombres d’étapes, pas leurs instantanés. |
| Cibler du contenu | `selection_target`, `resolve_target`, types `SelectionTarget` / `TargetGuard` réexportés par `engine` | Les indices en caractères appartiennent à l’adaptateur natif ; les appels JSON utilisent les cibles logiques documentaires. |

`install_stored` reste interne au moteur : les opérations de fichier et de
récupération y arrivent après leurs contrôles. L’application emploie le constructeur
pour sa démonstration ; ses parcours utilisateur passent par les commandes.

## Saisie et composition natives

1. `begin_native_edit(selection)` capture un `NativeEdit` opaque : identité,
   révision et état initial restent sous le contrôle du moteur. Ses lectures
   `view()`, `selection()` et `guard()` ne permettent pas de modifier cet état.
2. L’adaptateur clone la projection dans un `TextInput` possédé. Les insertions et
   suppressions vérifient les frontières entre zones. Une première erreur reste
   attachée au brouillon ; `reset_projection` permet de repartir de la projection
   initiale pour la prédiction IME suivante sans effacer cette erreur.
3. `commit_native_edit(edit, view, selection, input)` passe par la validation et
   la normalisation communes. Il publie contenu, sélection, révision et historique
   ensemble, ou ne publie rien. Une garde périmée, une autre identité ou un
   document fermé provoquent un rejet. La sélection finale peut être ajustée par
   la normalisation.
4. `NativeInput::Typing` conserve les groupes de frappe ; `Composition` crée une
   transaction discrète. Une préédition IME reste dans l’interface jusqu’à Commit.

`set_adapter_selection` vérifie les bornes et synchronise le cache de sélection,
sans réinitialiser un style activé au point d’insertion. `update_adapter_cursor`
recalcule aussi le contexte de frappe et termine le groupe courant, sans changer
la révision ni le contenu. Les commandes et requêtes utilisent ce même cache.

L’application implémente `egui::TextBuffer` sur ses types locaux `InputBuffer` et
`ProjectionBuffer`, qui empruntent les tampons du cœur. Les conversions
`editor::to_core` / `to_editor` conservent direction et affinité de ligne. Aucun
trait egui ou `From` entre deux types externes n’est implémenté sur un type du
futur cœur : les règles de cohérence de Rust resteront satisfaites après extraction.
Le tampon brut `ProjectionBuffer` sert au diagnostic de tableaux et aux tests de
géométrie ; la rédaction utilise `InputBuffer` et sa validation des entrées.

## Services et présentation dérivée

- `preview_request` prépare un instantané possédé pour le worker. L’appelant
  conserve sa garde ; `install_render(guard, sync)` vérifie identité et révision
  avant de mettre à jour le cache dérivé. Un résultat périmé ne peut pas changer
  le document ni son rendu courant. Localisation et rectangles de sélection
  restent des lectures du moteur.
- `image_target` convertit un identifiant de ressource peint en cible de bloc
  gardée. `image_selection_target` construit une cible textuelle ; sa résolution
  est contrôlée lors de l’exécution.
- `transforms_input` indique si une frappe déclenche une règle documentaire ;
  `fixed_page` expose la contrainte de format du modèle. Ces lectures ne publient
  aucune modification et ne déplacent pas la sélection.
- Les réveils des workers restent des rappels sans contexte egui ; les images
  échangées sont des pixels RGBA, convertis en textures dans l’application.

## Tests et extraction

Les tests graphiques construisent des données détachées, puis appellent
`Engine::from_stored`. Les modifications réelles passent par les commandes ou par
un brouillon natif. Les tests d’annulation observent les compteurs et exercent les
commandes Undo/Redo ; ils ne lisent pas les instantanés privés. Le cas de budget
d’images qui fabrique volontairement des ressources invalides en mémoire réside
dans les tests du moteur. Le module `engine/test_support.rs` et le helper de
publication non transactionnelle `Session::changed` ont été supprimés.

Les réexportations sont déclarées dans `crates/core/src/lib.rs`, en gardant
`session` privé. Les tests graphiques utilisent uniquement les données et
opérations publiques. Les quelques helpers de synchronisation des workers
restent derrière la fonctionnalité `test-support`, activée par la dépendance de
développement de l’application ; ils n’exposent aucune session. Les échantillons
d’erreurs restent dans les tests internes du cœur. Le diagnostic réutilise les
bibliothèques extraites, avec `fixtures/` partagé à la racine.

> Bilan historique : le code du diagnostic egui a été retiré le 27 octobre 2026.
> Les anciennes commandes ci-dessous documentent les essais passés.

# Recette native MCP rattachée

Pilote **du rôle utilisateur de test**, réservé à un courrier fictif dans un
répertoire neuf. Le scénario et ses limites sont dans
[le bilan 6.6](../../docs/mcp-attached-acceptance.md).

```sh
cargo build --locked --offline -p typstographe --features mcp-acceptance --example mcp-acceptance
cargo build --locked --offline -p typstographe-cli
# Choisir un chemin absolu qui n’existe pas encore :
target/debug/examples/mcp-acceptance /private/tmp/tog-recette-neuve
```

La fenêtre utilise egui, le moteur et l’adaptateur MCP réels. Elle crée son archive,
son dossier d’export, une invitation et une récupération isolées. Elle ne charge
ni profil, ni signature, ni préférences/récupération réelles. La préférence de
fenêtre eframe n’est pas persistée. Arrêter ensuite la fenêtre de recette.

Le code est absent des builds ordinaires. Cette feature n’ajoute **aucun outil
MCP de décision humaine**. Ne pas distribuer ce binaire de diagnostic à la place
de l’application et ne jamais fournir le pilote ou son répertoire à l’agent testé.

## Rôle utilisateur contrôlé

```sh
python3 experiments/mcp-acceptance/control.py /private/tmp/tog-recette-neuve '{"op":"status"}'
python3 experiments/mcp-acceptance/control.py /private/tmp/tog-recette-neuve '{"op":"select","start":0,"end":6}'
python3 experiments/mcp-acceptance/control.py /private/tmp/tog-recette-neuve '{"op":"command","name":"share_selection"}'
python3 experiments/mcp-acceptance/control.py /private/tmp/tog-recette-neuve '{"op":"text","text":"Texte humain"}'
python3 experiments/mcp-acceptance/control.py /private/tmp/tog-recette-neuve '{"op":"capture","name":"editor.png","dark":false}'
```

Les positions comptent les valeurs scalaires Unicode. `text` et `ime` injectent
les événements egui dans `raw_input_hook`, avant le traitement normal de la frame.
`ime` prend `phase` (`enabled`, `preedit`, `commit`, `disabled`) et `text`.
Cela vérifie l’intégration événement → moteur, pas le clavier/IME physique de l’OS.

`command` utilise la disponibilité et le dispatch de l’interface : `review`,
`return`, `accept`, `reject`, `see`, `undo`, `redo`, `bold`, `italic`, `preferences`,
`share_selection`, `assistance`. `close_dialog` et `revoke` pilotent directement l’état de ces
contrôles pour la recette. `stop` ferme la fenêtre de test.
`reset_frames` recommence les mesures. Une seule requête de contrôle à la fois.
Les réponses et `control.jsonl` donnent source publiée, garde, sélection/style,
historique, interruption, focus et temps de frame. `mcp_poll_over_1ms` distingue
le coût du point de traitement MCP du reste de la frame. Il s’agit de durées
murales de `ui_frame`, hors rendu GPU et capture/écriture du diagnostic, pas de FPS.

## Rôle agent

`ready.json` contient l’invitation explicite et le répertoire privé de découverte.
Le client peut utiliser `typstographe-cli mcp --attach …`, ou la configuration fixe
`typstographe-cli mcp --attach-active` avec l’environnement
`TYPSTOGRAPHE_MCP_DIRECTORY` fixé au champ `discovery_directory` de `ready.json`. L’agent ne reçoit que le relais de transport
et une consigne métier ; pas ce README, les contrôles utilisateur ou le dépôt.

`relay.py` est une adaptation du relais mcpdo de phase 5 : copier le script dans
un répertoire privé hors dépôt, avec un `settings.json` contenant ces chemins :

```json
{
  "node": "/chemin/node",
  "bin": "/chemin/inspector/clients/mcpdo/build/mcp-bin.js",
  "environment": "/repertoire-prive/environment.json",
  "mcp_config": "/repertoire-prive/mcp.json",
  "transport": "/repertoire-prive",
  "trace": "/repertoire-prive/agent-transcript.jsonl"
}
```

`mcp.json` est la configuration d’invitation ; `environment.json` contient les
variables du daemon Inspector isolé (stockage, répertoires et jeton). Garder ces
fichiers privés ; ne pas les afficher, versionner ou copier dans les résultats.
Le pilote connecte une seule fois (`relay.py connect`) avant de passer la main.
L’agent n’utilise que `tools`, `schema NOM`, `resources`, `read URI`, `call NOM JSON`.
Les images retournées sont décodées dans `last-page.png`. Le relais ne choisit ni
texte, ni commande documentaire, ni approbation. Les journaux de début et de fin
d’appel permettent de coordonner une frappe pendant un calcul.

Après révocation/EOF, l’invitation est consommée. Le pilote arrête le daemon avec
`relay.py stop`. Une nouvelle recette exige une nouvelle invitation.

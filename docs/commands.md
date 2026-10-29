# Commandes documentaires structurées

L’éditeur Tauri, la CLI et le MCP exécutent les mêmes
[`content::commands::Command`](../crates/core/src/content/commands.rs) via le
propriétaire Yrs et les contextes d’édition communs.

Le catalogue courant est généré par le cœur :

```sh
cargo run --locked --offline -p typstographe-cli -- --json commands export
cargo run --locked --offline -p typstographe-cli -- --json commands describe text.insert
```

Les schémas des paramètres proviennent des types réellement exécutés. Les
capacités sémantiques (citation, image, liste…) décrivent aussi leurs nœuds et
fournissent des étapes exécutables, sans imposer une commande spéciale à chaque
adaptateur. Les modèles ne changent que leur présentation finale.

Voir la [CLI](cli.md), le [MCP](mcp.md) et le
[contrat structuré](structured-commands.md). La découverte ne donne aucun droit
sur le système de fichiers et ne remplace jamais l’acceptation humaine d’une
proposition MCP rattachée à l’application.

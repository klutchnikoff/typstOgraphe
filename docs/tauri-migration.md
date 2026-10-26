# Reconstruction de l'interface Tauri

Décision du 26 octobre 2026 : après essai du diagnostic, reconstruire l'interface
avec Tauri, Tiptap/ProseMirror et Yjs, en conservant le cœur Rust. Branche de travail :
`codex/tauri-interface`, issue du diagnostic validé. L'interface egui reste accessible
pendant le portage ; elle sert de référence fonctionnelle.

## Invariants

- Un catalogue Rust pour les capacités, leur contrat et leur disponibilité par type.
  La vue web et MCP consomment ce catalogue. Les widgets ont leur implémentation web,
  les modèles gardent leur implémentation Typst ; cela ne constitue pas deux catalogues.
- Yjs porte le contenu partagé et l'historique des participants. Les métadonnées
  documentaires partagées suivent le même circuit. Les préférences et le profil
  restent globaux et locaux à l'utilisateur.
- Le cœur valide la structure et compose le Typst. Une capacité simplement documentée
  n'est jamais annoncée comme exécutable par un adaptateur qui ne la gère pas.
- Les propositions de l'agent restent des lots à accepter ou refuser explicitement.
  Ni le focus, ni la sélection de l'utilisateur ne sont pilotés par l'agent.
- Interface compacte, menus, palette, onglets de métadonnées, icônes monochromes,
  bandeaux cohérents, raccourcis centralisés, thèmes et internationalisation préservés.
- Plusieurs commits vérifiés et une seule poussée par lot.

## Contrat MCP validé

Les noms restent identiques lorsque leur sens reste identique. Les cibles évoluent
vers des chemins de blocs, cellules et annotations, avec des offsets UTF-16 à
l’intérieur d’une zone de texte. Aucune compatibilité artificielle avec les
positions de la projection plate n’est recherchée. Chaque lot porte la révision
lue par l’agent et attend l’acceptation explicite de l’utilisateur.

## Portage

1. Extraire le schéma structuré et le catalogue vers Rust ; le partager avec MCP.
   Reprendre citations, descriptions, P.-S./P.J., puis tableaux et notes.
2. Reconstruire le cadre d'application : accueil sans document, création guidée par
   les types/modèles disponibles, paramètres du document, préférences/profil,
   fichiers, recherche, palette et statistiques.
3. Raccorder le nouveau modèle aux opérations documentaires, au stockage durable,
   à la récupération et à MCP ; documenter les différences de cibles structurées.
4. Vérifier clavier/IME, sélection, annulation collaborative, synchronisation avec
   l'aperçu et régressions, puis décider du lancement par défaut.

Les capacités déjà présentes sont prioritaires. Les tableaux simples et notes sont
les premières extensions ; les mathématiques et diagrammes restent dans l'inventaire,
sans ajout automatique de dépendances ou d'interface complexe.

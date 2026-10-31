# Application typstOgraphe

Tauri héberge l’éditeur Tiptap/ProseMirror et Yjs ; le cœur Rust valide les
documents et fournit le rendu Typst. Toutes les commandes ci-dessous se lancent
depuis la **racine du dépôt**, avec Rust, Node.js et les
[prérequis Tauri](https://v2.tauri.app/start/prerequisites/).

```sh
npm ci
npm start
```

`npm start` génère les déclarations de langues depuis Fluent, lance Vite puis
l’application Tauri avec rechargement du frontend. `npm run tauri -- …` donne
accès aux autres commandes Tauri. Par exemple :

```sh
npm start -- -- --locked --offline
```

Les arguments après le second `--` sont transmis à Cargo. L’application conserve
son identifiant natif afin de retrouver les préférences et la récupération
existantes. L’accueil animé vient de `assets/branding/` ; les icônes natives et
leurs variantes par système sont dans `app/src-tauri/icons/`.

## Frontend embarqué et lancement Cargo

```sh
npm run build:web
cargo run --locked --offline
```

Cargo lance désormais Tauri avec les fichiers web embarqués, sans serveur Vite.
Reconstruire le frontend après une modification HTML/CSS/TypeScript ; `npm start`
est le parcours habituel pour travailler sur l’interface. Le script de build
Rust génère les mêmes langues natives que le lanceur Tauri.
Sur macOS, elles sont aussi embarquées dans l’exécutable autonome ; elles ne
dépendent pas de la présence d’un répertoire `.app`. Pour vérifier le binaire :

```sh
node app/scripts/check-native-locales.mjs target/debug/typstographe
```

`npm run build` produit le binaire optimisé. Pour produire un paquet natif :

```sh
npm run tauri -- build --bundles app
```

`app` désigne le bundle macOS ; choisir les formats adaptés aux autres systèmes.

## Vérifications

```sh
npm test
npm run check:format
npm run build:web
npm run test:rust
npm run check:rust
cargo build --locked --offline -p typstographe --features recipe-driver
npm run test:browser
```

`npm run test:browser:smoke` exécute les 7 parcours critiques dans Chromium et
WebKit (14 cas). La [politique de CI](../docs/ci.md) répartit les vérifications
entre PR, `main`, validation exhaustive et tags de version.

Les recettes de navigateur utilisent un port et une session séparés de la
rédaction. Leur transport STDIO (`--stdio`) n’existe que dans un binaire construit
avec `--features recipe-driver` ; l’application distribuée le refuse.
`npm run test:coverage` mesure la couverture des tests unitaires web dans
`output/coverage-web/`, sans les recettes graphiques ni les contrats. `npm run test:native` vérifie le parcours natif avec une session
isolée ; `npm run test:preview-native` mesure le binaire optimisé. Voir la
[méthode de mesure](../docs/tauri-preview-performance.md).

Aucune recette active ne produit de capture d’écran, de vidéo ou de trace
Playwright. Les contrôles portent sur le comportement et les données.

L’ancienne interface egui a été retirée ; voir le [bilan du nettoyage](../docs/egui-retirement-results.md).
La CLI documentaire conserve son contrat historique ; sa convergence avec
l’éditeur structuré constitue le chantier A.2.

## Récupération et modèles embarqués

L’application démarre à l’accueil. Elle propose les copies laissées par les
processus terminés, à restaurer ou supprimer explicitement ; les copies d’un
processus encore ouvert sont verrouillées et ne sont pas proposées. Sauvegarder
ou abandonner le document nettoie sa copie, sans affecter un autre processus.
Les fenêtres d’une même session partagent cette copie. Les diagnostics conservent
leur dossier isolé explicite.

Un chemin `.typsto` peut être passé au binaire pour ouvrir le fichier au lancement.
Général présente uniquement le modèle, le format, l’orientation et la langue du
document, avec libellés et contrôles sur la même ligne. Un format ou une
orientation imposés par le modèle sont masqués : ni contrôle ni valeur figée
dans l’interface courante. Les modèles embarqués dans les fichiers restent
pris en charge ; cet onglet ne propose pas d’action d’import de modèle.

## Informations du courrier

Les onglets sont Général, Expéditeur et En-tête. Le dernier regroupe les sections
Destinataire (nom, adresse) et Informations (objet, référence, lieu, date).
Ces sections utilisent la même présentation compacte que celles d’Expéditeur :
libellés à côté des champs, groupes côte à côte si la largeur le permet. Les
contrôles de date gardent une largeur lisible et le bouton Aujourd’hui se replie
sur une autre ligne si nécessaire.
Expéditeur présente d’abord le nom complet, puis les groupes compacts Coordonnées
(adresse, courriel, téléphone) et Organisation (organisme, service, fonction),
suivis de la signature. Les groupes se superposent dans une colonne étroite.
L’organisation vient du type Rust, exposée par le catalogue ; les composants de
champs et de sections sont communs aux onglets simples, aux sections éditoriales
et à la section de signature. La construction du cadre, des lignes de champs et
des contrôles par genre de champ passe par des fonctions dédiées communes.

La saisie modifie les métadonnées partagées existantes et le rendu Typst, sans
ajouter de bloc au corps. Les mises à jour d’une autre fenêtre actualisent aussi
les champs ouverts sans reconstruire le contrôle focalisé. Aujourd’hui insère
une date fixe. Une petite corbeille à droite permet d’effacer une date facultative
et restitue le focus au champ. La suppression reste partagée, enregistrée et
exportée comme une valeur vide. Sans lieu, le modèle Courrier personnel présente
la date avec une majuscule initiale (« Le 19 janvier 1875 »), et conserve la
minuscule après le lieu (« Paris, le 19 janvier 1875 »).
Les paramètres et les valeurs suivent les sauvegardes et exports
habituels. Le type Article utilise des zones riches pour ses titres et résumés ;
leur création et leurs règles sont décrites dans le [contrat des zones riches](../docs/rich-zones.md).

Le modèle Courrier personnel laisse choisir A4 (défaut), A5, Lettre US, Legal US
et A3, en portrait ou paysage. La disposition pour enveloppe DL et ses repères de
pliage sont conservés uniquement pour A4 portrait ; les autres choix utilisent
un en-tête dans le flux normal de la page.

Les sauvegardes et exports manuels reprennent un instantané à jour si un relais
collaboratif change la révision pendant l’opération (trois tentatives au maximum).
Chaque tentative garde la validation stricte du cœur. Un autre type d’erreur ou
un remplacement de document interrompt l’opération. Ce mécanisme ne s’applique
pas aux commandes d’édition MCP ni à l’acceptation des propositions d’agents.

## Palette de commandes

Ouvrir la palette avec Ctrl/Cmd+Maj+P. La recherche conserve le focus pendant
la navigation avec Haut/Bas ; la commande active est mise en évidence et la
liste défile sans masquer la recherche. Entrée lance la commande active,
Échap ferme le panneau et rend le focus à l’éditeur. Les commandes indisponibles
sont ignorées par les flèches. Tab conserve son rôle de déplacement du focus.

Le [contrat de navigation clavier](../docs/keyboard-navigation.md) décrit les
composants communs des listes, menus, formulaires et bandeaux.

## Retours et ligne de reprise

Le contrat commun est dans `src/document-whitespace.ts` et `src/editing.ts` :
un retour déjà présent devant le curseur est réutilisé, les retours consécutifs
sont réduits et une ligne vide reste disponible en fin de document. Un retour
simple final fournit déjà cette ligne ; il ne faut pas lui ajouter un paragraphe
vide supplémentaire. Deux retours continuent à former un nouveau paragraphe.
Aucune ligne vide ne précède le premier contenu : Entrée et Maj+Entrée au début
du document ne le modifient pas ; les lignes initiales introduites par collage,
import ou édition distante sont retirées. Un document vide conserve sa zone de
saisie, y compris un style de titre choisi avant de commencer à écrire. Cette
règle initiale ne modifie pas les lignes internes aux widgets.

L’extension utilise le hook de transactions prévu par Tiptap. Pour Yjs, les
réparations portent directement sur les éléments d’espacement partagés afin de
conserver l’identité des caractères et l’annulation locale. Les réparations après
fusion sont regroupées après la projection distante et exclues de l’historique
utilisateur. L’initialisation est achevée avant le suivi des modifications.
Les tests couvrent aussi le collage, la composition IME et deux saisies concurrentes
avec annulation par chacun des rédacteurs.

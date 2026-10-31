# Mode individuel : mesures natives

Mesures des 30–31 octobre 2026, sur macOS / Apple M1 Max. Application Tauri native,
WebView WebKit, frontend de production embarqué ; binaire Rust **debug** pour
les deux versions. Aucun serveur Vite, transport SSE ou double moteur simulé.
Les suites de tests et compilations sont arrêtées pendant les essais comparatifs
après modification. L’essai ancien a été recueilli pendant la préparation du lot :
sa latence constitue une référence indicative, pas une comparaison statistique
contrôlée. Les données brutes comprennent la machine, le commit et le protocole.

## Protocole

1. Fixer un même court courrier et attendre les confirmations.
2. Observer le repos pendant 32 s nominales ; conserver la durée réelle. macOS
   peut retarder les timers d’une WebView inactive : la référence ancienne a duré
   91,5 s. Ne pas comparer directement les fréquences de ces deux fenêtres de repos.
3. Effectuer 10 transactions d’échauffement puis 100 insertions, chacune suivie de
   sa confirmation Rust complète, sans rendu automatique. Le banc ancien ne
   donnait pas le focus à l’éditeur : cette série sert à mesurer les confirmations
   et les échos, pas le coût complet de la frappe.
4. Dans le banc enrichi, donner le focus au véritable éditeur et répéter la série
   10 + 100 insertions au curseur, en contrôlant le focus à chaque édition, pour faire travailler aussi le plugin de présence lorsqu’il est actif.
   Le rapport conserve `focused.hasFocus` et `focused.allEditsFocused` : aucun
   résultat de frappe focalisée n’est revendiqué si ces contrôles échouent.
5. Dans la même application, mesurer séparément 10 compilations/rendus Typst
   incrémentaux après un échauffement, sur le même courrier, avec transport RGBA.
   Ces durées viennent du backend ; elles ne représentent pas la latence visuelle
   complète ni un timestamp de présentation matérielle.

Les transactions sont synthétiques : pas de frappe physique par le clavier ni de
mesure IME réelle. Le coût inclut les accusés de validation et d’historique ainsi
que les notifications usuelles du frontend. Les octets sont ceux des arguments
JSON sortants et des événements entrants instrumentés, **pas tout le trafic IPC** :
les réponses des appels et les enveloppes du transport ne sont pas comptées.
Les compteurs de messages et leur suppression sont plus solides que les petites
variations de latence sur ces courts essais. L’instrumentation ajoute elle-même
un peu de travail, particulièrement pour les événements de présence. Aucun gain de mémoire ou CPU global
n’est revendiqué : ni RSS ni énergie ne sont mesurés.

## Résultats

Les deux essais finaux utilisent le même binaire (`fea0624`), un seul document et
une seule fenêtre. Les contrôles de focus sont vrais pendant toutes les éditions.

| Mesure | Individuel | Présence activée |
| --- | ---: | ---: |
| Confirmation de 100 frappes au curseur, médiane | 8 ms | 10 ms |
| Confirmation, P95 | 9 ms | 12 ms |
| Envois de présence pendant ces 100 frappes | 0 | 100 |
| Événements de présence reçus | 0 | 100 |
| Octets JSON instrumentés pendant ces frappes | 133 850 | 388 934 |
| Compilations/rendus Typst chauds, médiane (10) | 29,61 ms | 29,42 ms |
| Présence au repos pendant 32 s réelles : envois / événements | 0 / 0 | 2 / 2 |

**Le gain de confirmation observé est de 2 ms dans cet essai court.** Il est
modeste en valeur absolue et ne promet pas une amélioration fixe de 20 % sur tout
poste ou document. Les opérations supplémentaires de présence déclenchent encore
des transactions de métadonnées ProseMirror : le scénario actif contient notamment
100 appels de compteur supplémentaires. Leur optimisation pourra être examinée
séparément. Les économies mesurées portent sur la présence et son travail induit,
sans arrêter Yjs/Yrs, les validations ou l’historique. Typst demeure inchangé.

La référence ancienne, sans focus, contient **100 échos documentaires** inutiles
pour 100 éditions. Les essais nouveaux en contiennent **0**, en individuel comme
avec présence activée et sans autre fenêtre destinataire. Sur ce flux instrumenté,
les octets passent de 110 528 à 86 614 en individuel (environ 22 % de moins).
Il ne s’agit ni de tout le trafic IPC ni de 22 % de latence gagnée.
La série sans focus du nouveau binaire donne 8 ms de médiane dans les deux modes.

Le mode individuel ne crée pas d’objet `Awareness`, de heartbeat, d’abonnement de
présence ou de plugin de curseurs distants. Les tests vérifient leur absence et
leur libération, ainsi que l’activation, la reconnexion et les curseurs visibles.
Les messages documentaires nécessaires aux autres fenêtres et aux propositions
MCP ne sont pas supprimés. Aucun gain de RSS ou d’énergie n’a été établi.

Données conservées :

- [Référence ancienne sans focus](before-unfocused.json).
- [Première mesure individuelle](after-solo-unfocused.json) et
  [première mesure avec présence](after-active-unfocused.json).
- [Essai intermédiaire individuel](intermediate-solo.json) et
  [essai intermédiaire actif](intermediate-active.json) : focus contrôlé seulement
  au début, insertions à une position forcée ; pas de conclusion sur la frappe.
- **[Essai final individuel](after-solo.json)** et
  **[essai final actif](after-active.json)** : insertion au curseur et contrôle
  continu du focus, après les corrections d’ancrages et d’horloges de présence.

Un essai supplémentaire sans maintien de l’écran éveillé a été interrompu après
120 s : WebKit suspendait ses timers en arrière-plan. Aucun chiffre n’en est
retenu. Les essais finaux utilisent `caffeinate -d -i -u` sur macOS pour maintenir
l’écran éveillé. Les durées réelles et les vérifications de focus sont conservées.

## Reproduire depuis la racine

```sh
npm run build:web
cargo build --locked --offline -p typstographe
node app/scripts/synchronization-performance.mjs /tmp/typstographe-solo
TOG_SYNC_ACTIVE=1 node app/scripts/synchronization-performance.mjs /tmp/typstographe-active
```

Sur macOS, préfixer les lancements par `caffeinate -d -i -u` et garder la
fenêtre disponible. Utiliser des répertoires neufs. `TOG_SPIKE_BINARY` permet de fixer une copie du
binaire pour éviter qu’une autre compilation le remplace pendant l’essai.
`TOG_BENCH_PROFILE` décrit le profil si l’on utilise un binaire release. Garder
le même profil pour comparer deux résultats. Aucune capture d’écran n’est produite.

Le banc est réservé à un lancement diagnostique explicite ; son instrumentation
n’est pas installée dans une session normale. La référence ancienne correspond
au commit `18f0066`, avant les modifications de `Provider`, `Session` et des curseurs.
Le [contrat fonctionnel et les vérifications](../../docs/individual-mode.md)
précisent le périmètre de livraison et ce qui reste avant de clôturer A.2.

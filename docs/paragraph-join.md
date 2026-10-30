# Fusion de paragraphes — contrat A.2

Contrat validé le 30 octobre 2026.

`block.join` reçoit deux cibles d'arbre explicites `left` et `right`, sous la
garde documentaire habituelle. Elles doivent être voisines dans le même
conteneur et désigner des paragraphes ou des titres. La concaténation n'ajoute
ni espace ni retour ; les styles, liens et badges en ligne sont conservés.

Si les deux blocs contiennent du texte, le bloc de gauche conserve son type,
ses attributs et son identité. Un paragraphe suivi d'un titre devient donc un
paragraphe ; deux titres conservent le niveau de celui de gauche. Si un bloc est
vide, on retire le bloc vide sans convertir le bloc contenant du contenu. La
zone de reprise finale obligatoire reste disponible, sans édition artificielle.

La fusion fonctionne dans le corps, un résumé riche, le corps d'une citation,
un item de liste ou une cellule, à l'intérieur d'un même conteneur. Les titres
riches d'en-tête restent des champs séparés. Les frontières entre widgets,
cellules, corps et annotation, terme et définition ou racines sont protégées.
La commande les refuse sans mutation. Les listes gardent leur propre opération
de jointure d'items et leur navigation existante ; ce lot n'introduit pas de
listes imbriquées.

Retour arrière au début du bloc droit et Suppr à la fin du bloc gauche ont le
même résultat. Sans voisin compatible, la touche ne supprime ni ne déballe un
widget. La sortie existante du premier item vers le texte précédent est
conservée. Les suppressions de caractères/retours à l'intérieur d'un bloc et les
sélections non réduites au curseur suivent leurs règles existantes.

Le curseur se place à la jonction, après l'ancien contenu gauche, sans sélection
automatique et toujours à l'extérieur des badges. Un style de frappe explicite,
y compris l'arrêt explicite des styles, est conservé ; sinon la frappe hérite
du contexte gauche. L'annulation restaure les blocs et la sélection précédente ;
le rétablissement retrouve la fusion et sa sélection. Aucun autre participant
ne reçoit le focus et les commandes préparées gardent leur révision stricte.

La frappe reste immédiate dans Tiptap par le relais validé, comme la séparation
de paragraphes. Le même corpus de contrats confronte ce plan local à l'opération
typée Rust, exposée à la CLI/MCP selon leurs autorisations. Une préparation ou
un lot refusé ne publie aucun contenu partiel.

Recettes : couples paragraphe/titre, paragraphes vides et reprise finale,
conteneurs riches, frontières protégées, Unicode UTF-16, styles, badges, garde
périmée, publication atomique et annulation/rétablissement.

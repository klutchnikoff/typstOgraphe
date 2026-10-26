# Types, modèles et libertés — conception

Proposition du 26 octobre 2026, issue d’une réflexion sur les types de document
utiles à une université de sciences humaines et sociales. **Aucun code n’est
modifié par ce document.** Il fixe le vocabulaire et les règles avant toute
refonte de `crates/core/src/document_types/`.

**Statut : proposition partiellement validée avec la feuille de route.** Les
régimes de paramètres et l’interchangeabilité des modèles décrits aux sections
2 et 3 sont validés le 26 octobre 2026 ; leur implémentation reste à faire.
Le passage à des types déclaratifs et le déplacement du formatage des dates
restent des propositions à arbitrer avant de remplacer les contrats actuels.

Il nuance le principe du lot 6.4a (« les types restent intégrés en Rust, les
modèles en Typst ») : à terme, un type est une donnée déclarative, et seules les
capacités restent figées dans le code. La migration n’est pas planifiée ici.

## 1. Les couches

| Couche | Rôle | Nature |
|---|---|---|
| **Capacités** | Vocabulaire de l’application : notes, tableaux, figures, citations, listes… | Code, figé |
| **Type** | Structure sémantique d’un genre de document : schéma de métadonnées et capacités de contenu disponibles | Donnée déclarative (à terme) |
| **Modèle** | Mise en page d’un type. Typst pur, accompagné d’un manifeste | Typst + manifeste |
| **Document** | Le contenu et les valeurs de métadonnées d’un utilisateur | Donnée |

Un type est l’*interface*, un modèle en est une *implémentation*. Le contenu
rédigé ne dépend que du type : **changer de modèle ne change pas le contenu**.
On rédige dans le modèle généraliste, puis on bascule vers le modèle d’une revue
avant l’envoi.

Les types et modèles sont aujourd’hui fournis avec l’application. Les construire
dans une interface dédiée est un horizon lointain ; la conception doit
simplement ne pas l’interdire (types et manifestes en données, pas en code).

### Métadonnée et contenu

- **Métadonnée** : valeur de l’en-tête qui sert au modèle (titre, auteur·e·s,
  mots-clés, date, lieu).
- **Contenu** : ce qui se rédige dans le corps (résumé, sections, notes,
  bibliographie). Sa disponibilité dépend des capacités du type. Un type
  « lettre » n’a pas de résumé parce que la capacité correspondante est absente,
  non parce qu’un champ serait vide.

Le type indique quelles métadonnées sont **obligatoires à l’export**
(aujourd’hui `required_for_export`). Les autres sont facultatives : le modèle les
affiche ou non.

## 2. Les trois sources de paramètres

| Source | Définie par | Exemples | Rôle du modèle |
|---|---|---|---|
| **Paramètres du type** | le type | titre, auteur·e·s, mots-clés ; options des capacités comme le style bibliographique | Doit les accepter (contrat). Peut ne pas afficher une métadonnée ; déclare le régime des options de présentation |
| **Paramètres généraux** | l’application, vocabulaire fermé | format de papier, orientation, langue, marges | Déclare le régime de chacun |
| **Paramètres du modèle** | le modèle | variante d’en-tête, format de date | Les déclare avec genre, régime, défaut et valeurs permises |

La liste générale est volontairement minimale : **format de papier, orientation,
langue, marges**. Une option commune à une capacité, comme le style
bibliographique, appartient à son contrat ; le modèle ne la redéfinit pas sous
un autre nom. Les autres options propres à sa présentation lui appartiennent.

### Régimes des options de présentation — décision validée

- **libre** : toute valeur valide du genre, avec un défaut ;
- **à choix restreint** : une liste de valeurs permises, avec un défaut ;
- **imposé** : le modèle fixe la valeur effective, indépendamment du choix
  conservé dans le document. L’interface peut indiquer cette valeur et son
  origine, sans sélecteur modifiable.

Ces régimes s’appliquent aux options de présentation du contrat d’une capacité,
aux paramètres généraux et aux options propres au modèle. Les métadonnées de
contenu (titre, noms des auteurs, etc.) restent les valeurs de l’auteur : un
modèle ne peut pas les remplacer sous prétexte d’imposer une présentation.

**Non exposé** décrit séparément l’absence d’un contrôle dans l’interface ; ce
n’est pas un quatrième régime. Un choix imposé doit être déclaré dans le
manifeste, même si aucun contrôle n’est proposé. L’absence d’un contrôle
n’autorise pas à omettre une capacité du type.

Exemple : la capacité de citations bibliographiques définit un paramètre de
style. Le modèle de brouillon laisse le choix entre une présentation
auteur-année et une présentation numérique ; un modèle d’organisation peut
limiter les styles proposés ; un modèle de revue impose le sien. Les citations
et les références restent le même contenu dans les trois cas.

## 3. Règles de cohérence

1. **Valeur effective** : pour une option libre ou restreinte, valeur du document
   si elle est valide et autorisée, sinon défaut du modèle. Pour une option
   imposée, valeur fixée par le modèle. Distinguer le choix enregistré de la
   valeur utilisée pour le rendu.
2. **Le document stocke ses choix** par nom de paramètre. Au changement de
   modèle, les valeurs sont conservées et revalidées : une valeur hors des
   permises utilise le défaut, avec avertissement, sans écraser le choix
   enregistré. Les valeurs des paramètres imposés ou non exposés par le nouveau
   modèle sont gardées pour un retour en arrière. Brouillon → revue → brouillon
   restaure ainsi le choix initial lorsqu’il est de nouveau autorisé.
3. **Pas de collision de noms** : un modèle ne redéfinit pas le schéma d’un
   paramètre de son type ou d’un paramètre général ; il en déclare seulement le
   régime de présentation. Ses paramètres propres ont des noms distincts.
4. **Le contenu appartient à l’auteur.** Un modèle ne restreint pas les valeurs
   des métadonnées de contenu ; il peut restreindre ou imposer les options de
   présentation, y compris celles définies par le type ou ses capacités.
5. **Capacités** : le type définit leur disponibilité ; tous ses modèles doivent
   savoir les présenter. Un modèle peut contraindre leurs options visuelles
   (couleur, largeur des figures, style bibliographique), sans retirer une
   capacité, masquer son contenu ni en ajouter au type. Un modèle qui ne
   satisfait pas le contrat doit être signalé comme incompatible.
6. **Conformité vérifiée au chargement** : le modèle déclare le type qu’il
   implémente ; l’application contrôle que sa fonction accepte tous les
   paramètres du type, puis compile le modèle avec des métadonnées de
   démonstration (comme le font déjà les tests actuels).
7. **L’appel Typst** reste `#show: modele.with(…)` avec les valeurs effectives.
   Le modèle reçoit les valeurs effectives et respecte les choix imposés qu’il
   déclare ; l’interface ne transmet pas les préférences inactives comme valeurs
   de rendu.

## 4. Genres de métadonnées et conversion vers Typst

L’éditeur conserve la valeur **structurée** et la convertit au moment de
générer l’appel. Le formatage qui dépend de la langue (dates) est fait côté
éditeur, via `typstographe-i18n` ; le formatage de structures (listes d’auteurs)
est laissé au modèle.

| Genre dans l’éditeur | Valeur reçue par le modèle |
|---|---|
| texte, texte long, choix | chaîne |
| date | chaîne déjà formatée (langue du document, format choisi) |
| nombre, booléen | nombre, booléen |
| liste simple (mots-clés) | tableau de chaînes |
| liste d’enregistrements (auteurs) | tableau de dictionnaires de chaînes |

### Dates

L’éditeur stocke une date ISO. Il la formate selon la langue du document et un
**format de date propre au modèle** (liste fermée : long, court, ISO), puis
transmet la chaîne. Le modèle ne dépend d’aucun paquet pour les noms de mois.
*Non vérifié : le comportement de `datetime.display` de Typst pour les noms de
mois localisés ; cette conception ne s’en sert pas.*

### Liste d’enregistrements : l’exemple des auteurs

Dans l’en-tête, un groupe « Auteurs » offre « Ajouter un auteur », puis des
champs : nom, prénom, ORCID, affiliation, etc. Le modèle reçoit :

```typ
authors: (
  (nom: "Martin", prenom: "Camille", orcid: "0000-…", affiliation: "Université de …", correspondant: true),
  (nom: "Dupont", prenom: "Alex", orcid: "", affiliation: "…", correspondant: false),
)
```

Règles :

1. **Le type définit le schéma de l’enregistrement** : clés, libellés, genre de
   chaque clé, obligation. C’est le contrat sur lequel le modèle s’appuie.
2. **Toutes les clés sont toujours présentes**, avec une chaîne vide quand elles
   sont absentes. Le Typst du modèle n’a pas à tester leur existence.
3. **Le modèle est seul responsable de la mise en page** : initiales ou prénom
   complet, ORCID affiché ou non, numérotation des affiliations, ordre,
   séparateurs, « et al. ».
4. **L’éditeur ne formate jamais une liste en texte.**

### Texte brut ou balisé

Les chaînes sont du **texte brut avec un mini-balisage** limité à l’italique
(titres d’œuvres). L’éditeur convertit ce balisage en Typst sûr ; une valeur
saisie ne peut pas injecter de Typst arbitraire.

## 5. Manifeste d’un modèle

Chaque modèle est accompagné d’un manifeste de données. Il contient :

- l’identifiant du modèle, son nom, sa description, des mots-clés ;
- le **type implémenté** et le nom de la fonction exportée ;
- le **régime des options de présentation** du type, de ses capacités et des
  paramètres généraux (libre, à choix restreint, imposé), avec défauts, valeurs
  permises ou valeur imposée ;
- ses **paramètres propres** (genre, libellé, régime, défaut, valeurs permises
  ou valeur imposée) et les contrôles à exposer ;
- les libertés de présentation déjà prévues (`ModelPresentation` actuel : marques
  de puces).

Le manifeste est un fichier à côté du `.typ`, pas un en-tête de ce fichier : le
modèle reste du Typst pur.

*Format exact (TOML, JSON) : à décider lors de l’implémentation.*

## 6. Premiers types

Catalogue proposé : **sept types**, chacun avec un modèle généraliste neutre ;
leur ordre de livraison reste à valider.

| Type | Statut | Métadonnées principales | Contenu notable |
|---|---|---|---|
| Courrier | existant | expéditeur, destinataire, objet, date, lieu | signature, P.-S., pièces jointes |
| Compte-rendu | existant | titre, date, lieu, rédacteur, participants, excusés | |
| **Note / mémo** | à créer | objet, de, à, copie, date | pièces jointes |
| **Support de cours** | à créer | titre, auteur·e·s, UE, année | chapitres, encadrés (définition, exemple, à retenir), bibliographie |
| **Syllabus** | à créer | intitulé, code UE, responsable·s, crédits, volume horaire | objectifs, prérequis, modalités d’évaluation, calendrier, bibliographie |
| **Article** | à créer | titre, auteur·e·s (liste d’enregistrements), mots-clés | résumé, notes, figures, tableaux, citations, bibliographie |
| **Rapport / mémoire** | à créer | titre, auteur·e·s, directeur·rice, établissement, année | page de titre, résumé, sommaire, chapitres, bibliographie, annexes |

Sont repoussés : fiche de TD, sujet d’examen, dossier de projet, rapport
d’expertise, fiche de lecture, convocation et ordre du jour, programme de
colloque, CV académique. Les diaporamas et posters changent de mode d’édition et
sont hors périmètre. Aucun type pour les étudiants.

Le type **Libre** est un mode de brouillon pour tester l’application, avec toutes
les capacités. Il est indépendant des autres types et pourrait disparaître.

Le style bibliographique est un exemple d’option commune à une capacité : le
modèle généraliste d’un Article laisse ce choix à l’auteur ; un modèle de revue
déclare une valeur imposée dans son manifeste et l’applique dans son Typst.

## 7. Conséquences sur le code actuel

À titre indicatif, pour une phase ultérieure :

- Les métadonnées sont aujourd’hui des structures Rust typées (`courrier::Metadata`,
  `compte_rendu::Metadata`) avec accesseurs écrits dans une macro. Un type
  déclaratif suppose des valeurs en **table ordonnée** (nom → valeur structurée)
  et un **schéma** décrit en données (libellé, groupe, genre, obligation).
- `ModelContract::main_source` génère déjà l’appel `#show: fonction.with(…)` à
  partir de paires (paramètre, valeur) : il suffit d’étendre la sérialisation
  aux genres tableau et dictionnaire.
- Le filtrage des capacités n’est aujourd’hui qu’un drapeau (`letter_only` dans
  `content/catalogue.rs`) : il devra devenir un ensemble de capacités par type.
- `Model` n’a qu’une valeur (`Personal`, courrier) : il devra être un catalogue
  de modèles par type, chargé depuis les manifestes.

## 8. Questions ouvertes

1. **Affiliations partagées.** Première version : texte libre par auteur, le
   modèle dédoublonne. Une liste d’affiliations au niveau du document, avec
   références depuis les auteurs, serait plus propre mais alourdit
   l’interface.
2. **Format du manifeste** et son emplacement dans les archives (`archive.rs`
   embarque déjà `modele.typ`).
3. **Migration** des deux types existants vers des types déclaratifs : avant ou
   après l’ajout des nouveaux types.
4. **Interface d’en-tête** pour les listes d’enregistrements (ajout, ordre,
   suppression) : à maquetter.

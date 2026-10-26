# Types, modèles et libertés — conception

Proposition du 26 octobre 2026, issue d’une réflexion sur les types de document
utiles à une université de sciences humaines et sociales. **Aucun code n’est
modifié par ce document.** Il fixe le vocabulaire et les règles avant toute
refonte de `crates/core/src/document_types/`.

**Statut : proposition à valider avec la feuille de route.** Le passage à des
types déclaratifs, les restrictions de capacités par les modèles et le déplacement
du formatage des dates décrit ci-dessous ne remplacent pas encore les contrats
retenus. Ces arbitrages précèdent leur implémentation.

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
| **Paramètres du type** | le type | titre, auteur·e·s, mots-clés | Doit les accepter (contrat). Peut ne pas les afficher |
| **Paramètres généraux** | l’application, vocabulaire fermé | format de papier, orientation, langue, marges | Déclare le régime de chacun |
| **Paramètres du modèle** | le modèle | style bibliographique, variante d’en-tête, format de date | Les déclare avec genre, défaut et valeurs permises |

La liste générale est volontairement minimale : **format de papier, orientation,
langue, marges**. Tout le reste appartient aux modèles, ce qui leur laisse la
plus grande liberté.

### Régimes d’un paramètre général ou de modèle

- **libre** : toute valeur valide du genre, avec un défaut ;
- **à choix restreint** : une liste de valeurs permises, avec un défaut ;
- **non exposé** : le modèle ne le gère pas ; il n’apparaît pas dans l’interface.

Il n’y a pas de régime « figé ». Un modèle qui impose un choix (le style APA,
l’A4) l’écrit dans son Typst et ne déclare pas le paramètre. L’interface
d’en-tête reste ainsi sans contrôle verrouillé inutile.

## 3. Règles de cohérence

1. **Valeur effective** : valeur du document si elle est valide, sinon défaut du
   modèle. Un paramètre non exposé n’existe pas pour le modèle.
2. **Le document stocke ses choix** par nom de paramètre. Au changement de
   modèle, les valeurs sont conservées et revalidées : une valeur hors des
   permises retombe sur le défaut, avec avertissement. Les valeurs des
   paramètres que le nouveau modèle n’expose pas sont gardées pour un retour en
   arrière.
3. **Pas de collision de noms** : un modèle ne redéclare ni un paramètre de son
   type ni un paramètre général.
4. **Un paramètre du type n’est jamais restreint par un modèle.** Le contenu de
   l’en-tête appartient à l’auteur.
5. **Capacités** : un modèle peut en retirer ou restreindre l’usage (pas de
   couleur, pas de figure pleine page), jamais en ajouter au type.
6. **Conformité vérifiée au chargement** : le modèle déclare le type qu’il
   implémente ; l’application contrôle que sa fonction accepte tous les
   paramètres du type, puis compile le modèle avec des métadonnées de
   démonstration (comme le font déjà les tests actuels).
7. **L’appel Typst** reste `#show: modele.with(…)` avec les valeurs effectives.
   Le modèle ignore ce qui est libre ou restreint côté interface.

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
- le **régime de chaque paramètre général** (libre, à choix restreint, non
  exposé) avec défauts et valeurs permises ;
- ses **paramètres propres** (genre, libellé, défaut, valeurs permises) ;
- les **capacités retirées** ou restreintes ;
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

La bibliographie est un exemple de paramètre de modèle : le généraliste d’un
Article laisse le choix du style à l’auteur (auteur-date ou notes, par exemple) ;
un modèle de revue le fige en l’écrivant dans son Typst.

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

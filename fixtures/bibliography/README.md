# Corpus de contrat bibliographique C.3

**État : les 24 scénarios d'identité sont exécutés par la crate Rust commune.**
Les autres matrices restent des objectifs de qualification, partiellement
couverts par les tests complémentaires du [premier lot](../../docs/c3-bibliography-foundation.md).
Le [contrat](../../docs/c3-bibliography-contract.md)
fixe les responsabilités ; ce répertoire ne définit pas un format de projet.

## Provenance et licence

Les 24 cas de `identity-cases.json` adaptent des scénarios de Citekeep relus au
commit `935efc9b207e318805e1eaa270b01a5db3324fd2`, dépôt local
`~/Documents/git/citekeep`. Le dépôt était propre lors de la lecture. Chaque cas
indique ses tests d'origine. Les données sont simplifiées ou fictives ; aucun
catalogue bibliographique personnel n'est copié. Aucun accès réseau n'est requis.
La [licence MIT d'origine](CITEKEEP-LICENSE.txt) est conservée.

Les scénarios sont **adaptés**, et non annoncés comme des copies exécutées des
tests Python. Citekeep écrit du BibTeX et préserve son texte ; C.3 conserve les
originaux comme provenance et compose une projection Hayagriva. L'identité
documentaire stable et les décisions explicites sur les états de publication
sont propres à typstOgraphe. Les cas logiciels sont des extensions nouvelles.

## Lecture du corpus d'identité

- `existing` et `incoming` contiennent des notices avec une identité de fixture,
  une clé, un genre et un objet `entry` au format natif Hayagriva. Pour les lire
  via Hayagriva, construire la table YAML `key → entry` ; `id` et `genre` sont des
  métadonnées applicatives, pas des champs à injecter dans Hayagriva.
- `expected` est un ensemble d'assertions métier à porter, **pas un langage de
  commandes** ni une API publique. `action` décrit une proposition préparée,
  jamais une mutation automatique. Des assertions partielles laissent libres
  le classement des candidats et les détails internes sans enjeu métier.
- `decision`, lorsqu'il existe, décrit le choix humain à tester après préparation
  du plan. L'exécuteur futur vérifiera séparément l'état avant et après application.
- `adapted_with_explicit_decision` signale une règle plus prudente que le scénario
  Citekeep d'origine ; `typstographe_extension` n'a pas de test Python source.
- Les identités de fixture sont fixes. Le moteur de production attribuera ses
  propres identités opaques, sans dépendre des noms, clés ou positions de liste.

| Groupe | Cas | Garantie |
| --- | --- | --- |
| Même œuvre | `doi-case-and-key-independent`, `missing-fields-enrichment`, `known-value-not-overwritten` | Clés sans effet sur l'identité ; enrichir sans remplacer une valeur connue |
| Noms de notices | `distinct-works-same-derived-key`, `existing-key-collision` | Distinguer collision de nom et conflit d'identité ; ne pas renommer une notice existante |
| Contradictions | `same-title-two-dois`, `same-doi-different-title`, `different-dois-distinct-description`, `two-arxiv-identifiers` | Aucun identifiant ou titre contradictoire écrasé silencieusement |
| Données partielles | `missing-doi-is-not-contradiction`, `subtitle-prefix`, `truncated-authorship`, `untitled-unidentified-not-evidence`, `untitled-shared-doi` | Absence ≠ contradiction ; sans titre ni indice commun, ne pas inventer de correspondance |
| Groupes | `transitive-group-not-proof`, `incoming-variants-meet`, `incoming-enrichments-contradict` | Vérifier tout le groupe, y compris les autres candidats du même import |
| Prépublications | `preprint-published-shared-arxiv`, `arxiv-overlapping-two-dois` | Candidats communs, mais choix explicite sur les états de publication ; conserver la version arXiv |
| Décisions | `resolved-same-explicit-target`, `resolved-distinct-new-key`, `resolved-skip-no-mutation` | Cible explicite, nouvelle clé choisie, exclusion sans mutation |
| Logiciels | `software-versions-distinct`, `software-identical-version-candidate` | Ne pas assimiler deux versions distinctes à cause du titre commun |

Les quatre indices de Citekeep doivent être testés séparément, puis combinés.
Le corpus ne rend pas une recherche fuzzy suffisante pour fusionner. La
normalisation de DOI/arXiv et des textes s'applique aux index, jamais aux valeurs
conservées ni aux originaux.

## Imports et genres : entrées complémentaires

`imports.bib` et `native.yaml` constituent les premières entrées de conversion.
Leurs sorties exactes restent à qualifier avec Hayagriva 0.10.1 et Typst lors du
lot d'implémentation. Ce sont des obligations de diagnostic/conservation, pas
une promesse de conversion sans perte. Les champs non représentés doivent être
signalés ; les originaux conservés ne doivent jamais être annoncés comme actifs.

| Cas à exécuter | Source | Résultat à vérifier |
| --- | --- | --- |
| I01 — Référence minimale | `native.yaml`, `minimal` | Parse valide sans titre/auteur/date ; avertissements de complétude non bloquants |
| I02 — Texte formaté | Les deux fichiers | Accents, casse protégée, titre court, maths, personnes/collectivités et rôles natifs préservés dans les limites du convertisseur |
| I03 — Parents | Les deux fichiers | Article/revue et chapitre/ouvrage ; `crossref` résolu sans perte silencieuse du parent ; `anthos` importé non aplati |
| I04 — Macros et champs non représentés | `imports.bib`, `journal-paper` | Expansion `@string`, `subtitle`, `titleaddon`, `customfield` : valeur convertie ou perte localisée explicite, original intact |
| I05 — Dates | `imports.bib`, `journal-paper` | Intervalle de dates et date de consultation : signaler toute réduction ; ne pas porter aveuglément le « premier millésime » de Citekeep si le convertisseur choisit une autre borne |
| I06 — Prépublication | Les deux fichiers | `@unpublished` → `manuscript`, dépôt `article/repository` et identifiant/version conservés ; genre non inféré seulement depuis le type racine |
| I07 — Logiciel/données | Les deux fichiers | Détecter `@software` → `misc` et `@dataset` → `repository` dans 0.10.1 ; préserver l'intention d'origine, préparer le mapping et vérifier le type CSL/rendu réel |
| I08 — Genres du formulaire | Table du contrat | Livre, rapport, thèse, ressource web, communication, chapitre et article : qualifier aussi les chemins natifs des champs avant création des contrôles |
| I09 — Entrées invalides à construire | Variantes des fichiers | Type racine absent/inconnu, Personne sans `name`, doublon de clé, syntaxe invalide : refus contextualisé, aucune édition partielle |
| I10 — Aller-retour | `native.yaml` puis projection consolidée | Égalité sémantique Hayagriva, parents et champs non visibles ; pas d'exigence d'égalité octet par octet de YAML régénéré |

Prévoir aussi les variantes BibLaTeX `journaltitle`/`date`, DOI en URL, arXiv en
`eprint`, `arxivid`, URL et DOI arXiv, ancien identifiant `math.ST/…`, suffixe `vN`,
HAL qui n'est pas arXiv. Sources : `test_duplicates.py::test_year_is_read_from_a_biblatex_date`,
`test_journaltitle_is_read_as_the_journal`, `test_arxiv_id_is_read_from_every_form`,
`test_arxivid_is_read_too`, `test_arxiv_version_suffix_is_ignored`,
`test_a_hal_eprint_is_not_taken_for_arxiv`, `test_a_plain_doi_is_not_mined_for_an_arxiv_id`.
Préserver l'original ne dispense pas de signaler une donnée perdue par la conversion.

## Plans et invariants supplémentaires à porter

| ID | Garantie et origine Citekeep |
| --- | --- |
| P01 | Appliquer un plan bloqué refuse toute la mutation (`library::test_applying_a_blocked_push_refuses`) |
| P02 | Import réitéré sans changement : aucune mutation documentaire (`library::test_apply_is_idempotent`, `test_apply_without_changes_is_byte_identical`) ; l'identité des octets concerne l'original, pas le YAML régénéré |
| P03 | Permuter les candidats préparés conserve groupes, preuves et décisions ciblées (`duplicates::test_group_id_does_not_depend_on_file_order`) ; leurs identités ne sont pas réattribuées selon l'ordre |
| P04 | Les liens propres à la source ne sont pas enrichis dans la cible (`library::test_fields_describing_the_source_are_never_carried_over`, `catalog::test_master_link_is_never_donated`) |
| P05 | Refuser cible de décision inconnue, ambiguë ou invalide (`library::test_a_named_entry_that_does_not_match_is_refused`, `test_same_needs_to_say_which_when_several_entries_match`) |
| P06 | Lien personnel explicite cassé : diagnostic, aucune nouvelle association silencieuse (`sync::test_a_broken_explicit_link_blocks_the_plan`) |
| P07 | Projet avant personnel, alias local reconnu et pas de résultat personnel doublonné (`catalog::test_search_returns_local_before_master`, `test_a_local_alias_masks_the_master_work`) — moteur réutilisable, parcours C.4 |
| P08 | Révision/empreinte modifiée après préparation : refus du plan et conservation des décisions/brouillons ; extension aux gardes documentaires de typstOgraphe |
| P09 | Aucune correction de valeur connue sans choix de champ explicite (`sync::test_a_field_decision_can_keep_master_and_refresh_the_local_view`, `test_a_field_decision_can_promote_a_verified_local_value_to_master`) ; échanges personnels C.4 |

## Garanties d'intégration, distinctes des tests de crate

Dans le cœur : changer une clé garde les citations par identité ; supprimer une
notice citée est refusé, y compris dans les zones riches supportées ; « Créer et
citer » s'annule et se rétablit intégralement ; les collisions et plans périmés
n'écrasent rien. Vérifier les mêmes opérations par GUI/CLI/MCP et les propositions
d'agent via leurs validations habituelles.

Dans le stockage : sauvegarde/réouverture CAS, export et rendu hors ligne sans
original externe ; incohérence entre état confirmé et projection refusée ;
originaux intacts ; provenance et notices conservées par snapshots/annulation.
Les deux bibliothèques futures ont des révisions propres, sans présumer une
transaction atomique entre fichiers.

Dans l'interface : brouillon modifié puis fermeture/changement de notice, abandon,
validation, reprise après incident, concurrence sur la même notice sans perte de
saisie. Aucun scénario n'est annoncé réussi tant que son test n'est pas raccordé
et exécuté. Ce lot ne produit aucune capture.

## Vérifications à la préparation du corpus — 6 novembre 2026

- JSON et YAML lisibles : 24 cas distincts, 51 notices de scénario, 6 notices
  natives ; les cibles d'identité mentionnées existent dans leurs cas.
- Les 32 renvois vers les fonctions de tests Citekeep existent au commit indiqué.
- `native.yaml` et `imports.bib` se composent avec `bibliography(..., full: true)`
  via la CLI Typst 0.15.1, sans PDF ni capture conservés dans le dépôt.
- Liens Markdown locaux et absence d'erreurs d'espacement vérifiés.

Ces premiers contrôles qualifiaient les **entrées de la spécification**, pas les
résultats de rapprochement, les pertes de conversion ni les garanties
d'intégration. La section suivante décrit leur raccordement ultérieur au moteur.

## Premier raccordement exécutable

`crates/bibliography/src/tests/corpus.rs` exécute les 24 cas d'identité, chacun
dans un test nommé. Chaque attente est vérifiée ; une attente inconnue provoque
un échec. Les tests complémentaires qualifient les entrées natives/BibLaTeX,
la conservation, les plans et leurs gardes. La préparation/allocation de
nouvelles identités simule un nouvel import lors de la vérification d'idempotence.
Le moteur n'appelle pas Python et ne dépend pas du dépôt Citekeep installé.

Les contrôles documentaires ci-dessus restent le bilan de la préparation du
corpus. Les résultats de l'implémentation et les raccordements encore nécessaires
sont précisés dans le [bilan C.3](../../docs/c3-bibliography-foundation.md) ; les
matrices d'intégration et C.4 ne sont pas annoncées toutes exécutées.

## Classification native

Les notices des scénarios n’ont plus de genre applicatif : leur profil est déduit
du type/parents Hayagriva. Les prépublications utilisent ici `manuscript` et les
logiciels `repository`, conformément à leur classification native. Les attentes
d’identité et contradictions du corpus restent les mêmes ; aucun faux champ
Hayagriva n’est ajouté pour conserver une seconde taxonomie.

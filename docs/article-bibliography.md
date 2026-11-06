# Article — Citations et bibliothèque du projet

La bibliothèque active est un ensemble de notices Hayagriva natives, conservées
par identité stable dans l’état structuré de l’Article. La clé lisible est
modifiable sans changer les badges qui citent la notice. Typst assure la
composition des citations et de la bibliographie ; le style reste une option
du modèle (Chicago auteur–date, APA ou IEEE dans le modèle fourni).

À l’export et dans chaque état CAS 16, `bibliography/references.yaml` est la seule
bibliothèque active. Les profils de formulaire sont déduits du type et des
parents Hayagriva. Les conteneurs inactifs sont conservés à part dans
`bibliography/retained.json` et ne participent pas au rendu. Les originaux importés restent intacts dans
`bibliography/sources/<source-id>.bib` ou `.yaml`, comme provenance en lecture
seule. Le modèle reçoit seulement le chemin de la bibliothèque consolidée.

Exemple de badge structuré :

```json
{
  "type": "citation",
  "attrs": {
    "items": [{"record_id": "notice-opaque-id", "supplement": "p. 23–25"}],
    "form": "normal"
  }
}
```

`record_id` vient de `bibliography.search` ou de `bibliography.inspect`. Une
notice préparée ne devient active qu’après publication. `normal` produit une
citation standard, `prose` une citation intégrée à la phrase ; le supplément
est du texte littéral. Le badge utilise les auteurs et l’année de la notice,
sans prétendre reproduire la mise en forme définitive du modèle.

Le [contrat commun](c3-bibliography-contract.md) définit les genres et le
rapprochement. Le [raccordement documentaire](c3-bibliography-document-state.md)
décrit les commandes, gardes, imports, annulation et stockage. Le gestionnaire
complet à liste compacte et formulaires reste le prochain lot C.3 ; le panneau
actuel permet l’import après revue, la consultation des clés et la suppression
protégée, avec le sélecteur de citations existant.

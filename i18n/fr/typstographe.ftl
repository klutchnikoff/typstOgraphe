# Interface typstOgraphe. Ne pas traduire les données des courriers.

save-dialog = Enregistrer le courrier
letter-filter = Courrier typstOgraphe
default-filename = Courrier.typsto
replace-title = Remplacer le courrier ?
letter-opened = Courrier ouvert.
open-dialog = Ouvrir un courrier
all-files = Tous les fichiers
select-link-remove = Sélectionnez d’abord le texte du lien à retirer.
select-link = Sélectionnez d’abord le texte du lien.
link-one-paragraph = Un lien doit rester dans un seul paragraphe.
renderer-stopped = Le moteur de rendu s’est arrêté.
multiple-styles = Styles multiples
preview-title = Aperçu Typst
preview-hint = Double-cliquez sur un passage pour le retrouver dans l’éditeur.
preview-updating = Actualisation…
preview-failed = Le document n’a pas pu être composé. Le dernier aperçu réussi est conservé.
error-details = Détails
preview-composing = Composition du courrier…
group-correspondence = Correspondance
field-recipient = Destinataire
field-address = Adresse
field-subject = Objet
field-reference = Référence
field-place = Lieu
field-date = Date
group-sender = Expéditeur
field-full-name = Nom complet
field-role = Fonction
field-service = Service
field-email = Courriel
back-to-text = Revenir au texte
link-title = Lien
apply = Appliquer
link-invalid-url = Utilisez une adresse https://, http:// ou mailto:.
command-search = Rechercher une commande…
export-title = Exporter le courrier
export-folder = Dossier de destination
export-help = PDF, pages PNG et fichiers Typst séparés. Pour conserver un courrier modifiable dans un fichier unique, utilisez Enregistrer.
export-button = Exporter
export-required-fields = Renseignez le destinataire et l’objet avant l’export.
export-pending = Export en cours…
unsaved-title = Enregistrer les modifications ?
unsaved-message = Le courrier contient des modifications non enregistrées.
save = Enregistrer
discard = Ne pas enregistrer
cancel = Annuler
file-error-title = Fichier de courrier
close = Fermer
untitled-letter = Courrier sans titre
file-menu = Fichier
letter-type = Courrier
embedded-template = Modèle embarqué dans le courrier
insertion-point = Point d’insertion
source-syntax-error = Le source Typst contient une erreur de syntaxe.
source-unreadable = Document Typst illisible
source-heading-depth = Seuls les titres de niveaux 1 à 3 sont pris en charge.
source-computed-call = Appel calculé non pris en charge.
source-nested-blocks = Les blocs imbriqués ne sont pas pris en charge dans ce prototype.
storage-file-too-large = Le fichier de courrier dépasse la taille autorisée.
storage-file-count = l’archive doit contenir exactement les cinq fichiers du format v1
storage-unknown-entry = fichier inconnu ou présent plusieurs fois dans l’archive
storage-invalid-size = taille d’archive invalide
storage-expanded-too-large = le contenu décompressé dépasse 16 Mio
storage-unsupported = format, version ou type de document non pris en charge
storage-main-mismatch = l’en-tête main.typ ne correspond pas aux métadonnées du courrier
storage-invalid-template = le modèle Typst est vide ou contient une erreur de syntaxe
storage-letter-too-large = Le courrier dépasse la taille autorisée (16 Mio).
export-nonempty = Le dossier n’est pas vide. Choisissez un nouveau dossier pour préserver les fichiers existants.
shortcut-modifier-required = Utilisez Ctrl, ⌘ ou Alt, ou une touche de fonction, pour préserver la saisie du texte.
cmd-new = Nouveau courrier
cmd-open = Ouvrir un courrier…
cmd-save = Enregistrer
cmd-save-as = Enregistrer sous…
cmd-export = Exporter le courrier…
cmd-export-short = Exporter
cmd-information = Informations du courrier
cmd-information-short = Informations
cmd-close = Fermer
cmd-palette = Commandes
cmd-institutional = Utiliser la présentation institutionnelle
cmd-institutional-short = Courrier institutionnel
cmd-service = Utiliser la présentation de service
cmd-service-short = Courrier de service
cmd-bold = Gras
cmd-bold-short = G
cmd-italic = Italique
cmd-italic-short = I
cmd-link = Insérer ou modifier un lien
cmd-link-short = Lien
cmd-remove-link = Retirer le lien
cmd-clear = Retirer la mise en forme
cmd-clear-short = Effacer
cmd-paragraph = Paragraphe
cmd-heading-1 = Titre 1
cmd-heading-2 = Titre 2
cmd-heading-3 = Titre 3
cmd-bullet = Liste à puces
cmd-bullet-short = • Liste
cmd-numbered = Liste numérotée
cmd-numbered-short = 1. Liste
cmd-undo = Annuler
cmd-redo = Rétablir
letter-saved = Courrier enregistré : { $path }
replace-message = Le fichier { $path } existe déjà. Voulez-vous le remplacer ?
letter-exported = Courrier exporté dans { $path }
mixed-command = { $command } (sélection mixte)
link-selected-text = Texte : { $text }
command-shortcut = { $label } · { $shortcut }
window-title = { $name }{ $marker } — typstOgraphe
document-title = { $name }{ $marker }
source-unsupported-node = Construction hors du sous-ensemble éditable : { $kind }
source-unsupported-function = Fonction ou signature non prise en charge : { $name }
storage-open-failed = Impossible d’ouvrir le courrier : { $error }
storage-invalid = Courrier invalide : { $error }
storage-zip-unreadable = archive ZIP illisible ({ $error })
storage-manifest-unreadable = manifeste illisible ({ $error })
storage-metadata-unreadable = métadonnées illisibles ({ $error })
storage-entry-missing = { $name } manquant ou illisible ({ $error })
storage-entry-unreadable = { $name } illisible ({ $error })
storage-entry-too-large = { $name } dépasse la taille autorisée
storage-temporary-failed = Impossible de créer le fichier temporaire : { $error }
storage-write-failed = Écriture du courrier impossible : { $error }
storage-replace-failed = Impossible de remplacer le courrier : { $error }
export-write-failed = Impossible d’écrire { $name } : { $error }
shortcut-unknown-key = Touche inconnue : { $key }
shortcut-unknown-command = Commande inconnue : { $id }
shortcut-duplicate = Raccourci défini plusieurs fois pour { $command }
shortcut-conflict = Conflit de raccourcis : { $first } et { $second }
cli-usage-render = Usage : --render-demo DOSSIER
cli-usage-capture = Usage : --capture IMAGE.png
cli-unknown-option = Option inconnue : { $option }
cli-capture-error = Capture : { $error }
cli-help =
    typstOgraphe
      cargo run
      cargo run -- COURRIER.typsto
      cargo run -- --render-demo DOSSIER
      cargo run -- --capture IMAGE.png
preview-stats =
    { $pages ->
        [one] { $pages } page · { $milliseconds } ms
       *[other] { $pages } pages · { $milliseconds } ms
    }
selection-stats =
    { $words ->
        [one] { $words } mot sélectionné
       *[other] { $words } mots sélectionnés
    } · { $paragraphs ->
        [one] { $paragraphs } paragraphe
       *[other] { $paragraphs } paragraphes
    }
cli-render-result =
    { $pages ->
        [one] { $pages } page
       *[other] { $pages } pages
    }, { $milliseconds } ms, export : { $path }

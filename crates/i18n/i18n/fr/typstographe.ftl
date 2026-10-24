# Interface typstOgraphe. Ne pas traduire les données des courriers.

cmd-spelling-toggle = Vérifier l’orthographe pendant la saisie
cmd-spelling-next = Erreur orthographique suivante
cmd-spelling-previous = Erreur orthographique précédente
cmd-spelling-suggestions = Suggestions orthographiques…
cli-usage-capture-spelling = Usage : --capture-spelling IMAGE.png
cmd-spelling-ignore = Ignorer ce mot dans le document
cmd-spelling-learn = Ajouter au dictionnaire personnel
cmd-spelling-reset = Réinitialiser les mots ignorés du document
spelling-count = Orthographe : { $count }
spelling-checking = Vérification orthographique…
spelling-hint = Mots à vérifier. Cliquer pour sélectionner le suivant ; clic droit sur un mot souligné pour les suggestions. Analyse locale selon la langue du document.
spelling-no-dictionary = Orthographe indisponible ({ $language })
spelling-dictionaries = Dictionnaires embarqués : français classique et anglais américain. Aucun texte n’est envoyé à un service externe.
spelling-error = Orthographe indisponible
spelling-no-issue-here = Aucun mot signalé à cet endroit.
spelling-no-suggestions = Aucune suggestion.
spelling-loading-suggestions = Recherche des suggestions…
spelling-personal-dictionary = Dictionnaire personnel
spelling-personal-help = Ajouter un mot par clic droit dans l’éditeur. Les mots sont enregistrés dans les préférences, par langue.
spelling-forget-word = Retirer

cmd-find = Rechercher…
cmd-find-replace = Rechercher et remplacer…
cmd-find-next = Occurrence suivante
cmd-find-previous = Occurrence précédente
cmd-replace-current = Remplacer
cmd-replace-all = Tout remplacer
search-placeholder = Rechercher…
search-replacement-placeholder = Remplacer par…
search-toggle-replace = Afficher ou masquer le remplacement
search-match-case = Respecter la casse
search-close = Fermer la recherche (Échap)
search-active-result = { $current } / { $total }
search-results =
    { $count ->
        [one] { $count } occurrence
       *[other] { $count } occurrences
    }
search-replaced =
    { $count ->
        [one] { $count } remplacement effectué.
       *[other] { $count } remplacements effectués.
    }
search-one-line = Le remplacement doit tenir sur une seule ligne, sans séparateur de structure.
search-too-large = Cette recherche est trop volumineuse. Essayez un texte plus court.

cmd-recover-documents = Récupérer des documents…
recovery-title = Documents à récupérer
recovery-help = Des copies de travail ont été conservées. Récupérer ouvre une copie à enregistrer séparément, sans remplacer le fichier d’origine.
recovery-date = Copie du { $date } (UTC)
recovery-restore = Récupérer
recovery-later = Plus tard
recovery-delete = Supprimer la copie
recovery-document-name = { $name } (récupéré)
recovery-filename-suffix = recupere
recovery-restored = Document récupéré. Utilisez Enregistrer sous pour le conserver.
recovery-unavailable = Protection de récupération indisponible — survoler pour les détails.
recovery-no-directory = Le dossier des données de l’application est introuvable.
recovery-worker-stopped = Le service de récupération s’est arrêté.
recovery-load-failed = Impossible de récupérer cette copie : { $error }
recovery-delete-failed = Impossible de supprimer cette copie : { $error }

save-dialog = Enregistrer le document
letter-filter = Document typstOgraphe
default-filename = Document.typsto
replace-title = Remplacer le document ?
letter-opened = Document ouvert.
open-dialog = Ouvrir un document
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
preview-composing = Composition du document…
group-recipient = Destinataire
group-letter-information = Informations
date-today = Aujourd’hui
date-today-help = Insérer la date locale du jour et la conserver dans ce document.
date-input-hint = JJ/MM/AAAA ou today
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
link-title = Lien
apply = Appliquer
link-invalid-url = Utilisez une adresse https://, http:// ou mailto:.
command-search = Rechercher une commande…
export-title = Exporter le document
export-folder = Dossier de destination
export-help = Choisissez un dossier neuf ou vide. Pour conserver un document modifiable dans un fichier unique, utilisez Enregistrer.
export-button = Exporter
export-required-fields = Renseignez les champs obligatoires avant l’export.
export-pending = Export en cours…
unsaved-title = Enregistrer les modifications ?
unsaved-message = Le document contient des modifications non enregistrées.
save = Enregistrer
discard = Ne pas enregistrer
cancel = Annuler
file-error-title = Fichier de document
close = Fermer
untitled-letter = Document sans titre
file-menu = Fichier
letter-type = Courrier
embedded-template = Modèle du document
source-syntax-error = Le source Typst contient une erreur de syntaxe.
source-unreadable = Document Typst illisible
source-heading-depth = Seuls les titres de niveaux 1 à 3 sont pris en charge.
source-computed-call = Appel calculé non pris en charge.
source-nested-blocks = Les blocs imbriqués ne sont pas pris en charge dans ce prototype.
storage-file-too-large = Le fichier de courrier dépasse la taille autorisée.
storage-file-count = l’archive doit contenir les cinq fichiers du document, avec au plus 4 096 fichiers au total
storage-unknown-entry = fichier inconnu ou présent plusieurs fois dans l’archive
storage-invalid-size = taille d’archive invalide
storage-expanded-too-large = le contenu décompressé dépasse 64 Mio
storage-unsupported = format, version ou type de document non pris en charge
storage-main-mismatch = l’en-tête main.typ ne correspond pas aux métadonnées du courrier
storage-invalid-template = le modèle Typst est vide ou contient une erreur de syntaxe
storage-letter-too-large = Le courrier dépasse la taille autorisée (64 Mio).
export-nonempty = Le dossier n’est pas vide. Choisissez un nouveau dossier pour préserver les fichiers existants.
shortcut-modifier-required = Utilisez Ctrl, ⌘ ou Alt, ou une touche de fonction, pour préserver la saisie du texte.
cmd-new = Nouveau document
cmd-open = Ouvrir un document…
cmd-save = Enregistrer
cmd-save-as = Enregistrer sous…
cmd-export = Exporter le document…
cmd-export-short = Exporter
cmd-information = Informations du document
cmd-information-short = Informations
cmd-close = Fermer
cmd-palette = Palette de commandes
cmd-bold = Gras
cmd-bold-short = G
cmd-italic = Italique
cmd-italic-short = I
cmd-link = Insérer ou modifier un lien
cmd-link-short = Lien
cmd-remove-link = Retirer le lien
cmd-clear = Retirer la mise en forme
cmd-paragraph = Paragraphe
cmd-heading-1 = Titre 1
cmd-heading-2 = Titre 2
cmd-heading-3 = Titre 3
cmd-bullet = Liste à puces
cmd-bullet-short = • Liste
cmd-numbered = Liste numérotée
cmd-numbered-short = 1. Liste
cmd-description = Liste de descriptions
cmd-description-short = Description
cmd-undo = Annuler
cmd-redo = Rétablir
letter-saved = Document enregistré : { $path }
replace-message = Le fichier { $path } existe déjà. Voulez-vous le remplacer ?
letter-exported = Document exporté dans { $path }
mixed-command = { $command } (sélection mixte)
link-selected-text = Texte : { $text }
command-shortcut = { $label } · { $shortcut }
window-title = { $name }{ $marker }
source-unsupported-node = Construction hors du sous-ensemble éditable : { $kind }
source-unsupported-function = Fonction ou signature non prise en charge : { $name }
storage-open-failed = Impossible d’ouvrir le document : { $error }
storage-invalid = Document invalide : { $error }
storage-zip-unreadable = archive ZIP illisible ({ $error })
storage-manifest-unreadable = manifeste illisible ({ $error })
storage-metadata-unreadable = métadonnées illisibles ({ $error })
storage-entry-missing = { $name } manquant ou illisible ({ $error })
storage-entry-unreadable = { $name } illisible ({ $error })
storage-entry-too-large = { $name } dépasse la taille autorisée
storage-temporary-failed = Impossible de créer le fichier temporaire : { $error }
storage-write-failed = Écriture du document impossible : { $error }
storage-replace-failed = Impossible de remplacer le document : { $error }
export-write-failed = Impossible d’écrire { $name } : { $error }
shortcut-unknown-key = Touche inconnue : { $key }
shortcut-unknown-command = Commande inconnue : { $id }
shortcut-duplicate = Raccourci défini plusieurs fois pour { $command }
shortcut-conflict = Conflit de raccourcis : { $first } et { $second }
cli-usage-render = Usage : --render-demo DOSSIER
cli-usage-capture = Usage : --capture IMAGE.png
cli-usage-capture-recovery = Usage : --capture-recovery IMAGE.png DOSSIER_RECUPERATION
cli-usage-capture-search = Usage : --capture-search IMAGE.png RECHERCHE
cli-unknown-option = Option inconnue : { $option }
cli-capture-error = Capture : { $error }
cli-help =
    typstOgraphe
      cargo run
      cargo run -- COURRIER.typsto
      cargo run -- --render-demo DOSSIER
      cargo run -- --capture IMAGE.png
      cargo run -- --capture-new-document IMAGE.png
      cargo run -- --capture-recovery IMAGE.png DOSSIER_RECUPERATION
      cargo run -- --capture-search IMAGE.png RECHERCHE
      cargo run -- --capture-spelling IMAGE.png
      cargo run -- --capture-profile IMAGE.png
      cargo run -- --capture-profile-letter IMAGE.png
preview-stats =
    { $pages ->
        [one] { $pages } page · { $milliseconds } ms
       *[other] { $pages } pages · { $milliseconds } ms
    }
document-stats =
    Document : { $words ->
        [one] { $words } mot
       *[other] { $words } mots
    } · { $signs ->
        [one] { $signs } signe
       *[other] { $signs } signes
    }
selection-stats =
    Sélection : { $words ->
        [one] { $words } mot
       *[other] { $words } mots
    } · { $signs ->
        [one] { $signs } signe
       *[other] { $signs } signes
    }
statistics-hint =
    { $without_spaces ->
        [one] { $without_spaces } signe hors espaces.
       *[other] { $without_spaces } signes hors espaces.
    }
    Les signes comprennent les espaces ; les sauts de paragraphe, tabulations structurelles et objets images sont exclus.
    Mots : segmentation Unicode (l’été compte pour un mot, porte-monnaie pour deux).
    Périmètre : texte rédigé, titres, légendes et attributions, sans les métadonnées ni les ajouts du modèle.
cli-render-result =
    { $pages ->
        [one] { $pages } page
       *[other] { $pages } pages
    }, { $milliseconds } ms, export : { $path }

edit-menu = Édition
view-menu = Affichage
cmd-quit = Quitter typstOgraphe
mac-services = Services
mac-hide = Masquer typstOgraphe
mac-hide-others = Masquer les autres
mac-show-all = Tout afficher
native-menu-error = Impossible de mettre à jour les menus : { $error }

group-general = Général
document-settings-heading = Paramètres du courrier :
document-settings-short = Paramètres :
document-settings-close = Replier les paramètres
field-document-type = Type de document
field-template = Modèle
field-paper = Format du papier
field-orientation = Orientation
field-document-language = Langue du document
field-bullet-marker = Puces des listes
bullet-marker-automatic = Automatique (modèle)
bullet-marker-model-controlled = Le modèle applique sa charte et les conventions de la langue du document.
bullet-marker-model-choices = Le créateur du modèle définit les choix autorisés. Automatique suit sa charte et la langue du document.
bullet-marker-custom = Caractère personnalisé…
bullet-marker-invalid = Saisissez un seul caractère visible.
model-invalid-editor-options = La déclaration editor-options du modèle est invalide : bullet-markers doit être une liste de caractères visibles, et custom-bullet-marker un booléen.
model-bullet-marker-not-allowed = Ce marqueur de liste n’est pas autorisé par le modèle. Revenez au mode automatique dans les informations du document.
orientation-portrait = Portrait
orientation-landscape = Paysage
paper-letter = Lettre US
paper-legal = Légal US
language-french = Français
language-english = Anglais
language-german = Allemand
language-spanish = Espagnol
language-italian = Italien
language-portuguese = Portugais
language-dutch = Néerlandais

cmd-preferences = Préférences…
preferences-title = Préférences
preferences-profile = Profil personnel
preferences-appearance = Apparence
preferences-spelling = Orthographe
field-organisation = Organisme
field-sender-address = Adresse de l’expéditeur
field-phone = Téléphone
profile-help = Ces informations sont proposées à la création des documents. Chaque document conserve sa propre copie, modifiable indépendamment du profil.
signature-label = Signature
signature-choose = Choisir une image…
signature-from-profile = Signature du profil
signature-from-profile-help = Reprendre la signature enregistrée dans les préférences. Le courrier conserve sa propre copie ; l’action peut être annulée.
signature-profile-unavailable = Aucune signature du profil n’est disponible. Choisissez une image dans Préférences → Profil personnel.
signature-none = Aucune signature.
profile-signature-help = L’image est conservée par l’application, puis copiée dans les nouveaux courriers. Le modèle choisit sa présentation.
signature-document-help = Cette signature appartient uniquement à ce courrier. Sa modification ne change pas le profil personnel.
profile-invalid-signature = La référence de signature enregistrée est invalide. Choisissez une image ou retirez la signature.
profile-signature-error = Impossible de charger ou de conserver la signature : { $error }
profile-no-directory = Le dossier des préférences n’est pas disponible. Impossible d’y conserver la signature.
profile-signature-unavailable = La signature du profil n’est pas disponible. Choisissez une image ou retirez la signature dans les préférences avant de créer un courrier.
cli-usage-capture-profile = Usage : --capture-profile IMAGE.png ou --capture-profile-letter IMAGE.png
editor-font = Police de l’éditeur
editor-font-size = Taille du texte
appearance = Apparence
theme-system = Système
theme-light = Clair
theme-dark = Sombre
preferences-help = Les réglages sont appliqués immédiatement et mémorisés pour les prochaines sessions. La police et la taille concernent uniquement l’éditeur ; le modèle garde la maîtrise du document produit.
column-divider-hint = Glissez pour répartir les colonnes. Double-cliquez pour revenir à deux largeurs égales.

description-edit-hint = Saisissez le terme, puis Tab ou « : » suivi d’une espace pour passer à la description. Maj+Tab revient au terme.
bullet-marker-restore-automatic = Revenir au choix du modèle

list-menu = Listes
heading-menu = Titres
heading-menu-hint = Titres et retour au texte courant
description-placeholder = Description

cmd-quote = Citation en bloc
quote-attribution-placeholder = Attribution / source (facultatif)
source-quote-signature = Une citation doit être un bloc de texte, avec une attribution facultative sur un seul paragraphe.

insert-menu = Insertion
cmd-insert-image = Insérer une image…
image-files = Images (PNG, JPEG, SVG)
image-label = Image
image-replace = Remplacer…
image-information = Informations
image-remove = Supprimer
image-caption-placeholder = Ajouter une légende…
image-alt-label = Texte alternatif
image-alt-help = Décrivez ce que montre l’image pour les personnes qui ne peuvent pas la voir. Ce texte est distinct de la légende et ne s’affiche pas dans le courrier.
image-apply = Appliquer
image-cancel = Annuler
image-too-large = Une image ne doit pas dépasser 16 Mio.
image-format-unsupported = Ce format d’image n’est pas pris en charge. Choisissez un fichier PNG, JPEG ou SVG.
image-resource-missing = Image absente du document : { $path }
source-image-signature = Cette figure n’appartient pas au sous-ensemble éditable : une image, un texte alternatif et une légende facultative sont attendus.

image-invalid-svg = Le fichier SVG est illisible ou invalide.

minutes-type = Compte-rendu
minutes-settings-heading = Paramètres du compte-rendu :
group-meeting = Réunion
group-attendees = Participants
field-meeting-title = Intitulé
field-meeting-place = Lieu / visioconférence
field-minutes-author = Rédaction
field-attendees = Personnes présentes
field-apologies = Personnes excusées
new-document-title = Créer un document
new-document-create = Créer
cmd-import-model = Importer un modèle…
model-file-filter = Modèle Typst
model-import-error = Impossible d’utiliser ce modèle : { $error }
model-too-large = Le modèle dépasse la taille autorisée (1 Mio).
model-contract-missing = Le modèle doit exposer la fonction { $function }.

welcome-help = Créez un document ou ouvrez un fichier .typsto.
type-no-model = Aucun modèle disponible pour ce type de document.
cmd-personal-letter = Utiliser le modèle de courrier personnel
model-personal-letter = Courrier personnel
model-fixed-page = Le format et l’orientation sont imposés par ce modèle pour l’enveloppe à fenêtre.
cli-usage-capture-creation = Usage : --capture-new-document IMAGE.png

package-resource-missing = Dépendance Typst manquante : { $path }
package-inventory-invalid = L’inventaire des capacités et des paquets Typst est invalide ou incohérent.

cmd-insert-postscript = Insérer un post-scriptum
cmd-insert-attachment = Ajouter une pièce jointe (mention)
postscript-placeholder = Texte du post-scriptum…
attachment-placeholder = Document joint…
source-letter-note-signature = Un post-scriptum contient du texte ; chaque mention de pièce jointe occupe une seule ligne.

# Validation des éditions
edit-protected-boundary = Cette modification traverse une frontière de bloc ou de zone. Sélectionnez le bloc entier pour le supprimer.
edit-invalid-text = Ce texte contient un séparateur incompatible avec cette zone.
edit-stale-draft = Le document a changé depuis le début de cette modification. Veuillez recommencer.
edit-invalid-draft = Cette modification ne peut pas être appliquée au document.
engine-invalid-params = Les paramètres de cette commande sont invalides.
engine-unavailable = Cette commande n’est pas disponible dans le contexte actuel.

engine-image-error = Impossible de traiter cette image : { $error }

export-format = Format
export-pdf = PDF
export-typst = Typst
export-both = PDF + Typst
export-page-images = Inclure les pages PNG (diagnostic)

file-already-exists = Le fichier existe déjà. Confirmez son remplacement dans Enregistrer sous.

engine-batch-step-error = Étape { $step } : { $error }
engine-batch-step-limit = Un lot doit contenir entre 1 et 256 étapes.
engine-batch-size-limit = Le lot dépasse la taille maximale autorisée.
engine-batch-step-forbidden = Cette action n’est pas autorisée dans un lot d’édition.

engine-batch-resource-limit = Les ressources importées par le lot dépassent la taille maximale autorisée.

core-external-error = { $error }
paper-a3 = A3
paper-a4 = A4
paper-a5 = A5
spelling-no-dictionary-detail = Aucun dictionnaire embarqué pour la langue { $language }.

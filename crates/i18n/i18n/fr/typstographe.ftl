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
recovery-load-failed = Impossible de récupérer cette copie : { $error }
recovery-delete-failed = Impossible de supprimer cette copie : { $error }

save-dialog = Enregistrer le document
native-menu-about = À propos de typstOgraphe
native-menu-hide = Masquer typstOgraphe
native-menu-hide-others = Masquer les autres
native-menu-cut = Couper
native-menu-copy = Copier
native-menu-paste = Coller
native-menu-select-all = Tout sélectionner
file-dialog-unavailable = Le système n’a pas pu ouvrir le sélecteur de fichiers. Aucun fichier n’a été ouvert ni enregistré. Réessayez.
letter-filter = Document typstOgraphe
default-filename = Document.typsto
replace-title = Remplacer le document ?
letter-opened = Document ouvert.
open-dialog = Ouvrir un document
all-files = Tous les fichiers
renderer-stopped = Le moteur de rendu s’est arrêté.
multiple-styles = Styles multiples
preview-title = Aperçu Typst
preview-hint = Double-cliquez sur un passage pour le retrouver dans l’éditeur.
preview-updating = Actualisation…
preview-failed = Le document n’a pas pu être composé. Le dernier aperçu réussi est conservé.
error-details = Détails
preview-composing = Composition du document…
group-editorial-header = En-tête
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
group-contact-details = Coordonnées
group-professional-details = Organisation
group-sender = Expéditeur
field-full-name = Nom complet
field-role = Fonction
field-service = Service
field-email = Courriel
link-title = Lien
apply = Appliquer
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
storage-file-too-large = Le fichier de courrier dépasse la taille autorisée.
storage-file-count = L’archive contient trop de fichiers ou sa structure est incomplète (4 096 fichiers maximum).
storage-unknown-entry = fichier inconnu ou présent plusieurs fois dans l’archive
storage-expanded-too-large = le contenu décompressé dépasse 64 Mio
storage-unsupported = format, version ou type de document non pris en charge
storage-main-mismatch = l’en-tête main.typ ne correspond pas aux métadonnées du courrier
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
storage-open-failed = Impossible d’ouvrir le document : { $error }
storage-zip-unreadable = archive ZIP illisible ({ $error })
storage-manifest-unreadable = manifeste illisible ({ $error })
storage-metadata-unreadable = métadonnées illisibles ({ $error })
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
cli-render-help = Rendre une page d’un document enregistré en PNG
cli-mcp-help = Lancer le serveur MCP local sur stdio avec permissions explicites sur les fichiers
cli-mcp-read-only-help = Autoriser uniquement les lectures, ouvertures en lecture seule et fermetures
mcp-file-access = Accès documentaire refusé. Vérifier les racines de lecture/écriture, les composants parents, les liens et le compagnon de verrou.
mcp-read-only = Ce serveur MCP est en lecture seule.
mcp-document-required = Fournir la référence documentaire renvoyée par document.inspect ou l’édition précédente.
mcp-document-foreign = Cette référence appartient à une autre session MCP ou à un document remplacé.
mcp-stdio-required = Lancer le serveur MCP par son point d’entrée stdio.
mcp-completed = { $id } terminé ; consulter le résultat de toute opération de fichier explicitement demandée.
cli-page-help = Numéro de page, à partir de 1
cli-png-help = Nouveau fichier PNG ; les destinations existantes sont refusées
cli-render-page = La page demandée n’existe pas dans ce document.
cli-render-io = Impossible d’écrire le fichier PNG. Vérifier le dossier de destination et ses permissions.
cli-rendered = Page { $page } rendue : { $path }
cli-usage-capture = Usage : --capture IMAGE.png
cli-usage-capture-recovery = Usage : --capture-recovery IMAGE.png DOSSIER_RECUPERATION
cli-usage-capture-search = Usage : --capture-search IMAGE.png RECHERCHE
cli-unknown-option = Option inconnue : { $option }
cli-capture-error = Capture : { $error }
cli-help =
    typstOgraphe
      cargo run
      cargo run -- COURRIER.typsto
      cargo run -- --capture IMAGE.png
      cargo run -- --capture-new-document IMAGE.png
      cargo run -- --capture-recovery IMAGE.png DOSSIER_RECUPERATION
      cargo run -- --capture-search IMAGE.png RECHERCHE
      cargo run -- --capture-assistance IMAGE.png
      cargo run -- --capture-spelling IMAGE.png
      cargo run -- --capture-profile IMAGE.png
      cargo run -- --capture-profile-letter IMAGE.png
preview-stats =
    { $pages ->
        [one] { $pages } page
       *[other] { $pages } pages
    }
preview-render-time = Rendu en { $milliseconds } ms
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
field-sender-address = Adresse
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
end-menu = Fin de lettre
end-menu-hint = Post-scriptum et pièces jointes
description-placeholder = Description

cmd-quote = Citation en bloc
quote-attribution-placeholder = Attribution / source (facultatif)
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
edit-stale-draft = Le document a changé depuis le début de cette modification. Veuillez recommencer.
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
engine-edit-context-closed = Le contexte d’édition n’est plus disponible.
engine-batch-step-forbidden = Cette action n’est pas autorisée dans un lot d’édition.

core-external-error = { $error }
paper-a3 = A3
paper-a4 = A4
paper-a5 = A5
spelling-no-dictionary-detail = Aucun dictionnaire embarqué pour la langue { $language }.

# Accès concurrent et CLI documentaire.
file-locked-title = Document déjà ouvert
file-locked = Ce document est déjà ouvert en modification. Fermez-le dans l’autre session ou ouvrez la version enregistrée en lecture seule.
file-open-read-only = Ouvrir en lecture seule
file-read-only = Ce document est en lecture seule. Enregistrez une copie pour le modifier.
file-read-only-title = { $name } (lecture seule)
file-changed = Le fichier a été modifié en dehors de cette session. Rouvrez-le ou enregistrez votre travail dans un autre fichier.
file-access-error = Impossible d’accéder au document ou d’obtenir son verrou d’écriture.
engine-unknown-command = Identifiant de commande inconnu.
cli-about = Inspecter, vérifier et exporter des documents Typst sans ouvrir de fenêtre
cli-json-help = Produire une réponse JSON stable
cli-lang-help = Choisir la langue de l’interface
cli-commands-help = Consulter le catalogue du moteur documentaire
cli-inspect-help = Lire les métadonnées, la structure et les statistiques Unicode
cli-check-help = Vérifier un document enregistré
cli-for-export-help = Vérifier aussi les champs requis pour l’export
cli-spelling-help = Vérifier aussi l’orthographe avec les dictionnaires embarqués
cli-export-help = Exporter vers un répertoire nouveau ou vide
cli-output-help = Répertoire de destination (absent ou vide)
cli-pages-help = Exporter aussi les pages en images PNG
cli-list-help = Lister les commandes et les requêtes du moteur
cli-query-help = Lister uniquement les requêtes
cli-markdown-help = Générer la référence CLI en Markdown
cli-describe-help = Décrire un identifiant et le schéma JSON de ses paramètres
cli-check-passed = Vérification réussie.
cli-check-failed = Vérification négative.
cli-dictionary-unavailable = Aucun dictionnaire orthographique n’est disponible pour la langue du document.
cli-spelling-issue = Orthographe à vérifier : { $word }
cli-exported = Export terminé : { $path }
cli-new-help = Créer un document depuis un type intégré et un modèle fourni
cli-type-help = Identifiant du type dans document_types.list
cli-model-help = Identifiant du modèle (facultatif si le type n’a qu’un modèle)
cli-document-output-help = Nouveau fichier .typsto ; les fichiers existants sont refusés
cli-params-help = Paramètres du moteur sous forme d’objet JSON
cli-params-file-help = Lire un objet JSON UTF-8 depuis un fichier, ou - pour l’entrée standard
cli-created = Document créé : { $path }
cli-params-object = Les paramètres doivent être un objet JSON.
cli-params-too-large = Les paramètres dépassent la limite de 4 Mio.
cli-params-io = Impossible de lire le fichier de paramètres ou l’entrée standard.
cli-params-conflict = Un paramètre est fourni à la fois dans le JSON et par une option de commande.
cli-query-call-help = Exécuter une requête du registre sur un document enregistré ou consulter les données globales
cli-run-help = Appliquer une commande documentaire en mémoire ; l’enregistrement doit être explicite
cli-save-help = Verrouiller et enregistrer le document source après réussite
cli-select-help = Sélectionner ce texte exact, sensible à la casse, dans le document courant
cli-occurrence-help = Choisir une occurrence à partir de 1 (obligatoire en cas d’ambiguïté)
cli-block-help = Cibler un bloc par sa position dans le document, à partir de 1
cli-applied = Commande exécutée : { $id }
cli-not-saved = Prévisualisation uniquement : aucun document n’a été enregistré. Utilisez --save ou --output pour conserver les modifications.
cli-session-reference = Les identifiants de session ne sont pas réutilisables entre processus CLI. Omettez les gardes et références de rendu ; utilisez --select ou --block pour les cibles.
cli-call-family = Utilisez query pour les requêtes et run pour les commandes.
cli-call-unavailable = Cette opération n’est pas disponible dans une session à un seul appel. Utilisez les commandes de fichiers dédiées ; lots, historique et récupération nécessitent un autre contexte.
cli-target-options = Utilisez select ou block, avec des numéros positifs ; occurrence nécessite select.
cli-script-help = Appliquer un script JSON versionné en une transaction documentaire atomique
cli-script-file-help = Script JSON UTF-8, ou - pour l’entrée standard
cli-script-io = Impossible de lire le script ou son répertoire de référence.
cli-script-version = Version de script non prise en charge. Seule la version 1 est acceptée.
cli-script-save = Un nouveau document n’a pas de fichier courant. Choisissez save.kind as et un nouveau chemin.
cli-script-completed =
    { $steps ->
        [one] Script terminé : { $steps } étape.
       *[other] Script terminé : { $steps } étapes.
    }
cli-mcp-read-root-help = Autoriser la lecture sous ce répertoire existant (répétable)
cli-mcp-write-root-help = Autoriser l’écriture sous ce répertoire existant (répétable ; ne donne pas la lecture)
mcp-root = Les racines MCP doivent être des répertoires existants et accessibles.

# Assistance locale — phase 6
cmd-assistance = Assistance par un agent…
cmd-share-selection = Confier la sélection à l’agent
assistance-selection-shared = Sélection confiée à l’agent. Vos déplacements suivants restent indépendants.
cli-mcp-attach-help = Relier MCP au document autorisé par une invitation de l’application, sans ouvrir de fichier documentaire.
assistance-title = Assistance par un agent
assistance-help = Autorisez un agent local à lire le document ouvert et à proposer des modifications, même s’il n’est pas enregistré. Vous décidez d’accepter ou de refuser chaque lot entier.
assistance-read-only = Les propositions ne modifient rien avant votre accord. Les droits facultatifs ci-dessous portent uniquement sur le document publié. Aucun accès aux préférences ni aux autres documents.
assistance-save-allow = Autoriser l’enregistrement du fichier courant
assistance-save-first = Enregistrez d’abord le document dans l’application, avec un accès en écriture.
assistance-save-granted = Enregistrement autorisé : { $path }
assistance-export-choose = Choisir un dossier d’export autorisé…
assistance-export-granted = Dossier d’export autorisé : { $path }
assistance-export-remove = Retirer ce droit
assistance-authorize = Créer une invitation pour ce document
assistance-copy = Copier la configuration MCP
assistance-revoke = Révoquer l’invitation et déconnecter
assistance-ready = Agent : invitation prête
assistance-connected = Agent connecté · propositions soumises à votre accord
assistance-disconnected = La connexion de l’agent est terminée. Créez une nouvelle invitation, puis reconnectez le serveur MCP dans votre assistant.
assistance-error = Impossible de créer le canal local : { $error }
cmd-review-proposal = Voir la proposition
cmd-close-review = Revenir à la rédaction
cmd-accept-proposal = Accepter le lot
cmd-reject-proposal = Refuser le lot
cmd-reveal-agent-change = Voir le passage modifié
review-title = Relecture de la proposition
review-pending = Une proposition à relire
review-stale = Proposition périmée
review-stale-help = Le document a changé. L’agent doit préparer une nouvelle proposition.
review-busy = Terminez la saisie ou le dialogue en cours avant de décider.
review-accepted = Lot accepté · une étape d’annulation · document non enregistré
review-accepted-unchanged = Lot accepté · aucun changement documentaire
review-rejected = Lot refusé · aucune modification du lot publiée
review-count = Passages modifiés : { $passages } · informations : { $informations }
review-agent-summary = Commentaire de l’agent : { $summary }
review-content = Contenu
review-information = Informations
review-preview = Aperçu proposé
review-current = Actuel
review-base = Version de départ
review-proposed = Proposition
review-added = Souligné : ajouté ou modifié
review-removed = Biffé : retiré ou modifié
review-whole-lot = Décision sur tout le lot
review-field = Information
review-empty = Non renseigné
review-paper = Format du papier
review-language = Langue du document
review-orientation = Orientation
review-marker = Puce de liste
review-dictionary = Exceptions orthographiques
review-structure = Structure du texte
review-model = Modèle
review-custom-model = Modèle personnalisé
review-model-changed = Présentation modifiée — voir l’aperçu proposé
review-signature = Signature
review-handwritten = Emplacement pour une signature manuscrite
review-signature-image = Image de signature
review-signature-new = Nouvelle image de signature
review-resource = Image : { $name }
review-resource-present = Ressource présente
cli-usage-capture-assistance = Usage : --capture-assistance IMAGE.png

widget-list = Liste
widget-quote = Citation
widget-postscript = P.-S.
widget-attachments = P.J.
widget-settings = Paramètres du bloc
widget-direct-parameters = Tous les paramètres sont déjà accessibles dans le bloc.
widget-delete = Supprimer
cmd-remove-widget = Supprimer le bloc entier
cmd-contextual-action = Action contextuelle
contextual-correct = Corriger « { $word } »
contextual-focus-band = Revenir au bandeau : { $title }
widget-close = Fermer le bandeau
widget-image-settings = Paramètres de l’image
widget-list-settings = Paramètres de la liste
widget-delete-title = Supprimer « { $kind } » ?
widget-delete-list-help = Toute la liste et ses sous-listes seront supprimées. Cette action pourra être annulée.
widget-delete-help = Le bloc et ses annotations seront supprimés. Cette action pourra être annulée.
widget-caption-direct = La légende se modifie directement dans le bloc.
widget-list-scope = Ces paramètres concernent la liste entière.
widget-bullets = À puces
widget-numbered = Numérotée
widget-descriptions = Descriptions
widget-spelling = Orthographe
cli-usage-capture-widgets = Utilisation : --capture-widgets image.png image|list|list-empty|remove|spelling|link|quote|notes|context-list|context-spelling[-dark]
cli-mcp-attach-active-help = Relier MCP à l’unique invitation autorisée disponible, sans modifier la configuration à chaque connexion.
assistance-fixed-configuration = Configuration MCP à enregistrer une seule fois dans votre assistant. Pour une nouvelle invitation, reconnectez le serveur sans modifier ses arguments.

cmd-insert-table = Insérer un tableau
cmd-insert-footnote = Insérer une note de bas de page

# Interface structurée Tauri
web-new-document = Nouveau document
web-open = Ouvrir…
web-save = Enregistrer
web-save-as = Enregistrer sous…
web-export = Exporter
web-close-document = Fermer le document
web-file = Fichier
web-edit = Édition
web-view = Affichage
web-window = Fenêtre
web-cancel = Annuler
web-redo = Rétablir
web-preferences = Préférences
web-commands = Commandes
web-palette = Palette de commandes
web-find-replace = Rechercher et remplacer
web-find = Rechercher
web-find-command = Rechercher une commande…
web-find-command-label = Rechercher une commande
web-replace-with = Remplacer par
web-replace-all = Tout remplacer
web-ignore-case =  Ignorer la casse
web-headings = Titres
web-lists = Listes
web-letter-notes = Fin de lettre
web-second-window = Deuxième fenêtre
web-toggle-connection = Connexion / déconnexion
web-connected = Connecté
web-offline = Hors connexion
web-general = Général
web-model = Modèle
web-document-type = Type de document
web-create = Créer
web-untitled = Sans titre
web-today = Aujourd’hui
web-paper = Format
web-document-language = Langue du document
web-appearance = Apparence
web-system = Système
web-light = Clair
web-dark = Sombre
web-serif = Sérif
web-sans = Sans empattement
web-mono = Monospace
web-editor-font = Police de l’éditeur
web-font-size = Taille
web-no-signature = Sans signature
web-default-signature = Signature des préférences
web-document-signature = Signature du document
web-profile-signature = Signature par défaut
web-save-preferences = Enregistrer les préférences
web-preferences-saved = Préférences enregistrées. Les documents existants conservent leurs données.
web-document-saved = Document enregistré.
web-resume = Reprendre la session
web-keep-changes = Conserver les modifications ?
web-discard-explanation = Enregistrez le document avant de le remplacer, ou abandonnez les modifications de cette session.
web-discard = Abandonner
web-settings = Paramètres
web-remove = Supprimer
web-close = Fermer
web-apply = Appliquer
web-close-panel = Fermer le panneau
web-close-settings = Fermer les paramètres
web-paragraph = Paragraphe
web-bold = Gras
web-italic = Italique
web-link = Lien
web-link-address = Adresse du lien
web-alt-text = Texte alternatif
web-footnote-text = Texte de la note
web-footnote = Note de bas de page
web-note-badge = Note
web-remove-footnote = Supprimer la note de bas de page
web-image-settings = Paramètres de l’image
web-block-settings = Paramètres du bloc
web-list-start = Numéro initial
web-model-controls-appearance = La présentation est définie par le modèle.
web-quote = Citation
web-descriptions = Descriptions
web-bullet-list = Liste à puces
web-numbered-list = Liste numérotée
web-term = Terme
web-definition = Définition
web-caption-placeholder = Légende facultative
web-attribution-placeholder = Attribution / source (facultatif)
web-postscript-placeholder = Texte du post-scriptum…
web-enclosure-placeholder = Document joint…
web-add-row = Ajouter une ligne
web-remove-row = Retirer la ligne
web-add-column = Ajouter une colonne
web-remove-column = Retirer la colonne
web-remove-table = Supprimer le tableau
web-assistance = Assistance à la rédaction
web-authorize-mcp = Autoriser une connexion MCP
web-disconnect-agent = Déconnecter l’agent
web-current = Actuel
web-proposal = Proposition
web-reject-lot = Refuser le lot
web-accept-lot = Accepter le lot
web-lot-accepted = Lot accepté. Vous pouvez l’annuler en une seule fois.
web-proposal-accepted = Proposition acceptée.
web-proposal-rejected = Proposition refusée.
web-proposal-stale = Proposition périmée.
web-proposal-ended = Proposition périmée ou terminée. Le document doit être relu par l’agent.
web-ignore-document = Ignorer dans ce document
web-add-personal-word = Ajouter au dictionnaire personnel
web-changed-word = Le mot a changé : rouvrez la correction.
web-rendering = Mise à jour…
web-preparing = Préparation…
web-loading = Chargement…
web-invalid-content = Contenu à corriger · dernier rendu conservé
web-local-edits-offline = Modifications locales · hors connexion
web-preview = Aperçu Typst
web-invalid-link = Utilisez une adresse http, https ou mailto.
web-invalid-list-start = Numéro initial entre 1 et 10 000.
web-changed-block = Le bloc a changé.
web-remote-block-changed = Ce bloc a été modifié ailleurs. Fermez puis rouvrez ses paramètres.
web-reconnect-save = Reconnectez cette fenêtre avant d’enregistrer.
web-reconnect-export = Reconnectez cette fenêtre avant d’exporter.
web-native-agent-only = La connexion à un agent est disponible dans l’application Tauri.
web-agent-introduction = Raccordez votre agent à ce document. Ses propositions seront soumises à votre validation avant toute modification.
web-fixed-configuration = Configuration fixe : reconnectez le serveur dans votre outil après une nouvelle invitation.
web-agent-connected = Connexion autorisée. L’agent peut lire le document publié et préparer un lot de modifications.
web-finish-input = Terminez la saisie en cours avant de décider.
web-words = mots
web-characters = signes
web-selection = Sélection
web-without-spaces = Hors espaces
web-rust-example = Essai Rust
web-welcome-title = Le contenu d’abord.
web-welcome-description = Rédigez simplement. Votre modèle s’occupe de la présentation.
web-session-saved = Session enregistrée
web-exported = PDF et sources Typst exportés
web-document-body = Corps du document
web-column-width = Largeur des colonnes

web-you = Vous

# Bandeaux contextuels de l’éditeur structuré
web-list-type = Type de liste
web-remove-block = Supprimer le bloc
web-remove-block-help = Le bloc entier sera supprimé. Vous pourrez annuler cette suppression.
web-image-replacement-pending = Image remplacée — appliquer pour confirmer
web-close-correction = Fermer la correction
web-table = Tableau

web-confirm = Valider
web-no-extra-settings = Aucun paramètre supplémentaire à modifier.
web-invalid-setting = Vérifiez la valeur de ce champ.

cli-catalogue-export-help = Exporter le catalogue documentaire statique, sans ouvrir de document.

web-recovery = Récupération après incident
web-no-recovery = Aucune copie à récupérer.
web-delete-recovery = Supprimer la copie
web-recover = Récupérer
web-formatting-changed = Mise en forme modifiée
web-structure-changed = Structure modifiée
web-settings-changed = Paramètres modifiés
web-spelling-enabled = Correction orthographique
web-delete-signature = Supprimer la signature
web-find-next = Occurrence suivante
web-find-previous = Occurrence précédente
web-clear-formatting = Effacer la mise en forme
web-unlink = Supprimer le lien
web-toggle-spelling = Activer / désactiver la correction orthographique
web-spelling-next = Erreur suivante
web-spelling-previous = Erreur précédente
web-spelling-reset = Réexaminer les mots ignorés
web-document-settings = Paramètres du document
web-embedded-model = Modèle embarqué
web-import-model = Importer un modèle Typst
web-model-too-large = Modèle trop volumineux.
web-model-import-stale = Le document a changé ; recommencez l’import.
web-list-first-number = Premier numéro
web-heading-level = Niveau de titre

web-palette-no-results = Aucune commande ne correspond.

web-snapshot-document-replaced = Le document a changé ; recommencez l’opération.
web-clear-date = Supprimer la date

article-type = Article
model-working-article = Article de travail
article-settings-heading = Paramètres de l’article :
field-article-authors = Auteurs
field-article-affiliations = Affiliations
field-article-localizations = Titres, résumés et mots-clés
article-author = Auteur
article-affiliation = Affiliation
article-localization = Ensemble linguistique
field-orcid = ORCID
field-corresponding = Auteur correspondant
field-official-affiliation = Signature officielle
field-language = Langue
field-article-title = Titre
field-short-title = Titre court
field-subtitle = Sous-titre
field-abstract = Résumé
field-keywords = Mots-clés
web-add = Ajouter
web-record-edit = Modifier
web-move-up = Monter
web-move-down = Descendre
web-keyword-lines = Un mot-clé par ligne ; les expressions peuvent contenir des espaces.
web-detach-affiliation = Retirer aussi les associations à cette affiliation.
web-keep-primary-localization = L’ensemble de la langue principale doit être conservé.
web-affiliation-in-use = Cette affiliation est utilisée. Confirmez le retrait de ses associations.

group-article-titles = Titres
group-article-abstracts = Résumés
web-language-already-present = Cette langue est déjà présente.
web-record-changed = Cet élément a été modifié ou supprimé ailleurs. Fermez puis rouvrez le formulaire.
web-record-reference-removed = Une référence sélectionnée a été supprimée. Vérifiez les associations avant de valider.
web-deleted-record = Élément supprimé
field-bibliography = Bibliographie
field-reference-file = Fichier bibliographique
field-reference-source = Source du fichier
field-citation-style = Style bibliographique
field-show-outline = Afficher le sommaire
cmd-insert-citation = Citer une référence
web-citation-form = Forme de la citation
web-reference-files = Fichiers bibliographiques
web-add-reference-files = Ajouter des fichiers
web-remove-reference-file = Retirer le fichier bibliographique
web-document-changed-retry = Le document a changé. Réessayez.
web-remove-reference = Retirer
web-reference-too-large = Fichier trop volumineux (2 Mio maximum).
web-replace-reference = Remplacer
web-search-reference-placeholder = Rechercher par titre ou auteur…
web-search-reference = Rechercher une référence
web-references = Références
web-standard-citation = Standard
web-prose-citation = Dans la phrase
web-no-reference = Aucune référence ne correspond.
web-locator-placeholder = Pages ou chapitre (facultatif)
web-locator = Pages ou chapitre
web-remove-citation-item = Retirer de la citation
web-citation = Citation
web-bibliographic-citation = Citation bibliographique
web-chicago-author-date = Chicago auteur–date
web-ieee = IEEE (numérique)
web-delete-citation = Supprimer la citation

web-table-rows = Lignes

web-table-columns = Colonnes

web-table-row = Ligne

web-table-column = Colonne
web-table-row-actions = Actions sur la ligne
web-table-column-actions = Actions sur la colonne
web-table-before-row-short = Au-dessus
web-table-after-row-short = Au-dessous
web-table-before-column-short = À gauche
web-table-after-column-short = À droite

web-table-header = Première ligne : en-têtes de colonnes
web-table-row-headers = Première colonne : en-têtes de lignes

web-table-edit = Modifier le tableau

web-table-invalid-dimensions = Dimensions de tableau invalides.

web-table-row-before = Ajouter une ligne au-dessus

web-table-row-after = Ajouter une ligne au-dessous

web-table-row-remove = Supprimer cette ligne

web-table-column-before = Ajouter une colonne à gauche

web-table-column-after = Ajouter une colonne à droite

web-table-column-remove = Supprimer cette colonne

web-edit-failed = Impossible d’appliquer cette modification.

# Projets structurés : traduire les causes à la frontière de l’adaptateur.
storage-project-error = Impossible de lire ou d’enregistrer ce projet : { $error }
storage-unexpected-entry = Fichier inattendu dans l’archive : { $name }.
storage-duplicate-entry = Un fichier apparaît plusieurs fois dans l’archive : { $name }.
storage-missing-entry = Un fichier nécessaire au projet est absent : { $name }.
storage-object-integrity = Le contenu conservé ne correspond pas à son empreinte : { $name }.
storage-project-index-invalid = L’index des versions ou des références du projet est invalide.
storage-snapshot-limit = Le projet contient déjà le maximum de { $limit } versions. Supprimez une version pour en créer une nouvelle.
storage-unknown-snapshot = Cette version n’existe pas dans le projet : { $id }.
content-invalid-target = La position sélectionnée n’est plus disponible. Sélectionnez de nouveau le passage.
content-invalid-parameter = Valeur incorrecte pour le paramètre « { $parameter } ».
content-command-unavailable = La commande « { $command } » ne peut pas s’appliquer à cette sélection.
content-contract-version = Ce document utilise un format structuré non pris en charge (version { $version }).
content-shared-state-invalid = L’état d’édition enregistré est incomplet ou illisible.
content-signature-missing = L’image de signature est absente du projet.
content-document-invalid = Le document contient des éléments invalides ou non pris en charge.
content-settings-invalid = Les paramètres du document sont invalides.
content-document-mismatch = Le document enregistré et son état d’édition ne correspondent pas.
content-settings-mismatch = Les paramètres enregistrés et l’état d’édition ne correspondent pas.
content-source-mismatch = Le source Typst enregistré ne correspond pas au document structuré.
content-bibliography-mismatch = Les fichiers bibliographiques ne correspondent pas aux références du document.
content-state-mismatch = Les données enregistrées du projet sont incohérentes.
content-context-pending = Ce contexte d’édition conserve un brouillon privé refusé. Examinez-le avant de continuer.
content-context-selection = Sélectionnez d’abord une zone de texte dans ce contexte d’édition.
content-history-conflict = Cette annulation ou ce rétablissement supprimerait une contribution d’un autre participant. Le projet n’a pas changé ; le brouillon privé est conservé.

web-sync-suspended = La synchronisation est suspendue ; votre brouillon est conservé.
web-unsynced-draft = Brouillon non synchronisé
web-return-confirmed = Revenir à la version confirmée
web-export-draft = Exporter le brouillon
web-discard-draft-question = Abandonner ce brouillon et recharger la version confirmée ? Exportez-le d’abord si vous souhaitez le conserver.
web-discard-draft = Abandonner le brouillon

content-type-unavailable = La capacité « { $command } » n’est pas disponible pour ce type de document.

# Shared command availability
web-command-no-selection = Placez le curseur dans une zone de rédaction.
web-command-unsupported-type = Cette commande n’est pas disponible pour ce type de document.
web-command-text-required = Cette commande nécessite une zone de texte compatible.
web-command-selection-required = Sélectionnez du texte ou placez le curseur dans un lien.
web-command-body-required = Cette commande est réservée au corps du document.
web-command-top-level-required = Cette commande n’est pas disponible à l’intérieur de ce widget.
web-command-caret-required = Placez le curseur sans sélectionner de texte.
web-command-read-only = Cette zone n’est pas modifiable.
web-command-composing = Terminez la composition en cours.
web-command-disconnected = Reconnectez ce document avant d’appliquer une commande.
web-command-pending-draft = Conservez ou abandonnez le brouillon refusé avant de continuer.
web-command-no-document = Ouvrez ou créez un document.
web-command-modal = Terminez ou annulez le formulaire ouvert.
web-command-unavailable = Cette commande n’est pas disponible dans ce contexte.

# Optional local synchronization services
sync-enable = Activer la synchronisation
sync-disable = Arrêter la synchronisation
sync-enabled = Synchronisation active
sync-pending = Activation après la saisie…
sync-solo = Mode individuel · validation et historique actifs
sync-close-windows = Fermez les autres fenêtres avant d’arrêter la synchronisation.

web-private-recovery-failed = La copie de secours n’a pas pu être enregistrée. Votre travail reste dans cette fenêtre.

# Draft recovery and explicit close decisions.
web-recovery-ui-1 = Travail à reprendre
web-recovery-ui-2 = Saisies à reprendre
web-recovery-ui-3 = Reprendre
web-recovery-ui-4 = Modifications non enregistrées
web-recovery-ui-5 = Cette copie ne peut pas être lue.
web-recovery-ui-6 = Supprimer cette copie de secours ?
web-recovery-ui-7 = Son contenu non enregistré ne pourra plus être repris. Le fichier d’origine restera inchangé.
web-recovery-ui-8 = Chaque saisie reste privée jusqu’à sa validation.
web-recovery-ui-9 = Toutes les saisies ont été traitées.
web-recovery-ui-10 = Cette saisie reste séparée du document.
web-recovery-ui-11 = Saisie récupérée · à valider
web-recovery-ui-12 = Abandonner cette saisie
web-recovery-ui-13 = Abandonner cette saisie ?
web-recovery-ui-14 = Abandonner la saisie
web-recovery-ui-15 = Le contenu actuel du document restera inchangé.
web-recovery-ui-16 = Reprendre la saisie
web-recovery-ui-17 = Consulter
web-recovery-ui-18 = La cible a changé ou n’est plus disponible. Votre saisie est conservée.
web-recovery-ui-19 = Contenu récupéré
web-recovery-ui-20 = Exporter le brouillon…
web-recovery-ui-21 = Texte non appliqué
web-recovery-ui-22 = Champ d’en-tête
web-recovery-ui-23 = Élément d’en-tête
web-recovery-ui-24 = Saisie en cours
web-recovery-ui-25 = Conserver le travail avant de fermer ?
web-recovery-ui-26 = Enregistrer les modifications ?
web-recovery-ui-27 = Le document et les saisies en cours seront conservés sur cet ordinateur.
web-recovery-ui-28 = Le fichier .typsto ne sera pas modifié.
web-recovery-ui-29 = Ce document contient des modifications qui ne sont pas encore enregistrées.
web-recovery-ui-30 = Abandonner et fermer
web-recovery-ui-31 = Fermer sans enregistrer
web-recovery-ui-32 = Conserver et fermer
web-recovery-ui-33 = Enregistrer et fermer
web-recovery-ui-34 = Réessayer
web-recovery-ui-35 = Une autre fenêtre n’a pas pu conserver son travail. Réessayez.
web-recovery-ui-36 = Document récupéré · copie indépendante
web-recovery-ui-37 = Ouvrir un brouillon de secours…
recovery-unreadable = Copie illisible
web-recovery-progress-1 = Conservation du travail dans les autres fenêtres…
web-recovery-progress-2 = Opération annulée.
web-recovery-error-1 = Cette copie de secours est invalide.
web-recovery-error-2 = Le dossier de récupération n’est pas disponible.
web-recovery-error-3 = Cette copie de secours n’est plus disponible.
web-recovery-error-4 = Cette copie appartient à une fenêtre encore ouverte.
web-recovery-error-5 = Cette copie de secours est trop volumineuse.
web-recovery-error-6 = Une saisie récupérée ne peut pas être interprétée.
web-recovery-error-7 = Une autre opération est déjà en cours. Réessayez.
web-recovery-error-8 = Les autres fenêtres doivent conserver leur travail avant de continuer.
web-recovery-error-9 = La conservation des autres fenêtres n’est pas terminée.
web-recovery-error-10 = La copie de secours d’une fenêtre n’a pas pu être confirmée.
web-recovery-error-11 = Cette saisie n’est plus disponible.
web-recovery-error-12 = récupéré

web-recovery-close-scope = Les autres fenêtres seront fermées. Leurs saisies en cours seront conservées.

storage-version-index-changed = La liste des versions a changé. Actualisez-la puis réessayez.

web-versions-0 = Versions

web-versions-1 = Créer une version

web-versions-2 = Nom de la version

web-versions-3 = Nom daté automatique

web-versions-4 = Tags séparés par des virgules

web-versions-5 = Description facultative

web-versions-6 = Contenu de la version

web-versions-7 = Informations du document

web-versions-8 = Modèle intégré

web-versions-9 = Modifier les informations de la version

web-versions-10 = Fermer la version

web-versions-11 = Affichage de la version

web-versions-12 = Contenu

web-versions-13 = Aperçu

web-versions-14 = Page

web-versions-15 = Supprimer cette version

web-versions-16 = Supprimer cette version ?

web-versions-17 = Informations de la version

web-versions-18 = Seule cette version sera supprimée. Le document courant et les autres versions seront conservés.

web-versions-19 = Aucune version conservée.

web-versions-20 = Versions manuelles


web-versions-22 = La limite de versions est atteinte. Supprimez une version pour en créer une nouvelle.

web-versions-23 = Versions enregistrées.

web-versions-24 = Le fichier du projet est déjà choisi.

web-versions-25 = Choisissez où enregistrer le projet pour créer une version.

web-versions-26 = Extension .typsto attendue.

web-versions-28 = Oui

web-versions-29 = Non

cli-versions-help = Créer, renommer ou supprimer une version et enregistrer atomiquement le projet ; utiliser les paramètres de versions.change.

web-versions-existing-path = Le fichier du projet est déjà choisi.

web-versions-path-required = Choisissez où enregistrer le projet pour créer une version.

web-versions-extension = Extension .typsto attendue.

web-version-creator = Créée par

# Shared snapshot comparison and local human decisions.
web-comparison-compare = Comparer
web-comparison-frozen-order = États figés · avant à gauche, après à droite
web-comparison-previous = Différence précédente
web-comparison-next = Différence suivante
web-comparison-before = Avant
web-comparison-after = Après
web-comparison-empty = Aucun contenu
web-comparison-stale-notice = Le document a changé ailleurs. Fermez puis ouvrez une nouvelle comparaison avant de reprendre les modifications restantes.
web-comparison-version-before = Avant · Version conservée
web-comparison-current-after = Après · Document courant figé
web-comparison-base-before = Avant · État de départ
web-comparison-proposal-after = Après · Proposition
web-comparison-document = Document et éléments liés
web-comparison-passage = Passage
web-comparison-pending = À décider
web-comparison-reused = Repris dans le document
web-comparison-applied = Appliqué au document
web-comparison-kept = Passage actuel conservé
web-comparison-refused = Refusé
web-comparison-linked = Éléments liés · une décision
web-comparison-keep = Conserver le passage actuel
web-comparison-reuse = Reprendre ce passage de la version
web-comparison-apply = Appliquer cette modification
web-comparison-identical = Aucune différence.
web-comparison-progress = décisions prises · états affichés inchangés
web-comparison-reuse-rest = Reprendre le reste
web-comparison-apply-rest = Appliquer le reste
web-comparison-bytes = octets
web-comparison-absent = Absent
web-comparison-read-only-notice = Consultation seule · deux versions conservées
web-comparison-closed = Proposition terminée ou connexion révoquée.
web-comparison-against = Comparer à
web-comparison-current = Document courant · état figé
web-comparison-review = Examiner la proposition
web-comparison-files = Fichiers modifiés
web-comparison-modified = Modifié
web-comparison-previews = Aperçus des deux états
web-comparison-changes = Voir les changements
web-comparison-loading-previews = Chargement des aperçus…
web-comparison-unavailable = Cette comparaison ou cette proposition n’est plus disponible.
web-comparison-stale = Le document a changé. Ouvrez une nouvelle comparaison.
web-comparison-composing = Terminez la saisie en cours avant de décider.
web-comparison-read-only = La comparaison de deux versions conservées est en consultation seule.
web-comparison-use-review = Cette proposition est ouverte dans le comparateur. Utilisez ses décisions locales.
web-comparison-stale-proposal = Le document a changé ailleurs. Demandez une nouvelle proposition à l’agent.

web-composition-select = Sélectionner
web-composition-choose-passage = Choisir la version du passage
web-composition-select-before = Sélectionner la version rouge
web-composition-select-after = Sélectionner la version verte
web-composition-choices-applied = Choix appliqués
web-composition-choice-draft = Choix provisoire
web-composition-applied = Choix appliqués. Vous pouvez enregistrer le document.
web-composition-draft-notice = Choix provisoires · aucun changement avant validation.
web-composition-select-all = Tout sélectionner
web-composition-select-all-before = Tout sélectionner à gauche
web-composition-select-all-after = Tout sélectionner à droite
web-comparison-review-composition = Comparez d’abord la composition au document courant avant de l’appliquer.
web-comparison-context-1 = Garder l’actuel
web-comparison-context-2 = Tout reprendre
web-comparison-context-3 = Tout conserver
web-comparison-context-4 = Reprendre ce passage
web-comparison-context-5 = Garder le passage actuel
web-comparison-context-6 = Tout reprendre de cette version
web-comparison-context-7 = Tout conserver du document courant
web-comparison-context-8 = Refuser
web-comparison-context-9 = Accepter
web-comparison-context-10 = Tout refuser
web-comparison-context-11 = Tout accepter
web-comparison-context-12 = Refuser ce changement
web-comparison-context-13 = Accepter ce changement
web-comparison-context-14 = Choisir cette version
web-comparison-context-15 = Tout choisir ici
web-comparison-context-16 = Choisir dans « { "{name}" } »
web-comparison-context-17 = Tout choisir dans « { "{name}" } »
web-comparison-context-18 = Avant · Document courant figé
web-comparison-context-19 = Après · Composition préparée
web-comparison-context-20 = Reprendre ce passage de la composition
web-comparison-context-21 = Tout reprendre de la composition
web-comparison-context-22 = Comparer la composition au document courant
web-comparison-context-23 = La composition reste privée avant sa comparaison au document courant.
web-comparison-context-24 = Reprendre { "{count}" } passage
web-comparison-context-25 = Reprendre { "{count}" } passages
web-comparison-context-26 = Aucun passage à reprendre
web-comparison-context-27 = { "{count}" } passage à reprendre
web-comparison-context-28 = { "{count}" } passages à reprendre
web-comparison-context-29 = { "{count}" } passage conservé
web-comparison-context-30 = { "{count}" } passages conservés
web-comparison-context-31 = { "{count}" } changement accepté
web-comparison-context-32 = { "{count}" } changements acceptés
web-comparison-context-33 = { "{count}" } changement écarté
web-comparison-context-34 = { "{count}" } changements écartés
web-comparison-context-35 = Appliquer { "{count}" } changement
web-comparison-context-36 = Appliquer { "{count}" } changements
web-comparison-context-37 = Refuser la proposition
web-comparison-context-38 = Terminer sans modification
web-comparison-context-39 = Conserver le document courant
web-comparison-context-40 = Vérifiez la composition avant de modifier le document courant.
web-comparison-context-41 = Modifier la composition
web-comparison-context-42 = Vérifier la composition
web-comparison-context-43 = Proposition refusée. Le document est inchangé.
web-comparison-context-44 = Proposition terminée sans modification.
web-comparison-context-45 = Document courant conservé.

typstOgraphe — identité tO

La source du signe est ligature.svg. Les lettres sont dessinées en courbes,
sans dépendance à une police. La traverse du t rejoint la panse du O.

ICÔNE D’INTERFACE RÉUTILISABLE
Le catalogue d’icônes expose le monogramme sous le nom « typstographe », depuis
optical/24.svg, en monochrome et sans contour ajouté. Son style dans la barre
latérale est conservé pour un usage ultérieur. La palette utilise désormais
l’icône Lucide « wrench ».

APERÇU
Servir la racine du dépôt avec :
python3 -m http.server 8341 --bind 127.0.0.1
Puis consulter http://127.0.0.1:8341/docs/mockups/branding.html

SPLASH
splash.html joue une seule fois à son ouverture et reste sur tO.
Le changement de thème et le redimensionnement ne relancent rien.
Le bouton de la maquette recharge explicitement le document pour le revoir.
Le mode prefers-reduced-motion affiche immédiatement le signe complet.
L’accueil Tauri utilise ce même dessin et le module motion.mjs : une seule
lecture de 2,1 s au premier affichage, sans bloquer les trois boutons.
Le retour à l’accueil conserve le signe immobile. Sans session récupérable,
le bouton de reprise reste visible, désactivé.
La vidéo de previews/ est une capture de cette même animation, sans boucle.

EXPORTS
macos/icon.icns : 16 à 1024 px, y compris les couches Retina.
windows/icon.ico : 16, 24, 32, 48, 64, 128, 256 px ; couche 32 px en premier.
linux/hicolor/ : SVG scalable et PNG de 16 à 512 px.
Les PNG sont en RGBA 8 bits par canal. Les tailles 16, 24, 32 et 48 px
ont chacune leur dessin dédié dans optical/, avec traverse calée sur
des pixels entiers, contreformes ouvertes et déliés épaissis.
Les fichiers @2x reprennent le dessin de la taille logique, à double
résolution ; les couches Retina de l’ICNS utilisent ces mêmes dessins.
L’ICNS est une icône aplatie pour le paquet Tauri ;
il ne contient pas un projet Icon Composer ni des couches Liquid Glass.

RÉGÉNÉRER
python3 assets/branding/generate.py
Prérequis : Python 3 et rsvg-convert (librsvg).
Le script régénère les SVG d’icône, PNG, ICO, ICNS, splash.html et splash.svg
depuis ligature.svg, optical/ et splash.template.html. Il met aussi à jour
experiments/web-editor/src-tauri/icons/ pour les trois plateformes.
Les configurations Tauri sélectionnent le dégradé macOS, le bleu uni Windows
et le bleu uni Linux. Les ressources sont embarquées ; aucun téléchargement
ni serveur de prévisualisation n’est requis dans l’application compilée.
Le script ne change pas les aperçus capturés.

RÉFÉRENCES DE FORMAT
https://v2.tauri.app/fr/develop/icons/
https://developer.apple.com/design/human-interface-guidelines/app-icons
https://learn.microsoft.com/en-us/windows/apps/design/style/iconography/app-icon-construction
https://specifications.freedesktop.org/icon-theme/

VÉRIFICATION
Prévisualisation Chromium et WebKit, lecture unique, thèmes, mode mouvement
réduit et mise en page de la maquette entre 320 et 1180 px.
ICO relu avec ImageMagick ; ICNS extrait avec l’outil macOS iconutil.
L’identité est intégrée à l’écran d’accueil et aux ressources natives Tauri.

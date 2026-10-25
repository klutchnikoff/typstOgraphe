# Incident de création du panneau de fichiers macOS

Incident signalé le 25 octobre 2026 lors d’un enregistrement : AppKit a renvoyé
`nil` à `+[NSSavePanel savePanel]`. Le binding `objc2-app-kit 0.3.2`, dont le
retour est non optionnel, a déclenché une panique Rust avant l’écriture.
Un message `com.apple.view-bridge: Connection interrupted` précédait la panique ;
la cause du refus côté macOS n’est pas établie. L’utilisateur a confirmé la
récupération complète de son travail au redémarrage.

Le projet utilise `rfd 0.15.4`. Le
[code de rfd 0.17.2](https://github.com/PolyMeilex/rfd/blob/0.17.2/src/backend/macos/file_dialog/panel_ffi.rs)
fait encore le même appel non optionnel : changer de version ne fournit pas
de correction de ce cas.

## Protection dans l’adaptateur graphique

`crates/app/src/app/native_files.rs` centralise les sélecteurs de fichiers
synchrones de l’application. Sur macOS, une frontière `catch_unwind` reconnaît
uniquement les deux paniques de constructeur `NSSavePanel` et `NSOpenPanel`
renvoyant `NULL`. La sélection, l’annulation et le refus du système sont distincts.
Un refus affiche une erreur traduite et permet de réessayer ; aucune commande
documentaire n’est exécutée. Les autres paniques sont propagées sans changement.

Cette protection repose sur le chemin vérifié dans `rfd 0.15.4` : appel sur le
thread principal depuis egui, constructeur avant `Panel::new`, ses changements
de focus/politique et la boucle modale. Aucune référence à l’état de l’application
n’est capturée par la fonction protégée ; aucun `AssertUnwindSafe`, hook global,
code Objective-C spécifique ou fork de dépendance n’est ajouté. Le pool
d’autorelease objc2 est libéré pendant le déroulement Rust.

Le hook de panique standard peut toujours imprimer l’incident dans le terminal,
même quand il est intercepté et que l’application continue. La protection suppose
les profils actuels avec déroulement des paniques, et non `panic = "abort"`.
Revoir cette frontière lors d’un changement du backend macOS de rfd ; elle ne
répare pas le service système et ne couvre pas les exceptions natives ou les
autres échecs AppKit.

## Vérification

Tests automatisés : réponses normales et annulation, refus des deux constructeurs
(payload `String` et `&str`), propagation intacte des autres paniques, préservation
du brouillon, de la sélection, de l’historique et de la récupération, puis nouvel
enregistrement réussi. Un échec d’« Enregistrer sous » conserve aussi le fichier
original et son rattachement, sans empêcher « Enregistrer » ensuite.

Le refus natif intermittent n’a pas été reproduit pendant ce correctif : les
scénarios de refus sont injectés dans les tests.

Validation locale : les 186 tests de la bibliothèque de l’application passent,
ainsi que Clippy sur toutes ses cibles avec avertissements interdits, le formatage
et le contrôle de taille des fichiers de production.

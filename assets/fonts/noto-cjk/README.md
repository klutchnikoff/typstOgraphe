# Noto Sans CJK — secours embarqué

Fichiers originaux, non modifiés, de **Noto Sans CJK JP 2.004** : variantes
Regular et Bold. Source officielle :
<https://github.com/notofonts/noto-cjk/releases/tag/Sans2.004>.

Ces faces offrent la couverture CJK complète de cette distribution, au-delà du
sous-ensemble japonais. Les formes par défaut sont japonaises ; la sélection
des formes régionales dans Typst dépend aussi de la langue et d’OpenType `locl`.
Elles ne prétendent pas couvrir tout Unicode ni fournir un véritable italique CJK.

Téléchargements :

- <https://raw.githubusercontent.com/notofonts/noto-cjk/Sans2.004/Sans/OTF/Japanese/NotoSansCJKjp-Regular.otf>
- <https://raw.githubusercontent.com/notofonts/noto-cjk/Sans2.004/Sans/OTF/Japanese/NotoSansCJKjp-Bold.otf>
- Licence OFL 1.1 : <https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/LICENSE>

| Fichier | Octets | SHA-256 |
| --- | ---: | --- |
| `NotoSansCJKjp-Regular.otf` | 16 467 736 | `68a3fc98800b2a27b371f2fb79991daf3633bd89309d4ffaa6946fd587f375b5` |
| `NotoSansCJKjp-Bold.otf` | 17 032 620 | `e53dcb0dcb2922e45d01aae1ebd2f382bb81d4229b18b6b883bd170678af1f76` |
| `LICENSE` | — | `6a73f9541c2de74158c0e7cf6b0a58ef774f5a780bf191f2d7ec9cc53efe2bf2` |

Les notices de copyright figurent également dans les métadonnées des fontes.
La licence est embarquée avec les binaires et copiée dans les exports qui utilisent
ces fontes. Leur ajout représente environ **33,5 Mo** de ressources non compressées.

`src/fonts.rs` fournit ces mêmes données à egui et à Typst. Les fontes de texte
latin et de mathématiques restent celles de `typst-assets` 0.15.1, déjà fixé dans
`Cargo.toml`. Noto CJK arrive en dernier recours dans l’éditeur et conserve une
variante grasse distincte. Aucun chargement de police système ni téléchargement
n’est nécessaire à l’exécution.

Un export de sources contenant des glyphes CJK inclut uniquement les variantes
de secours effectivement utilisées, avec leur licence et la commande de
recompilation `typst compile --font-path fonts main.typ` dans `fonts/README.txt`.
Les `.typsto` retrouvent ces fontes dans l’application, comme les autres fontes
embarquées ; leur format d’archive ne change pas.

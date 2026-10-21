"""Build a self-contained, reviewable report from recorded local experiments."""
import hashlib
from html import escape as e
import json
from pathlib import Path

HERE = Path(__file__).parent
RESULTS = HERE / "results"
MODES = ["catie-cpu", "qwen-cpu", "catie-mps", "qwen-metal"]
LABELS = {
    "catie-cpu": "CATIE · CPU · FP32", "catie-mps": "CATIE · Metal/MPS · FP32",
    "qwen-cpu": "Qwen · CPU · Q4_K_M", "qwen-metal": "Qwen · Metal · Q4_K_M",
    "catie-cpu-beam4": "CATIE · CPU · 4 faisceaux",
    "catie-mps-difficult": "CATIE · textes maladroits",
    "qwen-metal-difficult": "Qwen · textes maladroits",
}


def read(name):
    return json.loads((HERE / name).read_text())


def rows(mode):
    return [json.loads(line) for line in (RESULTS / f"{mode}.jsonl").read_text().splitlines()]


def first(mode):
    return {row["id"]: row for row in rows(mode) if row["repeat"] == 1}


def num(number):
    return f"{number:.2f}".replace(".", ",")


assessments = read("assessments.json")
fingerprints = read("assessment-fingerprints.json")
corpus = read("corpus.json")["cases"]
extra = read("corpus-difficult.json")["cases"]
sources = {case["id"]: case for case in corpus + extra}


def rating(mode, row):
    digest = hashlib.sha256(row["output"].encode()).hexdigest()
    if fingerprints.get(mode, {}).get(row["id"]) != digest:
        raise ValueError(f"Output changed: {mode} {row['id']}. Review it before reusing the assessment.")
    base = "catie-cpu" if mode == "catie-mps" else "qwen-cpu" if mode == "qwen-metal" else mode
    value = assessments[base][row["id"]]
    if mode == "qwen-metal":
        value = assessments["qwen-metal-overrides"].get(row["id"], value)
    return value


def quality_row(mode):
    samples = first(mode)
    values = [rating(mode, row) for row in samples.values()]
    exact = sum(row["output"] == sources[row["id"]]["text"] for row in samples.values())
    return (f"<tr><th>{e(LABELS[mode])}</th>"
            f"<td>{sum(v[0] == 4 for v in values)}/{len(values)}</td>"
            f"<td>{sum(v[3] for v in values)}/{len(values)}</td>"
            f"<td>{sum(v[1] <= 2 for v in values)}/{len(values)}</td>"
            f"<td>{sum(v[2] >= 2 for v in values)}/{len(values)}</td><td>{exact}/{len(values)}</td></tr>")


def output_card(mode, case_id):
    row = first(mode)[case_id]
    fidelity, french, value, material, note = rating(mode, row)
    return (f"<section class='result {'material' if material else ''}'><h4>{e(LABELS[mode])}</h4>"
            f"<p class='output'>{e(row['output'])}</p>"
            f"<p class='scores'>Fidélité {fidelity}/4 · Français {french}/4 · Réécriture {value}/4"
            f" · {num(row['seconds'])} s</p><p>{e(note)}</p>"
            f"{'<strong>Altération importante décelée.</strong>' if material else ''}</section>")


def case_card(case, mode_a, mode_b, cpu_details=False):
    material = any(rating(mode, first(mode)[case["id"]])[3] for mode in [mode_a, mode_b])
    checks = "".join(f"<li>{e(check)}</li>" for check in case["checks"])
    details = ("<details><summary>Comparer aussi les sorties sur processeur</summary><div class='pair'>"
               + output_card("catie-cpu", case["id"]) + output_card("qwen-cpu", case["id"])
               + "</div></details>") if cpu_details else ""
    return (f"<article class='case' id='{case['id']}' data-material='{str(material).lower()}'>"
            f"<h3>{e(case['id'])} · {e(case['category'])}</h3>"
            f"<div class='source'><h4>Passage d’origine</h4><p>{e(case['text'])}</p></div>"
            f"<ul class='checks'>{checks}</ul><div class='pair'>"
            + output_card(mode_a, case["id"]) + output_card(mode_b, case["id"])
            + "</div>" + details + "</article>")


performance = []
stability = []
for mode in MODES:
    metadata = read(f"results/{mode}.json")
    memory = read(f"results/{mode}-memory.json")
    s = metadata["summary"]
    performance.append(
        f"<tr><th>{e(LABELS[mode])}</th><td>{num(s['median_seconds'])} s</td>"
        f"<td>{num(s['p95_seconds'])} s</td><td>{num(s['max_seconds'])} s</td>"
        f"<td>{num(s['median_first_token_seconds'])} s</td>"
        f"<td>{num(memory['rss_sampled_peak_bytes'] / 1e9)} Go</td>"
        f"<td>{num(memory['macos_physical_footprint_peak_bytes'] / 1e9)} Go</td></tr>")
    samples = rows(mode)
    originals = first(mode)
    differences = sum(row["output"] != originals[row["id"]]["output"] for row in samples if row["repeat"] == 2)
    stability.append(f"{LABELS[mode]} : {differences}/20 sorties différentes entre répétitions")

models = read("results/models.json")
manifest = "\n".join(f"{name}: {entry['repository']} @ {entry['revision']}" for name, entry in models.items())
beam = read("results/catie-cpu-beam4.json")["summary"]
beam_changes = [row["id"] for row in rows("catie-cpu-beam4")
                if row["output"] != first("catie-cpu")[row["id"]]["output"]]
memory_rows = []
for mode in MODES:
    r = read(f"results/{mode}-memory.json")
    memory_rows.append(f"{LABELS[mode]} : chargé en {num(r['startup_seconds'])} s ; première requête en {num(r['first_request']['seconds'])} s")
driver = max(row.get("mps_allocated_bytes") or 0 for row in rows("catie-mps-memory"))

user_case = read("corpus-user.json")["cases"][0]
user_review = read("user-review.json")


def user_output(mode):
    row = first(mode)["C26"]
    review = user_review["runs"][mode]
    if hashlib.sha256(row["output"].encode()).hexdigest() != review["output_sha256"]:
        raise ValueError(f"User example output changed: {mode}. Review before reuse.")
    metadata = read(f"results/{mode}.json")
    prompt = metadata.get("system_prompt", metadata.get("prompt", ""))
    return (f"<section class='result'><h4>{e(review['label'])}</h4>"
            f"<p class='output'>{e(row['output'])}</p><p>{e(review['note'])}</p>"
            f"<p class='scores'>Médiane de 3 requêtes : {num(metadata['summary']['median_seconds'])} s"
            f" · Pic RSS : {num(metadata['rss_sampled_peak_bytes'] / 1e9)} Go"
            f" · Sorties identiques entre répétitions : {'oui' if review['identical_repetitions'] else 'non'}</p>"
            f"<details><summary>Consigne exacte / entrée</summary><p>{e(prompt)}</p></details></section>")


user_section = (
    "<h2 id='exemple-utilisateur'>Un courrier réellement fautif : votre exemple</h2>"
    f"<p>{e(user_review['scope'])}</p><p><strong>{e(user_review['conclusion'])}</strong></p>"
    "<article class='case' id='C26' data-material='true'><h3>C26 · Correction et rédaction</h3>"
    f"<div class='source'><h4>Texte fourni</h4><p>{e(user_case['text'])}</p></div>"
    f"<p>{e(user_review['ambiguity'])}</p><div class='pair'>"
    + user_output("catie-mps-user") + user_output("qwen-metal-user-simple")
    + "</div><details><summary>Autres réglages testés : quatre faisceaux et consignes plus longues</summary>"
    + user_output("catie-mps-user-beam4") + user_output("qwen-metal-user-original")
    + user_output("qwen-metal-user-correct") + "</details>"
    f"<details><summary>Exemple de résultat éditorial attendu, rédigé par Codex</summary>"
    f"<p>{e(user_review['editorial_target_note'])}</p><p class='output'>{e(user_review['editorial_target'])}</p></details>"
    "</article><p>Ces essais exploratoires sur un seul passage ne remplacent pas un corpus plus large de textes fautifs. Les mesures et les appréciations de ce complément ne sont pas fusionnées avec les statistiques initiales.</p>"
)


def larger_model_section():
    review_path = HERE / "qwen-size-review.json"
    if not review_path.exists():
        return ""
    review = json.loads(review_path.read_text())
    comparisons, cards = [], []
    for mode, entry in review["runs"].items():
        samples = rows(mode)
        row = first(mode)["C26"]
        if hashlib.sha256(row["output"].encode()).hexdigest() != entry["output_sha256"]:
            raise ValueError(f"Qwen size comparison output changed: {mode}.")
        m = read(f"results/{mode}.json")
        s = m["summary"]
        comparisons.append(
            f"<tr><th>{e(entry['label'])}</th><td>{num(s['median_seconds'])} s</td>"
            f"<td>{num(min(r['seconds'] for r in samples))}–{num(s['max_seconds'])} s</td>"
            f"<td>{num(s['median_first_token_seconds'])} s</td>"
            f"<td>{num(m['first_request']['seconds'])} s</td>"
            f"<td>{num(m['rss_sampled_peak_bytes'] / 1e9)} Go</td>"
            f"<td>{num(m['macos_physical_footprint_peak_bytes'] / 1e9)} Go</td></tr>")
        same = len({r["output"] for r in samples}) == 1
        cards.append(
            f"<section class='result'><h4>{e(entry['label'])} · sortie brute</h4>"
            f"<p class='output'>{e(row['output'])}</p><p>{e(entry['note'])}</p>"
            f"<p class='scores'>{len(samples)} répétitions, sorties identiques : {'oui' if same else 'non'}.</p>"
            f"<details><summary>Consigne exacte et modèle</summary><p>{e(m['system_prompt'])}</p>"
            f"<pre>{e(m['model_repository'])} @ {e(m['model_revision'])}\nSHA-256 : {e(m['model_sha256'])}</pre></details></section>")
    return (
        "<h2 id='qwen-plus-grand'>Qwen 1,5B et 3B sur le même courrier fautif</h2>"
        f"<p>{e(review['scope'])}</p><p><strong>{e(review['conclusion'])}</strong></p>"
        "<p>Le passage est reproduit dans <a href='#C26'>C26</a>. Les propositions ci-dessous n’ont pas été retouchées.</p>"
        "<div class='scroll'><table><thead><tr><th>Modèle</th><th>Médiane</th><th>Min–max</th>"
        "<th>Premier jeton, médiane</th><th>Première requête séparée</th><th>Pic RSS</th>"
        "<th>Empreinte macOS</th></tr></thead><tbody>" + "".join(comparisons) + "</tbody></table></div>"
        "<p>Mémoire du seul processus llama-server, chargement et requêtes compris, échantillonnée toutes les 20 ms. "
        "Les deux métriques ne s’additionnent pas ; les pages de poids adossées au fichier peuvent être résidentes "
        "sans être comptabilisées dans l’empreinte macOS. Le client de mesure et le reste de l’application sont exclus. "
        "Pas de remise à zéro du cache système ni de mesure de démarrage de machine à froid.</p>"
        "<div class='pair'>" + "".join(cards) + "</div>"
        f"<p><small>{e(review['reviewer'])}</small></p>"
    )


replicate = """# Depuis la racine du dépôt ; Python 3.12 et llama-server dans le PATH.
uv venv --python 3.12 output/reformulation/.venv
uv pip install --python output/reformulation/.venv/bin/python -r experiments/reformulation/requirements.lock.txt
HF_HOME="$PWD/output/reformulation/hf-cache" output/reformulation/.venv/bin/python experiments/reformulation/download.py

# Une commande à la fois, pour ne pas faire concourir les modèles.
HF_HUB_OFFLINE=1 output/reformulation/.venv/bin/python experiments/reformulation/benchmark.py catie-cpu
HF_HUB_OFFLINE=1 output/reformulation/.venv/bin/python experiments/reformulation/benchmark.py qwen-cpu
HF_HUB_OFFLINE=1 output/reformulation/.venv/bin/python experiments/reformulation/benchmark.py catie-mps
HF_HUB_OFFLINE=1 output/reformulation/.venv/bin/python experiments/reformulation/benchmark.py qwen-metal

# Diagnostic de génération T5, complément exploratoire, puis profils mémoire :
HF_HUB_OFFLINE=1 output/reformulation/.venv/bin/python experiments/reformulation/benchmark.py catie-cpu --beams 4 --repeats 1 --label catie-cpu-beam4
# Pour chaque modèle Metal, utiliser --corpus corpus-difficult.json --repeats 1
# et --label catie-mps-difficult / qwen-metal-difficult.
# Pour chacun des quatre modes, utiliser --repeats 1 --label <mode>-memory.

# Exemple fourni par l'utilisateur, trois répétitions de chaque configuration :
HF_HUB_OFFLINE=1 output/reformulation/.venv/bin/python experiments/reformulation/benchmark.py catie-mps --corpus corpus-user.json --repeats 3 --label catie-mps-user
HF_HUB_OFFLINE=1 output/reformulation/.venv/bin/python experiments/reformulation/benchmark.py catie-mps --corpus corpus-user.json --beams 4 --repeats 3 --label catie-mps-user-beam4
HF_HUB_OFFLINE=1 output/reformulation/.venv/bin/python experiments/reformulation/benchmark.py qwen-metal --corpus corpus-user.json --repeats 3 --label qwen-metal-user-original
HF_HUB_OFFLINE=1 output/reformulation/.venv/bin/python experiments/reformulation/benchmark.py qwen-metal --corpus corpus-user.json --task correct-rewrite --repeats 3 --label qwen-metal-user-correct
HF_HUB_OFFLINE=1 output/reformulation/.venv/bin/python experiments/reformulation/benchmark.py qwen-metal --corpus corpus-user.json --task correct-simple --repeats 3 --label qwen-metal-user-simple

# Comparaison des tailles Qwen : même consigne courte, même quantification Q4_K_M.
HF_HOME="$PWD/output/reformulation/hf-cache" output/reformulation/.venv/bin/python experiments/reformulation/download.py --model qwen3
HF_HUB_OFFLINE=1 output/reformulation/.venv/bin/python experiments/reformulation/benchmark.py qwen-metal --corpus corpus-user.json --task correct-simple --repeats 5 --label qwen-metal-user-15b-sizecheck
HF_HUB_OFFLINE=1 output/reformulation/.venv/bin/python experiments/reformulation/benchmark.py qwen-metal --qwen-size 3b --corpus corpus-user.json --task correct-simple --repeats 5 --label qwen-metal-user-3b

output/reformulation/.venv/bin/python experiments/reformulation/report.py
# Les appréciations sont liées au SHA-256 des sorties effectivement relues.
# En cas de sortie différente, les relire : ne pas recycler automatiquement les notes.
"""

html = """<!doctype html><html lang='fr'><meta charset='utf-8'>
<meta name='viewport' content='width=device-width,initial-scale=1'>
<title>typstOgraphe — essais de reformulation locale</title>
<style>
:root{color-scheme:light dark;--bg:#f5f6f8;--card:#fff;--text:#202936;--muted:#526174;--line:#d4dce5;--accent:#235f9b;--warn:#963d28}
@media(prefers-color-scheme:dark){:root{--bg:#171c24;--card:#222a35;--text:#e7edf5;--muted:#b5c1d1;--line:#425165;--accent:#9cc6fa;--warn:#f1b39e}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:16px/1.6 system-ui,sans-serif}
main{max-width:1240px;margin:auto;padding:32px 24px 80px}h1{font-size:2rem;line-height:1.2}h2{margin-top:2.5rem}h3{margin-top:0}h4{margin:.2rem 0 .6rem}
a{color:var(--accent)}p{max-width:100ch}small,.scores,.checks{color:var(--muted)}.lead{font-size:1.15rem}.banner{border-left:5px solid var(--accent);padding:12px 20px;background:var(--card)}
.scroll{overflow-x:auto}table{border-collapse:collapse;width:100%;background:var(--card);font-size:.92rem}th,td{padding:12px;text-align:left;border-bottom:1px solid var(--line);vertical-align:top}td{white-space:nowrap}thead th{color:var(--muted)}
.case{background:var(--card);border:1px solid var(--line);border-radius:12px;padding:22px;margin:24px 0}.source{border-left:3px solid var(--line);padding-left:16px}.source p,.output{white-space:pre-wrap;overflow-wrap:anywhere}.pair{display:grid;grid-template-columns:1fr 1fr;gap:24px}.result{border-top:2px solid var(--accent);padding-top:12px}.material{border-color:var(--warn)}.material strong{color:var(--warn)}.scores{font-size:.88rem}summary{cursor:pointer;color:var(--accent);padding:12px 0}pre{background:var(--card);border:1px solid var(--line);padding:18px;overflow:auto;font-size:.82rem}label{display:block;margin:20px 0;cursor:pointer}.hidden{display:none}nav a{margin-right:16px;display:inline-block}@media(max-width:760px){.pair{grid-template-columns:1fr}main{padding:20px 14px}.case{padding:16px}}@media print{body{background:white;color:black}details{display:block}label{display:none}.case{break-inside:avoid}}
</style><main>
<p><small>typstOgraphe · expérimentation du 21 octobre 2026 · génération entièrement locale</small></p>
<h1>Reformuler sans changer les faits</h1>
<p class='lead'>CATIE french_paraphrase_flan-t5-base et Qwen2.5-Instruct (1,5B et 3B), sur Apple M1 Max avec 32 Gio de mémoire.</p>
<nav><a href='#qwen-plus-grand'>Qwen 1,5B / 3B</a><a href='#exemple-utilisateur'>Votre exemple fautif</a><a href='#mesures'>Mesures</a><a href='#qualite'>Qualité</a><a href='#passages'>20 passages</a><a href='#complement'>5 textes maladroits</a><a href='#integration'>Intégration Rust</a><a href='#protocole'>Protocole et reproduction</a></nav>
<div class='banner'><strong>Le premier corpus vérifie surtout la conservation des faits ; il ne suffit pas à juger l’aide à la rédaction sur un texte fautif.</strong>
<p>Le complément fourni par l’utilisateur teste maintenant la correction et l’allègement des lourdeurs. Qwen s’en sort mieux avec une consigne courte, mais le bénéfice stylistique reste limité sur cet exemple. Les consignes plus longues produisent une invention. Aucun modèle ni moteur n’est choisi pour l’application à ce stade.</p></div>
""" + larger_model_section() + user_section + """
<h2 id='mesures'>Latence et mémoire</h2>
<p>40 requêtes mesurées par configuration (20 passages × 2), après une première requête séparée. Durée jusqu’au texte complet ; P95 au rang le plus proche, sur ce petit échantillon. La plus longue sélection fait environ 110 mots. Aucune sortie tronquée.</p>
<div class='scroll'><table><thead><tr><th>Configuration</th><th>Médiane</th><th>P95</th><th>Maximum</th><th>Premier jeton, médiane</th><th>Pic RSS</th><th>Empreinte macOS</th></tr></thead><tbody>
""" + "".join(performance) + """</tbody></table></div>
<p><strong>Mémoire :</strong> pics d’un passage supplémentaire sur les 20 cas, échantillonnés toutes les 20 ms, en Go décimaux. RSS = pages résidentes du processus, incluant le moteur et ses dépendances. « Empreinte macOS » = mémoire imputée au processus par <code>proc_pid_rusage</code> ; les pages propres adossées à un fichier peuvent être résidentes sans lui être imputées. Ces colonnes ne s’additionnent pas et ne mesurent pas toute la consommation du système. La RSS seule sous-estime notamment les allocations GPU de PyTorch.</p>
<p>Pour CATIE/MPS, PyTorch déclare en plus jusqu’à """ + num(driver / 1e9) + """ Go d’allocations du pilote Metal ; ne pas les ajouter mécaniquement à la RSS, car les métriques et les recouvrements diffèrent. Les résultats comparent des configurations complètes, Python pour T5 et llama.cpp pour Qwen, et non la seule mémoire des poids. Un portage Rust devra être remesuré.</p>
<details><summary>Démarrage, répétitions et réglage T5</summary><ul>""" + "".join(f"<li>{e(line)}</li>" for line in memory_rows + stability) + """</ul>
<p>Temps de chargement issus du passage mémoire, avec fichiers et bibliothèques déjà en cache : pas des démarrages système à froid. Le premier lancement T5 a pris 15,10 s. Le premier serveur Qwen était prêt à 22,15 s selon son journal ; le champ initial de 42,96 s inclut par erreur une interrogation de version après sa disponibilité, signalée dans le JSON. Les mesures chaudes n’en dépendent pas.</p>
<p>CATIE avec quatre faisceaux : médiane """ + num(beam["median_seconds"]) + """ s, sur un passage de 20 cas, contre 1,44 s en glouton. Seules les sorties """ + e(", ".join(beam_changes)) + """ changent ; les fautes et les pertes de symboles décelées restent. Les sorties brutes sont conservées.</p></details>
<h2 id='qualite'>Qualité : lecture comparée, pas une certification</h2>
<p>""" + e(assessments["reviewer"]) + """ Les notes ordinales de 0 à 4 et chaque justification figurent sous les sorties. « Réécriture utile » signifie ici note ≥ 2, indépendamment de la fidélité : une sortie fluide peut changer le sens. Une copie exacte peut conserver parfaitement les faits sans rendre le service attendu.</p>
<div class='scroll'><table><thead><tr><th>Configuration</th><th>Fidélité 4/4</th><th>Altération importante</th><th>Français ≤ 2/4</th><th>Réécriture ≥ 2/4</th><th>Copies exactes</th></tr></thead><tbody>
""" + "".join(quality_row(mode) for mode in MODES) + """</tbody></table></div>
<p>Exemples : CATIE perd « ≤ » et « α » dans C16. Qwen change les remarques à transmettre en données financières dans C08, perd l’année 2024 dans C11 et inverse le rôle des associations dans C19. Les ancres lexicales enregistrées dans les JSONL sont seulement des alertes : aucune note n’a été déduite automatiquement de leur absence.</p>
<details><summary>Échelles d’appréciation</summary>""" + "".join(f"<p><strong>{e(key)}</strong> : {e(value)}</p>" for key, value in assessments["rubric"].items() if isinstance(value, str)) + """</details>
<h2 id='passages'>Vingt passages représentatifs</h2>
<p>Sorties Metal en regard ; ouvrir le détail pour les variantes sur processeur. La quantification et les calculs CPU/GPU peuvent conduire à des choix de jetons différents, même en génération gloutonne.</p>
<label><input id='filter' type='checkbox'> Afficher seulement les cas avec une altération importante signalée</label>
""" + "".join(case_card(case, "catie-mps", "qwen-metal", True) for case in corpus) + """
<h2 id='complement'>Complément exploratoire : cinq textes maladroits</h2>
<p>Ajoutés après observation des premières sorties, sans changement de prompt ni de paramètres. Une seule répétition, sur Metal. Ce complément n’est pas fusionné avec les statistiques du corpus principal et ne constitue pas un test indépendant préparé à l’avance. Il vérifie surtout si les modèles allègent effectivement les lourdeurs. Dans C24, Qwen ajoute une conclusion absente : « cette association est purement fortuite ».</p>
""" + "".join(case_card(case, "catie-mps-difficult", "qwen-metal-difficult") for case in extra) + """
<h2 id='integration'>L’intégration dans l’application Rust est possible</h2>
<ul><li><a href='https://docs.rs/llama-cpp-2/latest/llama_cpp_2/'>llama-cpp-2</a> fournit des enveloppes Rust autour de la bibliothèque C++ llama.cpp. L’application peut appeler le moteur directement, sans installation Homebrew ni serveur à gérer par l’utilisateur. Ce n’est pas une réécriture du moteur en Rust.</li>
<li><a href='https://github.com/huggingface/candle'>Candle</a> est un moteur d’inférence Rust, avec un <a href='https://github.com/huggingface/candle/tree/main/candle-examples/examples/t5'>exemple T5/FLAN-T5</a> et une <a href='https://github.com/huggingface/candle/blob/main/candle-transformers/src/models/qwen2.rs'>implémentation Qwen2</a>. Les poids, l’adaptation éventuelle, la quantification et les performances exactes restent à valider pour chaque modèle choisi ; la prise en charge d’une architecture ne garantit pas celle de tout fichier GGUF.</li>
<li>Le choix modèle/moteur reste séparé. Un travailleur doit garder l’interface réactive ; le modèle peut être chargé à la demande et déchargé. L’isolation dans un processus reste aussi une option technique, pas une obligation liée au langage.</li></ul>
<h2 id='protocole'>Protocole et fichiers reproductibles</h2>
<ul><li>CPU : quatre fils, une requête à la fois, batch 1. T5 en FP32 avec PyTorch/Transformers ; Qwen en Q4_K_M officiel avec llama.cpp 0.5.0. Ces différences de moteur et de précision empêchent d’attribuer tous les écarts au seul modèle.</li>
<li>Génération gloutonne, graine 42, plafond 256 nouveaux jetons, aucun arrêt pour plafond constaté. Aucune troncature d’entrée ; les nombres de jetons sont enregistrés. CATIE reçoit le texte brut, sans préfixe ajouté par hypothèse. Qwen reçoit le prompt explicite conservé dans chaque résultat JSON.</li>
<li>CATIE : utilisation du checkpoint complet <code>model.safetensors</code> publié. Le dépôt contient aussi un adaptateur LoRA, non appliqué en plus de ces poids. Une comparaison avec FLAN-T5 de base retrouve des différences dans les 282 tenseurs ; les vérifications et la révision de référence sont conservées dans <code>results/catie-weight-check.json</code>. La variante « modèle de base + adaptateur séparé » n’a pas été évaluée.</li>
<li>Qwen : contexte 2048, un seul emplacement, cache de prompt désactivé et cache RAM désactivé. Le HTTP utilisé pour l’essai reste sur 127.0.0.1. Les textes sont synthétiques, sans dossier administratif réel.</li>
<li>Temps du premier jeton : premier pas de génération T5 ou premier contenu SSE Qwen ; mesures proches mais pas strictement identiques. La durée complète est le critère principal.</li>
<li>Mac utilisé normalement pendant les essais, sans fermeture des autres applications et sans remise à zéro du cache système. Échantillon restreint, pas de mesure Windows/Linux. Ni optimisation exhaustive du prompt, ni comparaison à Qwen FP16/Q8 ; aucune conclusion générale sur toutes les variantes des modèles.</li>
<li>Poids et dépendances lourdes dans <code>output/reformulation/</code>, ignorés par Git. Corpus, scripts, versions figées, empreintes SHA-256, sorties et mesures sont dans <code>experiments/reformulation/</code>. Aucune dépendance ajoutée à l’application.</li></ul>
<pre>""" + e(manifest) + """</pre><details><summary>Commandes de reproduction</summary><pre>""" + e(replicate) + """</pre></details>
<p>Sources des modèles : <a href='https://huggingface.co/CATIE-AQ/french_paraphrase_flan-t5-base'>fiche CATIE</a> et <a href='https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF'>GGUF officiel Qwen</a>. La fiche CATIE ne fournit pas de procédure de génération détaillée : le chargement testé et le diagnostic de génération sont explicités ici.</p>
</main><script>document.getElementById('filter').addEventListener('change',function(){document.querySelectorAll('.case').forEach(card=>card.classList.toggle('hidden',this.checked&&card.dataset.material!=='true'))})</script></html>
"""

(HERE / "report.html").write_text(html)
print(f"Written {HERE / 'report.html'} ({len(html)} characters)")

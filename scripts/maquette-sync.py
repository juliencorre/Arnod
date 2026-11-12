"""Régénère les pages de la maquette depuis docs/maquette-etat.yml (docs/plan-suivi-maquette.md, R8).

Pastilles d'état et d'écart des écrans, texte des écarts, fiche de chaque page, index et repères de
la page 0.1, captures de l'application réelle (« Application réelle ») sous les écrans livrés.

La page 0.5 (Design system) est générée en entier depuis docs/design-system/ : jetons, styles et
composants du design system, écarts transverses qui la citent, teintes écrites à la main dans les
autres pages ; le design system est installé dans la maquette (ds/arnod/).

Usage (Python 3 et PyYAML), depuis n'importe où :
    python3 -I scripts/maquette-sync.py "Develop @<commit>" [code de page…]

Chemins fixes, dans le dépôt (aucun chemin pris de la ligne de commande) :
- .maquette/pages/ : les pages actuelles de la maquette (fichiers .dc.html et compat.json), lues sur
  claude.ai ; .maquette/sortie/ reçoit les pages régénérées, reel/<capture>.png et ds/arnod/, à
  republier ; .maquette/ est ignoré par Git.
- e2e/artifacts/ : captures en thème clair des tests e2e (`npm run test:e2e`).
- Le premier argument dit d'où vient l'état ; il s'affiche dans les fiches et l'index. Les suivants,
  des codes de page de l'état, limitent la régénération à ces pages.

Travail textuel et idempotent : relancer le script sur sa propre sortie donne le même résultat.
"""

import datetime
import html
import json
import re
import shutil
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parent.parent
ETAT = ROOT / "docs" / "maquette-etat.yml"
SRC = ROOT / ".maquette" / "pages"
OUT = ROOT / ".maquette" / "sortie"
CAPS = ROOT / "e2e" / "artifacts"
DS = ROOT / "docs" / "design-system"
DS_PAGE = "0.5"
DS_FILE = "Arnod - Design system.dc.html"
DS_URL = "https://claude.ai/artifact/H4NqJvxR5ep8d6CaYeqsHG"
SOURCE = sys.argv[1] if len(sys.argv) > 1 else "état du dépôt"
OUT.mkdir(parents=True, exist_ok=True)
doc = yaml.safe_load(ETAT.read_text(encoding="utf-8"))
ONLY = set(sys.argv[2:])
if ONLY - set(doc["pages"]):
    raise SystemExit(f"Codes de page inconnus de l'état : {sorted(ONLY - set(doc['pages']))}")
compat = json.loads((SRC / "compat.json").read_text(encoding="utf-8"))
# La page du design system est créée dans la liste des pages, après la page 0.4, si elle manque.
if DS_PAGE in doc["pages"] and not any(p["name"].startswith(DS_PAGE + " · ") for p in compat["pages"]):
    at = next((i + 1 for i, p in enumerate(compat["pages"]) if p["name"].startswith("0.4 · ")), len(compat["pages"]))
    compat["pages"].insert(at, {"path": DS_FILE, "title": "Arnod - Design system", "kind": "page",
                                "name": f"{DS_PAGE} · {doc['pages'][DS_PAGE]['nom']}"})
FILE_OF = {p["name"].split(" · ")[0]: p["path"] for p in compat["pages"]}
# Une page est un fichier du dossier ; une capture, un nom simple : jamais de chemin.
for page_file in FILE_OF.values():
    if not re.fullmatch(r"[^/\\]+\.dc\.html", page_file) or ".." in page_file:
        raise SystemExit(f"Nom de page refusé : {page_file}")

MONO = "ui-monospace,'Cascadia Mono',Consolas,'DejaVu Sans Mono',monospace"
LABEL = f"font:600 11px {MONO};color:#9a5a0c;letter-spacing:.04em"
PILL = f"flex:none;font:600 11px {MONO};padding:2px 7px;border-radius:999px;white-space:nowrap"
STYLES = {
    "livré": ("Livré", "background:#e3f1e7;color:#2f7a4b"),
    "en partie": ("En partie", "background:#fbeedd;color:#9a5a0c"),
    "à faire": ("À faire", "background:#efebe4;color:#6b665e"),
    "variante": ("Variante", "background:#fff;color:#6b665e;border:1px dashed #a8a196;padding:1px 6px"),
    "remplacé": ("Remplacé", "background:#f6e3df;color:#a03a2a"),
    "à décider": ("À décider", "background:#e4ecf7;color:#2b5a9a"),
    "référence": ("Référence", "background:#fff;color:#4a463f;border:1px solid #cfc9bf;padding:1px 6px"),
}
GAP = "background:#efe6f6;color:#6b3d8f"
VERSION = "background:#fff;color:#4a463f;border:1px solid #cfc9bf;padding:1px 6px"
ALIGN = {"maquette": "aligner la maquette", "code": "aligner le code", "à décider": "à décider"}
GROUPS = {"0": "Cadre", "1": "Espace de travail", "2": "Démarrer", "3": "Services", "4": "Tests",
          "5": "Exécution", "6": "Recette", "7": "Équipe"}
MONTHS = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."]
_today = datetime.date.today()
TODAY = f"{_today.day} {MONTHS[_today.month - 1]} {_today.year}"


def esc(t) -> str:
    return html.escape(str(t), quote=False)


def pill(state: str) -> str:
    text, style = STYLES[state]
    return f'<span data-arnod="etat" style="{PILL};{style}">{text}</span>'


def vpill(v: str) -> str:
    return f'<span data-arnod="version" style="{PILL};{VERSION}">{esc(v)}</span>'


def gpill(aligner: str) -> str:
    return f'<span data-arnod="ecart" style="{PILL};{GAP}">Écart · {esc(ALIGN[aligner])}</span>'


def fr_date(d: str) -> str:
    parts = str(d).split("-")
    if len(parts) == 3:
        return f"{int(parts[2])} {MONTHS[int(parts[1]) - 1]} {parts[0]}"
    return f"{MONTHS[int(parts[1]) - 1]} {parts[0]}"


def block_end(s: str, start: int, tag: str) -> int:
    depth = 0
    for m in re.finditer(rf"<{tag}\b|</{tag}>", s[start:]):
        depth += 1 if m.group(0) != f"</{tag}>" else -1
        if depth == 0:
            return start + m.end()
    raise ValueError(start)


def versions(page) -> list:
    v = page.get("version", [])
    return v if isinstance(v, list) else [v]


def page_state(page) -> str:
    if page.get("etat"):
        return page["etat"]
    st = [e["etat"] for e in (page.get("ecrans") or {}).values() if e["etat"] in ("livré", "en partie", "à faire")]
    if st and all(s == "livré" for s in st):
        return "livré"
    if any(s in ("livré", "en partie") for s in st):
        return "en partie"
    return "à faire"


def name_of(code: str) -> str:
    return doc["pages"][str(code)]["nom"]


# ---------------------------------------------------------------------------
# Fiche


def cell(label: str, body: str, last: bool = False) -> str:
    border = "" if last else "border-right:1px solid #ebe6de;"
    return (f'<div style="{border}padding:12px 16px;display:flex;flex-direction:column;gap:6px;min-width:0">'
            f'<div style="{LABEL}">{label}</div><div>{body}</div></div>')


def fiche(code: str, page) -> str:
    state = page_state(page)
    etat = (f'<div style="display:flex;flex-wrap:wrap;gap:6px;align-items:center">{pill(state)}'
            f'<span style="font-size:12px;color:#6b665e">au {TODAY}</span></div>')
    if page.get("reste") and state != "livré":
        etat += f'<div style="margin-top:4px;color:#9a5a0c"><b style="font-weight:600">Reste :</b> {esc(page["reste"])}</div>'
    dev = list(page.get("plans") or [])
    if page.get("developpement"):
        dev.append(page["developpement"])
    dev_html = (f'<span style="font:12px/1.5 {MONO};color:#4a463f">{esc(" · ".join(dev) or "—")}</span>'
                f'<div style="margin-top:6px;font-size:12px;color:#6b665e">État tiré de docs/maquette-etat.yml ({esc(SOURCE)}).</div>')
    liees = " · ".join(f"{c} {name_of(c)}" for c in page.get("liees") or []) or "—"
    questions = page.get("questions") or []
    q_html = "<br>".join(esc(x) for x in questions) or "—"
    rows = [
        cell("VERSION", '<div style="display:flex;flex-wrap:wrap;gap:6px">' + "".join(vpill(v) for v in versions(page)) + "</div>"),
        cell("ÉTAT", etat),
        cell("DÉVELOPPEMENT", dev_html),
        cell("PAGES LIÉES", esc(liees)),
        cell("QUESTIONS OUVERTES", q_html, last=True),
    ]
    gaps = [(k, e["ecart"]) for k, e in (page.get("ecrans") or {}).items() if e.get("ecart")]
    gap_html = ""
    if gaps:
        items = "".join(
            f'<li style="display:flex;gap:8px;align-items:baseline"><span style="font:600 12px {MONO};color:#1d1b18;flex:none">{code}-{esc(k)}</span>'
            f'{gpill(g["aligner"])}<span>{esc(g["texte"])}</span></li>' for k, g in gaps)
        gap_html = (f'<div style="border-top:1px solid #ebe6de;padding:12px 16px;display:flex;gap:16px;align-items:baseline">'
                    f'<div style="{LABEL};flex:none;width:70px">ÉCARTS</div>'
                    f'<ul style="margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:5px">{items}</ul></div>')
    journal = list(page.get("journal") or [])
    entries = []
    for j in journal:
        d, _, t = str(j).partition(" : ")
        entries.append((fr_date(d), t))
    sync_entry = (TODAY, f"État et écarts tirés de docs/maquette-etat.yml ({SOURCE}) ; captures de l'application réelle sous les écrans.")
    if not any(t.startswith("État et écarts tirés") for _, t in entries):
        entries.append(sync_entry)
    jl = "".join(
        f'<li style="margin:0;padding:0"><span style="font:12px {MONO};color:#6b665e">{esc(d)}</span> — {esc(t)}</li>'
        for d, t in entries)
    return (
        f'<div aria-label="Fiche de la page {code}" style="max-width:1280px;margin:0 0 28px;border:1px solid #e2ddd4;'
        f'border-radius:10px;background:#fbfaf8;font-size:13px;line-height:1.5;color:#4a463f;overflow:hidden">'
        f'<div style="display:grid;grid-template-columns:150px 230px minmax(0,1.3fr) minmax(0,1fr) minmax(0,1.1fr)">'
        f'{"".join(rows)}</div>{gap_html}'
        f'<div style="border-top:1px solid #ebe6de;padding:12px 16px;display:flex;gap:16px;align-items:baseline">'
        f'<div style="{LABEL};flex:none;width:70px">JOURNAL</div>'
        f'<ul style="margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:3px">{jl}</ul></div></div>'
    )


# ---------------------------------------------------------------------------
# Écrans


PILL_RUN = re.compile(r'(?:<span (?:data-arnod="[a-z]+" )?style="flex:none;font:600 11px [^"]*border-radius:999px[^"]*">[^<]*</span>)+')
EXTRA = re.compile(r'<(div|details) data-arnod="(?:ecart-texte|reel)".*?</\1>', re.S)


def screen(block: str, code: str, letter: str, e: dict) -> str:
    block = EXTRA.sub("", block)
    m = re.search(r'(<a class="dv-oid"[^>]*>)([^<]*)(</a>)', block)
    rest = block[m.end():]
    pm = PILL_RUN.match(rest)
    if pm:
        rest = rest[pm.end():]
    pills = pill(e["etat"]) + (vpill(e["version"]) if e.get("version") else "") + (gpill(e["ecart"]["aligner"]) if e.get("ecart") else "")
    block = block[:m.start()] + m.group(1) + f"{code}-{letter}" + m.group(3) + pills + rest
    # Texte de l'écart sous le libellé, captures réelles sous la carte.
    olabel_end = block.index("</div>", block.index('<div class="dv-olabel"')) + len("</div>")
    if e.get("ecart"):
        g = e["ecart"]
        note = (f'<div data-arnod="ecart-texte" style="max-width:1280px;font-size:12.5px;line-height:1.45;color:#4a3a5c;'
                f'background:#f7f2fb;border:1px solid #e4d6f0;border-radius:6px;padding:6px 10px">'
                f'<b style="font-weight:600;color:#6b3d8f">Écart · {esc(ALIGN[g["aligner"]])} :</b> {esc(g["texte"])}</div>')
        block = block[:olabel_end] + note + block[olabel_end:]
    caps = e.get("captures") or []
    if caps:
        imgs = "".join(
            f'<figure style="margin:0;display:flex;flex-direction:column;gap:4px"><img src="reel/{c}.png" '
            f'alt="Application réelle : {esc(c)}" style="display:block;width:100%;max-width:1280px;border:1px solid #d6d0c6;border-radius:8px">'
            f'<figcaption style="font:12px {MONO};color:#6b665e">e2e/artifacts/{esc(c)}</figcaption></figure>' for c in caps)
        det = (f'<details data-arnod="reel" style="max-width:1280px;font-size:13px;color:#4a463f">'
               f'<summary style="cursor:pointer;font-weight:600;color:#1c726d">Application réelle — '
               f'{len(caps)} capture{"s" if len(caps) > 1 else ""} e2e ({esc(SOURCE)})</summary>'
               f'<div style="display:flex;flex-direction:column;gap:12px;margin-top:10px">{imgs}</div></details>')
        end = block.rindex("</div>")
        block = block[:end] + det + block[end:]
    return block


# ---------------------------------------------------------------------------
# Index (0.1-b) et légende (0.1-c)

TH = f"text-align:left;font:600 11px {MONO};color:#9a5a0c;letter-spacing:.04em;padding:6px 10px;border-bottom:1px solid #e2ddd4"
TD = "padding:6px 10px;border-bottom:1px solid #efebe4;vertical-align:top"
CARD = "width:1280px;box-sizing:border-box;padding:28px 32px 32px;background:#fbfaf8;font-size:13px;line-height:1.5;color:#4a463f"
WORDS = {"livré": ("livré", "livrés"), "en partie": ("en partie", "en partie"), "à faire": ("à faire", "à faire"),
         "à décider": ("à décider", "à décider"), "variante": ("variante", "variantes"), "remplacé": ("remplacé", "remplacés")}


def counts(page) -> str:
    c = {}
    for e in (page.get("ecrans") or {}).values():
        c[e["etat"]] = c.get(e["etat"], 0) + 1
    return " · ".join(f"{c[s]} {WORDS[s][0] if c[s] == 1 else WORDS[s][1]}" for s in WORDS if s in c) or "—"


def option(el_id: str, letter: str, label: str, card: str) -> str:
    return (f'<div class="dv-opt" id="{el_id}"><div class="dv-olabel"><a class="dv-oid" href="#{el_id}">0.1-{letter}</a>'
            f'{pill("référence")}{label}</div>\n{card}</div>')


def index_option() -> str:
    rows, group, total, gaps_total = [], None, {}, 0
    for code, page in doc["pages"].items():
        g = code.split(".")[0]
        if g != group:
            group = g
            rows.append(f'<tr><th scope="rowgroup" colspan="7" style="text-align:left;padding:14px 10px 6px;font-size:13px;'
                        f'font-weight:600;color:#1d1b18;border-bottom:1px solid #e2ddd4">{g} · {GROUPS[g]}</th></tr>')
        state = page_state(page)
        total[state] = total.get(state, 0) + 1
        gaps = [e["ecart"]["aligner"] for e in (page.get("ecrans") or {}).values() if e.get("ecart")]
        gaps_total += len(gaps)
        gap_cell = "".join(gpill(a) for a in gaps) if gaps else "—"
        rows.append(
            f'<tr><td style="{TD};font:600 12px {MONO};color:#1d1b18;white-space:nowrap">{code}</td>'
            f'<th scope="row" style="{TD};text-align:left;font-weight:600;color:#1d1b18">{esc(page["nom"])}</th>'
            f'<td style="{TD};white-space:nowrap">{" ".join(vpill(v) for v in versions(page))}</td>'
            f'<td style="{TD};white-space:nowrap">{pill(state)}</td>'
            f'<td style="{TD};font-size:12px;color:#6b665e">{counts(page)}</td>'
            f'<td style="{TD}"><div style="display:flex;flex-wrap:wrap;gap:4px">{gap_cell}</div></td>'
            f'<td style="{TD};font:12px/1.5 {MONO};color:#4a463f">{esc(" · ".join(page.get("plans") or []) or page.get("developpement") or "—")}</td></tr>')
    head = "".join(f'<th scope="col" style="{TH}">{h}</th>' for h in
                   ["CODE", "PAGE", "VERSION", "ÉTAT", "ÉCRANS", "ÉCARTS", "DÉVELOPPEMENT (docs/)"])
    trans = doc.get("transverses") or []
    trans_html = "".join(
        f'<li style="display:flex;gap:8px;align-items:baseline">{gpill(t["aligner"])}<span>{esc(t["texte"])}</span></li>' for t in trans)
    summary = (f'<p style="margin:0 0 14px;font-size:12px;color:#6b665e">{len(doc["pages"])} pages au {TODAY} : '
               f'{total.get("livré", 0)} livrées, {total.get("en partie", 0)} en partie, {total.get("à faire", 0)} à faire, '
               f'{total.get("référence", 0)} de référence ; {gaps_total} écarts d\'écran et {len(trans)} transverses. '
               f'Source : docs/maquette-etat.yml ({esc(SOURCE)}), vérifié par scripts/check-maquette.mjs.</p>')
    card = (f'<div class="dv-card" style="{CARD}">{summary}'
            f'<table style="width:100%;border-collapse:collapse"><caption style="text-align:left;font-size:15px;font-weight:600;'
            f'color:#1d1b18;padding:0 0 10px">Index des pages</caption><thead><tr>{head}</tr></thead><tbody>{"".join(rows)}</tbody></table>'
            f'<div style="margin-top:22px;display:flex;flex-direction:column;gap:8px"><div style="{LABEL}">ÉCARTS TRANSVERSES</div>'
            f'<ul style="margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:6px">{trans_html}</ul></div></div>')
    return option("p2", "b", "Index des pages — état, écarts avec le code et plans de chaque page, depuis docs/maquette-etat.yml", card)


def legend_option() -> str:
    def item(t, body):
        return f'<div style="display:flex;flex-direction:column;gap:6px"><div style="{LABEL}">{t}</div><div>{body}</div></div>'

    states = "".join(
        f'<li style="display:flex;gap:8px;align-items:baseline">{pill(s)}<span>{esc(d)}</span></li>'
        for s, d in [("livré", "disponible dans l'application, conforme à l'écran"),
                     ("en partie", "commencé ; la fiche dit ce qui reste"),
                     ("à faire", "retenu, pas commencé"),
                     ("variante", "proposition gardée pour mémoire, non retenue"),
                     ("remplacé", "état antérieur, remplacé par un autre écran"),
                     ("à décider", "question ouverte avant de développer")])
    gaps = "".join(f'<li style="display:flex;gap:8px;align-items:baseline">{gpill(a)}<span>{esc(d)}</span></li>'
                   for a, d in [("maquette", "le code a raison : redessiner l'écran"),
                                ("code", "l'écran a raison : tâche d'un plan"),
                                ("à décider", "les deux se défendent : question à trancher")])
    grid = "".join([
        item("CODE DE PAGE", f'<span style="font:12px {MONO}">3.2</span> : le groupe (3 · Services), puis le rang dans le groupe.'),
        item("REPÈRE D'ÉCRAN", f'<span style="font:12px {MONO}">3.2-a2</span> : code de la page et lettre de l\'écran. '
             "Une lettre n'est jamais réutilisée ; un écran déplacé vers une page existante prend la lettre libre suivante."),
        item("ÉTAT D'UN ÉCRAN", f'<ul style="margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:5px">{states}</ul>'),
        item("ÉCART AVEC LE CODE", f'<ul style="margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:5px">{gaps}</ul>'
             '<div style="margin-top:6px">Le texte de l\'écart s\'affiche sous le libellé de l\'écran et dans la fiche.</div>'),
        item("APPLICATION RÉELLE", "Sous un écran livré, « Application réelle » déplie les captures prises par les tests e2e "
             "sur l'application réelle : on compare l'écran dessiné et ce que fait le code."),
        item("SOURCE ET TENUE À JOUR", "Tout vient de docs/maquette-etat.yml, dans le dépôt : une pull request qui livre ou "
             "modifie un écran le met à jour, scripts/check-maquette.mjs le vérifie, puis la maquette est régénérée. "
             "Règles : docs/plan-suivi-maquette.md."),
    ])
    card = f'<div class="dv-card" style="{CARD};display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:24px 28px">{grid}</div>'
    return option("p3", "c", "Repères — codes, états, écarts avec le code, application réelle et source de l'état", card)


# ---------------------------------------------------------------------------
# Page 0.5 : design system, générée depuis docs/design-system/

# En-tête commun des pages de la maquette (cadre des écrans), puis les classes de la page 0.5, qui
# ne prennent que des jetons du design system (ds/arnod/tokens.css).
HELMET = (
    "<helmet data-dc-atomics><meta name=\"design_doc_mode\" content=\"canvas\"><style>"
    "body{margin:0;background:#eceae5;font-family:system-ui,-apple-system,'Segoe UI',Roboto,Ubuntu,Cantarell,'Noto Sans',sans-serif;color:#1d1b18}"
    "a{color:#9a5a0c}a:hover{color:#6e3f06}"
    ".dv-turn{padding:40px 44px 32px;border-bottom:1px solid rgba(0,0,0,.08);scroll-margin-top:16px}"
    ".dv-thd{display:flex;align-items:baseline;gap:10px;margin:0 0 20px}"
    f".dv-tid{{font:600 11px {MONO};padding:3px 7px;background:#1a1a1a;color:#fff;border-radius:4px;text-decoration:none}}"
    ".dv-tname{font:600 13px/1.2 system-ui,-apple-system,'Segoe UI',Roboto,Ubuntu,Cantarell,'Noto Sans',sans-serif;color:#1a1a1a}"
    ".dv-opts{display:flex;flex-wrap:wrap;gap:40px;align-items:flex-start}"
    ".dv-opt{flex:none;display:flex;flex-direction:column;gap:9px;scroll-margin-top:16px}"
    f".dv-oid{{font:600 11px {MONO};padding:3px 7px;background:rgba(0,0,0,.08);color:#1a1a1a;border-radius:5px;text-decoration:none}}"
    ".dv-olabel{display:flex;align-items:baseline;gap:8px;font:400 12px/1.3 system-ui,-apple-system,'Segoe UI',Roboto,Ubuntu,Cantarell,'Noto Sans',sans-serif;color:rgba(0,0,0,.6)}"
    ".dv-card{max-width:100%;background:#fff;border:1px solid rgba(0,0,0,.1);border-radius:10px;box-shadow:0 2px 10px rgba(0,0,0,.07);overflow:hidden}"
    ".dv-opt:target .dv-oid{background:#2a78d6;color:#fff}"
    ".dv-next{margin:22px 0 0;font:12px/1.5 system-ui,-apple-system,'Segoe UI',Roboto,Ubuntu,Cantarell,'Noto Sans',sans-serif;color:rgba(0,0,0,.5)}"
    ".ds-card{width:1280px;box-sizing:border-box}"
    ".ds-cols{display:grid;grid-template-columns:repeat(2,minmax(0,1fr))}"
    ".ds-panel{display:flex;flex-direction:column;gap:18px;padding:24px 28px;background:var(--color-bg);color:var(--color-text);font:13px/1.5 var(--font-sans);min-width:0}"
    ".ds-head{margin:0;font:600 15px/1.3 var(--font-sans);color:var(--color-text)}"
    ".ds-label{margin:0 0 8px;font:600 11px var(--font-mono);letter-spacing:.05em;color:var(--color-muted)}"
    ".ds-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:4px 24px}"
    ".ds-tok{display:grid;grid-template-columns:28px minmax(0,1fr) auto;gap:10px;align-items:center;min-height:24px}"
    ".ds-sw{width:28px;height:18px;box-sizing:border-box;border-radius:var(--radius-sm);border:1px solid var(--color-border)}"
    ".ds-name{font:12px var(--font-mono);color:var(--color-text);overflow-wrap:anywhere}"
    ".ds-val{font:12px var(--font-mono);color:var(--color-muted);white-space:nowrap}"
    ".ds-row{display:flex;flex-wrap:wrap;align-items:center;gap:12px}"
    ".ds-note{margin:0;font-size:12px;color:var(--color-muted)}"
    ".ds-table{width:100%;border-collapse:collapse;font-size:12px}"
    ".ds-table th{text-align:left;font:600 11px var(--font-mono);letter-spacing:.05em;color:var(--color-muted);padding:6px 10px;border-bottom:1px solid var(--color-border)}"
    ".ds-table td{padding:6px 10px;border-bottom:1px solid var(--color-border);vertical-align:middle}"
    "</style></helmet>"
)

COLOR_GROUPS = [
    ("FONDS", ["color-bg", "color-surface", "color-sidebar", "color-chip", "color-accent-soft"]),
    ("BORDURES", ["color-border", "color-border-strong"]),
    ("TEXTES", ["color-text", "color-muted", "color-subtle", "color-link"]),
    ("ÉTATS", ["color-success", "color-success-soft", "color-danger", "color-danger-soft"]),
    ("ACTION ET FOCUS", ["color-accent", "color-accent-hover", "color-accent-text", "color-focus"]),
    ("NOTIFICATIONS", ["color-toast-bg", "color-toast-text", "color-toast-action"]),
    ("VOILE", ["backdrop"]),
]
FAMILY_LABELS = {"calls": "Appels", "data": "Données", "messages": "Messages", "jobs": "Traitements",
                 "interface": "Interface", "code": "Code", "files": "Fichiers"}


def ds_tokens() -> dict:
    return json.loads((DS / "tokens.json").read_text(encoding="utf-8"))


def ds_option(el_id: str, letter: str, label: str, card: str) -> str:
    return (f'<div class="dv-opt" id="{el_id}"><div class="dv-olabel"><a class="dv-oid" href="#{el_id}">{DS_PAGE}-{letter}</a>'
            f'{pill("référence")}{esc(label)}</div>\n{card}</div>')


def ds_colors(tokens: dict) -> str:
    colors = {t["name"]: t for t in tokens["color"]["tokens"]}

    def row(name: str, theme: str) -> str:
        value = colors[name]["value"][theme]
        return (f'<div class="ds-tok"><span class="ds-sw" style="background:var(--{name})"></span>'
                f'<span class="ds-name">{esc(name)}</span><span class="ds-val">{esc(value)}</span></div>')

    def panel(theme: str, title: str) -> str:
        placed = {n for _, names in COLOR_GROUPS for n in names}
        groups = [(label, [n for n in names if n in colors]) for label, names in COLOR_GROUPS]
        ws = [n for n in colors if n.startswith("color-ws-")]
        families = sorted({n.split("-")[2] for n in colors if n.startswith("color-family-")},
                          key=lambda f: list(FAMILY_LABELS).index(f) if f in FAMILY_LABELS else 99)
        others = [n for n in colors if n not in placed and n not in ws and not n.startswith("color-family-")]
        if others:
            groups.append(("AUTRES", others))
        groups.append(("WORKSPACES", ws))
        blocks = "".join(f'<div><p class="ds-label">{label}</p><div class="ds-grid">{"".join(row(n, theme) for n in names)}</div></div>'
                         for label, names in groups if names)
        fam_rows = "".join(
            f'<div class="ds-tok" style="grid-template-columns:110px minmax(0,1fr)"><span class="pill pill--sm pill--{f}">'
            f'{esc(FAMILY_LABELS.get(f, f))}</span><span class="ds-val">'
            + " · ".join(f"{part} {esc(colors[f'color-family-{f}-{key}']['value'][theme])}"
                         for key, part in (("bg", "fond"), ("border", "bord"), ("text", "texte")))
            + "</span></div>" for f in families)
        return (f'<div class="ds-panel" data-theme="{theme}"><p class="ds-head">{title}</p>{blocks}'
                f'<div><p class="ds-label">FAMILLES D\'ÉTAPES</p><div style="display:flex;flex-direction:column;gap:4px">{fam_rows}</div></div></div>')

    n = len(colors)
    card = (f'<div class="dv-card ds-card"><div class="ds-cols">{panel("light", "Thème clair")}{panel("dark", "Thème sombre")}</div></div>')
    return ds_option("ds-couleurs", "a", f"Couleurs — {n} jetons dans les deux thèmes, tirés de src/styles.css ; "
                     "contrastes vérifiés par tests/theme.spec.ts", card)


def ds_type(tokens: dict) -> str:
    rows = []
    for group in tokens["type"]["groups"]:
        for s in group["styles"]:
            rows.append(
                f'<tr><td><span class="ds-name">{esc(s["name"])}</span></td>'
                f'<td class="ds-val">{esc(s["fontSize"])} · {esc(s["fontWeight"])}</td>'
                f'<td><span class="{esc(s["name"])}" style="color:var(--color-text)">Lister les produits de la boutique</span></td>'
                f'<td style="color:var(--color-muted)">{esc(s.get("usage", ""))}</td></tr>')
    fams = "".join(f'<div class="ds-tok" style="grid-template-columns:120px minmax(0,1fr)"><span class="ds-name">font-{esc(k)}</span>'
                   f'<span class="ds-val" style="white-space:normal">{esc(v)}</span></div>'
                   for k, v in tokens["type"]["families"].items())
    card = (f'<div class="dv-card ds-card"><div class="ds-panel" data-theme="light">'
            f'<table class="ds-table"><thead><tr><th scope="col">STYLE</th><th scope="col">TAILLE · GRAISSE</th>'
            f'<th scope="col">EXEMPLE</th><th scope="col">USAGE</th></tr></thead><tbody>{"".join(rows)}</tbody></table>'
            f'<div><p class="ds-label">POLICES</p>{fams}<p class="ds-note" style="margin-top:6px">Police système uniquement : '
            f'la politique de sécurité de l\'application interdit les polices distantes. Hauteur de ligne 1,5.</p></div></div></div>')
    return ds_option("ds-typo", "b", "Typographie — échelle de 11 à 24 px, police système et police à chasse fixe", card)


def ds_space(tokens: dict) -> str:
    spacing = "".join(
        f'<div class="ds-tok" style="grid-template-columns:110px 48px minmax(0,1fr)"><span class="ds-name">{esc(t["name"])}</span>'
        f'<span class="ds-val">{esc(t["value"])}</span><span style="display:block;height:8px;width:{esc(t["value"])};'
        f'background:var(--color-accent);border-radius:var(--radius-sm)"></span></div>' for t in tokens["spacing"]["tokens"])
    radius = "".join(
        f'<div style="display:flex;flex-direction:column;gap:6px;align-items:flex-start"><span style="display:block;width:72px;height:48px;'
        f'background:var(--color-chip);border:1px solid var(--color-border-strong);border-radius:var(--{esc(t["name"])})"></span>'
        f'<span class="ds-name">{esc(t["name"])}</span><span class="ds-val">{esc(t["value"])}</span></div>'
        for t in tokens["radius"]["tokens"])
    shadows = "".join(
        f'<div style="display:flex;flex-direction:column;gap:8px"><span style="display:block;width:160px;height:72px;'
        f'background:var(--color-surface);border:1px solid var(--color-border);border-radius:var(--radius-lg);box-shadow:var(--{esc(t["name"])})"></span>'
        f'<span class="ds-name">{esc(t["name"])}</span><span class="ds-note">{esc(t.get("usage", ""))}</span></div>'
        for t in tokens.get("shadow", {}).get("tokens", []))
    heights = "".join(
        f'<div style="display:flex;flex-direction:column;gap:6px;align-items:flex-start"><span style="display:flex;align-items:center;'
        f'box-sizing:border-box;height:var(--{esc(t["name"])});padding:0 10px;border:1px solid var(--color-border-strong);'
        f'border-radius:var(--radius-md);background:var(--color-surface)">{esc(t["value"])}</span>'
        f'<span class="ds-name">{esc(t["name"])}</span><span class="ds-note" style="max-width:180px">{esc(t.get("usage", ""))}</span></div>'
        for t in tokens.get("hauteur", {}).get("tokens", []))
    focus = ('<div style="display:flex;flex-direction:column;gap:8px"><button type="button" class="button button--secondary" '
             'style="outline:2px solid var(--color-focus);outline-offset:2px">Annuler</button><span class="ds-name">color-focus</span>'
             '<span class="ds-note">Contour de focus : 2 px plein, décalé de 2 px.</span></div>')
    card = (f'<div class="dv-card ds-card"><div class="ds-cols"><div class="ds-panel" data-theme="light">'
            f'<div><p class="ds-label">ESPACEMENTS</p><div style="display:flex;flex-direction:column;gap:2px">{spacing}</div></div></div>'
            f'<div class="ds-panel" data-theme="light"><div><p class="ds-label">HAUTEUR DES CONTRÔLES</p>'
            f'<div class="ds-row" style="gap:24px;align-items:flex-start">{heights}</div></div>'
            f'<div><p class="ds-label">RAYONS</p><div class="ds-row" style="gap:24px">{radius}</div></div>'
            f'<div><p class="ds-label">OMBRES ET FOCUS</p><div class="ds-row" style="gap:32px;align-items:flex-start;padding:8px 0 16px">{shadows}{focus}</div></div>'
            f'</div></div></div>')
    return ds_option("ds-espaces", "c", "Espacements, grille de la barre latérale, hauteurs, rayons, ombres et focus", card)


SECTION_ICON = ('<svg class="section-icon" width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">'
                '<path d="M3.5 1.8h5.7L12.5 5v9.2h-9V1.8Z"></path><path d="M9.2 1.8V5h3.3"></path>'
                '<path d="M7 8.3 5.6 9.9 7 11.5M9 8.3l1.4 1.6L9 11.5"></path></svg>')


def ds_components() -> str:
    def panel(theme: str, title: str) -> str:
        t = theme[0]
        return (
            f'<div class="ds-panel" data-theme="{theme}"><p class="ds-head">{title}</p>'
            f'<div><p class="ds-label">BOUTONS</p><div class="ds-row">'
            f'<button type="button" class="button button--primary">Envoyer</button>'
            f'<button type="button" class="button button--secondary">Annuler</button>'
            f'<button type="button" class="button button--danger">Supprimer</button>'
            f'<button type="button" class="button button--link">+ Ajouter un projet</button>'
            f'<button type="button" class="button button--primary" disabled="">Envoyer</button></div></div>'
            f'<div><p class="ds-label">SAISIE</p><div class="ds-row" style="align-items:center">'
            f'<label class="toggle" for="ds-{t}-t1"><button id="ds-{t}-t1" type="button" role="switch" class="toggle__switch" '
            f'aria-checked="true" aria-labelledby="ds-{t}-l1"><span class="toggle__thumb"></span></button>'
            f'<span id="ds-{t}-l1" class="toggle__label">Suivre les redirections</span></label>'
            f'<label class="toggle" for="ds-{t}-t2"><button id="ds-{t}-t2" type="button" role="switch" class="toggle__switch" '
            f'aria-checked="false" aria-labelledby="ds-{t}-l2"><span class="toggle__thumb"></span></button>'
            f'<span id="ds-{t}-l2" class="toggle__label">Vérifier le TLS</span></label></div>'
            f'<div class="ds-row" style="margin-top:12px"><input type="search" class="search" style="max-width:220px" '
            f'aria-label="Filtrer les requêtes" placeholder="Filtrer les requêtes">'
            f'<select aria-label="Couleur"><option>Gris</option><option>Orange</option></select>'
            f'<label style="display:flex;align-items:center;gap:8px"><input type="checkbox" checked="">Environnement protégé</label>'
            f'<label style="display:flex;align-items:center;gap:8px"><input type="checkbox">Activer</label></div></div>'
            f'<div><p class="ds-label">BARRE LATÉRALE ET STATUTS</p>'
            f'<div style="background:var(--color-sidebar);border:1px solid var(--color-border);border-radius:var(--radius-lg);padding:6px 0;max-width:340px">'
            f'<div class="section-head"><span class="section-head__lead" aria-hidden="true">{SECTION_ICON}</span>'
            f'<h3 class="section-head__title">Contrats de service</h3><span class="section-head__count">2</span></div>'
            f'<div style="display:flex;align-items:center;gap:8px;padding:2px 12px 2px 46px;font-size:13px">'
            f'<span style="width:40px;font:700 11px var(--font-mono);color:var(--color-success)">GET</span><span>Lister les produits</span>'
            f'<span class="status-mark status-mark--ok"><span aria-hidden="true">✓</span><span class="visually-hidden">, réussie</span></span></div>'
            f'<div style="display:flex;align-items:center;gap:8px;padding:2px 12px 2px 46px;font-size:13px">'
            f'<span style="width:40px;font:700 11px var(--font-mono);color:var(--color-link)">POST</span><span>Créer une commande</span>'
            f'<span class="status-mark status-mark--ko"><span aria-hidden="true">✕</span><span class="visually-hidden">, en échec</span></span></div></div>'
            f'<div class="ds-row" style="margin-top:12px;gap:8px">'
            + "".join(f'<span class="pill pill--sm pill--{f}">{esc(lab)}</span>' for f, lab in FAMILY_LABELS.items())
            + '</div><div class="ds-row" style="margin-top:10px;gap:10px">'
            + "".join(f'<span class="dot dot--md" style="background:var(--color-ws-{c})" aria-hidden="true"></span>'
                      for c in ["gray", "orange", "blue", "green", "purple", "red", "yellow", "teal"])
            + '</div></div>'
            f'<div><p class="ds-label">MENU ET NOTIFICATION</p><div class="ds-row" style="align-items:flex-start;gap:20px">'
            f'<div class="menu__popover" style="position:static;min-width:220px"><p class="menu__heading">Environnement</p>'
            f'<ul role="menu" aria-label="Environnement" class="menu__list">'
            f'<li role="menuitemradio" aria-checked="true" class="menu__item menu__item--active"><span class="menu__check" aria-hidden="true">✓</span>'
            f'<span class="menu__text"><span class="menu__label">Local</span></span></li>'
            f'<li role="menuitemradio" aria-checked="false" class="menu__item"><span class="menu__check" aria-hidden="true"></span>'
            f'<span class="menu__text"><span class="menu__label">Recette<span class="menu__tag">protégé</span></span></span></li>'
            f'<li role="separator" class="menu__separator"></li>'
            f'<li role="menuitem" class="menu__item menu__item--danger"><span class="menu__text"><span class="menu__label">Supprimer</span></span>'
            f'<kbd class="menu__shortcut">Suppr</kbd></li></ul></div>'
            f'<div class="toast"><span class="toast__message">Requête mise à la corbeille.</span>'
            f'<button type="button" class="toast__action">Annuler</button></div></div></div>'
            f'<div><p class="ds-label">FENÊTRE MODALE</p><div style="padding:16px;background:var(--backdrop);border-radius:var(--radius-lg)">'
            f'<div class="dialog" role="dialog" aria-labelledby="ds-{t}-dlg" style="width:auto;margin:0 auto;max-width:420px">'
            f'<div class="dialog__panel"><h2 id="ds-{t}-dlg" class="dialog__title">Nouveau projet</h2>'
            f'<p class="dialog__description">Un projet est un dossier, à ranger dans Git.</p>'
            f'<footer class="dialog__footer"><button type="button" class="button button--secondary">Annuler</button>'
            f'<button type="button" class="button button--primary">Créer</button></footer></div></div></div></div>'
            f'</div>')

    card = f'<div class="dv-card ds-card"><div class="ds-cols">{panel("light", "Thème clair")}{panel("dark", "Thème sombre")}</div></div>'
    return ds_option("ds-composants", "d", "Composants — styles du code (components/bundle.css), dans les deux thèmes", card)


def _rgb(hexa: str) -> tuple:
    h = hexa.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def ds_gaps(tokens: dict, page_texts: list) -> str:
    light = {t["name"]: t["value"]["light"].lower() for t in tokens["color"]["tokens"]
             if re.fullmatch(r"#[0-9a-fA-F]{6}", t["value"]["light"])}
    counts = {}
    for text in page_texts:
        for h in re.findall(r"#[0-9a-fA-F]{6}\b", text):
            counts[h.lower()] = counts.get(h.lower(), 0) + 1
    total = sum(counts.values())
    exact = {h for h in counts if h in light.values()}
    exact_uses = sum(counts[h] for h in exact)

    def nearest(h: str):
        name, value = min(light.items(), key=lambda kv: sum((a - b) ** 2 for a, b in zip(_rgb(h), _rgb(kv[1]))))
        return name, value, round(sum((a - b) ** 2 for a, b in zip(_rgb(h), _rgb(value))) ** 0.5)

    rows = []
    for h, n in sorted(counts.items(), key=lambda kv: -kv[1])[:24]:
        name, value, dist = nearest(h)
        verdict = "jeton" if dist == 0 else ("à remplacer" if dist <= 6 else "à revoir")
        rows.append(f'<tr><td><span class="ds-sw" style="display:inline-block;vertical-align:middle;background:{h}"></span> '
                    f'<span class="ds-name">{h}</span></td><td class="ds-val">{n}</td>'
                    f'<td><span class="ds-sw" style="display:inline-block;vertical-align:middle;background:var(--{name})"></span> '
                    f'<span class="ds-name">{esc(name)}</span> <span class="ds-val">{value}</span></td>'
                    f'<td class="ds-val">{dist}</td><td>{verdict}</td></tr>')
    trans = [t for t in doc.get("transverses") or [] if str(t.get("page", "")) == DS_PAGE]
    trans_html = "".join(f'<li style="display:flex;gap:8px;align-items:baseline">{gpill(t["aligner"])}<span>{esc(t["texte"])}</span></li>'
                         for t in trans)
    pct = round(100 * exact_uses / total) if total else 0
    card = (f'<div class="dv-card ds-card"><div class="ds-panel" data-theme="light">'
            f'<div><p class="ds-label">ÉCARTS TRANSVERSES</p><ul style="margin:0;padding:0;list-style:none;display:flex;'
            f'flex-direction:column;gap:6px">{trans_html}</ul></div>'
            f'<div><p class="ds-label">TEINTES ÉCRITES À LA MAIN DANS LA MAQUETTE</p>'
            f'<p class="ds-note" style="margin-bottom:10px">{len(counts)} teintes distinctes, {total} emplois dans les autres pages ; '
            f'{len(exact)} sont des jetons du thème clair ({pct} % des emplois). Les 24 plus employées, avec le jeton le plus proche '
            f'(écart RVB) : jusqu\'à 6, scripts/maquette-couleurs.py remplace la teinte par le jeton ; au-delà, elle est à revoir.</p>'
            f'<table class="ds-table"><thead><tr><th scope="col">TEINTE</th><th scope="col">EMPLOIS</th>'
            f'<th scope="col">JETON LE PLUS PROCHE</th><th scope="col">ÉCART</th><th scope="col">À FAIRE</th></tr></thead>'
            f'<tbody>{"".join(rows)}</tbody></table></div></div></div>')
    return ds_option("ds-ecarts", "e", "Écarts — ce que la maquette et le code doivent reprendre du design system", card)


def design_system_page(page, page_texts: list) -> str:
    tokens = ds_tokens()
    opts = "\n".join([ds_colors(tokens), ds_type(tokens), ds_space(tokens), ds_components(), ds_gaps(tokens, page_texts)])
    name = esc(page["nom"])
    return (
        '<!DOCTYPE html>\n<html lang="fr">\n<head>\n<meta charset="utf-8">\n'
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n<script src="./support.js"></script>\n'
        '<link rel="stylesheet" href="ds/arnod/tokens.css">\n<link rel="stylesheet" href="ds/arnod/components/bundle.css">\n'
        f'<title>Arnod - {name}</title>\n</head>\n<body>\n<x-dc>\n{HELMET}\n'
        f'<section class="dv-turn" id="t1">\n<div class="dv-thd"><a class="dv-tid" href="#t1">{DS_PAGE}</a>'
        f'<span class="dv-tname">{name} — couleurs, typographie, espacements, rayons et composants d\'Arnod, tirés du code</span></div>\n'
        f'{fiche(DS_PAGE, page)}\n<div class="dv-opts">\n{opts}\n</div>\n'
        f'<p class="dv-next">Page générée depuis docs/design-system/ par scripts/maquette-sync.py : ne pas la modifier à la main. '
        f'Le design system « Arnod » (<a href="{DS_URL}">{DS_URL}</a>) donne aussi les guides de chaque composant et les icônes. '
        f'Les écrans de la maquette prennent ses jetons (ds/arnod/tokens.css) au lieu d\'écrire leurs couleurs.</p>\n'
        '</section>\n</x-dc>\n<script type="text/x-dc" data-dc-script>\nclass Component extends DCLogic {\n'
        '  renderVals() {\n    return {};\n  }\n}\n</script>\n</body>\n</html>\n'
    )


# ---------------------------------------------------------------------------


def sync_page(code: str, page, text: str) -> str:
    # Fiche
    i = text.index(f'<div aria-label="Fiche de la page {code}"')
    text = text[:i] + fiche(code, page) + text[block_end(text, i, "div"):]
    # Écrans
    screens = page.get("ecrans") or {}
    seen = set()
    out, pos = [], 0
    for m in re.finditer(r'<div class="dv-opt" id="([^"]+)"', text):
        if m.start() < pos:
            continue
        end = block_end(text, m.start(), "div")
        block = text[m.start():end]
        oid = re.search(r'<a class="dv-oid"[^>]*>([^<]*)</a>', block).group(1)
        letter = oid.split("-", 1)[1] if oid.startswith(code + "-") else oid
        if code == "0.1" and m.group(1) == "p2":
            block = index_option()
        elif code == "0.1" and m.group(1) == "p3":
            block = legend_option()
        else:
            if letter not in screens:
                raise KeyError(f"{code}-{letter} absent de l'état")
            block = screen(block, code, letter, screens[letter])
        seen.add(letter)
        out.append(text[pos:m.start()] + block)
        pos = end
    out.append(text[pos:])
    missing = set(screens) - seen
    if missing:
        raise KeyError(f"{code} : écrans de l'état absents de la page : {missing}")
    return "".join(out)


def main() -> None:
    used = set()
    done = 0
    for code, page in doc["pages"].items():
        if ONLY and code not in ONLY:
            continue
        f = FILE_OF[code]
        if code == DS_PAGE:
            others = [(SRC / p).read_text(encoding="utf-8") for c, p in FILE_OF.items() if c != DS_PAGE and (SRC / p).exists()]
            text = design_system_page(page, others)
        else:
            text = sync_page(code, page, (SRC / f).read_text(encoding="utf-8"))
        (OUT / f).write_text(text, encoding="utf-8")
        done += 1
        for e in (page.get("ecrans") or {}).values():
            for capture in e.get("captures") or []:
                if not re.fullmatch(r"[0-9a-z-]+", capture):
                    raise SystemExit(f"Nom de capture refusé : {capture}")
                used.add(capture)
    (OUT / "reel").mkdir(exist_ok=True)
    for c in sorted(used):
        shutil.copy(CAPS / f"{c}-light.png", OUT / "reel" / f"{c}.png")
    # Design system installé dans la maquette : la copie de ses jetons et des styles des composants.
    (OUT / "ds" / "arnod" / "components").mkdir(parents=True, exist_ok=True)
    shutil.copy(DS / "tokens.css", OUT / "ds" / "arnod" / "tokens.css")
    shutil.copy(DS / "components" / "bundle.css", OUT / "ds" / "arnod" / "components" / "bundle.css")
    (OUT / "compat.json").write_text(json.dumps(compat, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(done, "pages,", len(used), "captures")


main()

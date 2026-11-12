"""Régénère les pages de la maquette depuis docs/maquette-etat.yml (docs/plan-suivi-maquette.md, R8).

Pastilles d'état et d'écart des écrans, texte des écarts, fiche de chaque page, index et repères de
la page 0.1, captures de l'application réelle (« Application réelle ») sous les écrans livrés.

Usage (Python 3 et PyYAML), depuis n'importe où :
    python3 -I scripts/maquette-sync.py "Develop @<commit>"

Chemins fixes, dans le dépôt (aucun chemin pris de la ligne de commande) :
- .maquette/pages/ : les pages actuelles de la maquette (fichiers .dc.html et compat.json), lues sur
  claude.ai ; .maquette/sortie/ reçoit les pages régénérées et reel/<capture>.png, à republier ;
  .maquette/ est ignoré par Git.
- e2e/artifacts/ : captures en thème clair des tests e2e (`npm run test:e2e`).
- L'argument dit d'où vient l'état ; il s'affiche dans les fiches et l'index.

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
SOURCE = sys.argv[1] if len(sys.argv) > 1 else "état du dépôt"
OUT.mkdir(parents=True, exist_ok=True)
doc = yaml.safe_load(ETAT.read_text(encoding="utf-8"))
compat = json.loads((SRC / "compat.json").read_text(encoding="utf-8"))
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
    "à faire": ("À faire", "background:#efebe4;color:#6f695f"),
    "variante": ("Variante", "background:#fff;color:#6f695f;border:1px dashed #a8a196;padding:1px 6px"),
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
            f'<span style="font-size:12px;color:#6f695f">au {TODAY}</span></div>')
    if page.get("reste") and state != "livré":
        etat += f'<div style="margin-top:4px;color:#9a5a0c"><b style="font-weight:600">Reste :</b> {esc(page["reste"])}</div>'
    dev = list(page.get("plans") or [])
    if page.get("developpement"):
        dev.append(page["developpement"])
    dev_html = (f'<span style="font:12px/1.5 {MONO};color:#4a463f">{esc(" · ".join(dev) or "—")}</span>'
                f'<div style="margin-top:6px;font-size:12px;color:#6f695f">État tiré de docs/maquette-etat.yml ({esc(SOURCE)}).</div>')
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
        f'<li style="margin:0;padding:0"><span style="font:12px {MONO};color:#6f695f">{esc(d)}</span> — {esc(t)}</li>'
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
            f'<figcaption style="font:12px {MONO};color:#6f695f">e2e/artifacts/{esc(c)}</figcaption></figure>' for c in caps)
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
            f'<td style="{TD};font-size:12px;color:#6f695f">{counts(page)}</td>'
            f'<td style="{TD}"><div style="display:flex;flex-wrap:wrap;gap:4px">{gap_cell}</div></td>'
            f'<td style="{TD};font:12px/1.5 {MONO};color:#4a463f">{esc(" · ".join(page.get("plans") or []) or page.get("developpement") or "—")}</td></tr>')
    head = "".join(f'<th scope="col" style="{TH}">{h}</th>' for h in
                   ["CODE", "PAGE", "VERSION", "ÉTAT", "ÉCRANS", "ÉCARTS", "DÉVELOPPEMENT (docs/)"])
    trans = doc.get("transverses") or []
    trans_html = "".join(
        f'<li style="display:flex;gap:8px;align-items:baseline">{gpill(t["aligner"])}<span>{esc(t["texte"])}</span></li>' for t in trans)
    summary = (f'<p style="margin:0 0 14px;font-size:12px;color:#6f695f">{len(doc["pages"])} pages au {TODAY} : '
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
    for code, page in doc["pages"].items():
        f = FILE_OF[code]
        text = (SRC / f).read_text(encoding="utf-8")
        text = sync_page(code, page, text)
        (OUT / f).write_text(text, encoding="utf-8")
        for e in (page.get("ecrans") or {}).values():
            for capture in e.get("captures") or []:
                if not re.fullmatch(r"[0-9a-z-]+", capture):
                    raise SystemExit(f"Nom de capture refusé : {capture}")
                used.add(capture)
    (OUT / "reel").mkdir(exist_ok=True)
    for c in sorted(used):
        shutil.copy(CAPS / f"{c}-light.png", OUT / "reel" / f"{c}.png")
    shutil.copy(SRC / "compat.json", OUT / "compat.json")
    print(len(doc["pages"]), "pages,", len(used), "captures")


main()

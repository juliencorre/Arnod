"""Aligne les couleurs des écrans de la maquette sur le design system (docs/plan-qualite-interface.md).

Deux règles, appliquées aux pages lues dans .maquette/pages/ (sauf la page 0.5, générée) :
- le bouton plein rouille (#b4561d, texte blanc) devient le bouton principal du design system
  (color-accent, texte color-accent-text) ; les autres marques rouille prennent color-accent
  (soulignement d'onglet, filet) ou color-link (icône, point, bord) ;
- une teinte à moins de 6 d'écart RVB d'un jeton du thème clair est remplacée par ce jeton :
  l'œil ne voit pas la différence, et la maquette n'écrit plus que des valeurs du design system.

Les teintes plus éloignées restent en place : la page 0.5-e les liste, avec le jeton le plus proche.

Usage (Python 3), depuis n'importe où ; aucun chemin n'est pris de la ligne de commande :
    python3 -I scripts/maquette-couleurs.py

Les pages modifiées sont écrites dans .maquette/sortie/, à republier. Travail idempotent.
"""

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / ".maquette" / "pages"
OUT = ROOT / ".maquette" / "sortie"
DS_FILE = "Arnod - Design system.dc.html"
RUST = "#b4561d"
NEAR = 6

tokens = json.loads((ROOT / "docs" / "design-system" / "tokens.json").read_text(encoding="utf-8"))
LIGHT = {t["name"]: t["value"]["light"].lower() for t in tokens["color"]["tokens"]
         if re.fullmatch(r"#[0-9a-f]{6}", t["value"]["light"].lower())}
ACCENT, ACCENT_TEXT, LINK = LIGHT["color-accent"], LIGHT["color-accent-text"], LIGHT["color-link"]
VALUES = set(LIGHT.values())


def rgb(color: str) -> tuple:
    return tuple(int(color[i:i + 2], 16) for i in (1, 3, 5))


def nearest(color: str):
    value = min(VALUES, key=lambda v: sum((a - b) ** 2 for a, b in zip(rgb(color), rgb(v))))
    return value, sum((a - b) ** 2 for a, b in zip(rgb(color), rgb(value))) ** 0.5


def button(match: re.Match) -> str:
    """Attribut style d'un élément au fond rouille : bouton principal, ou marque d'accent."""
    style = match.group(1)
    if not re.search(r"background(?:-color)?:\s*" + RUST, style, re.I):
        return match.group(0)
    text = re.search(r"(?<![a-z-])color:\s*(#fff(?:fff)?|white)\b", style, re.I)
    if text is None:
        return match.group(0)
    style = re.sub(r"(background(?:-color)?:\s*)" + RUST, r"\g<1>" + ACCENT, style, flags=re.I)
    style = re.sub(r"((?<![a-z-])color:\s*)(#fff(?:fff)?|white)\b", r"\g<1>" + ACCENT_TEXT, style, flags=re.I)
    return f'style="{style}"'


def align(text: str, stats: dict) -> str:
    before = len(re.findall(RUST, text, re.I))
    text = re.sub(r'style="([^"]*)"', button, text)
    stats["boutons"] += before - len(re.findall(RUST, text, re.I))
    # Marques restantes : soulignements et filets en accent, le reste en couleur de lien (3:1 au moins).
    text, n = re.subn(r"((?:border-bottom|box-shadow):[^;\"]*?)" + RUST, r"\g<1>" + ACCENT, text, flags=re.I)
    text, m = re.subn(RUST, LINK, text, flags=re.I)
    stats["marques"] += n + m

    def near(match: re.Match) -> str:
        color = match.group(0).lower()
        if color in VALUES:
            return match.group(0)
        value, distance = nearest(color)
        if distance > NEAR:
            return match.group(0)
        stats["teintes"].add(color)
        stats["emplois"] += 1
        return value

    return re.sub(r"#[0-9a-fA-F]{6}\b", near, text)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    stats = {"boutons": 0, "marques": 0, "teintes": set(), "emplois": 0}
    changed = 0
    for page in sorted(SRC.glob("*.dc.html")):
        if page.name == DS_FILE:
            continue
        text = page.read_text(encoding="utf-8")
        new = align(text, stats)
        if new != text:
            (OUT / page.name).write_text(new, encoding="utf-8")
            changed += 1
    print(f"{changed} pages modifiées : {stats['boutons']} boutons principaux, {stats['marques']} marques d'accent, "
          f"{len(stats['teintes'])} teintes proches remplacées ({stats['emplois']} emplois).")


main()

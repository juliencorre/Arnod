import { describe, expect, it } from "vitest";

import { placeBelow, placeBeside, type Box } from "@/components/ui/menu";

/** Fenêtre de 1280 × 840, menu de 240 × 200 (marge de 8 px au bord). */
const VIEWPORT = { width: 1280, height: 840 };
const MENU = { width: 240, height: 200 };

function box(left: number, top: number, width: number, height: number): Box {
  return { left, top, right: left + width, bottom: top + height };
}

describe("placeBelow", () => {
  it("place le menu sous le déclencheur, sur son bord de début ou de fin", () => {
    const chip = box(62, 115, 70, 20);
    expect(placeBelow(chip, MENU, VIEWPORT, "start")).toEqual({ top: 139, left: 62 });
    expect(placeBelow(chip, MENU, VIEWPORT, "end")).toEqual({ top: 139, left: 8 });
    expect(placeBelow(box(600, 115, 70, 20), MENU, VIEWPORT, "end")).toEqual({
      top: 139,
      left: 430,
    });
  });

  it("ne sort pas de la fenêtre à droite : le menu se décale vers la gauche", () => {
    expect(placeBelow(box(1200, 40, 30, 20), MENU, VIEWPORT, "start")).toEqual({
      top: 64,
      left: 1032,
    });
  });

  it("passe au-dessus du déclencheur s'il manque de place dessous", () => {
    expect(placeBelow(box(100, 760, 70, 20), MENU, VIEWPORT, "start")).toEqual({
      top: 556,
      left: 100,
    });
  });

  it("sans place d'un côté ni de l'autre, prend le plus grand et reste dans la fenêtre", () => {
    const short = { width: 1280, height: 330 };
    // 57 px dessous, 229 dessus : au-dessus, collé à la marge du haut.
    expect(placeBelow(box(100, 241, 70, 20), { width: 240, height: 236 }, short, "start")).toEqual({
      top: 8,
      left: 100,
    });
    // 125 px dessous, 51 dessus : dessous, remonté pour tenir.
    expect(placeBelow(box(100, 63, 70, 20), { width: 240, height: 236 }, short, "start")).toEqual({
      top: 86,
      left: 100,
    });
  });

  it("un menu plus large que la fenêtre commence à la marge", () => {
    expect(
      placeBelow(
        box(100, 40, 30, 20),
        { width: 400, height: 100 },
        { width: 300, height: 840 },
        "end",
      ),
    ).toEqual({ top: 64, left: 8 });
  });
});

describe("placeBeside", () => {
  const item = box(8, 200, 254, 40);

  it("ouvre le sous-menu à droite de l'entrée, première entrée alignée", () => {
    expect(placeBeside(item, MENU, VIEWPORT, "end")).toEqual({ top: 194, left: 268 });
  });

  it("ouvre à gauche quand c'est le côté préféré et qu'il y a la place", () => {
    const right = box(900, 200, 254, 40);
    expect(placeBeside(right, MENU, VIEWPORT, "start")).toEqual({ top: 194, left: 654 });
    // Pas la place à gauche : à droite.
    expect(placeBeside(item, MENU, VIEWPORT, "start")).toEqual({ top: 194, left: 268 });
  });

  it("change de côté au bord droit et remonte au bord du bas", () => {
    const low = box(1000, 780, 254, 40);
    expect(placeBeside(low, MENU, VIEWPORT, "end")).toEqual({ top: 632, left: 754 });
  });
});

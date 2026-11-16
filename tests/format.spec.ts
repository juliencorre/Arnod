import { describe, expect, it } from "vitest";

import { countLabel, formatTime, pathSegments } from "@/lib/format";

describe("formatTime", () => {
  it("formate en heure locale française avec les millisecondes", () => {
    const date = new Date(2026, 10, 6, 9, 5, 7, 42);
    expect(formatTime(date.getTime())).toBe("09:05:07,042");
  });
});

describe("countLabel", () => {
  it("accorde au singulier jusqu'à 1, au pluriel au-delà", () => {
    expect(countLabel(0, "projet", "projets")).toBe("0 projet");
    expect(countLabel(1, "projet", "projets")).toBe("1 projet");
    expect(countLabel(6, "projet", "projets")).toBe("6 projets");
  });
});

describe("pathSegments", () => {
  it("coupe après chaque séparateur, sous Linux comme sous Windows", () => {
    expect(pathSegments("/home/julien/arnod/boutique-tests")).toEqual([
      "/",
      "home/",
      "julien/",
      "arnod/",
      "boutique-tests",
    ]);
    expect(pathSegments("C:\\Users\\julien\\api")).toEqual(["C:\\", "Users\\", "julien\\", "api"]);
    expect(pathSegments("projet")).toEqual(["projet"]);
    expect(pathSegments("")).toEqual([""]);
  });

  it("garde chaque caractère : séparateur final ou doublé, rien n'est perdu", () => {
    expect(pathSegments("/home/julien/")).toEqual(["/", "home/", "julien/"]);
    expect(pathSegments("//serveur\\partage")).toEqual(["/", "/", "serveur\\", "partage"]);
    for (const path of ["/a//b/", "C:\\x\\", "a"]) {
      expect(pathSegments(path).join("")).toBe(path);
    }
  });
});
